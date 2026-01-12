//! Configuration module: defines the full `Config` and its sub-configuration structures.
//!
//! This module is the **Single Source of Truth** for user intent.
//! All command-line arguments are parsed by the CLI layer and converted into a `Config` structure.
//! Subsequent scanning, matching, rendering, and output layers depend solely on this configuration
//! and do not access the original arguments directly.
//!
//! File: src/config.rs
//! Author: WaterRun
//! Date: 2026-01-12

#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::path::PathBuf;

use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Configuration validation error.
///
/// Represents errors that occur when user-provided argument combinations are invalid
/// or cannot satisfy runtime conditions.
///
/// # Examples
///
/// ```
/// use treepp::config::ConfigError;
///
/// let err = ConfigError::ConflictingOptions {
///     opt_a: "--include".to_string(),
///     opt_b: "--exclude".to_string(),
///     reason: "cannot specify both include and exclude for the same pattern".to_string(),
/// };
/// assert!(err.to_string().contains("--include"));
/// assert!(err.to_string().contains("--exclude"));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// Options conflict with each other.
    #[error("Option conflict: {opt_a} and {opt_b} cannot be used together ({reason})")]
    ConflictingOptions {
        /// First conflicting option.
        opt_a: String,
        /// Second conflicting option.
        opt_b: String,
        /// Reason for the conflict.
        reason: String,
    },

    /// Parameter value is invalid.
    #[error("Invalid parameter value: {option} = {value} ({reason})")]
    InvalidValue {
        /// Option name.
        option: String,
        /// Provided value.
        value: String,
        /// Reason why the value is invalid.
        reason: String,
    },

    /// Path does not exist or is inaccessible.
    #[error("Invalid path: {path} ({reason})")]
    InvalidPath {
        /// The path that is invalid.
        path: PathBuf,
        /// Reason why the path is invalid.
        reason: String,
    },

    /// Output format cannot be inferred from file extension.
    #[error(
        "Unable to infer output format: {path} (supported extensions: .txt, .json, .yml, .yaml, .toml)"
    )]
    UnknownOutputFormat {
        /// The output file path with unrecognized extension.
        path: PathBuf,
    },
}

/// Result type for configuration validation.
pub type ConfigResult<T> = Result<T, ConfigError>;

// ============================================================================
// Output Format
// ============================================================================

/// Output format for tree results.
///
/// Specifies the file format for output results.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use treepp::config::OutputFormat;
///
/// let format = OutputFormat::from_extension(Path::new("tree.json"));
/// assert_eq!(format, Some(OutputFormat::Json));
///
/// let format = OutputFormat::from_extension(Path::new("tree.unknown"));
/// assert_eq!(format, None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Plain text format (default).
    #[default]
    Txt,
    /// JSON format.
    Json,
    /// YAML format.
    Yaml,
    /// TOML format.
    Toml,
}

impl OutputFormat {
    /// Infers output format from file extension.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to extract extension from.
    ///
    /// # Returns
    ///
    /// `Some(OutputFormat)` if the extension is recognized, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use treepp::config::OutputFormat;
    ///
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.txt")), Some(OutputFormat::Txt));
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.JSON")), Some(OutputFormat::Json));
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.yml")), Some(OutputFormat::Yaml));
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.yaml")), Some(OutputFormat::Yaml));
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.toml")), Some(OutputFormat::Toml));
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.unknown")), None);
    /// assert_eq!(OutputFormat::from_extension(Path::new("noext")), None);
    /// ```
    #[must_use]
    pub fn from_extension(path: &std::path::Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
                "txt" => Some(Self::Txt),
                "json" => Some(Self::Json),
                "yml" | "yaml" => Some(Self::Yaml),
                "toml" => Some(Self::Toml),
                _ => None,
            })
    }

    /// Returns the default file extension for this format.
    ///
    /// # Returns
    ///
    /// A static string representing the file extension.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::OutputFormat;
    ///
    /// assert_eq!(OutputFormat::Txt.extension(), "txt");
    /// assert_eq!(OutputFormat::Json.extension(), "json");
    /// assert_eq!(OutputFormat::Yaml.extension(), "yml");
    /// assert_eq!(OutputFormat::Toml.extension(), "toml");
    /// ```
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Json => "json",
            Self::Yaml => "yml",
            Self::Toml => "toml",
        }
    }
}

// ============================================================================
// Charset Mode
// ============================================================================

/// Character set mode for tree rendering.
///
/// Controls whether tree symbols use ASCII or Unicode characters.
///
/// # Examples
///
/// ```
/// use treepp::config::CharsetMode;
///
/// let unicode = CharsetMode::Unicode;
/// assert_eq!(unicode.branch(), "├─");
/// assert_eq!(unicode.last_branch(), "└─");
///
/// let ascii = CharsetMode::Ascii;
/// assert_eq!(ascii.branch(), "+---");
/// assert_eq!(ascii.last_branch(), "\\---");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharsetMode {
    /// Use Unicode characters for tree rendering (default).
    #[default]
    Unicode,
    /// Use ASCII characters for tree rendering (compatible with `tree /A`).
    Ascii,
}

