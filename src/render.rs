//! Rendering module: transforms scan IR into text tree output.
//!
//! This module renders `TreeNode` structures from the `scan` module into
//! displayable text formats with support for:
//!
//! - **Tree styles**: ASCII (`/A`) or Unicode (default)
//! - **No-indent mode**: whitespace-only indentation (`/NI`)
//! - **Path display**: relative names (default) or full paths (`/FP`)
//! - **Metadata display**: file size (`/S`), human-readable size (`/HR`),
//!   modification date (`/DT`), directory cumulative size (`/DU`)
//! - **Statistics report**: end-of-output statistics (`/RP`)
//! - **Windows banner**: system volume info (default), disabled with `/NB`
//! - **Streaming render**: `StreamRenderer` supports incremental rendering
//!
//! File: src/render.rs
//! Author: WaterRun
//! Date: 2026-01-12

#![forbid(unsafe_code)]

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::config::{CharsetMode, Config, PathMode};
use crate::error::RenderError;
use crate::scan::{EntryKind, EntryMetadata, ScanStats, StreamEntry, TreeNode};

// ============================================================================
// Constants
// ============================================================================

/// File name for the tree++ banner marker file.
const TREEPP_BANNER_FILE: &str = "tree++.txt";

/// Content of the banner marker file explaining directory purpose.
const TREEPP_BANNER_FILE_CONTENT: &str = r#"This directory is automatically created by tree++ to align with the native Windows tree command's banner (boilerplate) output.

You may safely delete this directory. If you do not want tree++ to create it, use the /NB option when running tree++.

GitHub: https://github.com/Water-Run/treepp
"#;

// ============================================================================
// Windows Banner
// ============================================================================

/// Windows tree command banner information.
///
/// Contains boilerplate text extracted from the native Windows `tree` command.
/// Obtained by executing `tree` in a controlled `C:\__tree++__` directory to
/// capture the system-localized banner text.
///
/// # Output Format
///
/// Windows `tree` command produces exactly 4 lines:
///
/// ```text
/// Folder PATH listing for volume OS   <- line 1: volume_line
/// Volume serial number is 2810-11C7   <- line 2: serial_line
/// C:.                                  <- line 3: current directory (ignored)
/// No subfolders exist                  <- line 4: no_subfolder
/// ```
///
/// # Examples
///
/// ```
/// use treepp::render::WinBanner;
///
/// let output = "Folder PATH listing\nSerial 1234\nC:.\nNo subfolders";
/// let banner = WinBanner::parse(output).unwrap();
/// assert_eq!(banner.volume_line, "Folder PATH listing");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinBanner {
    /// Volume information line (e.g., "Folder PATH listing for volume OS").
    pub volume_line: String,
    /// Volume serial number line (e.g., "Volume serial number is 2810-11C7").
    pub serial_line: String,
    /// No subfolder hint (e.g., "No subfolders exist").
    pub no_subfolder: String,
}

impl WinBanner {
    /// Fetches Windows banner information for the specified drive letter.
    ///
    /// Creates a marker directory `X:\__tree++__` (where X is the drive letter),
    /// executes the native `tree` command there, and parses the output.
    ///
    /// # Arguments
    ///
    /// * `drive` - Drive letter (e.g., 'C', 'D')
    ///
    /// # Returns
    ///
    /// The parsed `WinBanner` on success.
    ///
    /// # Errors
    ///
    /// Returns `RenderError::BannerFetchFailed` if:
    /// - The banner directory cannot be created
    /// - The tree command fails to execute
    /// - The tree output cannot be parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::render::WinBanner;
    ///
    /// let banner = WinBanner::fetch_for_drive('C').unwrap();
    /// println!("Volume: {}", banner.volume_line);
    /// ```
    pub fn fetch_for_drive(drive: char) -> Result<Self, RenderError> {
        let drive = drive.to_ascii_uppercase();
        let banner_dir = format!(r"{}:\__tree++__", drive);
        let dir_path = Path::new(&banner_dir);
        let file_path = dir_path.join(TREEPP_BANNER_FILE);

        if !dir_path.exists() {
            fs::create_dir_all(dir_path).map_err(|e| RenderError::BannerFetchFailed {
                reason: format!("Unable to create directory {}: {}", banner_dir, e),
            })?;
        }

        if !file_path.exists() {
            fs::write(&file_path, TREEPP_BANNER_FILE_CONTENT).map_err(|e| {
                RenderError::BannerFetchFailed {
                    reason: format!("Unable to create file {}: {}", file_path.display(), e),
                }
            })?;
        }

        let output = Command::new("cmd")
            .args(["/C", "tree"])
            .current_dir(dir_path)
            .output()
            .map_err(|e| RenderError::BannerFetchFailed {
                reason: format!("Failed to execute tree command: {}", e),
            })?;

        if !output.status.success() {
            return Err(RenderError::BannerFetchFailed {
                reason: format!("tree command returned error code: {:?}", output.status.code()),
            });
        }

        let stdout = Self::decode_system_output(&output.stdout)?;
        Self::parse_tree_output(&stdout)
    }

    /// Parses banner information from a string (for testing).
    ///
    /// # Arguments
    ///
    /// * `output` - Raw tree command output string
    ///
    /// # Returns
    ///
    /// The parsed `WinBanner` on success.
    ///
    /// # Errors
    ///
    /// Returns `RenderError::BannerFetchFailed` if parsing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::render::WinBanner;
    ///
    /// let output = "Line1\nLine2\nC:.\nLine4";
    /// let banner = WinBanner::parse(output).unwrap();
    /// assert_eq!(banner.no_subfolder, "Line4");
    /// ```
    #[cfg(test)]
    pub fn parse(output: &str) -> Result<Self, RenderError> {
        Self::parse_tree_output(output)
    }

    /// Decodes system output handling Windows GBK/CP936 encoding.
    fn decode_system_output(bytes: &[u8]) -> Result<String, RenderError> {
        let (decoded, _, had_errors) = encoding_rs::GBK.decode(bytes);

        if had_errors {
            match std::str::from_utf8(bytes) {
                Ok(s) => Ok(s.to_string()),
                Err(_) => Ok(String::from_utf8_lossy(bytes).into_owned()),
            }
        } else {
            Ok(decoded.into_owned())
        }
    }

    /// Parses the tree command output into banner components.
    ///
    /// The `__tree++__` directory contains only a marker file, so `tree`
    /// output is always exactly 4 lines.
    fn parse_tree_output(output: &str) -> Result<Self, RenderError> {
        let lines: Vec<&str> = output.lines().collect();

        if lines.len() < 4 {
            return Err(RenderError::BannerFetchFailed {
                reason: format!(
                    "tree output has insufficient lines, expected 4, got {}:\n{}",
                    lines.len(),
                    output
                ),
            });
        }

        Ok(Self {
            volume_line: lines[0].trim_start().to_string(),
            serial_line: lines[1].trim_start().to_string(),
            no_subfolder: lines[3].trim_start().to_string(),
        })
    }
}

// ============================================================================
// Tree Characters
// ============================================================================

/// Tree branch connector character set.
///
/// Defines the characters used to draw tree structure connections.
///
/// # Examples
///
/// ```
/// use treepp::render::TreeChars;
/// use treepp::config::CharsetMode;
///
/// let chars = TreeChars::from_charset(CharsetMode::Unicode);
/// assert_eq!(chars.branch, "├─");
///
/// let ascii_chars = TreeChars::from_charset(CharsetMode::Ascii);
/// assert_eq!(ascii_chars.branch, "+---");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TreeChars {
    /// Branch connector (├─ or +---).
    pub branch: &'static str,
    /// Last branch connector (└─ or \---).
    pub last_branch: &'static str,
    /// Vertical continuation line (│   or |   ).
    pub vertical: &'static str,
    /// Space placeholder for last branch children.
    pub space: &'static str,
}

impl TreeChars {
    /// Creates a character set from the specified charset mode.
    ///
    /// # Arguments
    ///
    /// * `charset` - The character set mode to use
    ///
    /// # Returns
    ///
    /// A `TreeChars` instance with appropriate characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::render::TreeChars;
    /// use treepp::config::CharsetMode;
    ///
    /// let unicode = TreeChars::from_charset(CharsetMode::Unicode);
    /// assert_eq!(unicode.last_branch, "└─");
    ///
    /// let ascii = TreeChars::from_charset(CharsetMode::Ascii);
    /// assert_eq!(ascii.last_branch, "\\---");
    /// ```
    #[must_use]
    pub fn from_charset(charset: CharsetMode) -> Self {
        match charset {
            CharsetMode::Unicode => Self {
                branch: "├─",
                last_branch: "└─",
                vertical: "│  ",
                space: "    ",
            },
            CharsetMode::Ascii => Self {
                branch: "+---",
                last_branch: "\\---",
                vertical: "|   ",
                space: "    ",
            },
        }
    }
}

// ============================================================================
// Render Result
// ============================================================================

/// Result of rendering a tree structure.
///
/// Contains the rendered text content along with statistics about
/// the rendered tree.
///
/// # Examples
///
/// ```
/// use treepp::render::RenderResult;
///
/// let result = RenderResult {
///     content: "root\n├─src\n└─lib".to_string(),
///     directory_count: 2,
///     file_count: 0,
/// };
/// assert!(result.content.contains("src"));
/// ```
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// Rendered text content.
    pub content: String,
    /// Number of directories in the tree.
    pub directory_count: usize,
    /// Number of files in the tree.
    pub file_count: usize,
}

// ============================================================================
// Stream Render Configuration
// ============================================================================

/// Configuration for streaming renderer.
///
/// Extracted from `Config` for use by `StreamRenderer`.
///
/// # Examples
///
/// ```
/// use treepp::render::StreamRenderConfig;
/// use treepp::config::Config;
///
/// let config = Config::default();
/// let stream_config = StreamRenderConfig::from_config(&config);
/// assert!(!stream_config.no_indent);
/// ```
#[derive(Debug, Clone)]
pub struct StreamRenderConfig {
    /// Character set mode.
    pub charset: CharsetMode,
    /// Whether to disable tree connectors.
    pub no_indent: bool,
    /// Whether to disable Windows banner.
    pub no_win_banner: bool,
    /// Whether to show statistics report.
    pub show_report: bool,
    /// Whether to show files.
    pub show_files: bool,
    /// Path display mode.
    pub path_mode: PathMode,
    /// Whether to show file sizes.
    pub show_size: bool,
    /// Whether to use human-readable size format.
    pub human_readable: bool,
    /// Whether to show modification dates.
    pub show_date: bool,
}

