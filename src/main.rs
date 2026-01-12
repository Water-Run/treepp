//! tree++ main program entry point.
//!
//! This module implements the main entry point for the `tree++` command-line tool,
//! orchestrating the following pipeline:
//!
//! 1. **CLI Parsing**: Parse command-line arguments, producing a `ParseResult`
//! 2. **Configuration Validation**: Validate configuration and populate derived fields
//! 3. **Directory Scanning**: Use streaming scan-render-output when possible, otherwise build complete tree
//! 4. **Tree Rendering**: Choose streaming or batch rendering based on scan mode
//! 5. **Result Output**: Output to stdout and/or file
//!
//! # Exit Codes
//!
//! | Code | Meaning |
//! |------|---------|
//! | `0`  | Success |
//! | `1`  | CLI/argument error |
//! | `2`  | Scan error |
//! | `3`  | Output error |
//!
//! File: src/main.rs
//! Author: WaterRun
//! Date: 2026-01-12

#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(missing_docs)]
#![allow(dead_code)]

mod cli;
mod config;
mod error;
mod output;
mod render;
mod scan;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Component, Path};
use std::process::ExitCode;

use cli::{CliError, CliParser, ParseResult};
use config::Config;
use error::{OutputError, ScanError, TreeppError};
use render::{StreamRenderConfig, StreamRenderer, TreeChars, WinBanner};
use scan::{EntryKind, StreamEvent};

/// Exit code indicating successful execution.
const EXIT_SUCCESS: u8 = 0;

/// Exit code indicating a CLI or argument parsing error.
const EXIT_CLI_ERROR: u8 = 1;

/// Exit code indicating a directory scanning error.
const EXIT_SCAN_ERROR: u8 = 2;

/// Exit code indicating an output writing error.
const EXIT_OUTPUT_ERROR: u8 = 3;

/// Program main entry point.
///
/// Parses command-line arguments and executes the appropriate action.
///
/// # Returns
///
/// Returns an `ExitCode` reflecting the execution result:
/// - `EXIT_SUCCESS` (0) on success
/// - `EXIT_CLI_ERROR` (1) on argument errors
/// - `EXIT_SCAN_ERROR` (2) on scan errors
/// - `EXIT_OUTPUT_ERROR` (3) on output errors
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::from(EXIT_SUCCESS),
        Err(e) => {
            let code = error_to_exit_code(&e);
            print_error(&e);
            ExitCode::from(code)
        }
    }
}

/// Executes the main workflow.
///
/// Selects between batch mode and streaming mode based on configuration.
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `TreeppError` on failure.
///
/// # Errors
///
/// Returns an error if:
/// - CLI parsing fails
/// - Configuration validation fails
/// - Directory scanning fails
/// - Output writing fails
fn run() -> Result<(), TreeppError> {
    let parser = CliParser::from_env();
    let parse_result = parser.parse()?;

    match parse_result {
        ParseResult::Help => {
            cli::print_help();
            Ok(())
        }
        ParseResult::Version => {
            cli::print_version();
            Ok(())
        }
        ParseResult::Config(config) => {
            if config.batch_mode {
                batch_mode(&config)
            } else {
                stream_mode(&config)
            }
        }
    }
}

/// Executes the batch processing pipeline.
///
/// Performs a complete scan of the directory tree, then renders and outputs
/// the result. This mode is required for structured output formats (JSON,
/// YAML, TOML) and disk usage calculation.
///
/// # Arguments
///
/// * `config` - The validated configuration specifying scan and render options.
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `TreeppError` on failure.
///
/// # Errors
///
/// Returns an error if:
/// - Directory scanning fails
/// - Output writing fails
fn batch_mode(config: &Config) -> Result<(), TreeppError> {
    let stats = scan::scan(config)?;
    let render_result = render::render(&stats, config);
    output::execute_output(&render_result, &stats.tree, config)?;
    Ok(())
}