impl CharsetMode {
    /// Returns the branch symbol for non-last siblings.
    ///
    /// # Returns
    ///
    /// A static string representing the branch symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::CharsetMode;
    ///
    /// assert_eq!(CharsetMode::Unicode.branch(), "├─");
    /// assert_eq!(CharsetMode::Ascii.branch(), "+---");
    /// ```
    #[must_use]
    pub const fn branch(&self) -> &'static str {
        match self {
            Self::Unicode => "├─",
            Self::Ascii => "+---",
        }
    }

    /// Returns the branch symbol for the last sibling.
    ///
    /// # Returns
    ///
    /// A static string representing the last branch symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::CharsetMode;
    ///
    /// assert_eq!(CharsetMode::Unicode.last_branch(), "└─");
    /// assert_eq!(CharsetMode::Ascii.last_branch(), "\\---");
    /// ```
    #[must_use]
    pub const fn last_branch(&self) -> &'static str {
        match self {
            Self::Unicode => "└─",
            Self::Ascii => "\\---",
        }
    }

    /// Returns the vertical connector for items with subsequent siblings.
    ///
    /// # Returns
    ///
    /// A static string representing the vertical line with spacing.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::CharsetMode;
    ///
    /// assert_eq!(CharsetMode::Unicode.vertical(), "│  ");
    /// assert_eq!(CharsetMode::Ascii.vertical(), "|   ");
    /// ```
    #[must_use]
    pub const fn vertical(&self) -> &'static str {
        match self {
            Self::Unicode => "│  ",
            Self::Ascii => "|   ",
        }
    }

    /// Returns the blank indent for items without subsequent siblings.
    ///
    /// # Returns
    ///
    /// A static string of spaces matching the vertical connector width.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::CharsetMode;
    ///
    /// assert_eq!(CharsetMode::Unicode.indent(), "   ");
    /// assert_eq!(CharsetMode::Ascii.indent(), "    ");
    /// ```
    #[must_use]
    pub const fn indent(&self) -> &'static str {
        match self {
            Self::Unicode => "   ",
            Self::Ascii => "    ",
        }
    }
}

// ============================================================================
// Path Mode
// ============================================================================

/// Path display mode.
///
/// Controls whether output displays full paths or relative names.
///
/// # Examples
///
/// ```
/// use treepp::config::PathMode;
///
/// let mode = PathMode::default();
/// assert_eq!(mode, PathMode::Relative);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathMode {
    /// Display only the name (default).
    #[default]
    Relative,
    /// Display the full absolute path.
    Full,
}

// ============================================================================
// Sub-Configuration Structures
// ============================================================================

/// Scan options.
///
/// Configuration controlling directory traversal behavior.
///
/// # Examples
///
/// ```
/// use treepp::config::ScanOptions;
///
/// let opts = ScanOptions::default();
/// assert_eq!(opts.max_depth, None);
/// assert!(!opts.show_files);
/// assert_eq!(opts.thread_count.get(), 8);
/// assert!(!opts.respect_gitignore);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOptions {
    /// Maximum recursion depth (`None` means unlimited).
    pub max_depth: Option<usize>,
    /// Whether to show files (corresponds to `/F`).
    pub show_files: bool,
    /// Number of scanning threads.
    pub thread_count: NonZeroUsize,
    /// Whether to respect `.gitignore` rules.
    pub respect_gitignore: bool,
}

impl Default for ScanOptions {
    /// Creates default scan options with 8 threads and no file display.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::ScanOptions;
    ///
    /// let opts = ScanOptions::default();
    /// assert_eq!(opts.thread_count.get(), 8);
    /// ```
    fn default() -> Self {
        Self {
            max_depth: None,
            show_files: false,
            thread_count: NonZeroUsize::new(8).expect("8 is non-zero"),
            respect_gitignore: false,
        }
    }
}

/// Match options.
///
/// Configuration controlling file/directory filtering behavior.
///
/// # Examples
///
/// ```
/// use treepp::config::MatchOptions;
///
/// let opts = MatchOptions::default();
/// assert!(opts.include_patterns.is_empty());
/// assert!(opts.exclude_patterns.is_empty());
/// assert!(!opts.prune_empty);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MatchOptions {
    /// Include patterns (only show matching items).
    pub include_patterns: Vec<String>,
    /// Exclude patterns (ignore matching items).
    pub exclude_patterns: Vec<String>,
    /// Whether to prune empty directories.
    pub prune_empty: bool,
}

/// Render options.
///
/// Configuration controlling tree output appearance.
///
/// # Examples
///
/// ```
/// use treepp::config::{RenderOptions, CharsetMode, PathMode};
///
/// let opts = RenderOptions::default();
/// assert_eq!(opts.charset, CharsetMode::Unicode);
/// assert_eq!(opts.path_mode, PathMode::Relative);
/// assert!(!opts.show_size);
/// assert!(!opts.human_readable);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderOptions {
    /// Character set mode.
    pub charset: CharsetMode,
    /// Path display mode.
    pub path_mode: PathMode,
    /// Whether to show file size.
    pub show_size: bool,
    /// Whether to display size in human-readable format.
    pub human_readable: bool,
    /// Whether to show last modification date.
    pub show_date: bool,
    /// Whether to show cumulative directory size.
    pub show_disk_usage: bool,
    /// Whether to hide tree connectors (indent only).
    pub no_indent: bool,
    /// Whether to reverse sort order.
    pub reverse_sort: bool,
    /// Whether to show summary report at the end.
    pub show_report: bool,
    /// Whether to hide Windows native banner.
    pub no_win_banner: bool,
}