impl StreamRenderConfig {
    /// Creates a stream render configuration from a full config.
    ///
    /// # Arguments
    ///
    /// * `config` - The full configuration
    ///
    /// # Returns
    ///
    /// A `StreamRenderConfig` with relevant settings extracted.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::render::StreamRenderConfig;
    /// use treepp::config::{Config, CharsetMode};
    ///
    /// let mut config = Config::default();
    /// config.render.charset = CharsetMode::Ascii;
    /// let stream_config = StreamRenderConfig::from_config(&config);
    /// assert_eq!(stream_config.charset, CharsetMode::Ascii);
    /// ```
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            charset: config.render.charset,
            no_indent: config.render.no_indent,
            no_win_banner: config.render.no_win_banner,
            show_report: config.render.show_report,
            show_files: config.scan.show_files,
            path_mode: config.render.path_mode,
            show_size: config.render.show_size,
            human_readable: config.render.human_readable,
            show_date: config.render.show_date,
        }
    }
}

// ============================================================================
// Stream Renderer
// ============================================================================

/// Streaming tree renderer with prefix state management.
///
/// Manages the tree prefix stack for incremental entry-by-entry rendering.
/// Supports proper trailing line insertion for visual alignment.
///
/// # Examples
///
/// ```
/// use treepp::render::{StreamRenderer, StreamRenderConfig};
/// use treepp::config::Config;
///
/// let config = Config::default();
/// let render_config = StreamRenderConfig::from_config(&config);
/// let renderer = StreamRenderer::new(render_config);
/// assert!(renderer.is_at_root_level());
/// ```
#[derive(Debug)]
pub struct StreamRenderer {
    /// Prefix stack: whether each level has more siblings (true = has more).
    prefix_stack: Vec<bool>,
    /// Tree character set.
    chars: TreeChars,
    /// Render configuration.
    config: StreamRenderConfig,
    /// Whether the last rendered entry was a file.
    last_was_file: bool,
    /// Per-level state: (file prefix, whether last rendered was file).
    level_state_stack: Vec<(Option<String>, bool)>,
    /// Whether a trailing line was just emitted (prevents duplicates).
    trailing_line_emitted: bool,
}

impl StreamRenderer {
    /// Creates a new streaming renderer with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Stream render configuration
    ///
    /// # Returns
    ///
    /// A new `StreamRenderer` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::render::{StreamRenderer, StreamRenderConfig};
    /// use treepp::config::Config;
    ///
    /// let config = Config::default();
    /// let render_config = StreamRenderConfig::from_config(&config);
    /// let renderer = StreamRenderer::new(render_config);
    /// assert!(renderer.is_at_root_level());
    /// ```
    #[must_use]
    pub fn new(config: StreamRenderConfig) -> Self {
        let chars = TreeChars::from_charset(config.charset);
        Self {
            prefix_stack: Vec::new(),
            chars,
            config,
            last_was_file: false,
            level_state_stack: Vec::new(),
            trailing_line_emitted: false,
        }
    }