/// Executes the streaming pipeline.
///
/// Scans, renders, and outputs the directory tree simultaneously for
/// improved responsiveness. This mode has the following constraints:
///
/// - Always outputs TXT format (JSON/YAML/TOML require batch mode)
/// - If an output file is specified, writes to both file and stdout (unless silent)
/// - `disk_usage` is unavailable (requires batch mode)
///
/// # Arguments
///
/// * `config` - The validated configuration specifying scan and render options.
///
/// # Returns
///
/// Returns `Ok(())` on success, or a `TreeppError` on failure.
///
/// # Errors
///
/// Returns an error if:
/// - Output file creation fails
/// - Directory scanning fails
/// - Writing to file or stdout fails
fn stream_mode(config: &Config) -> Result<(), TreeppError> {
    let mut file_writer = create_file_writer_if_needed(config)?;
    let mut output_context = StreamOutputContext::new(config, &mut file_writer);

    let mut renderer = StreamRenderer::new(StreamRenderConfig::from_config(config));
    let chars = TreeChars::from_charset(config.render.charset);

    let header = renderer.render_header(&config.root_path, config.path_explicitly_set);
    output_context.write(&header)?;

    let mut has_subdirs = false;
    let mut has_files = false;

    let stats = scan::scan_streaming(config, |event| {
        handle_stream_event(
            event,
            &mut renderer,
            &mut output_context,
            &mut has_subdirs,
            &mut has_files,
        )
    })?;

    render_empty_directory_notice(config, &chars, has_subdirs, has_files, &mut output_context)?;

    if config.render.show_report {
        let report =
            renderer.render_report(stats.directory_count, stats.file_count, stats.duration);
        if !report.is_empty() {
            output_context.write(&report)?;
        }
    }

    output_context.flush()?;
    print_output_path_notice(config);

    Ok(())
}

/// Creates a buffered file writer if an output path is configured.
///
/// # Arguments
///
/// * `config` - The configuration containing the optional output path.
///
/// # Returns
///
/// Returns `Some(BufWriter<File>)` if an output path is specified,
/// `None` otherwise.
///
/// # Errors
///
/// Returns an error if the file cannot be created.
fn create_file_writer_if_needed(config: &Config) -> Result<Option<BufWriter<File>>, TreeppError> {
    match config.output.output_path {
        Some(ref path) => {
            let file = File::create(path).map_err(|e| OutputError::FileCreateFailed {
                path: path.clone(),
                source: e,
            })?;
            Ok(Some(BufWriter::new(file)))
        }
        None => Ok(None),
    }
}

/// Context for managing streaming output to stdout and optional file.
struct StreamOutputContext<'a> {
    /// Reference to the configuration.
    config: &'a Config,
    /// Mutable reference to an optional file writer.
    file_writer: &'a mut Option<BufWriter<File>>,
}

impl<'a> StreamOutputContext<'a> {
    /// Creates a new streaming output context.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration controlling output behavior.
    /// * `file_writer` - Mutable reference to an optional file writer.
    ///
    /// # Returns
    ///
    /// Returns a new `StreamOutputContext` instance.
    fn new(config: &'a Config, file_writer: &'a mut Option<BufWriter<File>>) -> Self {
        Self {
            config,
            file_writer,
        }
    }