/// Output options.
///
/// Configuration controlling result output destination and format.
///
/// # Examples
///
/// ```
/// use treepp::config::OutputOptions;
///
/// let opts = OutputOptions::default();
/// assert!(opts.output_path.is_none());
/// assert!(!opts.silent);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutputOptions {
    /// Output file path (`None` means terminal output only).
    pub output_path: Option<PathBuf>,
    /// Output format (inferred from `output_path` extension, or default `Txt`).
    pub format: OutputFormat,
    /// Whether to suppress terminal output.
    pub silent: bool,
}

// ============================================================================
// Main Configuration Structure
// ============================================================================

/// Full configuration.
///
/// The single source of truth for user intent. Generated by CLI parsing,
/// and all subsequent modules (scan, match, render, output) depend on this configuration.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::config::Config;
///
/// let config = Config::default();
/// assert_eq!(config.root_path, PathBuf::from("."));
/// assert!(!config.scan.show_files);
/// assert!(!config.batch_mode);
/// ```
///
/// ```
/// use std::path::PathBuf;
/// use std::num::NonZeroUsize;
/// use treepp::config::{Config, OutputFormat};
///
/// let mut config = Config::default();
/// config.scan.show_files = true;
/// config.batch_mode = true;
/// config.scan.thread_count = NonZeroUsize::new(16).unwrap();
/// config.output.output_path = Some(PathBuf::from("tree.json"));
///
/// let validated = config.validate().expect("validation should pass");
/// assert_eq!(validated.output.format, OutputFormat::Json);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Root path (starting directory).
    pub root_path: PathBuf,
    /// Whether the user explicitly specified a path.
    pub path_explicitly_set: bool,
    /// Whether to show help information.
    pub show_help: bool,
    /// Whether to show version information.
    pub show_version: bool,
    /// Whether to use batch mode (default `false`, uses streaming mode).
    pub batch_mode: bool,
    /// Scan options.
    pub scan: ScanOptions,
    /// Match options.
    pub matching: MatchOptions,
    /// Render options.
    pub render: RenderOptions,
    /// Output options.
    pub output: OutputOptions,
}

impl Default for Config {
    /// Creates a default configuration with current directory as root.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::config::Config;
    ///
    /// let config = Config::default();
    /// assert_eq!(config.root_path, PathBuf::from("."));
    /// ```
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("."),
            path_explicitly_set: false,
            show_help: false,
            show_version: false,
            batch_mode: false,
            scan: ScanOptions::default(),
            matching: MatchOptions::default(),
            render: RenderOptions::default(),
            output: OutputOptions::default(),
        }
    }
}

impl Config {
    /// Creates a configuration with the specified root path.
    ///
    /// # Arguments
    ///
    /// * `root_path` - The starting directory for tree traversal.
    ///
    /// # Returns
    ///
    /// A new `Config` with the specified root path and `path_explicitly_set` set to `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::config::Config;
    ///
    /// let config = Config::with_root(PathBuf::from("C:\\Users"));
    /// assert_eq!(config.root_path, PathBuf::from("C:\\Users"));
    /// assert!(config.path_explicitly_set);
    /// ```
    #[must_use]
    pub fn with_root(root_path: PathBuf) -> Self {
        Self {
            root_path,
            path_explicitly_set: true,
            ..Self::default()
        }
    }

    /// Validates the configuration and populates derived fields.
    ///
    /// Performs the following operations:
    /// - Validates root path existence and canonicalizes it
    /// - Infers output format from file extension
    /// - Checks for option conflicts
    /// - Applies implicit dependencies
    ///
    /// # Returns
    ///
    /// The validated and normalized `Config`.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if:
    /// - Options have irreconcilable conflicts
    /// - Root path does not exist or is not a directory
    /// - Output path extension is unrecognized
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::config::{Config, OutputFormat};
    ///
    /// let mut config = Config::default();
    /// config.batch_mode = true;
    /// config.output.output_path = Some(PathBuf::from("result.json"));
    ///
    /// let validated = config.validate().unwrap();
    /// assert_eq!(validated.output.format, OutputFormat::Json);
    /// ```
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::config::{Config, ConfigError};
    ///
    /// let mut config = Config::default();
    /// config.output.output_path = Some(PathBuf::from("result.xyz"));
    ///
    /// let err = config.validate().unwrap_err();
    /// assert!(matches!(err, ConfigError::UnknownOutputFormat { .. }));
    /// ```
    pub fn validate(mut self) -> ConfigResult<Self> {
        self.validate_and_canonicalize_root_path()?;
        self.infer_output_format()?;
        self.check_conflicts()?;
        self.apply_implicit_dependencies();
        Ok(self)
    }