    /// Renders the banner and root path header.
    ///
    /// # Arguments
    ///
    /// * `root_path` - The root directory path
    /// * `path_explicitly_set` - Whether the path was explicitly specified by user
    ///
    /// # Returns
    ///
    /// The rendered header string.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use treepp::render::{StreamRenderer, StreamRenderConfig};
    /// use treepp::config::Config;
    ///
    /// let config = Config::default();
    /// let render_config = StreamRenderConfig::from_config(&config);
    /// let renderer = StreamRenderer::new(render_config);
    /// let header = renderer.render_header(Path::new("C:\\test"), false);
    /// ```
    #[must_use]
    pub fn render_header(&self, root_path: &Path, path_explicitly_set: bool) -> String {
        let mut output = String::new();
        let drive = extract_drive_letter(root_path).ok();

        let banner = if self.config.no_win_banner {
            None
        } else if let Some(d) = drive {
            match WinBanner::fetch_for_drive(d) {
                Ok(b) => Some(b),
                Err(e) => {
                    let _ = writeln!(output, "Warning: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if let Some(b) = &banner {
            output.push_str(&b.volume_line);
            output.push('\n');
            output.push_str(&b.serial_line);
            output.push('\n');
        }

        let root_display = match format_root_path_display(root_path, path_explicitly_set) {
            Ok(s) => s,
            Err(e) => {
                let _ = writeln!(output, "Warning: {}", e);
                root_path.to_string_lossy().to_uppercase()
            }
        };
        output.push_str(&root_display);
        output.push('\n');

        output
    }

    /// Renders a single entry as one line of text.
    ///
    /// # Arguments
    ///
    /// * `entry` - The stream entry to render
    ///
    /// # Returns
    ///
    /// The rendered line string.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::render::{StreamRenderer, StreamRenderConfig};
    /// use treepp::scan::{StreamEntry, EntryKind, EntryMetadata};
    /// use treepp::config::Config;
    ///
    /// let config = Config::default();
    /// let render_config = StreamRenderConfig::from_config(&config);
    /// let mut renderer = StreamRenderer::new(render_config);
    ///
    /// let entry = StreamEntry {
    ///     path: PathBuf::from("test"),
    ///     name: "test".to_string(),
    ///     kind: EntryKind::Directory,
    ///     metadata: EntryMetadata::default(),
    ///     depth: 0,
    ///     is_last: true,
    ///     is_file: false,
    ///     has_more_dirs: false,
    /// };
    /// let line = renderer.render_entry(&entry);
    /// assert!(line.contains("test"));
    /// ```
    #[must_use]
    pub fn render_entry(&mut self, entry: &StreamEntry) -> String {
        if entry.is_file {
            let file_prefix = self.build_file_prefix(entry.has_more_dirs);
            if let Some(last) = self.level_state_stack.last_mut() {
                last.0 = Some(file_prefix);
                last.1 = true;
            }
        } else if let Some(last) = self.level_state_stack.last_mut() {
            last.1 = false;
        }

        if self.config.no_indent {
            return self.render_entry_no_indent(entry);
        }

        let mut output = String::new();

        if self.config.show_files && self.last_was_file && !entry.is_file {
            let prefix = self.build_prefix();
            let _ = writeln!(output, "{}{}", prefix, self.chars.vertical);
        }

        if entry.is_file {
            output.push_str(&self.render_file_entry(entry));
        } else {
            output.push_str(&self.render_dir_entry(entry));
        }

        self.last_was_file = entry.is_file;
        output
    }

    /// Enters a subdirectory level.
    ///
    /// # Arguments
    ///
    /// * `has_more_siblings` - Whether there are more sibling directories
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::render::{StreamRenderer, StreamRenderConfig};
    /// use treepp::config::Config;
    ///
    /// let config = Config::default();
    /// let render_config = StreamRenderConfig::from_config(&config);
    /// let mut renderer = StreamRenderer::new(render_config);
    ///
    /// renderer.push_level(true);
    /// assert!(!renderer.is_at_root_level());
    /// ```
    pub fn push_level(&mut self, has_more_siblings: bool) {
        self.prefix_stack.push(has_more_siblings);
        self.level_state_stack.push((None, false));
        self.last_was_file = false;
        self.trailing_line_emitted = false;
    }

    /// Exits a subdirectory level and restores prefix stack.
    ///
    /// # Returns
    ///
    /// Optional trailing line if the directory's last rendered entry was a file.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::render::{StreamRenderer, StreamRenderConfig};
    /// use treepp::config::Config;
    ///
    /// let config = Config::default();
    /// let render_config = StreamRenderConfig::from_config(&config);
    /// let mut renderer = StreamRenderer::new(render_config);
    ///
    /// renderer.push_level(true);
    /// let trailing = renderer.pop_level();
    /// assert!(renderer.is_at_root_level());
    /// ```
    #[must_use]
    pub fn pop_level(&mut self) -> Option<String> {
        let level_state = self.level_state_stack.pop();
        self.last_was_file = false;

        if self.trailing_line_emitted {
            let _ = self.prefix_stack.pop();
            return None;
        }

        let result = if let Some((file_prefix, last_was_file)) = level_state {
            if self.config.show_files && file_prefix.is_some() && last_was_file && !self.config.no_indent {
                self.trailing_line_emitted = true;
                file_prefix
            } else {
                None
            }
        } else {
            None
        };

        let _ = self.prefix_stack.pop();
        result
    }

    /// Renders the statistics report.
    ///
    /// # Arguments
    ///
    /// * `directory_count` - Number of directories
    /// * `file_count` - Number of files
    /// * `duration` - Scan duration
    ///
    /// # Returns
    ///
    /// The rendered report string.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use treepp::render::{StreamRenderer, StreamRenderConfig};
    /// use treepp::config::Config;
    ///
    /// let mut config = Config::default();
    /// config.render.show_report = true;
    /// config.scan.show_files = true;
    /// let render_config = StreamRenderConfig::from_config(&config);
    /// let renderer = StreamRenderer::new(render_config);
    ///
    /// let report = renderer.render_report(5, 10, Duration::from_millis(100));
    /// assert!(report.contains("5 directory"));
    /// ```
    #[must_use]
    pub fn render_report(
        &self,
        directory_count: usize,
        file_count: usize,
        duration: Duration,
    ) -> String {
        let mut output = String::new();

        if self.config.show_report {
            let time_str = format!(" in {:.3}s", duration.as_secs_f64());

            if self.config.show_files {
                let _ = writeln!(
                    output,
                    "{} directory, {} files{}",
                    directory_count, file_count, time_str
                );
            } else {
                let _ = writeln!(output, "{} directory{}", directory_count, time_str);
            }
        }

        output
    }

    /// Checks if currently at root level (no subdirectories entered).
    ///
    /// # Returns
    ///
    /// `true` if at root level, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::render::{StreamRenderer, StreamRenderConfig};
    /// use treepp::config::Config;
    ///
    /// let config = Config::default();
    /// let render_config = StreamRenderConfig::from_config(&config);
    /// let mut renderer = StreamRenderer::new(render_config);
    ///
    /// assert!(renderer.is_at_root_level());
    /// renderer.push_level(false);
    /// assert!(!renderer.is_at_root_level());
    /// ```
    #[must_use]
    pub fn is_at_root_level(&self) -> bool {
        self.prefix_stack.is_empty()
    }

    /// Checks if root level has any rendered content.
    ///
    /// # Returns
    ///
    /// `true` if content has been rendered at root level.
    #[must_use]
    pub fn root_has_content(&self) -> bool {
        self.last_was_file || !self.prefix_stack.is_empty()
    }

    /// Renders a file entry with indentation (no branch connectors).
    fn render_file_entry(&self, entry: &StreamEntry) -> String {
        let mut line = String::new();
        let prefix = self.build_prefix();
        line.push_str(&prefix);

        if entry.has_more_dirs {
            line.push_str(self.chars.vertical);
        } else {
            line.push_str(self.chars.space);
        }

        line.push_str(&self.format_name(&entry.name, &entry.path));
        line.push_str(&self.format_meta(&entry.metadata, entry.kind));
        line
    }

    /// Renders a directory entry with branch connectors.
    fn render_dir_entry(&self, entry: &StreamEntry) -> String {
        let mut line = String::new();
        let prefix = self.build_prefix();
        line.push_str(&prefix);

        let connector = if entry.is_last {
            self.chars.last_branch
        } else {
            self.chars.branch
        };
        line.push_str(connector);

        line.push_str(&self.format_name(&entry.name, &entry.path));
        line.push_str(&self.format_meta(&entry.metadata, entry.kind));
        line
    }

    /// Renders an entry without tree connectors (indent-only mode).
    fn render_entry_no_indent(&mut self, entry: &StreamEntry) -> String {
        let mut line = String::new();
        let indent = "  ".repeat(entry.depth);
        line.push_str(&indent);
        line.push_str(&self.format_name(&entry.name, &entry.path));
        line.push_str(&self.format_meta(&entry.metadata, entry.kind));
        self.last_was_file = entry.is_file;
        line
    }

    /// Builds the complete file prefix for trailing line alignment.
    fn build_file_prefix(&self, has_more_dirs: bool) -> String {
        let mut prefix = self.build_prefix();
        if has_more_dirs {
            prefix.push_str(self.chars.vertical);
        } else {
            prefix.push_str(self.chars.space);
        }
        prefix
    }

    /// Builds the current prefix string from the prefix stack.
    fn build_prefix(&self) -> String {
        let mut prefix = String::new();
        for &has_more in &self.prefix_stack {
            if has_more {
                prefix.push_str(self.chars.vertical);
            } else {
                prefix.push_str(self.chars.space);
            }
        }
        prefix
    }

    /// Formats entry name based on path mode.
    fn format_name(&self, name: &str, path: &Path) -> String {
        match self.config.path_mode {
            PathMode::Full => path.to_string_lossy().into_owned(),
            PathMode::Relative => name.to_string(),
        }
    }

    /// Formats entry metadata (size, date).
    fn format_meta(&self, metadata: &EntryMetadata, kind: EntryKind) -> String {
        let mut parts = Vec::new();

        if self.config.show_size && kind == EntryKind::File {
            let size_str = if self.config.human_readable {
                format_size_human(metadata.size)
            } else {
                metadata.size.to_string()
            };
            parts.push(size_str);
        }

        if self.config.show_date {
            if let Some(ref modified) = metadata.modified {
                parts.push(format_datetime(modified));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("        {}", parts.join("  "))
        }
    }
}

// ============================================================================
// Batch Render State
// ============================================================================

/// Batch rendering state that mirrors `StreamRenderer` trailing line logic.
///
/// Maintains per-level state stack to track file prefixes and determine
/// when trailing lines should be emitted after directory traversal.
#[derive(Debug, Default)]
struct BatchRenderState {
    /// Per-level state: (file prefix, whether last rendered was file).
    level_state_stack: Vec<(Option<String>, bool)>,
}

impl BatchRenderState {
    /// Creates a new batch render state.
    #[must_use]
    fn new() -> Self {
        Self {
            level_state_stack: Vec::new(),
        }
    }

    /// Enters a new directory level.
    fn push_level(&mut self) {
        self.level_state_stack.push((None, false));
    }

    /// Exits a directory level, returning trailing line if applicable.
    #[must_use]
    fn pop_level(&mut self) -> Option<String> {
        if let Some((file_prefix, last_was_file)) = self.level_state_stack.pop() {
            if last_was_file {
                return file_prefix;
            }
        }
        None
    }

    /// Records a file render with its prefix.
    fn record_file(&mut self, prefix: String) {
        if let Some(last) = self.level_state_stack.last_mut() {
            last.0 = Some(prefix);
            last.1 = true;
        }
    }

    /// Records a directory render.
    fn record_directory(&mut self) {
        if let Some(last) = self.level_state_stack.last_mut() {
            last.1 = false;
        }
    }
}

// ============================================================================
// Public Formatting Functions
// ============================================================================

/// Formats a file size into human-readable form.
///
/// Converts byte sizes to KB, MB, GB, or TB with one decimal place.
///
/// # Arguments
///
/// * `size` - Size in bytes
///
/// # Returns
///
/// Formatted size string with unit suffix.
///
/// # Examples
///
/// ```
/// use treepp::render::format_size_human;
///
/// assert_eq!(format_size_human(0), "0 B");
/// assert_eq!(format_size_human(1024), "1.0 KB");
/// assert_eq!(format_size_human(1048576), "1.0 MB");
/// assert_eq!(format_size_human(1073741824), "1.0 GB");
/// ```
#[must_use]
pub fn format_size_human(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if size >= TB {
        format!("{:.1} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{size} B")
    }
}

/// Formats a `SystemTime` as a local timezone datetime string.
///
/// Converts UTC time to local timezone and formats as "YYYY-MM-DD HH:MM:SS".
///
/// # Arguments
///
/// * `time` - The system time to format
///
/// # Returns
///
/// Formatted datetime string in local timezone.
///
/// # Examples
///
/// ```
/// use std::time::SystemTime;
/// use treepp::render::format_datetime;
///
/// let now = SystemTime::now();
/// let formatted = format_datetime(&now);
/// assert_eq!(formatted.len(), 19);
/// assert!(formatted.contains("-"));
/// assert!(formatted.contains(":"));
/// ```
#[must_use]
pub fn format_datetime(time: &SystemTime) -> String {
    use chrono::{DateTime, Local};
    let datetime: DateTime<Local> = (*time).into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Formats root path display to match Windows tree command style.
///
/// When path is not explicitly specified, displays as `D:.` format.
/// When explicitly specified, displays full uppercase path.
///
/// # Arguments
///
/// * `root_path` - The root path to format
/// * `path_explicitly_set` - Whether path was explicitly specified by user
///
/// # Returns
///
/// Formatted path string.
///
/// # Errors
///
/// Returns `RenderError::InvalidPath` if drive letter cannot be extracted
/// when path is not explicitly set.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use treepp::render::format_root_path_display;
///
/// let explicit = format_root_path_display(Path::new(r"D:\Users"), true).unwrap();
/// assert_eq!(explicit, r"D:\USERS");
///
/// let implicit = format_root_path_display(Path::new(r"C:\Test"), false).unwrap();
/// assert_eq!(implicit, "C:.");
/// ```
pub fn format_root_path_display(
    root_path: &Path,
    path_explicitly_set: bool,
) -> Result<String, RenderError> {
    if path_explicitly_set {
        Ok(root_path.to_string_lossy().to_uppercase())
    } else {
        let drive = extract_drive_letter(root_path)?;
        Ok(format!("{}:.", drive))
    }
}

// ============================================================================
// Main Render Function
// ============================================================================

/// Renders a complete tree structure to text.
///
/// Produces output matching Windows `tree` command format with optional
/// enhancements like file sizes, dates, and statistics.
///
/// # Arguments
///
/// * `stats` - Scan statistics containing the tree and timing info
/// * `config` - Render configuration
///
/// # Returns
///
/// A `RenderResult` containing the rendered text and statistics.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use std::time::Duration;
/// use treepp::render::{render, RenderResult};
/// use treepp::scan::{TreeNode, ScanStats, EntryKind, EntryMetadata};
/// use treepp::config::Config;
///
/// let root = TreeNode::new(
///     PathBuf::from("test"),
///     EntryKind::Directory,
///     EntryMetadata::default(),
/// );
/// let stats = ScanStats {
///     tree: root,
///     duration: Duration::from_millis(100),
///     directory_count: 0,
///     file_count: 0,
/// };
/// let mut config = Config::with_root(PathBuf::from("test"));
/// config.render.no_win_banner = true;
///
/// let result = render(&stats, &config);
/// assert!(result.content.len() > 0);
/// ```
#[must_use]
pub fn render(stats: &ScanStats, config: &Config) -> RenderResult {
    let mut output = String::new();
    let chars = TreeChars::from_charset(config.render.charset);
    let drive = extract_drive_letter(&config.root_path).ok();

    let banner = if config.render.no_win_banner {
        None
    } else if let Some(d) = drive {
        match WinBanner::fetch_for_drive(d) {
            Ok(b) => Some(b),
            Err(e) => {
                let _ = writeln!(output, "Warning: {}", e);
                None
            }
        }
    } else {
        None
    };

    if let Some(b) = &banner {
        output.push_str(&b.volume_line);
        output.push('\n');
        output.push_str(&b.serial_line);
        output.push('\n');
    }

    let root_display = match format_root_path_display(&config.root_path, config.path_explicitly_set)
    {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(output, "Warning: {}", e);
            config.root_path.to_string_lossy().to_uppercase()
        }
    };
    output.push_str(&root_display);
    output.push('\n');

    if config.render.no_indent {
        render_children_no_indent(&mut output, &stats.tree, config, 1);
    } else {
        let mut state = BatchRenderState::new();
        render_children(&mut output, &stats.tree, &chars, config, "", 1, &mut state);
    }

    if !tree_has_subdirectories(&stats.tree) {
        let has_files = stats
            .tree
            .children
            .iter()
            .any(|c| c.kind == EntryKind::File);
        if has_files && config.scan.show_files {
            let _ = writeln!(output, "{}", chars.space);
        }

        if let Some(b) = &banner {
            if !b.no_subfolder.is_empty() {
                output.push_str(&b.no_subfolder);
                output.push('\n');
            }
        }
        output.push('\n');
    }

    if config.render.show_report {
        let time_str = format!(" in {:.3}s", stats.duration.as_secs_f64());

        if config.scan.show_files {
            let _ = writeln!(
                output,
                "{} directory, {} files{}",
                stats.directory_count, stats.file_count, time_str
            );
        } else {
            let _ = writeln!(output, "{} directory{}", stats.directory_count, time_str);
        }
    }

    let output = remove_trailing_pipe_only_line(output);

    RenderResult {
        content: output,
        directory_count: stats.directory_count,
        file_count: stats.file_count,
    }
}

/// Renders only the tree structure without banner or statistics.
///
/// # Arguments
///
/// * `node` - The root tree node
/// * `config` - Render configuration
///
/// # Returns
///
/// The rendered tree string.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::render::render_tree_only;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
/// use treepp::config::Config;
///
/// let root = TreeNode::new(
///     PathBuf::from("project"),
///     EntryKind::Directory,
///     EntryMetadata::default(),
/// );
/// let config = Config::with_root(PathBuf::from("project"));
///
/// let output = render_tree_only(&root, &config);
/// assert!(output.contains("project"));
/// ```
pub fn render_tree_only(node: &TreeNode, config: &Config) -> String {
    let mut output = String::new();
    let chars = TreeChars::from_charset(config.render.charset);

    let root_name = format_entry_name(node, config);
    let root_meta = format_entry_meta(node, config);
    let _ = writeln!(output, "{root_name}{root_meta}");

    if config.render.no_indent {
        render_children_no_indent(&mut output, node, config, 1);
    } else {
        let mut state = BatchRenderState::new();
        render_children(&mut output, node, &chars, config, "", 1, &mut state);
    }

    output
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

/// Checks if depth is within the optional limit.
#[inline]
fn depth_within_limit(depth: usize, max_depth: Option<usize>) -> bool {
    max_depth.map_or(true, |m| depth <= m)
}

/// Checks if recursion is allowed at the given depth.
#[inline]
fn can_recurse(depth: usize, max_depth: Option<usize>) -> bool {
    max_depth.map_or(true, |m| depth < m)
}

/// Extracts the drive letter from a canonicalized path.
fn extract_drive_letter(root_path: &Path) -> Result<char, RenderError> {
    use std::path::Component;

    if let Some(Component::Prefix(prefix)) = root_path.components().next() {
        let prefix_str = prefix.as_os_str().to_string_lossy();
        let chars: Vec<char> = prefix_str.chars().collect();

        if chars.len() >= 2 && chars[1] == ':' {
            return Ok(chars[0].to_ascii_uppercase());
        }

        if prefix_str.starts_with(r"\\?\") && chars.len() >= 6 && chars[5] == ':' {
            return Ok(chars[4].to_ascii_uppercase());
        }
    }

    Err(RenderError::InvalidPath {
        path: root_path.to_path_buf(),
        reason: "Unable to extract drive letter".to_string(),
    })
}

/// Checks if tree has any subdirectories (not counting root itself).
#[must_use]
fn tree_has_subdirectories(node: &TreeNode) -> bool {
    node.children
        .iter()
        .any(|child| child.kind == EntryKind::Directory)
}

/// Formats entry name based on path mode.
fn format_entry_name(node: &TreeNode, config: &Config) -> String {
    match config.render.path_mode {
        PathMode::Full => node.path.to_string_lossy().into_owned(),
        PathMode::Relative => node.name.clone(),
    }
}

/// Formats entry metadata (size, date, disk usage).
fn format_entry_meta(node: &TreeNode, config: &Config) -> String {
    let mut parts = Vec::new();

    if config.render.show_size && node.kind == EntryKind::File {
        let size = node.metadata.size;
        let size_str = if config.render.human_readable {
            format_size_human(size)
        } else {
            size.to_string()
        };
        parts.push(size_str);
    }

    if config.render.show_disk_usage
        && node.kind == EntryKind::Directory
        && let Some(usage) = node.disk_usage
    {
        let usage_str = if config.render.human_readable {
            format_size_human(usage)
        } else {
            usage.to_string()
        };
        parts.push(usage_str);
    }

    if config.render.show_date
        && let Some(ref modified) = node.metadata.modified
    {
        parts.push(format_datetime(modified));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("        {}", parts.join("  "))
    }
}

/// Renders children with tree connectors.
fn render_children(
    output: &mut String,
    node: &TreeNode,
    chars: &TreeChars,
    config: &Config,
    prefix: &str,
    depth: usize,
    state: &mut BatchRenderState,
) {
    if !depth_within_limit(depth, config.scan.max_depth) {
        return;
    }

    let (files, dirs): (Vec<_>, Vec<_>) = get_filtered_children(node, config)
        .into_iter()
        .partition(|c| c.kind == EntryKind::File);

    let has_dirs = !dirs.is_empty();

    if config.scan.show_files {
        let file_prefix = if has_dirs {
            format!("{}{}", prefix, chars.vertical)
        } else {
            format!("{}{}", prefix, chars.space)
        };

        for file in &files {
            if !depth_within_limit(depth, config.scan.max_depth) {
                continue;
            }

            let name = format_entry_name(file, config);
            let meta = format_entry_meta(file, config);
            let _ = writeln!(output, "{}{}{}", file_prefix, name, meta);

            state.record_file(file_prefix.clone());
        }

        if !files.is_empty() && has_dirs {
            let _ = writeln!(output, "{}", file_prefix);
        }
    }

    let dir_count = dirs.len();
    for (i, dir) in dirs.iter().enumerate() {
        if !depth_within_limit(depth, config.scan.max_depth) {
            continue;
        }

        let is_last = i == dir_count - 1;
        let connector = if is_last {
            chars.last_branch
        } else {
            chars.branch
        };

        let name = format_entry_name(dir, config);
        let meta = format_entry_meta(dir, config);
        let _ = writeln!(output, "{}{}{}{}", prefix, connector, name, meta);

        state.record_directory();

        if !dir.children.is_empty() && can_recurse(depth, config.scan.max_depth) {
            let new_prefix = if is_last {
                format!("{}{}", prefix, chars.space)
            } else {
                format!("{}{}", prefix, chars.vertical)
            };

            state.push_level();
            render_children(output, dir, chars, config, &new_prefix, depth + 1, state);

            if let Some(trailing) = state.pop_level() {
                if !is_last && config.scan.show_files {
                    let _ = writeln!(output, "{}", trailing);
                }
            }
        }
    }
}

/// Renders children without tree connectors (indent-only mode).
fn render_children_no_indent(output: &mut String, node: &TreeNode, config: &Config, depth: usize) {
    if !depth_within_limit(depth, config.scan.max_depth) {
        return;
    }

    let (files, dirs): (Vec<_>, Vec<_>) = get_filtered_children(node, config)
        .into_iter()
        .partition(|c| c.kind == EntryKind::File);

    let indent = "  ".repeat(depth);

    for file in &files {
        if !depth_within_limit(depth, config.scan.max_depth) {
            continue;
        }
        let name = format_entry_name(file, config);
        let meta = format_entry_meta(file, config);
        let _ = writeln!(output, "{}{}{}", indent, name, meta);
    }

    for dir in &dirs {
        if !depth_within_limit(depth, config.scan.max_depth) {
            continue;
        }
        let name = format_entry_name(dir, config);
        let meta = format_entry_meta(dir, config);
        let _ = writeln!(output, "{}{}{}", indent, name, meta);

        if !dir.children.is_empty() && can_recurse(depth, config.scan.max_depth) {
            render_children_no_indent(output, dir, config, depth + 1);
        }
    }
}

/// Gets filtered children based on configuration.
fn get_filtered_children<'a>(node: &'a TreeNode, config: &Config) -> Vec<&'a TreeNode> {
    node.children
        .iter()
        .filter(|c| config.scan.show_files || c.kind == EntryKind::Directory)
        .filter(|c| {
            !config.matching.prune_empty || c.kind != EntryKind::Directory || !c.is_empty_dir()
        })
        .collect()
}

/// Removes trailing line containing only pipe characters and whitespace.
fn remove_trailing_pipe_only_line(mut output: String) -> String {
    let trimmed = output.trim_end_matches('\n');
    if let Some(last_newline_pos) = trimmed.rfind('\n') {
        let last_line = &trimmed[last_newline_pos + 1..];

        let has_pipe = last_line.chars().any(|c| c == '|' || c == '│');
        let only_pipes_and_whitespace = !last_line.is_empty()
            && last_line
            .chars()
            .all(|c| c == '|' || c == '│' || c.is_whitespace());

        if has_pipe && only_pipes_and_whitespace {
            output.truncate(last_newline_pos + 1);
        }
    }
    output
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::EntryMetadata;
    use std::path::PathBuf;

    // ------------------------------------------------------------------------
    // Test Helpers
    // ------------------------------------------------------------------------

    fn create_test_tree() -> TreeNode {
        let mut root = TreeNode::new(
            PathBuf::from("test_root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut src = TreeNode::new(
            PathBuf::from("test_root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        src.children.push(TreeNode::new(
            PathBuf::from("test_root/src/main.rs"),
            EntryKind::File,
            EntryMetadata {
                size: 1024,
                ..Default::default()
            },
        ));

        src.children.push(TreeNode::new(
            PathBuf::from("test_root/src/lib.rs"),
            EntryKind::File,
            EntryMetadata {
                size: 2048,
                ..Default::default()
            },
        ));

        root.children.push(src);

        root.children.push(TreeNode::new(
            PathBuf::from("test_root/Cargo.toml"),
            EntryKind::File,
            EntryMetadata {
                size: 512,
                ..Default::default()
            },
        ));

        root
    }

    fn create_test_stats(tree: TreeNode) -> ScanStats {
        let directory_count = tree.count_directories();
        let file_count = tree.count_files();

        ScanStats {
            tree,
            duration: Duration::from_millis(100),
            directory_count,
            file_count,
        }
    }

    // ------------------------------------------------------------------------
    // WinBanner Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_valid_4_line_banner() {
        let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n没有子文件夹 ";
        let banner = WinBanner::parse(output).expect("should parse successfully");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹 ");
    }

    #[test]
    fn should_parse_banner_with_trailing_empty_lines() {
        let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n没有子文件夹 \n\n";
        let banner = WinBanner::parse(output).expect("should parse successfully");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.no_subfolder, "没有子文件夹 ");
    }

    #[test]
    fn should_preserve_trailing_whitespace_in_banner() {
        let output =
            "卷 系统 的文件夹 PATH 列表  \n  卷序列号为 2810-11C7\nC:.\n没有子文件夹  \n";
        let banner = WinBanner::parse(output).expect("should parse successfully");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表  ");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹  ");
    }

    #[test]
    fn should_parse_english_locale_banner() {
        let output = "Folder PATH listing for volume OS\nVolume serial number is ABCD-1234\nC:.\nNo subfolders exist ";
        let banner = WinBanner::parse(output).expect("should parse successfully");

        assert_eq!(banner.volume_line, "Folder PATH listing for volume OS");
        assert_eq!(banner.serial_line, "Volume serial number is ABCD-1234");
        assert_eq!(banner.no_subfolder, "No subfolders exist ");
    }

    #[test]
    fn should_fail_parsing_with_insufficient_lines() {
        assert!(WinBanner::parse("only one line").is_err());
        assert!(WinBanner::parse("line1\nline2").is_err());
        assert!(WinBanner::parse("line1\nline2\nline3").is_err());
    }

    #[test]
    fn should_parse_exactly_4_lines() {
        let output = "line1\nline2\nline3\nline4";
        let banner = WinBanner::parse(output).expect("should parse 4 lines");

        assert_eq!(banner.volume_line, "line1");
        assert_eq!(banner.serial_line, "line2");
        assert_eq!(banner.no_subfolder, "line4");
    }

    #[test]
    fn should_parse_more_than_4_lines() {
        let output = "line1\nline2\nline3\nline4\nline5\nline6";
        let banner = WinBanner::parse(output).expect("should parse with extra lines");

        assert_eq!(banner.volume_line, "line1");
        assert_eq!(banner.serial_line, "line2");
        assert_eq!(banner.no_subfolder, "line4");
    }

    #[test]
    fn should_fail_on_empty_input() {
        assert!(WinBanner::parse("").is_err());
    }

    #[test]
    fn should_parse_different_drive_banners() {
        let output_c = "卷 系统 的文件夹 PATH 列表\n卷序列号为 1234-5678\nC:.\n没有子文件夹";
        let output_d = "卷 数据 的文件夹 PATH 列表\n卷序列号为 ABCD-EF01\nD:.\n没有子文件夹";

        let banner_c = WinBanner::parse(output_c).expect("C drive parse");
        let banner_d = WinBanner::parse(output_d).expect("D drive parse");

        assert_ne!(banner_c.volume_line, banner_d.volume_line);
        assert_ne!(banner_c.serial_line, banner_d.serial_line);
    }

    // ------------------------------------------------------------------------
    // format_size_human Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_format_bytes_correctly() {
        assert_eq!(format_size_human(0), "0 B");
        assert_eq!(format_size_human(512), "512 B");
        assert_eq!(format_size_human(1023), "1023 B");
    }