    /// Writes content to stdout (unless silent) and file (if configured).
    ///
    /// # Arguments
    ///
    /// * `content` - The string content to write.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the file fails.
    fn write(&mut self, content: &str) -> Result<(), TreeppError> {
        if !self.config.output.silent {
            print!("{}", content);
        }
        if let Some(writer) = self.file_writer.as_mut() {
            write!(writer, "{}", content).map_err(|e| OutputError::WriteFailed {
                path: self.config.output.output_path.clone().unwrap(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Writes a line to stdout (unless silent) and file (if configured).
    ///
    /// # Arguments
    ///
    /// * `content` - The string content to write, followed by a newline.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the file fails.
    fn writeln(&mut self, content: &str) -> Result<(), TreeppError> {
        if !self.config.output.silent {
            println!("{}", content);
        }
        if let Some(writer) = self.file_writer.as_mut() {
            writeln!(writer, "{}", content).map_err(|e| OutputError::WriteFailed {
                path: self.config.output.output_path.clone().unwrap(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Writes an empty line to stdout (unless silent) and file (if configured).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the file fails.
    fn writeln_empty(&mut self) -> Result<(), TreeppError> {
        if !self.config.output.silent {
            println!();
        }
        if let Some(writer) = self.file_writer.as_mut() {
            writeln!(writer).map_err(|e| OutputError::WriteFailed {
                path: self.config.output.output_path.clone().unwrap(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Flushes the file writer buffer if present.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if flushing fails.
    fn flush(&mut self) -> Result<(), TreeppError> {
        if let Some(writer) = self.file_writer.as_mut() {
            writer.flush().map_err(|e| OutputError::WriteFailed {
                path: self.config.output.output_path.clone().unwrap(),
                source: e,
            })?;
        }
        Ok(())
    }
}

/// Handles a single stream event during streaming scan.
///
/// Processes directory entry events, directory enter/leave events,
/// and writes rendered output accordingly.
///
/// # Arguments
///
/// * `event` - The stream event to process.
/// * `renderer` - The stream renderer for generating output lines.
/// * `output_context` - The output context for writing results.
/// * `has_subdirs` - Mutable flag tracking whether subdirectories were found.
/// * `has_files` - Mutable flag tracking whether files were found.
///
/// # Returns
///
/// Returns `Ok(())` on success.
///
/// # Errors
///
/// Returns an error if writing output fails.
fn handle_stream_event(
    event: StreamEvent,
    renderer: &mut StreamRenderer,
    output_context: &mut StreamOutputContext<'_>,
    has_subdirs: &mut bool,
    has_files: &mut bool,
) -> Result<(), ScanError> {
    match event {
        StreamEvent::Entry(ref entry) => {
            if entry.kind == EntryKind::Directory {
                *has_subdirs = true;
            } else {
                *has_files = true;
            }

            let line = renderer.render_entry(&entry.clone());
            for l in line.lines() {
                if !output_context.config.output.silent {
                    println!("{}", l);
                }
                if let Some(writer) = output_context.file_writer.as_mut() {
                    writeln!(writer, "{}", l).map_err(|e| ScanError::WalkError {
                        message: e.to_string(),
                        path: None,
                    })?;
                }
            }
        }
        StreamEvent::EnterDir { is_last } => {
            renderer.push_level(!is_last);
        }
        StreamEvent::LeaveDir => {
            if let Some(trailing) = renderer.pop_level() {
                if !output_context.config.output.silent {
                    println!("{}", trailing);
                }
                if let Some(writer) = output_context.file_writer.as_mut() {
                    writeln!(writer, "{}", trailing).map_err(|e| ScanError::WalkError {
                        message: e.to_string(),
                        path: None,
                    })?;
                }
            }
        }
    }
    Ok(())
}

/// Renders the "no subfolders" notice for empty directories.
///
/// Mimics the behavior of the native Windows `tree` command when a directory
/// contains no subdirectories.
///
/// # Arguments
///
/// * `config` - The configuration specifying render options.
/// * `chars` - The tree characters for formatting.
/// * `has_subdirs` - Whether subdirectories were found.
/// * `has_files` - Whether files were found.
/// * `output_context` - The output context for writing results.
///
/// # Returns
///
/// Returns `Ok(())` on success.
///
/// # Errors
///
/// Returns an error if writing output fails.
fn render_empty_directory_notice(
    config: &Config,
    chars: &TreeChars,
    has_subdirs: bool,
    has_files: bool,
    output_context: &mut StreamOutputContext<'_>,
) -> Result<(), TreeppError> {
    if has_subdirs || config.render.no_win_banner {
        return Ok(());
    }

    if let Some(drive) = drive_letter_from_path(&config.root_path) {
        if let Ok(banner) = WinBanner::fetch_for_drive(drive) {
            if has_files && config.scan.show_files && !config.render.no_indent {
                output_context.writeln(&chars.space)?;
            }

            if !banner.no_subfolder.is_empty() {
                output_context.writeln(&banner.no_subfolder)?;
            }
        }
    }

    output_context.writeln_empty()?;
    Ok(())
}

/// Prints the output file path notice if applicable.
///
/// Informs the user where the output was written when an output file
/// was specified and silent mode is not enabled.
///
/// # Arguments
///
/// * `config` - The configuration containing the optional output path.
fn print_output_path_notice(config: &Config) {
    if let Some(ref path) = config.output.output_path {
        if !config.output.silent {
            println!("\nOutput written to: {}", path.display());
        }
    }
}

/// Extracts the drive letter (uppercase) from a path.
///
/// Handles both standard paths (e.g., `C:\`) and long path format
/// (e.g., `\\?\C:\`).
///
/// # Arguments
///
/// * `path` - The path to extract the drive letter from.
///
/// # Returns
///
/// Returns `Some(char)` containing the uppercase drive letter if found,
/// `None` otherwise.
///
/// # Examples
///
/// ```rust,ignore
/// use std::path::Path;
///
/// let path = Path::new("C:\\Users");
/// assert_eq!(drive_letter_from_path(path), Some('C'));
/// ```
fn drive_letter_from_path(path: &Path) -> Option<char> {
    let first_component = path.components().next()?;

    if let Component::Prefix(prefix) = first_component {
        let prefix_str = prefix.as_os_str().to_string_lossy();
        let chars: Vec<char> = prefix_str.chars().collect();

        if chars.len() >= 2 && chars[1] == ':' {
            return Some(chars[0].to_ascii_uppercase());
        }

        if prefix_str.starts_with(r"\\?\") && chars.len() >= 6 && chars[5] == ':' {
            return Some(chars[4].to_ascii_uppercase());
        }
    }

    None
}

/// Maps an error to its corresponding exit code.
///
/// # Arguments
///
/// * `err` - The error to map.
///
/// # Returns
///
/// Returns the appropriate exit code for the error type:
/// - `EXIT_CLI_ERROR` for CLI and config errors
/// - `EXIT_SCAN_ERROR` for scan and match errors
/// - `EXIT_OUTPUT_ERROR` for render and output errors
fn error_to_exit_code(err: &TreeppError) -> u8 {
    match err {
        TreeppError::Cli(_) | TreeppError::Config(_) => EXIT_CLI_ERROR,
        TreeppError::Scan(_) | TreeppError::Match(_) => EXIT_SCAN_ERROR,
        TreeppError::Render(_) | TreeppError::Output(_) => EXIT_OUTPUT_ERROR,
    }
}

/// Prints a formatted error message to stderr.
///
/// Provides user-friendly error output with contextual hints for common
/// error scenarios.
///
/// # Arguments
///
/// * `err` - The error to print.
fn print_error(err: &TreeppError) {
    let prefix = match err {
        TreeppError::Cli(_) => "CLI error",
        TreeppError::Config(_) => "Config error",
        TreeppError::Scan(_) => "Scan error",
        TreeppError::Match(_) => "Match error",
        TreeppError::Render(_) => "Render error",
        TreeppError::Output(_) => "Output error",
    };

    eprintln!("tree++: {}: {}", prefix, err);

    match err {
        TreeppError::Cli(CliError::UnknownOption { .. }) => {
            eprintln!("Hint: run `treepp --help` to list available options");
        }
        TreeppError::Cli(CliError::MultiplePaths { .. }) => {
            eprintln!("Hint: only one target path can be specified.");
        }
        _ => {}
    }
}