    /// Determines whether this is an "info-only" mode (help or version).
    ///
    /// # Returns
    ///
    /// `true` if either `show_help` or `show_version` is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::Config;
    ///
    /// let mut config = Config::default();
    /// assert!(!config.is_info_only());
    ///
    /// config.show_help = true;
    /// assert!(config.is_info_only());
    /// ```
    #[must_use]
    pub const fn is_info_only(&self) -> bool {
        self.show_help || self.show_version
    }

    /// Determines whether file size information is needed.
    ///
    /// Returns `true` when any of `show_size`, `human_readable`, or `show_disk_usage` is enabled.
    ///
    /// # Returns
    ///
    /// `true` if size information is required.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::Config;
    ///
    /// let mut config = Config::default();
    /// assert!(!config.needs_size_info());
    ///
    /// config.render.show_size = true;
    /// assert!(config.needs_size_info());
    /// ```
    #[must_use]
    pub const fn needs_size_info(&self) -> bool {
        self.render.show_size || self.render.human_readable || self.render.show_disk_usage
    }

    /// Determines whether time information is needed.
    ///
    /// Returns `true` when `show_date` is enabled.
    ///
    /// # Returns
    ///
    /// `true` if time information is required.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::Config;
    ///
    /// let mut config = Config::default();
    /// assert!(!config.needs_time_info());
    ///
    /// config.render.show_date = true;
    /// assert!(config.needs_time_info());
    /// ```
    #[must_use]
    pub const fn needs_time_info(&self) -> bool {
        self.render.show_date
    }

    fn validate_and_canonicalize_root_path(&mut self) -> ConfigResult<()> {
        if !self.root_path.exists() {
            return Err(ConfigError::InvalidPath {
                path: self.root_path.clone(),
                reason: "Path does not exist".to_string(),
            });
        }

        if !self.root_path.is_dir() {
            return Err(ConfigError::InvalidPath {
                path: self.root_path.clone(),
                reason: "Path is not a directory".to_string(),
            });
        }

        match dunce::canonicalize(&self.root_path) {
            Ok(canonical) => {
                self.root_path = canonical;
                Ok(())
            }
            Err(e) => Err(ConfigError::InvalidPath {
                path: self.root_path.clone(),
                reason: format!("Failed to canonicalize path: {}", e),
            }),
        }
    }

    fn infer_output_format(&mut self) -> ConfigResult<()> {
        if let Some(ref path) = self.output.output_path {
            if let Some(format) = OutputFormat::from_extension(path) {
                self.output.format = format;
            } else {
                return Err(ConfigError::UnknownOutputFormat { path: path.clone() });
            }
        }
        Ok(())
    }

    fn check_conflicts(&self) -> ConfigResult<()> {
        if self.output.silent && self.output.output_path.is_none() {
            return Err(ConfigError::ConflictingOptions {
                opt_a: "--silent".to_string(),
                opt_b: "(no --output)".to_string(),
                reason: "Silent mode requires an output file; otherwise no output will be produced."
                    .to_string(),
            });
        }

        if self.render.show_disk_usage && !self.batch_mode {
            return Err(ConfigError::ConflictingOptions {
                opt_a: "--disk-usage".to_string(),
                opt_b: "(no --batch)".to_string(),
                reason: "Disk usage calculation requires batch mode (--batch).".to_string(),
            });
        }

        if self.output.output_path.is_some() {
            let format = &self.output.format;
            let requires_batch = matches!(
                format,
                OutputFormat::Json | OutputFormat::Yaml | OutputFormat::Toml
            );
            if requires_batch && !self.batch_mode {
                return Err(ConfigError::ConflictingOptions {
                    opt_a: format!("--output (format: {:?})", format),
                    opt_b: "(no --batch)".to_string(),
                    reason:
                    "Structured output formats (JSON/YAML/TOML) require batch mode (--batch)."
                        .to_string(),
                });
            }
        }

        Ok(())
    }