    #[test]
    fn should_format_kilobytes_correctly() {
        assert_eq!(format_size_human(1024), "1.0 KB");
        assert_eq!(format_size_human(1536), "1.5 KB");
        assert_eq!(format_size_human(10240), "10.0 KB");
    }

    #[test]
    fn should_format_megabytes_correctly() {
        assert_eq!(format_size_human(1048576), "1.0 MB");
        assert_eq!(format_size_human(1572864), "1.5 MB");
    }

    #[test]
    fn should_format_gigabytes_correctly() {
        assert_eq!(format_size_human(1073741824), "1.0 GB");
    }

    #[test]
    fn should_format_terabytes_correctly() {
        assert_eq!(format_size_human(1099511627776), "1.0 TB");
    }

    #[test]
    fn should_format_boundary_values() {
        assert_eq!(format_size_human(1024 - 1), "1023 B");
        assert_eq!(format_size_human(1024), "1.0 KB");
        assert_eq!(format_size_human(1024 * 1024 - 1), "1024.0 KB");
        assert_eq!(format_size_human(1024 * 1024), "1.0 MB");
    }

    // ------------------------------------------------------------------------
    // format_datetime Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_format_datetime_with_correct_length() {
        let now = SystemTime::now();
        let formatted = format_datetime(&now);