    fn apply_implicit_dependencies(&mut self) {
        if self.render.human_readable {
            self.render.show_size = true;
        }
        if self.render.show_disk_usage {
            self.render.show_size = true;
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    mod output_format_tests {
        use super::*;

        #[test]
        fn from_extension_recognizes_txt() {
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.txt")),
                Some(OutputFormat::Txt)
            );
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.TXT")),
                Some(OutputFormat::Txt)
            );
            assert_eq!(
                OutputFormat::from_extension(Path::new("FILE.Txt")),
                Some(OutputFormat::Txt)
            );
        }

        #[test]
        fn from_extension_recognizes_json() {
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.json")),
                Some(OutputFormat::Json)
            );
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.JSON")),
                Some(OutputFormat::Json)
            );
        }

        #[test]
        fn from_extension_recognizes_yaml_variants() {
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.yml")),
                Some(OutputFormat::Yaml)
            );
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.yaml")),
                Some(OutputFormat::Yaml)
            );
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.YAML")),
                Some(OutputFormat::Yaml)
            );
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.YML")),
                Some(OutputFormat::Yaml)
            );
        }

        #[test]
        fn from_extension_recognizes_toml() {
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.toml")),
                Some(OutputFormat::Toml)
            );
            assert_eq!(
                OutputFormat::from_extension(Path::new("file.TOML")),
                Some(OutputFormat::Toml)
            );
        }

        #[test]
        fn from_extension_returns_none_for_unknown() {
            assert_eq!(OutputFormat::from_extension(Path::new("file.xyz")), None);
            assert_eq!(OutputFormat::from_extension(Path::new("file")), None);
            assert_eq!(OutputFormat::from_extension(Path::new("")), None);
            assert_eq!(OutputFormat::from_extension(Path::new("file.md")), None);
            assert_eq!(OutputFormat::from_extension(Path::new("file.rs")), None);
            assert_eq!(OutputFormat::from_extension(Path::new(".gitignore")), None);
        }

        #[test]
        fn extension_returns_correct_string() {
            assert_eq!(OutputFormat::Txt.extension(), "txt");
            assert_eq!(OutputFormat::Json.extension(), "json");
            assert_eq!(OutputFormat::Yaml.extension(), "yml");
            assert_eq!(OutputFormat::Toml.extension(), "toml");
        }

        #[test]
        fn default_is_txt() {
            assert_eq!(OutputFormat::default(), OutputFormat::Txt);
        }

        #[test]
        fn formats_are_distinct() {
            let formats = [
                OutputFormat::Txt,
                OutputFormat::Json,
                OutputFormat::Yaml,
                OutputFormat::Toml,
            ];
            for (i, a) in formats.iter().enumerate() {
                for (j, b) in formats.iter().enumerate() {
                    if i == j {
                        assert_eq!(a, b);
                    } else {
                        assert_ne!(a, b);
                    }
                }
            }
        }
    }

    mod charset_mode_tests {
        use super::*;

        #[test]
        fn unicode_returns_unicode_symbols() {
            let mode = CharsetMode::Unicode;
            assert_eq!(mode.branch(), "├─");
            assert_eq!(mode.last_branch(), "└─");
            assert_eq!(mode.vertical(), "│  ");
            assert_eq!(mode.indent(), "   ");
        }

        #[test]
        fn ascii_returns_ascii_symbols() {
            let mode = CharsetMode::Ascii;
            assert_eq!(mode.branch(), "+---");
            assert_eq!(mode.last_branch(), "\\---");
            assert_eq!(mode.vertical(), "|   ");
            assert_eq!(mode.indent(), "    ");
        }

        #[test]
        fn default_is_unicode() {
            assert_eq!(CharsetMode::default(), CharsetMode::Unicode);
        }

        #[test]
        fn vertical_and_indent_have_matching_widths() {
            let unicode = CharsetMode::Unicode;
            assert_eq!(unicode.vertical().chars().count(), unicode.indent().chars().count());

            let ascii = CharsetMode::Ascii;
            assert_eq!(ascii.vertical().len(), ascii.indent().len());
        }

        #[test]
        fn branch_and_last_branch_have_same_visual_width() {
            let ascii = CharsetMode::Ascii;
            assert_eq!(ascii.branch().len(), ascii.last_branch().len());
        }
    }

    mod path_mode_tests {
        use super::*;

        #[test]
        fn default_is_relative() {
            assert_eq!(PathMode::default(), PathMode::Relative);
        }

        #[test]
        fn modes_are_distinct() {
            assert_ne!(PathMode::Relative, PathMode::Full);
        }
    }

    mod scan_options_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let opts = ScanOptions::default();
            assert_eq!(opts.max_depth, None);
            assert!(!opts.show_files);
            assert_eq!(opts.thread_count.get(), 8);
            assert!(!opts.respect_gitignore);
        }

        #[test]
        fn thread_count_is_always_non_zero() {
            let opts = ScanOptions::default();
            assert!(opts.thread_count.get() > 0);
        }

        #[test]
        fn clone_produces_equal_copy() {
            let opts = ScanOptions {
                max_depth: Some(5),
                show_files: true,
                thread_count: NonZeroUsize::new(4).unwrap(),
                respect_gitignore: true,
            };
            let cloned = opts.clone();
            assert_eq!(opts, cloned);
        }
    }

    mod match_options_tests {
        use super::*;

        #[test]
        fn default_is_empty() {
            let opts = MatchOptions::default();
            assert!(opts.include_patterns.is_empty());
            assert!(opts.exclude_patterns.is_empty());
            assert!(!opts.prune_empty);
        }

        #[test]
        fn clone_produces_equal_copy() {
            let opts = MatchOptions {
                include_patterns: vec!["*.rs".to_string()],
                exclude_patterns: vec!["target".to_string()],
                prune_empty: true,
            };
            let cloned = opts.clone();
            assert_eq!(opts, cloned);
        }
    }

    mod render_options_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let opts = RenderOptions::default();
            assert_eq!(opts.charset, CharsetMode::Unicode);
            assert_eq!(opts.path_mode, PathMode::Relative);
            assert!(!opts.show_size);
            assert!(!opts.human_readable);
            assert!(!opts.show_date);
            assert!(!opts.show_disk_usage);
            assert!(!opts.no_indent);
            assert!(!opts.reverse_sort);
            assert!(!opts.show_report);
            assert!(!opts.no_win_banner);
        }
    }

    mod output_options_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let opts = OutputOptions::default();
            assert!(opts.output_path.is_none());
            assert_eq!(opts.format, OutputFormat::Txt);
            assert!(!opts.silent);
        }
    }

    mod config_basic_tests {
        use super::*;

        #[test]
        fn default_has_expected_values() {
            let config = Config::default();
            assert_eq!(config.root_path, PathBuf::from("."));
            assert!(!config.path_explicitly_set);
            assert!(!config.show_help);
            assert!(!config.show_version);
            assert!(!config.batch_mode);
        }

        #[test]
        fn with_root_sets_root_path_and_flag() {
            let config = Config::with_root(PathBuf::from("/some/path"));
            assert_eq!(config.root_path, PathBuf::from("/some/path"));
            assert!(config.path_explicitly_set);
            assert!(!config.show_help);
            assert!(!config.show_version);
        }

        #[test]
        fn clone_produces_equal_copy() {
            let mut config = Config::default();
            config.scan.show_files = true;
            config.render.show_size = true;

            let cloned = config.clone();
            assert_eq!(config, cloned);
        }

        #[test]
        fn debug_is_implemented() {
            let config = Config::default();
            let debug_str = format!("{:?}", config);
            assert!(debug_str.contains("Config"));
            assert!(debug_str.contains("root_path"));
        }
    }

    mod config_is_info_only_tests {
        use super::*;

        #[test]
        fn returns_false_by_default() {
            let config = Config::default();
            assert!(!config.is_info_only());
        }

        #[test]
        fn returns_true_for_help() {
            let mut config = Config::default();
            config.show_help = true;
            assert!(config.is_info_only());
        }

        #[test]
        fn returns_true_for_version() {
            let mut config = Config::default();
            config.show_version = true;
            assert!(config.is_info_only());
        }

        #[test]
        fn returns_true_for_both() {
            let mut config = Config::default();
            config.show_help = true;
            config.show_version = true;
            assert!(config.is_info_only());
        }
    }

    mod config_needs_size_info_tests {
        use super::*;

        #[test]
        fn returns_false_by_default() {
            let config = Config::default();
            assert!(!config.needs_size_info());
        }

        #[test]
        fn returns_true_when_show_size() {
            let mut config = Config::default();
            config.render.show_size = true;
            assert!(config.needs_size_info());
        }

        #[test]
        fn returns_true_when_human_readable() {
            let mut config = Config::default();
            config.render.human_readable = true;
            assert!(config.needs_size_info());
        }

        #[test]
        fn returns_true_when_show_disk_usage() {
            let mut config = Config::default();
            config.render.show_disk_usage = true;
            assert!(config.needs_size_info());
        }

        #[test]
        fn returns_true_when_multiple_size_options() {
            let mut config = Config::default();
            config.render.show_size = true;
            config.render.human_readable = true;
            config.render.show_disk_usage = true;
            assert!(config.needs_size_info());
        }
    }

    mod config_needs_time_info_tests {
        use super::*;

        #[test]
        fn returns_false_by_default() {
            let config = Config::default();
            assert!(!config.needs_time_info());
        }

        #[test]
        fn returns_true_when_show_date() {
            let mut config = Config::default();
            config.render.show_date = true;
            assert!(config.needs_time_info());
        }
    }

    mod config_validate_path_tests {
        use super::*;

        #[test]
        fn fails_for_nonexistent_path() {
            let config = Config::with_root(PathBuf::from("/nonexistent/path/that/does/not/exist"));
            let result = config.validate();
            assert!(result.is_err());

            let err = result.unwrap_err();
            assert!(matches!(err, ConfigError::InvalidPath { .. }));
            if let ConfigError::InvalidPath { reason, .. } = err {
                assert!(reason.contains("Path does not exist"));
            }
        }

        #[test]
        fn fails_for_file_as_root() {
            let config = Config::with_root(PathBuf::from("Cargo.toml"));
            let result = config.validate();
            assert!(result.is_err());

            let err = result.unwrap_err();
            if let ConfigError::InvalidPath { reason, .. } = err {
                assert!(reason.contains("Path is not a directory"));
            } else {
                panic!("Expected InvalidPath error");
            }
        }

        #[test]
        fn succeeds_for_current_directory() {
            let config = Config::default();
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn canonicalizes_path() {
            let config = Config::with_root(PathBuf::from("."));
            let validated = config.validate().unwrap();
            assert!(validated.root_path.is_absolute());
        }

        #[test]
        fn canonicalizes_relative_path() {
            let config = Config::with_root(PathBuf::from("src"));
            let result = config.validate();
            if result.is_ok() {
                let validated = result.unwrap();
                assert!(validated.root_path.is_absolute());
                assert!(validated.root_path.ends_with("src"));
            }
        }
    }

    mod config_validate_format_inference_tests {
        use super::*;

        #[test]
        fn infers_json_format() {
            let mut config = Config::default();
            config.batch_mode = true;
            config.output.output_path = Some(PathBuf::from("tree.json"));
            let validated = config.validate().unwrap();
            assert_eq!(validated.output.format, OutputFormat::Json);
        }

        #[test]
        fn infers_yaml_from_yml() {
            let mut config = Config::default();
            config.batch_mode = true;
            config.output.output_path = Some(PathBuf::from("tree.yml"));
            let validated = config.validate().unwrap();
            assert_eq!(validated.output.format, OutputFormat::Yaml);
        }

        #[test]
        fn infers_yaml_from_yaml() {
            let mut config = Config::default();
            config.batch_mode = true;
            config.output.output_path = Some(PathBuf::from("tree.yaml"));
            let validated = config.validate().unwrap();
            assert_eq!(validated.output.format, OutputFormat::Yaml);
        }

        #[test]
        fn infers_toml_format() {
            let mut config = Config::default();
            config.batch_mode = true;
            config.output.output_path = Some(PathBuf::from("tree.toml"));
            let validated = config.validate().unwrap();
            assert_eq!(validated.output.format, OutputFormat::Toml);
        }

        #[test]
        fn infers_txt_format() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.txt"));
            let validated = config.validate().unwrap();
            assert_eq!(validated.output.format, OutputFormat::Txt);
        }

        #[test]
        fn fails_for_unknown_extension() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.xyz"));
            let result = config.validate();
            assert!(result.is_err());

            let err = result.unwrap_err();
            if let ConfigError::UnknownOutputFormat { path } = err {
                assert_eq!(path, PathBuf::from("tree.xyz"));
            } else {
                panic!("Expected UnknownOutputFormat error");
            }
        }

        #[test]
        fn fails_for_no_extension() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree_output"));
            let result = config.validate();
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ConfigError::UnknownOutputFormat { .. }));
        }
    }

    mod config_validate_conflict_tests {
        use super::*;

        #[test]
        fn fails_silent_without_output() {
            let mut config = Config::default();
            config.output.silent = true;
            let result = config.validate();
            assert!(result.is_err());

            let err = result.unwrap_err();
            if let ConfigError::ConflictingOptions { opt_a, opt_b, .. } = err {
                assert!(opt_a.contains("silent"));
                assert!(opt_b.contains("output"));
            } else {
                panic!("Expected ConflictingOptions error");
            }
        }

        #[test]
        fn succeeds_silent_with_output() {
            let mut config = Config::default();
            config.output.silent = true;
            config.output.output_path = Some(PathBuf::from("tree.txt"));
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn fails_disk_usage_without_batch() {
            let mut config = Config::default();
            config.render.show_disk_usage = true;
            let result = config.validate();
            assert!(result.is_err());

            let err = result.unwrap_err();
            if let ConfigError::ConflictingOptions { opt_a, opt_b, .. } = err {
                assert!(opt_a.contains("disk-usage"));
                assert!(opt_b.contains("batch"));
            } else {
                panic!("Expected ConflictingOptions error");
            }
        }

        #[test]
        fn succeeds_disk_usage_with_batch() {
            let mut config = Config::default();
            config.render.show_disk_usage = true;
            config.batch_mode = true;
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn fails_json_output_without_batch() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.json"));
            let result = config.validate();
            assert!(result.is_err());

            let err = result.unwrap_err();
            if let ConfigError::ConflictingOptions { opt_a, reason, .. } = err {
                assert!(opt_a.contains("Json") || opt_a.contains("json"));
                assert!(reason.contains("batch"));
            } else {
                panic!("Expected ConflictingOptions error");
            }
        }

        #[test]
        fn fails_yaml_output_without_batch() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.yml"));
            let result = config.validate();
            assert!(result.is_err());
        }

        #[test]
        fn fails_toml_output_without_batch() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.toml"));
            let result = config.validate();
            assert!(result.is_err());
        }

        #[test]
        fn succeeds_txt_output_without_batch() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.txt"));
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn succeeds_json_output_with_batch() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.json"));
            config.batch_mode = true;
            let validated = config.validate().unwrap();
            assert_eq!(validated.output.format, OutputFormat::Json);
        }

        #[test]
        fn succeeds_yaml_output_with_batch() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.yaml"));
            config.batch_mode = true;
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn succeeds_toml_output_with_batch() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree.toml"));
            config.batch_mode = true;
            let result = config.validate();
            assert!(result.is_ok());
        }
    }

    mod config_validate_implicit_deps_tests {
        use super::*;

        #[test]
        fn human_readable_enables_show_size() {
            let mut config = Config::default();
            config.render.human_readable = true;
            config.render.show_size = false;
            let validated = config.validate().unwrap();
            assert!(validated.render.show_size);
        }

        #[test]
        fn disk_usage_enables_show_size() {
            let mut config = Config::default();
            config.render.show_disk_usage = true;
            config.render.show_size = false;
            config.batch_mode = true;
            let validated = config.validate().unwrap();
            assert!(validated.render.show_size);
        }

        #[test]
        fn show_size_already_enabled_stays_enabled() {
            let mut config = Config::default();
            config.render.show_size = true;
            config.render.human_readable = true;
            let validated = config.validate().unwrap();
            assert!(validated.render.show_size);
        }
    }

    mod config_batch_mode_tests {
        use super::*;

        #[test]
        fn default_is_false() {
            let config = Config::default();
            assert!(!config.batch_mode);
        }

        #[test]
        fn can_be_enabled() {
            let mut config = Config::default();
            config.batch_mode = true;
            assert!(config.batch_mode);
        }
    }

    mod config_all_options_tests {
        use super::*;

        #[test]
        fn all_options_enabled_validates() {
            let mut config = Config::default();
            config.batch_mode = true;
            config.scan.show_files = true;
            config.scan.max_depth = Some(10);
            config.scan.respect_gitignore = true;
            config.matching.include_patterns = vec!["*.rs".to_string()];
            config.matching.exclude_patterns = vec!["target".to_string()];
            config.matching.prune_empty = true;
            config.render.charset = CharsetMode::Ascii;
            config.render.path_mode = PathMode::Full;
            config.render.show_size = true;
            config.render.human_readable = true;
            config.render.show_date = true;
            config.render.show_disk_usage = true;
            config.render.reverse_sort = true;
            config.render.show_report = true;
            config.render.no_win_banner = true;
            config.output.output_path = Some(PathBuf::from("tree.json"));

            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn minimal_config_validates() {
            let config = Config::default();
            let result = config.validate();
            assert!(result.is_ok());
        }
    }

    mod config_error_tests {
        use super::*;

        #[test]
        fn conflicting_options_displays_correctly() {
            let err = ConfigError::ConflictingOptions {
                opt_a: "--foo".to_string(),
                opt_b: "--bar".to_string(),
                reason: "mutually exclusive".to_string(),
            };
            let msg = err.to_string();
            assert!(msg.contains("--foo"));
            assert!(msg.contains("--bar"));
            assert!(msg.contains("mutually exclusive"));
        }

        #[test]
        fn invalid_value_displays_correctly() {
            let err = ConfigError::InvalidValue {
                option: "--depth".to_string(),
                value: "-1".to_string(),
                reason: "must be a positive integer".to_string(),
            };
            let msg = err.to_string();
            assert!(msg.contains("--depth"));
            assert!(msg.contains("-1"));
            assert!(msg.contains("must be a positive integer"));
        }

        #[test]
        fn invalid_path_displays_correctly() {
            let err = ConfigError::InvalidPath {
                path: PathBuf::from("/invalid/path"),
                reason: "Path does not exist".to_string(),
            };
            let msg = err.to_string();
            assert!(msg.contains("invalid") && msg.contains("path"));
            assert!(msg.contains("Path does not exist"));
        }

        #[test]
        fn unknown_output_format_displays_correctly() {
            let err = ConfigError::UnknownOutputFormat {
                path: PathBuf::from("output.xyz"),
            };
            let msg = err.to_string();
            assert!(msg.contains("output.xyz"));
            assert!(msg.contains(".txt"));
            assert!(msg.contains(".json"));
        }

        #[test]
        fn errors_are_clone_and_eq() {
            let err1 = ConfigError::ConflictingOptions {
                opt_a: "a".to_string(),
                opt_b: "b".to_string(),
                reason: "r".to_string(),
            };
            let err2 = err1.clone();
            assert_eq!(err1, err2);
        }

        #[test]
        fn different_errors_are_not_equal() {
            let err1 = ConfigError::ConflictingOptions {
                opt_a: "a".to_string(),
                opt_b: "b".to_string(),
                reason: "r".to_string(),
            };
            let err2 = ConfigError::ConflictingOptions {
                opt_a: "x".to_string(),
                opt_b: "y".to_string(),
                reason: "z".to_string(),
            };
            assert_ne!(err1, err2);
        }

        #[test]
        fn different_error_variants_are_not_equal() {
            let err1 = ConfigError::InvalidPath {
                path: PathBuf::from("/path"),
                reason: "reason".to_string(),
            };
            let err2 = ConfigError::UnknownOutputFormat {
                path: PathBuf::from("/path"),
            };
            assert_ne!(err1, err2);
        }
    }

    mod config_edge_cases_tests {
        use super::*;

        #[test]
        fn empty_patterns_are_valid() {
            let mut config = Config::default();
            config.matching.include_patterns = vec![];
            config.matching.exclude_patterns = vec![];
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn zero_max_depth_is_valid() {
            let mut config = Config::default();
            config.scan.max_depth = Some(0);
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn large_max_depth_is_valid() {
            let mut config = Config::default();
            config.scan.max_depth = Some(usize::MAX);
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn various_thread_counts_are_valid() {
            for count in [1, 2, 4, 8, 16, 32, 64, 128] {
                let mut config = Config::default();
                config.scan.thread_count = NonZeroUsize::new(count).unwrap();
                let result = config.validate();
                assert!(result.is_ok(), "thread count {} should be valid", count);
            }
        }

        #[test]
        fn output_with_deep_path_validates() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("a/b/c/d/e/f/g/tree.txt"));
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn output_with_special_chars_validates() {
            let mut config = Config::default();
            config.output.output_path = Some(PathBuf::from("tree-output_2024.txt"));
            let result = config.validate();
            assert!(result.is_ok());
        }

        #[test]
        fn multiple_patterns_are_valid() {
            let mut config = Config::default();
            config.matching.include_patterns = vec![
                "*.rs".to_string(),
                "*.toml".to_string(),
                "*.md".to_string(),
            ];
            config.matching.exclude_patterns = vec![
                "target".to_string(),
                "node_modules".to_string(),
                ".git".to_string(),
            ];
            let result = config.validate();
            assert!(result.is_ok());
        }
    }
}