        assert_eq!(formatted.len(), 19);
        assert_eq!(&formatted[4..5], "-");
        assert_eq!(&formatted[7..8], "-");
        assert_eq!(&formatted[10..11], " ");
        assert_eq!(&formatted[13..14], ":");
        assert_eq!(&formatted[16..17], ":");
    }

    #[test]
    fn should_format_datetime_in_local_timezone() {
        use chrono::Local;

        let now = SystemTime::now();
        let formatted = format_datetime(&now);
        let local_now = Local::now();
        let expected_date = local_now.format("%Y-%m-%d").to_string();

        assert!(
            formatted.starts_with(&expected_date),
            "formatted {} should start with local date {}",
            formatted,
            expected_date
        );
    }

    // ------------------------------------------------------------------------
    // TreeChars Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_create_unicode_tree_chars() {
        let chars = TreeChars::from_charset(CharsetMode::Unicode);
        assert_eq!(chars.branch, "├─");
        assert_eq!(chars.last_branch, "└─");
        assert_eq!(chars.vertical, "│  ");
        assert_eq!(chars.space, "    ");
    }

    #[test]
    fn should_create_ascii_tree_chars() {
        let chars = TreeChars::from_charset(CharsetMode::Ascii);
        assert_eq!(chars.branch, "+---");
        assert_eq!(chars.last_branch, "\\---");
        assert_eq!(chars.vertical, "|   ");
        assert_eq!(chars.space, "    ");
    }

    // ------------------------------------------------------------------------
    // StreamRenderer Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_create_stream_renderer_at_root_level() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let renderer = StreamRenderer::new(render_config);

        assert!(renderer.prefix_stack.is_empty());
        assert!(renderer.is_at_root_level());
    }

    #[test]
    fn should_render_basic_directory_entry() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("test.txt"));
        assert!(line.contains("└─"));
    }

    #[test]
    fn should_render_non_last_directory_with_branch() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test"),
            name: "test".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: false,
            is_file: false,
            has_more_dirs: true,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("├─"));
    }

    #[test]
    fn should_render_entry_with_ascii_charset() {
        let mut config = Config::default();
        config.render.charset = CharsetMode::Ascii;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test"),
            name: "test".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("\\---"));
    }

    #[test]
    fn should_render_entry_in_no_indent_mode() {
        let mut config = Config::default();
        config.render.no_indent = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 2,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(!line.contains("├"));
        assert!(!line.contains("└"));
        assert!(line.starts_with("    "));
    }

    #[test]
    fn should_render_entry_with_file_size() {
        let mut config = Config::default();
        config.render.show_size = true;
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata {
                size: 1024,
                ..Default::default()
            },
            depth: 0,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("1024"));
    }

    #[test]
    fn should_render_entry_with_human_readable_size() {
        let mut config = Config::default();
        config.render.show_size = true;
        config.render.human_readable = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata {
                size: 1024,
                ..Default::default()
            },
            depth: 0,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("1.0 KB"));
    }

    #[test]
    fn should_render_prefix_with_siblings() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);

        let entry = StreamEntry {
            path: PathBuf::from("nested"),
            name: "nested".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("│"));
    }

    #[test]
    fn should_render_prefix_without_siblings() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(false);

        let entry = StreamEntry {
            path: PathBuf::from("nested"),
            name: "nested".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(!line.contains("│"));
        assert!(line.starts_with("    "));
    }

    #[test]
    fn should_render_report_with_files() {
        let mut config = Config::default();
        config.render.show_report = true;
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let renderer = StreamRenderer::new(render_config);

        let report = renderer.render_report(5, 20, Duration::from_millis(100));

        assert!(report.contains("5 directory"));
        assert!(report.contains("20 files"));
        assert!(report.contains("0.100s"));
    }

    #[test]
    fn should_render_report_directories_only() {
        let mut config = Config::default();
        config.render.show_report = true;
        config.scan.show_files = false;
        let render_config = StreamRenderConfig::from_config(&config);
        let renderer = StreamRenderer::new(render_config);

        let report = renderer.render_report(5, 0, Duration::from_millis(50));

        assert!(report.contains("5 directory"));
        assert!(!report.contains("files"));
    }

    #[test]
    fn should_manage_level_stack_correctly() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        assert!(renderer.is_at_root_level());

        renderer.push_level(true);
        assert!(!renderer.is_at_root_level());

        let _ = renderer.pop_level();
        assert!(renderer.is_at_root_level());
    }

    #[test]
    fn should_return_trailing_line_when_last_was_file() {
        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);

        let entry = StreamEntry {
            path: PathBuf::from("file.txt"),
            name: "file.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&entry);

        let trailing = renderer.pop_level();
        assert!(trailing.is_some());
    }

    #[test]
    fn should_not_return_trailing_line_when_last_was_directory() {
        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);

        let entry = StreamEntry {
            path: PathBuf::from("subdir"),
            name: "subdir".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&entry);

        let trailing = renderer.pop_level();
        assert!(trailing.is_none());
    }

    #[test]
    fn should_build_file_prefix_correctly() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);
        let prefix_with_more = renderer.build_file_prefix(true);
        assert!(prefix_with_more.contains("│"));

        renderer.push_level(false);
        let prefix_without_more = renderer.build_file_prefix(false);
        assert!(prefix_without_more.contains("    "));
    }

    // ------------------------------------------------------------------------
    // StreamRenderConfig Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_create_stream_render_config_from_config() {
        let mut config = Config::default();
        config.render.charset = CharsetMode::Ascii;
        config.render.show_size = true;
        config.render.human_readable = true;
        config.scan.show_files = true;

        let render_config = StreamRenderConfig::from_config(&config);

        assert_eq!(render_config.charset, CharsetMode::Ascii);
        assert!(render_config.show_size);
        assert!(render_config.human_readable);
        assert!(render_config.show_files);
    }

    // ------------------------------------------------------------------------
    // render Function Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_render_basic_tree() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.scan.show_files = true;
        config.path_explicitly_set = false;

        let result = render(&stats, &config);

        assert!(result.content.contains("src"));
        assert!(result.content.contains("main.rs"));
        assert!(result.content.contains("Cargo.toml"));
        assert_eq!(result.directory_count, 1);
        assert_eq!(result.file_count, 3);
    }

    #[test]
    fn should_render_tree_with_ascii_charset() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(result.content.contains("+--") || result.content.contains("\\--"));
    }

    #[test]
    fn should_render_tree_without_indent() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.render.no_indent = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(!result.content.contains("├"));
        assert!(!result.content.contains("└"));
        assert!(!result.content.contains("+--"));
    }

    #[test]
    fn should_render_tree_with_file_sizes() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.render.show_size = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(result.content.contains("1024") || result.content.contains("2048"));
    }

    #[test]
    fn should_render_tree_with_human_readable_sizes() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.render.show_size = true;
        config.render.human_readable = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(result.content.contains("KB") || result.content.contains("B"));
    }

    #[test]
    fn should_render_tree_with_report() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.render.show_report = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(result.content.contains("directory"));
        assert!(result.content.contains("files"));
    }

    #[test]
    fn should_render_directories_only() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.scan.show_files = false;

        let result = render(&stats, &config);

        assert!(!result.content.contains("main.rs"));
        assert!(!result.content.contains("Cargo.toml"));
        assert!(result.content.contains("src"));
    }

    // ------------------------------------------------------------------------
    // render_tree_only Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_render_tree_only_without_banner() {
        let tree = create_test_tree();

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.scan.show_files = true;

        let result = render_tree_only(&tree, &config);

        assert!(result.contains("test_root"));
        assert!(result.contains("src"));
        assert!(result.contains("main.rs"));
    }

    // ------------------------------------------------------------------------
    // format_root_path_display Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_format_explicit_path_uppercase() {
        let path = Path::new(r"D:\Users\Test\Project");
        let result = format_root_path_display(path, true).unwrap();
        assert_eq!(result, r"D:\USERS\TEST\PROJECT");
    }

    #[test]
    fn should_format_implicit_path_as_drive_dot() {
        let path = Path::new(r"C:\Some\Path");
        let result = format_root_path_display(path, false).unwrap();
        assert_eq!(result, "C:.");
    }

    #[test]
    fn should_uppercase_drive_letter() {
        let path = Path::new(r"d:\test");
        let result = format_root_path_display(path, false).unwrap();
        assert_eq!(result, "D:.");
    }

    #[test]
    fn should_extract_drive_letter_from_normal_path() {
        let path = Path::new(r"C:\Windows");
        let drive = extract_drive_letter(path).unwrap();
        assert_eq!(drive, 'C');
    }

    #[test]
    fn should_extract_drive_letter_and_uppercase() {
        let path = Path::new(r"d:\data");
        let drive = extract_drive_letter(path).unwrap();
        assert_eq!(drive, 'D');
    }

    #[test]
    fn should_fail_extracting_drive_from_relative_path() {
        let path = Path::new("relative/path");
        let result = extract_drive_letter(path);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // tree_has_subdirectories Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_detect_subdirectory() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        assert!(tree_has_subdirectories(&root));
    }

    #[test]
    fn should_not_detect_subdirectory_with_only_files() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        assert!(!tree_has_subdirectories(&root));
    }

    #[test]
    fn should_not_detect_subdirectory_when_empty() {
        let root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        assert!(!tree_has_subdirectories(&root));
    }

    #[test]
    fn should_detect_subdirectory_in_mixed_content() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        assert!(tree_has_subdirectories(&root));
    }

    // ------------------------------------------------------------------------
    // get_filtered_children Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_exclude_files_when_show_files_false() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        let mut config = Config::default();
        config.scan.show_files = false;

        let filtered = get_filtered_children(&root, &config);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "subdir");
    }

    #[test]
    fn should_include_files_when_show_files_true() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        let mut config = Config::default();
        config.scan.show_files = true;

        let filtered = get_filtered_children(&root, &config);

        assert_eq!(filtered.len(), 2);
    }

    // ------------------------------------------------------------------------
    // remove_trailing_pipe_only_line Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_remove_trailing_ascii_pipe_line() {
        let input = "line1\nline2\n|   \n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn should_preserve_content_line() {
        let input = "line1\nline2\nchild2\n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\nchild2\n");
    }

    #[test]
    fn should_remove_trailing_unicode_pipe_line() {
        let input = "line1\nline2\n│   \n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn should_preserve_pure_whitespace_line() {
        let input = "line1\nline2\n    \n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\n    \n");
    }

    #[test]
    fn should_handle_empty_input() {
        let input = "".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "");
    }

    #[test]
    fn should_remove_mixed_pipe_line() {
        let input = "line1\nline2\n|   |   \n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\n");
    }

    // ------------------------------------------------------------------------
    // RenderResult Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_create_render_result() {
        let result = RenderResult {
            content: "test".to_string(),
            directory_count: 5,
            file_count: 10,
        };
        assert_eq!(result.content, "test");
        assert_eq!(result.directory_count, 5);
        assert_eq!(result.file_count, 10);
    }

    // ------------------------------------------------------------------------
    // File vs Directory Rendering Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_render_files_with_indent_not_branch() {
        use crate::scan::sort_tree;

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 1,
            file_count: 1,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        let lines: Vec<&str> = result.content.lines().collect();
        let file_line = lines.iter().find(|l| l.contains("file.txt")).unwrap();
        assert!(!file_line.contains("├"), "file should not use ├");
        assert!(!file_line.contains("└"), "file should not use └");

        let dir_line = lines.iter().find(|l| l.contains("dir")).unwrap();
        assert!(
            dir_line.contains("├") || dir_line.contains("└"),
            "directory should use branch connector"
        );
    }

    // ------------------------------------------------------------------------
    // Disk Usage Rendering Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_render_disk_usage_without_files() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut subdir = TreeNode::new(
            PathBuf::from("root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        subdir.children.push(TreeNode::new(
            PathBuf::from("root/subdir/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 1024,
                ..Default::default()
            },
        ));
        subdir.disk_usage = Some(1024);

        root.children.push(subdir);
        root.disk_usage = Some(1024);

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.batch_mode = true;
        config.scan.show_files = false;
        config.render.show_disk_usage = true;
        config.render.show_size = true;

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 1,
            file_count: 0,
        };

        let result = render(&stats, &config);

        assert!(result.content.contains("1024") || result.content.contains("1.0 KB"));
        assert!(!result.content.contains("file.txt"));
        assert!(result.content.contains("subdir"));
    }

    #[test]
    fn should_respect_depth_limit_with_disk_usage() {
        let mut dir2 = TreeNode::new(
            PathBuf::from("root/dir1/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir2.children.push(TreeNode::new(
            PathBuf::from("root/dir1/dir2/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 5,
                ..Default::default()
            },
        ));

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(dir2);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);

        root.compute_disk_usage();
        let directory_count = root.count_directories();
        let file_count = root.count_files();
        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(1),
            directory_count,
            file_count,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.batch_mode = true;
        config.render.no_win_banner = true;
        config.render.show_disk_usage = true;
        config.render.show_size = true;
        config.render.human_readable = false;
        config.scan.show_files = false;
        config.scan.max_depth = Some(1);

        let rendered = render(&stats, &config).content;

        assert!(rendered.contains("dir1"));
        assert!(!rendered.contains("dir2"));
        assert!(!rendered.contains("file.txt"));
        assert!(rendered.contains("5"));
    }

    // ------------------------------------------------------------------------
    // Files and Directories Separator Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_render_separator_between_files_and_directories() {
        use crate::scan::sort_tree;

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(TreeNode::new(
            PathBuf::from("root/dir1/file_a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        dir1.children.push(TreeNode::new(
            PathBuf::from("root/dir1/file_b.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut dir2 = TreeNode::new(
            PathBuf::from("root/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir2.children.push(TreeNode::new(
            PathBuf::from("root/dir2/file_c.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);
        root.children.push(dir2);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 2,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let dir1_idx = lines.iter().position(|l| l.contains("dir1")).unwrap();

        assert!(lines[dir1_idx + 1].contains("file_a.txt"));
        assert!(lines[dir1_idx + 2].contains("file_b.txt"));

        let separator_line = lines[dir1_idx + 3];
        assert!(
            separator_line.trim() == "|" || separator_line.trim().is_empty(),
            "should have trailing line after files, got: '{}'",
            separator_line
        );

        assert!(
            lines[dir1_idx + 4].contains("dir2"),
            "dir2 should follow trailing line, got: '{}'",
            lines[dir1_idx + 4]
        );
    }

    #[test]
    fn should_not_render_separator_for_last_directory() {
        use crate::scan::sort_tree;

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(TreeNode::new(
            PathBuf::from("root/dir1/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 1,
            file_count: 1,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let dir1_idx = lines.iter().position(|l| l.contains("dir1")).unwrap();
        assert!(lines[dir1_idx + 1].contains("file.txt"));

        if lines.len() > dir1_idx + 2 {
            let after_file = lines[dir1_idx + 2];
            assert!(
                !after_file.starts_with("|")
                    && !after_file.contains("├")
                    && !after_file.contains("│"),
                "last directory uses space prefix, got: '{}'",
                after_file
            );
        }
    }

    #[test]
    fn should_not_render_separator_between_empty_directories() {
        use crate::scan::sort_tree;

        let dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        let dir2 = TreeNode::new(
            PathBuf::from("root/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);
        root.children.push(dir2);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 2,
            file_count: 0,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = false;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let dir1_idx = lines.iter().position(|l| l.contains("dir1")).unwrap();
        let dir2_idx = lines.iter().position(|l| l.contains("dir2")).unwrap();

        assert_eq!(dir2_idx - dir1_idx, 1, "empty directories should be adjacent");
    }

    // ------------------------------------------------------------------------
    // Trailing Line Alignment Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_align_trailing_line_with_file_prefix_ascii() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut guide = TreeNode::new(
            PathBuf::from("root/docs/guide"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        guide.children.push(TreeNode::new(
            PathBuf::from("root/docs/guide/intro.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(api);
        docs.children.push(guide);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(docs);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 3,
            file_count: 2,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let v1_idx = lines.iter().position(|l| l.contains("v1.md")).unwrap();
        let v1_line = lines[v1_idx];

        if v1_idx + 1 < lines.len() {
            let next_line = lines[v1_idx + 1];
            if !next_line.is_empty() && next_line.chars().all(|c| c.is_whitespace() || c == '|') {
                let v1_prefix_len = v1_line.find("v1.md").unwrap();
                assert_eq!(
                    next_line.len(),
                    v1_prefix_len,
                    "trailing line length should match file prefix. v1 prefix: '{}', trailing: '{}'",
                    &v1_line[..v1_prefix_len],
                    next_line
                );
            }
        }
    }

    #[test]
    fn should_align_trailing_line_with_file_prefix_unicode() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut guide = TreeNode::new(
            PathBuf::from("root/docs/guide"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        guide.children.push(TreeNode::new(
            PathBuf::from("root/docs/guide/intro.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(api);
        docs.children.push(guide);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(docs);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 3,
            file_count: 2,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let v1_idx = lines.iter().position(|l| l.contains("v1.md")).unwrap();
        let v1_line = lines[v1_idx];

        if v1_idx + 1 < lines.len() {
            let next_line = lines[v1_idx + 1];
            if !next_line.is_empty() && next_line.chars().all(|c| c.is_whitespace() || c == '│') {
                let v1_prefix_char_count = v1_line.chars().take_while(|c| *c != 'v').count();
                let trailing_char_count = next_line.chars().count();

                assert_eq!(
                    trailing_char_count,
                    v1_prefix_char_count,
                    "trailing line chars should match file prefix chars"
                );
            }
        }
    }

    #[test]
    fn should_not_render_trailing_line_when_last_is_directory() {
        use crate::scan::sort_tree;

        let grandchild = TreeNode::new(
            PathBuf::from("root/parent/child1/grandchild"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut child1 = TreeNode::new(
            PathBuf::from("root/parent/child1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        child1.children.push(grandchild);

        let child2 = TreeNode::new(
            PathBuf::from("root/parent/child2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut parent = TreeNode::new(
            PathBuf::from("root/parent"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        parent.children.push(TreeNode::new(
            PathBuf::from("root/parent/file1.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        parent.children.push(TreeNode::new(
            PathBuf::from("root/parent/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        parent.children.push(child1);
        parent.children.push(child2);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(parent);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 2,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let child2_idx = lines.iter().position(|l| l.contains("child2")).unwrap();

        if child2_idx + 1 < lines.len() {
            let after_child2 = lines[child2_idx + 1];
            let is_trailing_whitespace =
                !after_child2.is_empty() && after_child2.chars().all(|c| c.is_whitespace());
            assert!(
                !is_trailing_whitespace,
                "last directory should not have trailing line, got: '{}'",
                after_child2
            );
        }
    }

    #[test]
    fn should_not_render_duplicate_trailing_lines() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v2.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(TreeNode::new(
            PathBuf::from("root/docs/guide.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        docs.children.push(api);

        let mut lib = TreeNode::new(
            PathBuf::from("root/src/lib"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        lib.children.push(TreeNode::new(
            PathBuf::from("root/src/lib/mod.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        src.children.push(lib);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/README.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(docs);
        root.children.push(src);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 6,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        for i in 0..lines.len().saturating_sub(1) {
            let current_is_trailing = !lines[i].is_empty()
                && lines[i]
                .chars()
                .all(|c| c.is_whitespace() || c == '│' || c == '|');
            let next_is_trailing = !lines[i + 1].is_empty()
                && lines[i + 1]
                .chars()
                .all(|c| c.is_whitespace() || c == '│' || c == '|');

            assert!(
                !(current_is_trailing && next_is_trailing),
                "found consecutive trailing lines at {} and {}:\n'{}'\n'{}'",
                i + 1,
                i + 2,
                lines[i],
                lines[i + 1]
            );
        }
    }

    // ------------------------------------------------------------------------
    // Nested Directory Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_render_nested_directories_with_files() {
        use crate::scan::sort_tree;

        let mut subdir = TreeNode::new(
            PathBuf::from("root/dir1/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        subdir.children.push(TreeNode::new(
            PathBuf::from("root/dir1/subdir/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(TreeNode::new(
            PathBuf::from("root/dir1/file1.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        dir1.children.push(subdir);

        let mut dir2 = TreeNode::new(
            PathBuf::from("root/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir2.children.push(TreeNode::new(
            PathBuf::from("root/dir2/file3.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);
        root.children.push(dir2);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 3,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(result.content.contains("dir1"));
        assert!(result.content.contains("file1.txt"));
        assert!(result.content.contains("subdir"));
        assert!(result.content.contains("file2.txt"));
        assert!(result.content.contains("dir2"));
        assert!(result.content.contains("file3.txt"));

        let lines: Vec<&str> = result.content.lines().collect();
        let dir2_idx = lines.iter().position(|l| l.contains("\\---dir2")).unwrap();

        let before_dir2 = lines[dir2_idx - 1];
        assert!(
            before_dir2.starts_with("|"),
            "should have | placeholder before dir2, got: '{}'",
            before_dir2
        );
    }

    #[test]
    fn should_not_render_trailing_for_pure_directory_structure() {
        use crate::scan::sort_tree;

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        let subdir = TreeNode::new(
            PathBuf::from("root/dir1/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(subdir);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 2,
            file_count: 0,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = false;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let dir1_idx = lines.iter().position(|l| l.contains("dir1")).unwrap();
        let subdir_idx = lines.iter().position(|l| l.contains("subdir")).unwrap();

        assert_eq!(subdir_idx - dir1_idx, 1, "pure directory structure needs no trailing");
    }

    // ------------------------------------------------------------------------
    // Deep Nesting Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_handle_deeply_nested_files() {
        use crate::scan::sort_tree;

        let mut deep = TreeNode::new(
            PathBuf::from("root/dir1/subdir/deep"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        deep.children.push(TreeNode::new(
            PathBuf::from("root/dir1/subdir/deep/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut subdir = TreeNode::new(
            PathBuf::from("root/dir1/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        subdir.children.push(deep);

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(subdir);

        let mut dir2 = TreeNode::new(
            PathBuf::from("root/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir2.children.push(TreeNode::new(
            PathBuf::from("root/dir2/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);
        root.children.push(dir2);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 2,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let file_idx = lines
            .iter()
            .position(|l| l.contains("file.txt") && !l.contains("file2"))
            .unwrap();
        let file_line = lines[file_idx];

        if file_idx + 1 < lines.len() {
            let trailing = lines[file_idx + 1];
            if !trailing.is_empty() && trailing.chars().all(|c| c.is_whitespace() || c == '|') {
                let file_prefix_len = file_line
                    .chars()
                    .take_while(|c| *c == '|' || *c == '+' || *c == '\\' || *c == '-' || *c == ' ')
                    .count();

                assert_eq!(
                    trailing.len(),
                    file_prefix_len,
                    "trailing line should match deeply nested file prefix"
                );
            }
        }
    }

    #[test]
    fn should_verify_prefix_consistency_in_nested_dirs() {
        use crate::scan::sort_tree;

        let mut c = TreeNode::new(
            PathBuf::from("root/a/b/c"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        c.children.push(TreeNode::new(
            PathBuf::from("root/a/b/c/file1.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        c.children.push(TreeNode::new(
            PathBuf::from("root/a/b/c/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut b = TreeNode::new(
            PathBuf::from("root/a/b"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        b.children.push(c);
        b.children.push(TreeNode::new(
            PathBuf::from("root/a/b/file3.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut a = TreeNode::new(
            PathBuf::from("root/a"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        a.children.push(b);
        a.children.push(TreeNode::new(
            PathBuf::from("root/a/file4.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut d = TreeNode::new(
            PathBuf::from("root/d"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        d.children.push(TreeNode::new(
            PathBuf::from("root/d/file5.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(a);
        root.children.push(d);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 5,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        for i in 0..lines.len().saturating_sub(1) {
            let current = lines[i];
            let next = lines[i + 1];

            let current_is_file = current.contains(".txt");
            let next_is_trailing =
                !next.is_empty() && next.chars().all(|c| c.is_whitespace() || c == '|');

            if current_is_file && next_is_trailing {
                let current_prefix_len = current
                    .chars()
                    .take_while(|c| *c == '|' || *c == '+' || *c == '\\' || *c == '-' || *c == ' ')
                    .count();

                assert_eq!(
                    next.len(),
                    current_prefix_len,
                    "trailing should match file prefix. file: '{}', trailing: '{}'",
                    current,
                    next
                );
            }
        }
    }

    // ------------------------------------------------------------------------
    // Files Only Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_render_trailing_space_for_files_only() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/b.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/c.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 0,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let c_idx = lines.iter().position(|l| l.contains("c.txt")).unwrap();
        assert!(c_idx + 1 < lines.len(), "should have trailing line after c.txt");
        assert_eq!(lines[c_idx + 1], "    ", "trailing line should be 4 spaces");
    }

    #[test]
    fn should_verify_trailing_line_prefix_matches_previous() {
        use crate::scan::sort_tree;

        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut tmp = TreeNode::new(
            PathBuf::from("root/tmp"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        tmp.children.push(TreeNode::new(
            PathBuf::from("root/tmp/a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(src);
        root.children.push(tmp);

        sort_tree(&mut root, false);

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let output = render_tree_only(&root, &config);
        let lines: Vec<&str> = output.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                continue;
            }

            let trimmed = line.trim_end();
            let is_trailing_line =
                !trimmed.is_empty() && trimmed.chars().all(|c| c == ' ' || c == '│');

            if is_trailing_line {
                let prev_line = lines[i - 1];
                let trailing_len = trimmed.chars().count();
                let prev_prefix: String = prev_line.chars().take(trailing_len).collect();

                assert!(
                    prev_prefix.chars().all(|c| c == ' ' || c == '│'),
                    "trailing line at {} should align with prefix portion of previous line",
                    i + 1
                );
            }
        }
    }

    // ------------------------------------------------------------------------
    // Stream Renderer State Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_not_return_trailing_when_last_child_is_dir() {
        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(false);

        let file1 = StreamEntry {
            path: PathBuf::from("file1.txt"),
            name: "file1.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: false,
            is_file: true,
            has_more_dirs: true,
        };
        let _ = renderer.render_entry(&file1);

        let file2 = StreamEntry {
            path: PathBuf::from("file2.txt"),
            name: "file2.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: false,
            is_file: true,
            has_more_dirs: true,
        };
        let _ = renderer.render_entry(&file2);

        let child1 = StreamEntry {
            path: PathBuf::from("child1"),
            name: "child1".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: false,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&child1);

        renderer.push_level(true);
        let grandchild = StreamEntry {
            path: PathBuf::from("grandchild"),
            name: "grandchild".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 2,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&grandchild);
        let _ = renderer.pop_level();

        let child2 = StreamEntry {
            path: PathBuf::from("child2"),
            name: "child2".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&child2);

        renderer.push_level(false);
        let child2_trailing = renderer.pop_level();
        assert!(child2_trailing.is_none(), "empty dir child2 should have no trailing");

        let parent_trailing = renderer.pop_level();

        assert!(
            parent_trailing.is_none(),
            "parent's last child is dir, should have no trailing, got: {:?}",
            parent_trailing
        );
    }

    #[test]
    fn should_return_trailing_when_last_child_is_file() {
        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);

        let subdir = StreamEntry {
            path: PathBuf::from("subdir"),
            name: "subdir".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: false,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&subdir);

        let file = StreamEntry {
            path: PathBuf::from("file.txt"),
            name: "file.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&file);

        let trailing = renderer.pop_level();

        assert!(trailing.is_some(), "last child is file, should have trailing");
    }

    // ------------------------------------------------------------------------
    // Additional Coverage Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_render_full_path_mode() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.path_mode = PathMode::Full;
        config.scan.show_files = true;

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 0,
            file_count: 1,
        };

        let result = render(&stats, &config);
        assert!(result.content.contains("root/file.txt") || result.content.contains("root\\file.txt"));
    }

    #[test]
    fn should_handle_empty_tree() {
        let root = TreeNode::new(
            PathBuf::from("empty"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut config = Config::with_root(PathBuf::from("empty"));
        config.render.no_win_banner = true;

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(1),
            directory_count: 0,
            file_count: 0,
        };

        let result = render(&stats, &config);
        assert!(result.content.len() > 0);
        assert_eq!(result.directory_count, 0);
        assert_eq!(result.file_count, 0);
    }

    #[test]
    fn should_render_with_modification_date() {
        use std::time::SystemTime;

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 100,
                modified: Some(SystemTime::now()),
                ..Default::default()
            },
        ));

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.show_date = true;
        config.scan.show_files = true;

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 0,
            file_count: 1,
        };

        let result = render(&stats, &config);
        assert!(result.content.contains("-"));
        assert!(result.content.contains(":"));
    }

    #[test]
    fn should_filter_empty_directories_when_prune_enabled() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let empty_dir = TreeNode::new(
            PathBuf::from("root/empty"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut non_empty = TreeNode::new(
            PathBuf::from("root/nonempty"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        non_empty.children.push(TreeNode::new(
            PathBuf::from("root/nonempty/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        root.children.push(empty_dir);
        root.children.push(non_empty);

        let mut config = Config::default();
        config.matching.prune_empty = true;
        config.scan.show_files = true;

        let filtered = get_filtered_children(&root, &config);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "nonempty");
    }

    #[test]
    fn should_respect_max_depth_in_render() {
        let mut deep = TreeNode::new(
            PathBuf::from("root/level1/level2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        deep.children.push(TreeNode::new(
            PathBuf::from("root/level1/level2/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut level1 = TreeNode::new(
            PathBuf::from("root/level1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        level1.children.push(deep);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(level1);

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.scan.max_depth = Some(1);
        config.scan.show_files = true;

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 2,
            file_count: 1,
        };

        let result = render(&stats, &config);

        assert!(result.content.contains("level1"));
        assert!(!result.content.contains("level2"));
        assert!(!result.content.contains("file.txt"));
    }

    #[test]
    fn should_render_stream_entry_with_date() {
        use std::time::SystemTime;

        let mut config = Config::default();
        config.render.show_date = true;
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata {
                size: 100,
                modified: Some(SystemTime::now()),
                ..Default::default()
            },
            depth: 0,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("-"));
        assert!(line.contains(":"));
    }

    #[test]
    fn should_handle_very_large_file_sizes() {
        let result = format_size_human(u64::MAX);
        assert!(result.ends_with(" TB"), "very large size should be in TB, got: {}", result);
        assert!(result.contains("16777216"), "should be approximately 16777216 TB, got: {}", result);
    }

    #[test]
    fn should_handle_stream_renderer_multiple_levels() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);
        renderer.push_level(true);
        renderer.push_level(false);

        assert!(!renderer.is_at_root_level());

        let _ = renderer.pop_level();
        let _ = renderer.pop_level();
        let _ = renderer.pop_level();

        assert!(renderer.is_at_root_level());
    }

    #[test]
    fn should_render_report_without_show_report() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let renderer = StreamRenderer::new(render_config);

        let report = renderer.render_report(5, 10, Duration::from_millis(100));
        assert!(report.is_empty());
    }

    #[test]
    fn should_verify_root_has_content_tracking() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        assert!(!renderer.root_has_content());

        let entry = StreamEntry {
            path: PathBuf::from("test"),
            name: "test".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&entry);

        assert!(renderer.root_has_content());
    }

    // ------------------------------------------------------------------------
    // Batch vs Stream Mode Alignment Tests
    // ------------------------------------------------------------------------

    #[test]
    fn should_align_batch_and_stream_trailing_line_behavior() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(api);

        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(docs);
        root.children.push(src);

        sort_tree(&mut root, false);

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let batch_result = render(
            &ScanStats {
                tree: root.clone(),
                duration: Duration::from_millis(100),
                directory_count: 3,
                file_count: 2,
            },
            &config,
        );

        let stream_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(stream_config);
        let mut stream_output = renderer.render_header(Path::new("root"), false);

        fn render_node_stream(
            renderer: &mut StreamRenderer,
            node: &TreeNode,
            depth: usize,
            _is_last: bool,
            _has_more_siblings: bool,
            output: &mut String,
        ) {
            let (files, dirs): (Vec<_>, Vec<_>) = node
                .children
                .iter()
                .partition(|c| c.kind == EntryKind::File);

            let has_dirs = !dirs.is_empty();

            for file in &files {
                let entry = StreamEntry {
                    path: file.path.clone(),
                    name: file.name.clone(),
                    kind: EntryKind::File,
                    metadata: file.metadata.clone(),
                    depth,
                    is_last: false,
                    is_file: true,
                    has_more_dirs: has_dirs,
                };
                output.push_str(&renderer.render_entry(&entry));
                output.push('\n');
            }

            let dir_count = dirs.len();
            for (i, dir) in dirs.iter().enumerate() {
                let is_last_dir = i == dir_count - 1;

                let entry = StreamEntry {
                    path: dir.path.clone(),
                    name: dir.name.clone(),
                    kind: EntryKind::Directory,
                    metadata: dir.metadata.clone(),
                    depth,
                    is_last: is_last_dir,
                    is_file: false,
                    has_more_dirs: false,
                };
                output.push_str(&renderer.render_entry(&entry));
                output.push('\n');

                if !dir.children.is_empty() {
                    renderer.push_level(!is_last_dir);
                    render_node_stream(renderer, dir, depth + 1, is_last_dir, !is_last_dir, output);
                    if let Some(trailing) = renderer.pop_level() {
                        if !is_last_dir {
                            output.push_str(&trailing);
                            output.push('\n');
                        }
                    }
                }
            }
        }

        render_node_stream(&mut renderer, &root, 0, false, false, &mut stream_output);

        let batch_lines: Vec<&str> = batch_result.content.lines().collect();
        let stream_lines: Vec<&str> = stream_output.lines().collect();

        let batch_trailing: Vec<_> = batch_lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                !l.is_empty() && l.chars().all(|c| c.is_whitespace() || c == '│' || c == '|')
            })
            .collect();

        let stream_trailing: Vec<_> = stream_lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                !l.is_empty() && l.chars().all(|c| c.is_whitespace() || c == '│' || c == '|')
            })
            .collect();

        assert_eq!(
            batch_trailing.len(),
            stream_trailing.len(),
            "batch and stream should have same number of trailing lines.\nbatch: {:?}\nstream: {:?}",
            batch_trailing,
            stream_trailing
        );
    }

    #[test]
    fn should_align_batch_and_stream_no_trailing_for_dir_last() {
        use crate::scan::sort_tree;

        let mut child = TreeNode::new(
            PathBuf::from("root/parent/child"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        child.children.push(TreeNode::new(
            PathBuf::from("root/parent/child/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut parent = TreeNode::new(
            PathBuf::from("root/parent"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        parent.children.push(child);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(parent);

        sort_tree(&mut root, false);

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.scan.show_files = true;

        let batch_result = render(
            &ScanStats {
                tree: root,
                duration: Duration::from_millis(100),
                directory_count: 2,
                file_count: 1,
            },
            &config,
        );

        let lines: Vec<&str> = batch_result.content.lines().collect();
        let child_idx = lines.iter().position(|l| l.contains("child")).unwrap();

        if child_idx > 0 {
            let before_child = lines[child_idx - 1];
            let is_trailing = !before_child.is_empty()
                && before_child
                .chars()
                .all(|c| c.is_whitespace() || c == '│' || c == '|');
            assert!(
                !is_trailing,
                "should not have trailing line before last dir 'child', got: '{}'",
                before_child
            );
        }
    }
}