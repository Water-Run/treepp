//! Command-line argument parsing module.
//!
//! This module implements the argument parsing functionality for the `tree++` command-line tool,
//! supporting three argument styles that can be mixed freely:
//!
//! - Windows CMD style (`/F`), case-insensitive
//! - Unix short argument style (`-f`), case-sensitive
//! - GNU long argument style (`--files`), case-sensitive
//!
//! After parsing, a [`Config`] struct is produced for use by subsequent scan, match, render,
//! and output modules.
//!
//! # Examples
//!
//! ```no_run
//! use treepp::cli::{CliParser, ParseResult};
//!
//! let args = vec!["D:\\project".to_string(), "/F".to_string(), "--ascii".to_string()];
//! let parser = CliParser::new(args);
//! match parser.parse() {
//!     Ok(ParseResult::Config(config)) => println!("{:?}", config),
//!     Ok(ParseResult::Help) => println!("Show help"),
//!     Ok(ParseResult::Version) => println!("Show version"),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! File: src/cli.rs
//! Author: WaterRun
//! Date: 2026-01-26

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::env;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use crate::config::{CharsetMode, Config, PathMode};
pub(crate) use crate::error::CliError;

// ============================================================================
// Parse Result
// ============================================================================

/// Result of command-line parsing.
///
/// Represents the three possible outcomes after parsing command-line arguments.
///
/// # Variants
///
/// * `Config` - Normal configuration, scanning should be executed
/// * `Help` - User requested help information display
/// * `Version` - User requested version information display
///
/// # Examples
///
/// ```no_run
/// use treepp::cli::{CliParser, ParseResult};
///
/// let parser = CliParser::new(vec!["--help".to_string()]);
/// match parser.parse() {
///     Ok(ParseResult::Help) => println!("Show help"),
///     Ok(ParseResult::Version) => println!("Show version"),
///     Ok(ParseResult::Config(c)) => println!("Config: {:?}", c),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
#[derive(Debug)]
pub enum ParseResult {
    /// Normal configuration, scanning should be executed.
    Config(Config),
    /// User requested help information display.
    Help,
    /// User requested version information display.
    Version,
}

// ============================================================================
// Argument Definition Types
// ============================================================================

/// Argument type classification.
///
/// Determines whether an argument is a simple flag or requires a value.
///
/// # Variants
///
/// * `Flag` - Boolean flag that takes no value
/// * `Value` - Argument that requires a following value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    /// Boolean flag that takes no value.
    Flag,
    /// Argument that requires a following value.
    Value,
}

/// Definition of a command-line argument.
///
/// Contains all patterns that can match this argument across the three supported styles.
///
/// # Fields
///
/// * `canonical` - Canonical name used for duplicate detection and error messages
/// * `kind` - Whether this argument is a flag or requires a value
/// * `cmd_patterns` - Windows CMD style patterns (`/X`), matched case-insensitively
/// * `short_patterns` - Unix short patterns (`-x`), matched case-sensitively
/// * `long_patterns` - GNU long patterns (`--xxx`), matched case-sensitively
struct ArgDef {
    canonical: &'static str,
    kind: ArgKind,
    cmd_patterns: &'static [&'static str],
    short_patterns: &'static [&'static str],
    long_patterns: &'static [&'static str],
}

/// All supported argument definitions.
///
/// Arguments are organized by category for maintainability.
const ARG_DEFINITIONS: &[ArgDef] = &[
    // Information display
    ArgDef {
        canonical: "help",
        kind: ArgKind::Flag,
        cmd_patterns: &["/?"],
        short_patterns: &["-h"],
        long_patterns: &["--help"],
    },
    ArgDef {
        canonical: "version",
        kind: ArgKind::Flag,
        cmd_patterns: &["/V"],
        short_patterns: &["-v"],
        long_patterns: &["--version"],
    },
    // Display content
    ArgDef {
        canonical: "files",
        kind: ArgKind::Flag,
        cmd_patterns: &["/F"],
        short_patterns: &["-f"],
        long_patterns: &["--files"],
    },
    ArgDef {
        canonical: "full-path",
        kind: ArgKind::Flag,
        cmd_patterns: &["/FP"],
        short_patterns: &["-p"],
        long_patterns: &["--full-path"],
    },
    ArgDef {
        canonical: "size",
        kind: ArgKind::Flag,
        cmd_patterns: &["/S"],
        short_patterns: &["-s"],
        long_patterns: &["--size"],
    },
    ArgDef {
        canonical: "human-readable",
        kind: ArgKind::Flag,
        cmd_patterns: &["/HR"],
        short_patterns: &["-H"],
        long_patterns: &["--human-readable"],
    },
    ArgDef {
        canonical: "date",
        kind: ArgKind::Flag,
        cmd_patterns: &["/DT"],
        short_patterns: &["-d"],
        long_patterns: &["--date"],
    },
    ArgDef {
        canonical: "disk-usage",
        kind: ArgKind::Flag,
        cmd_patterns: &["/DU"],
        short_patterns: &["-u"],
        long_patterns: &["--disk-usage"],
    },
    // Rendering style
    ArgDef {
        canonical: "ascii",
        kind: ArgKind::Flag,
        cmd_patterns: &["/A"],
        short_patterns: &["-a"],
        long_patterns: &["--ascii"],
    },
    ArgDef {
        canonical: "no-indent",
        kind: ArgKind::Flag,
        cmd_patterns: &["/NI"],
        short_patterns: &["-i"],
        long_patterns: &["--no-indent"],
    },
    ArgDef {
        canonical: "reverse",
        kind: ArgKind::Flag,
        cmd_patterns: &["/R"],
        short_patterns: &["-r"],
        long_patterns: &["--reverse"],
    },
    // Filtering
    ArgDef {
        canonical: "level",
        kind: ArgKind::Value,
        cmd_patterns: &["/L"],
        short_patterns: &["-L"],
        long_patterns: &["--level"],
    },
    ArgDef {
        canonical: "include",
        kind: ArgKind::Value,
        cmd_patterns: &["/M"],
        short_patterns: &["-m"],
        long_patterns: &["--include"],
    },
    ArgDef {
        canonical: "exclude",
        kind: ArgKind::Value,
        cmd_patterns: &["/X"],
        short_patterns: &["-I"],
        long_patterns: &["--exclude"],
    },
    ArgDef {
        canonical: "gitignore",
        kind: ArgKind::Flag,
        cmd_patterns: &["/G"],
        short_patterns: &["-g"],
        long_patterns: &["--gitignore"],
    },
    ArgDef {
        canonical: "all",
        kind: ArgKind::Flag,
        cmd_patterns: &["/AL"],
        short_patterns: &["-k"],
        long_patterns: &["--all"],
    },
    // Output control
    ArgDef {
        canonical: "report",
        kind: ArgKind::Flag,
        cmd_patterns: &["/RP"],
        short_patterns: &["-e"],
        long_patterns: &["--report"],
    },
    ArgDef {
        canonical: "no-win-banner",
        kind: ArgKind::Flag,
        cmd_patterns: &["/NB"],
        short_patterns: &["-N"],
        long_patterns: &["--no-win-banner"],
    },
    ArgDef {
        canonical: "silent",
        kind: ArgKind::Flag,
        cmd_patterns: &["/SI"],
        short_patterns: &["-l"],
        long_patterns: &["--silent"],
    },
    ArgDef {
        canonical: "output",
        kind: ArgKind::Value,
        cmd_patterns: &["/O"],
        short_patterns: &["-o"],
        long_patterns: &["--output"],
    },
    // Mode
    ArgDef {
        canonical: "batch",
        kind: ArgKind::Flag,
        cmd_patterns: &["/B"],
        short_patterns: &["-b"],
        long_patterns: &["--batch"],
    },
    // Performance
    ArgDef {
        canonical: "thread",
        kind: ArgKind::Value,
        cmd_patterns: &["/T"],
        short_patterns: &["-t"],
        long_patterns: &["--thread"],
    },
];

/// Arguments that can be specified multiple times.
const ACCUMULATIVE_OPTIONS: &[&str] = &["include", "exclude"];

// ============================================================================
// Matched Argument
// ============================================================================

/// Result of matching an argument to a definition.
///
/// Contains the matched definition and optionally the parsed value.
struct MatchedArg {
    definition: &'static ArgDef,
    value: Option<String>,
}

// ============================================================================
// CLI Parser
// ============================================================================

/// Command-line argument parser.
///
/// Supports three argument styles that can be mixed freely:
/// - Windows CMD style (`/F`), case-insensitive
/// - Unix short argument style (`-f`), case-sensitive
/// - GNU long argument style (`--files`), case-sensitive
///
/// # Path Position Rules
///
/// Path arguments can appear at any position, including before, after, or between options.
///
/// # Examples
///
/// ```no_run
/// use treepp::cli::CliParser;
///
/// // Create from environment arguments
/// let parser = CliParser::from_env();
///
/// // Or specify arguments manually
/// let parser = CliParser::new(vec!["/F".to_string(), "--ascii".to_string()]);
/// ```
pub struct CliParser {
    args: Vec<String>,
    position: usize,
    seen_canonical_names: HashSet<String>,
    thread_explicitly_set: bool,
}

impl CliParser {
    /// Creates a new parser from an argument list.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments (excluding the program name)
    ///
    /// # Returns
    ///
    /// A new `CliParser` instance ready for parsing.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::cli::CliParser;
    ///
    /// let parser = CliParser::new(vec!["/F".to_string(), "--level".to_string(), "3".to_string()]);
    /// ```
    #[must_use]
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            position: 0,
            seen_canonical_names: HashSet::new(),
            thread_explicitly_set: false,
        }
    }

    /// Creates a parser from environment arguments.
    ///
    /// Automatically skips the program name (first argument).
    ///
    /// # Returns
    ///
    /// A new `CliParser` instance initialized with environment arguments.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::cli::CliParser;
    ///
    /// let parser = CliParser::from_env();
    /// let result = parser.parse();
    /// ```
    #[must_use]
    pub fn from_env() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        Self::new(args)
    }

    /// Parses command-line arguments.
    ///
    /// After parsing, calls `Config::validate()` to verify configuration validity.
    ///
    /// # Path Position Rules
    ///
    /// Path arguments can appear at any position. For example:
    /// - `treepp C:\dir /F` ✓
    /// - `treepp /F C:\dir` ✓
    /// - `treepp /F C:\dir --ascii` ✓
    ///
    /// # Returns
    ///
    /// * `Ok(ParseResult)` - Successfully parsed result
    /// * `Err(CliError)` - Parsing error
    ///
    /// # Errors
    ///
    /// * `CliError::UnknownOption` - Encountered unknown argument
    /// * `CliError::MissingValue` - Value-requiring argument missing its value
    /// * `CliError::InvalidValue` - Invalid argument value
    /// * `CliError::DuplicateOption` - Duplicate argument
    /// * `CliError::MultiplePaths` - Multiple paths specified
    /// * `CliError::ConflictingOptions` - Conflicting arguments (e.g., `--thread` without `--batch`)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::cli::{CliParser, ParseResult};
    ///
    /// let parser = CliParser::new(vec!["/F".to_string()]);
    /// match parser.parse() {
    ///     Ok(ParseResult::Config(config)) => {
    ///         assert!(config.scan.show_files);
    ///     }
    ///     _ => panic!("Parse failed"),
    /// }
    /// ```
    pub fn parse(mut self) -> Result<ParseResult, CliError> {
        let mut config = Config::default();
        let mut collected_paths: Vec<String> = Vec::new();

        while self.position < self.args.len() {
            let current_arg = self.args[self.position].clone();

            if Self::is_option_like(&current_arg) {
                let matched = self.try_match_argument(&current_arg)?;

                if !ACCUMULATIVE_OPTIONS.contains(&matched.definition.canonical) {
                    self.register_canonical_name(matched.definition.canonical)?;
                }

                self.apply_to_config(&mut config, &matched)?;

                if matched.definition.canonical == "help" {
                    return Ok(ParseResult::Help);
                }
                if matched.definition.canonical == "version" {
                    return Ok(ParseResult::Version);
                }
            } else {
                collected_paths.push(current_arg);
            }

            self.position += 1;
        }

        self.validate_paths(&collected_paths, &mut config)?;

        if self.thread_explicitly_set && !config.batch_mode {
            return Err(CliError::ConflictingOptions {
                opt_a: "--thread".to_string(),
                opt_b: "(no --batch)".to_string(),
            });
        }

        let validated_config = config.validate().map_err(|e| CliError::ParseError {
            message: e.to_string(),
        })?;

        Ok(ParseResult::Config(validated_config))
    }

    /// Determines if a string looks like an option argument.
    ///
    /// # Arguments
    ///
    /// * `arg` - The argument string to check
    ///
    /// # Returns
    ///
    /// `true` if the argument starts with `-` or `/`, indicating it's likely an option.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::cli::CliParser;
    ///
    /// // These are not accessible directly, but the logic is:
    /// // "-f" -> true (starts with -)
    /// // "/F" -> true (starts with /)
    /// // "path" -> false (regular path)
    /// ```
    fn is_option_like(arg: &str) -> bool {
        arg.starts_with('-') || arg.starts_with('/')
    }

    /// Attempts to match an argument to a known definition.
    ///
    /// Iterates through all argument definitions and attempts to match.
    ///
    /// # Arguments
    ///
    /// * `arg` - The argument string to match
    ///
    /// # Returns
    ///
    /// * `Ok(MatchedArg)` - Successfully matched argument
    /// * `Err(CliError::UnknownOption)` - No matching definition found
    fn try_match_argument(&mut self, arg: &str) -> Result<MatchedArg, CliError> {
        for def in ARG_DEFINITIONS {
            if let Some(matched) = self.try_match_definition(arg, def)? {
                return Ok(matched);
            }
        }
        Err(CliError::UnknownOption {
            option: arg.to_string(),
        })
    }

    /// Attempts to match an argument against a specific definition.
    ///
    /// Checks all three styles: CMD (case-insensitive), Unix short, and GNU long (both case-sensitive).
    ///
    /// # Arguments
    ///
    /// * `arg` - The argument string to match
    /// * `def` - The definition to match against
    ///
    /// # Returns
    ///
    /// * `Ok(Some(MatchedArg))` - Successfully matched
    /// * `Ok(None)` - No match for this definition
    /// * `Err(CliError)` - Error during matching (e.g., missing value)
    fn try_match_definition(
        &mut self,
        arg: &str,
        def: &'static ArgDef,
    ) -> Result<Option<MatchedArg>, CliError> {
        let arg_upper = arg.to_uppercase();

        for pattern in def.cmd_patterns {
            if arg_upper == pattern.to_uppercase() {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }
        }

        for pattern in def.short_patterns {
            if arg == *pattern {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }
        }

        for pattern in def.long_patterns {
            if arg == *pattern {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }

            let equals_prefix = format!("{pattern}=");
            if arg.starts_with(&equals_prefix) && def.kind == ArgKind::Value {
                let value = arg[equals_prefix.len()..].to_string();
                return Ok(Some(MatchedArg {
                    definition: def,
                    value: Some(value),
                }));
            }
        }

        Ok(None)
    }

    /// Consumes the next argument as a value if required.
    ///
    /// For value-type arguments, reads the next argument as the value.
    /// For flag-type arguments, returns `None`.
    ///
    /// # Arguments
    ///
    /// * `def` - The argument definition
    /// * `arg` - The original argument string (for error messages)
    ///
    /// # Returns
    ///
    /// * `Ok(Some(String))` - Value for value-type arguments
    /// * `Ok(None)` - No value for flag-type arguments
    /// * `Err(CliError::MissingValue)` - Value required but not provided
    fn consume_value_if_required(
        &mut self,
        def: &ArgDef,
        arg: &str,
    ) -> Result<Option<String>, CliError> {
        if def.kind == ArgKind::Flag {
            return Ok(None);
        }

        let next_position = self.position + 1;
        if next_position >= self.args.len() {
            return Err(CliError::MissingValue {
                option: arg.to_string(),
            });
        }

        let next_arg = &self.args[next_position];
        if Self::is_option_like(next_arg) {
            return Err(CliError::MissingValue {
                option: arg.to_string(),
            });
        }

        self.position += 1;
        Ok(Some(next_arg.clone()))
    }

    /// Registers a canonical name and checks for duplicates.
    ///
    /// # Arguments
    ///
    /// * `canonical` - The canonical name to register
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully registered
    /// * `Err(CliError::DuplicateOption)` - Name already registered
    fn register_canonical_name(&mut self, canonical: &str) -> Result<(), CliError> {
        if !self.seen_canonical_names.insert(canonical.to_string()) {
            return Err(CliError::DuplicateOption {
                option: canonical.to_string(),
            });
        }
        Ok(())
    }

    /// Applies a matched argument to the configuration.
    ///
    /// Updates the appropriate field in the config based on the matched argument.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration to update
    /// * `matched` - The matched argument with its value
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Successfully applied
    /// * `Err(CliError::InvalidValue)` - Invalid value for the argument
    fn apply_to_config(
        &mut self,
        config: &mut Config,
        matched: &MatchedArg,
    ) -> Result<(), CliError> {
        let canonical = matched.definition.canonical;

        match canonical {
            "help" => config.show_help = true,
            "version" => config.show_version = true,
            "batch" => config.batch_mode = true,
            "files" => config.scan.show_files = true,
            "gitignore" => config.scan.respect_gitignore = true,
            "all" => config.scan.show_hidden = true,
            "level" => {
                let value = matched.value.as_ref().expect("level requires a value");
                let depth: usize = value.parse().map_err(|_| CliError::InvalidValue {
                    option: canonical.to_string(),
                    value: value.clone(),
                    reason: "must be a positive integer".to_string(),
                })?;
                config.scan.max_depth = Some(depth);
            }
            "thread" => {
                let value = matched.value.as_ref().expect("thread requires a value");
                let count: usize = value.parse().map_err(|_| CliError::InvalidValue {
                    option: canonical.to_string(),
                    value: value.clone(),
                    reason: "must be a positive integer".to_string(),
                })?;
                config.scan.thread_count =
                    NonZeroUsize::new(count).ok_or_else(|| CliError::InvalidValue {
                        option: canonical.to_string(),
                        value: value.clone(),
                        reason: "thread count must be greater than 0".to_string(),
                    })?;
                self.thread_explicitly_set = true;
            }
            "include" => {
                if let Some(ref value) = matched.value {
                    config.matching.include_patterns.push(value.clone());
                }
            }
            "exclude" => {
                if let Some(ref value) = matched.value {
                    config.matching.exclude_patterns.push(value.clone());
                }
            }
            "ascii" => config.render.charset = CharsetMode::Ascii,
            "full-path" => config.render.path_mode = PathMode::Full,
            "size" => config.render.show_size = true,
            "human-readable" => config.render.human_readable = true,
            "date" => config.render.show_date = true,
            "disk-usage" => config.render.show_disk_usage = true,
            "no-indent" => config.render.no_indent = true,
            "reverse" => config.render.reverse_sort = true,
            "report" => config.render.show_report = true,
            "no-win-banner" => config.render.no_win_banner = true,
            "output" => {
                if let Some(ref value) = matched.value {
                    config.output.output_path = Some(PathBuf::from(value));
                }
            }
            "silent" => config.output.silent = true,
            _ => {}
        }

        Ok(())
    }

    /// Validates path arguments.
    ///
    /// Ensures at most one path is specified and updates the config accordingly.
    ///
    /// # Arguments
    ///
    /// * `paths` - Collected path arguments
    /// * `config` - The configuration to update
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Valid paths
    /// * `Err(CliError::MultiplePaths)` - More than one path specified
    fn validate_paths(&self, paths: &[String], config: &mut Config) -> Result<(), CliError> {
        match paths.len() {
            0 => {
                config.path_explicitly_set = false;
                Ok(())
            }
            1 => {
                config.root_path = PathBuf::from(&paths[0]);
                config.path_explicitly_set = true;
                Ok(())
            }
            _ => Err(CliError::MultiplePaths {
                paths: paths.to_vec(),
            }),
        }
    }
}

// ============================================================================
// Help and Version Text
// ============================================================================

/// Returns the help information string.
///
/// # Returns
///
/// A static string containing the complete help text.
///
/// # Examples
///
/// ```
/// use treepp::cli::help_text;
///
/// let help = help_text();
/// assert!(help.contains("tree++"));
/// assert!(help.contains("--help"));
/// ```
#[must_use]
pub fn help_text() -> &'static str {
    r#"tree++: A much better Windows tree command.

Usage:
  treepp [<PATH>] [<OPTIONS>...]

Options:
  --help, -h, /?              Show help information
  --version, -v, /V           Show version information
  --batch, -b, /B             Use batch processing mode
  --ascii, -a, /A             Draw the tree using ASCII characters
  --files, -f, /F             Show files
  --full-path, -p, /FP        Show full paths
  --human-readable, -H, /HR   Show file sizes in human-readable format
  --no-indent, -i, /NI        Do not display tree connector lines
  --reverse, -r, /R           Sort in reverse order
  --size, -s, /S              Show file size (bytes)
  --date, -d, /DT             Show last modified date
  --exclude, -I, /X <PATTERN> Exclude files matching the pattern
  --level, -L, /L <N>         Limit recursion depth
  --include, -m, /M <PATTERN> Show only files matching the pattern
  --disk-usage, -u, /DU       Show cumulative directory sizes (requires --batch)
  --report, -e, /RP           Show summary statistics at the end
  --no-win-banner, -N, /NB    Do not show the Windows native tree banner/header
  --silent, -l, /SI           Silent mode (requires --output)
  --output, -o, /O <FILE>     Write output to a file (.txt, .json, .yml, .toml)
                              Note: JSON/YAML/TOML formats require --batch
  --thread, -t, /T <N>        Number of scanning threads (requires --batch, default: 8)
  --gitignore, -g, /G         Respect .gitignore
  --all, -k, /AL              Show hidden files (Windows hidden attribute)

More info: https://github.com/Water-Run/treepp"#
}

/// Returns the version information string.
///
/// # Returns
///
/// A static string containing the version and author information.
///
/// # Examples
///
/// ```
/// use treepp::cli::version_text;
///
/// let version = version_text();
/// assert!(version.contains("0.1.0"));
/// ```
#[must_use]
pub fn version_text() -> &'static str {
    r#"tree++ version 0.3.0

A much better Windows tree command.

author: WaterRun
link: https://github.com/Water-Run/treepp"#
}

/// Prints help information to standard output.
///
/// # Examples
///
/// ```no_run
/// use treepp::cli::print_help;
///
/// print_help();
/// ```
pub fn print_help() {
    println!("{}", help_text());
}

/// Prints version information to standard output.
///
/// # Examples
///
/// ```no_run
/// use treepp::cli::print_version;
///
/// print_version();
/// ```
pub fn print_version() {
    println!("{}", version_text());
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OutputFormat;
    use tempfile::TempDir;

    fn create_temp_dir() -> TempDir {
        TempDir::new().expect("创建临时目录失败")
    }

    fn parser_with_temp_dir(temp_dir: &TempDir, extra_args: Vec<&str>) -> CliParser {
        let path_str = temp_dir.path().to_string_lossy().to_string();
        let mut args = vec![path_str];
        args.extend(extra_args.into_iter().map(String::from));
        CliParser::new(args)
    }

    // ========================================================================
    // Basic Parsing Tests
    // ========================================================================

    #[test]
    fn parse_empty_args_returns_default_config() {
        let parser = CliParser::new(vec![]);
        let result = parser.parse();

        assert!(result.is_ok(), "解析应成功: {:?}", result);
        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.root_path.is_absolute());
            assert!(!config.scan.show_files);
            assert_eq!(config.scan.thread_count.get(), 8);
            assert_eq!(config.scan.max_depth, None);
            assert!(!config.scan.respect_gitignore);
            assert!(config.matching.include_patterns.is_empty());
            assert!(config.matching.exclude_patterns.is_empty());
            assert_eq!(config.render.charset, CharsetMode::Unicode);
            assert_eq!(config.render.path_mode, PathMode::Relative);
            assert!(!config.render.show_size);
            assert!(!config.render.human_readable);
            assert!(!config.render.show_date);
            assert!(!config.render.show_disk_usage);
            assert!(!config.render.no_indent);
            assert!(!config.render.reverse_sort);
            assert!(!config.render.show_report);
            assert!(!config.render.no_win_banner);
            assert!(config.output.output_path.is_none());
            assert!(!config.output.silent);
        } else {
            panic!("期望 ParseResult::Config");
        }
    }

    #[test]
    fn parse_path_only_sets_root_path() {
        let temp_dir = create_temp_dir();
        let path_str = temp_dir.path().to_string_lossy().to_string();

        let parser = CliParser::new(vec![path_str]);
        let result = parser.parse();

        assert!(result.is_ok(), "解析应成功: {:?}", result);
        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.root_path.is_absolute());
            let expected = dunce::canonicalize(temp_dir.path()).expect("规范化失败");
            assert_eq!(config.root_path, expected);
        } else {
            panic!("期望 ParseResult::Config");
        }
    }

    #[test]
    fn parse_path_with_spaces_succeeds() {
        let temp_dir = create_temp_dir();
        let sub_dir = temp_dir.path().join("path with spaces");
        std::fs::create_dir(&sub_dir).expect("创建子目录失败");

        let parser = CliParser::new(vec![sub_dir.to_string_lossy().to_string()]);
        let result = parser.parse();

        assert!(result.is_ok(), "带空格路径解析应成功: {:?}", result);
    }

    #[test]
    fn parse_relative_path_normalizes_to_absolute() {
        let parser = CliParser::new(vec![".".to_string()]);
        let result = parser.parse();

        assert!(result.is_ok(), "相对路径解析应成功: {:?}", result);
        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.root_path.is_absolute());
        }
    }

    #[test]
    fn parse_current_dir_double_dot_normalizes() {
        let parser = CliParser::new(vec!["..".to_string()]);
        let result = parser.parse();

        assert!(result.is_ok(), "父目录路径解析应成功: {:?}", result);
        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.root_path.is_absolute());
        }
    }

    // ========================================================================
    // Help and Version Tests
    // ========================================================================

    #[test]
    fn parse_help_flags_returns_help() {
        for flag in &["--help", "-h", "/?"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            let result = parser.parse();
            assert!(matches!(result, Ok(ParseResult::Help)), "测试 {flag}");
        }
    }

    #[test]
    fn parse_version_flags_returns_version() {
        for flag in &["--version", "-v", "/V", "/v"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            let result = parser.parse();
            assert!(matches!(result, Ok(ParseResult::Version)), "测试 {flag}");
        }
    }

    #[test]
    fn parse_help_with_other_options_returns_help() {
        let parser = CliParser::new(vec!["/F".to_string(), "--help".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Ok(ParseResult::Help)));
    }

    #[test]
    fn parse_version_with_other_options_returns_version() {
        let parser = CliParser::new(vec!["/F".to_string(), "--version".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Ok(ParseResult::Version)));
    }

    #[test]
    fn parse_help_before_other_options_returns_help() {
        let parser = CliParser::new(vec!["--help".to_string(), "/F".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Ok(ParseResult::Help)));
    }

    // ========================================================================
    // Three-Style Mixing Tests
    // ========================================================================

    #[test]
    fn parse_cmd_style_case_insensitive() {
        let parser1 = CliParser::new(vec!["/F".to_string()]);
        let parser2 = CliParser::new(vec!["/f".to_string()]);

        if let (Ok(ParseResult::Config(c1)), Ok(ParseResult::Config(c2))) =
            (parser1.parse(), parser2.parse())
        {
            assert!(c1.scan.show_files);
            assert!(c2.scan.show_files);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_cmd_style_various_cases() {
        for flag in &["/A", "/a", "/HR", "/hr", "/Hr", "/hR", "/NI", "/ni", "/Ni"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            let result = parser.parse();
            assert!(
                matches!(result, Ok(ParseResult::Config(_))),
                "测试 {flag} 失败"
            );
        }
    }

    #[test]
    fn parse_unix_short_style_case_sensitive() {
        let parser1 = CliParser::new(vec!["-f".to_string()]);
        let result1 = parser1.parse();
        assert!(matches!(result1, Ok(ParseResult::Config(_))));

        let parser2 = CliParser::new(vec!["-F".to_string()]);
        let result2 = parser2.parse();
        assert!(matches!(result2, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_gnu_long_style_files() {
        let parser = CliParser::new(vec!["--files".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_gnu_long_style_wrong_case_fails() {
        let parser = CliParser::new(vec!["--FILES".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_mixed_styles_combined() {
        let parser = CliParser::new(vec![
            "/F".to_string(),
            "-a".to_string(),
            "--level".to_string(),
            "3".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert_eq!(config.scan.max_depth, Some(3));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_all_three_styles_together() {
        let temp_dir = create_temp_dir();
        let parser = parser_with_temp_dir(
            &temp_dir,
            vec!["/F", "-a", "--size", "/HR", "-d", "--reverse"],
        );

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert!(config.render.show_size);
            assert!(config.render.human_readable);
            assert!(config.render.show_date);
            assert!(config.render.reverse_sort);
        } else {
            panic!("解析失败");
        }
    }

    // ========================================================================
    // Equivalent Mapping Tests
    // ========================================================================

    #[test]
    fn map_equivalent_files_options() {
        let cases = vec!["/F", "-f", "--files"];

        for flag in cases {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.scan.show_files, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn map_equivalent_ascii_options() {
        for flag in &["/A", "/a", "-a", "--ascii"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(
                    config.render.charset,
                    CharsetMode::Ascii,
                    "测试 {flag} 失败"
                );
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn map_equivalent_level_options() {
        for (flag, value) in &[("/L", "5"), ("-L", "5"), ("--level", "5")] {
            let parser = CliParser::new(vec![flag.to_string(), value.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(config.scan.max_depth, Some(5), "测试 {flag} 失败");
            } else {
                panic!("解析 {flag} {value} 失败");
            }
        }
    }

    // ========================================================================
    // Value Argument Tests
    // ========================================================================

    #[test]
    fn parse_value_argument_with_space() {
        let parser = CliParser::new(vec!["--level".to_string(), "5".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.max_depth, Some(5));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_value_argument_with_equals_syntax() {
        let parser = CliParser::new(vec!["--level=10".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.max_depth, Some(10));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_equals_syntax_various_values() {
        let cases = vec![
            ("--level=1", 1usize),
            ("--level=100", 100),
            ("--level=999", 999),
        ];

        for (arg, expected) in cases {
            let parser = CliParser::new(vec![arg.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(config.scan.max_depth, Some(expected), "测试 {arg}");
            } else {
                panic!("解析 {arg} 失败");
            }
        }
    }

    #[test]
    fn parse_equals_syntax_for_output() {
        let parser = CliParser::new(vec!["--output=tree.txt".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.output.output_path, Some(PathBuf::from("tree.txt")));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_missing_value_fails() {
        let parser = CliParser::new(vec!["--level".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::MissingValue { .. })));
    }

    #[test]
    fn parse_value_followed_by_option_fails() {
        let parser = CliParser::new(vec!["--level".to_string(), "--files".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::MissingValue { .. })));
    }

    #[test]
    fn parse_invalid_value_fails() {
        let parser = CliParser::new(vec!["--level".to_string(), "abc".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn parse_negative_level_treated_as_missing_value() {
        let parser = CliParser::new(vec!["--level".to_string(), "-5".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::MissingValue { .. })));
    }

    #[test]
    fn parse_level_zero_succeeds() {
        let parser = CliParser::new(vec!["--level".to_string(), "0".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.max_depth, Some(0));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_large_level_succeeds() {
        let parser = CliParser::new(vec!["--level".to_string(), "1000000".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.max_depth, Some(1_000_000));
        } else {
            panic!("解析失败");
        }
    }

    // ========================================================================
    // Duplicate Option Tests
    // ========================================================================

    #[test]
    fn parse_duplicate_option_different_styles_fails() {
        let parser = CliParser::new(vec!["/F".to_string(), "--files".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::DuplicateOption { .. })));
    }

    #[test]
    fn parse_duplicate_option_same_style_fails() {
        let parser = CliParser::new(vec!["/F".to_string(), "/F".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::DuplicateOption { .. })));
    }

    #[test]
    fn parse_duplicate_option_different_case_cmd_fails() {
        let parser = CliParser::new(vec!["/F".to_string(), "/f".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::DuplicateOption { .. })));
    }

    #[test]
    fn parse_duplicate_level_fails() {
        let parser = CliParser::new(vec![
            "--level".to_string(),
            "3".to_string(),
            "-L".to_string(),
            "5".to_string(),
        ]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::DuplicateOption { .. })));
    }

    // ========================================================================
    // Unknown Option Tests
    // ========================================================================

    #[test]
    fn parse_unknown_cmd_option_fails() {
        let parser = CliParser::new(vec!["/UNKNOWN".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_unknown_short_option_fails() {
        let parser = CliParser::new(vec!["-z".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_unknown_long_option_fails() {
        let parser = CliParser::new(vec!["--unknown-option".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_typo_option_fails() {
        let parser = CliParser::new(vec!["--fies".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_partial_option_fails() {
        let parser = CliParser::new(vec!["--fil".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    // ========================================================================
    // Thread Count Tests
    // ========================================================================

    #[test]
    fn parse_thread_count_with_batch() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--thread".to_string(),
            "16".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.thread_count.get(), 16);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_thread_count_one() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--thread".to_string(),
            "1".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.thread_count.get(), 1);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_zero_thread_count_fails() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--thread".to_string(),
            "0".to_string(),
        ]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn parse_invalid_thread_count_fails() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--thread".to_string(),
            "abc".to_string(),
        ]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn parse_thread_with_cmd_style() {
        let parser =
            CliParser::new(vec!["/B".to_string(), "/T".to_string(), "4".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.thread_count.get(), 4);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_thread_large_value() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--thread".to_string(),
            "128".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.thread_count.get(), 128);
        } else {
            panic!("解析失败");
        }
    }

    // ========================================================================
    // Output Option Tests
    // ========================================================================

    #[test]
    fn parse_output_path_txt() {
        let parser = CliParser::new(vec!["--output".to_string(), "tree.txt".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.output.output_path, Some(PathBuf::from("tree.txt")));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_output_format_inference() {
        let txt_parser = CliParser::new(vec!["--output".to_string(), "tree.txt".to_string()]);
        if let Ok(ParseResult::Config(config)) = txt_parser.parse() {
            assert_eq!(
                config.output.format,
                OutputFormat::Txt,
                "测试 tree.txt 格式推断"
            );
        } else {
            panic!("解析 --output tree.txt 失败");
        }

        let structured_cases = vec![
            ("tree.json", OutputFormat::Json),
            ("tree.yml", OutputFormat::Yaml),
            ("tree.yaml", OutputFormat::Yaml),
            ("tree.toml", OutputFormat::Toml),
        ];

        for (filename, expected_format) in structured_cases {
            let parser = CliParser::new(vec![
                "--batch".to_string(),
                "--output".to_string(),
                filename.to_string(),
            ]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(
                    config.output.format, expected_format,
                    "测试 {filename} 格式推断"
                );
            } else {
                panic!("解析 --batch --output {filename} 失败");
            }
        }
    }

    #[test]
    fn parse_output_with_full_path() {
        let parser = CliParser::new(vec![
            "--output".to_string(),
            "C:\\output\\tree.txt".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.output.output_path,
                Some(PathBuf::from("C:\\output\\tree.txt"))
            );
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_silent_with_output() {
        let parser = CliParser::new(vec![
            "--silent".to_string(),
            "--output".to_string(),
            "tree.txt".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.output.silent);
            assert!(config.output.output_path.is_some());
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_output_with_relative_path() {
        let parser = CliParser::new(vec![
            "--output".to_string(),
            "./output/tree.txt".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.output.output_path.is_some());
        } else {
            panic!("解析失败");
        }
    }

    // ========================================================================
    // Pattern Matching Tests
    // ========================================================================

    #[test]
    fn parse_include_pattern() {
        let parser = CliParser::new(vec!["--include".to_string(), "*.rs".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.matching.include_patterns, vec!["*.rs".to_string()]);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_exclude_pattern() {
        let parser =
            CliParser::new(vec!["--exclude".to_string(), "node_modules".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.matching.exclude_patterns,
                vec!["node_modules".to_string()]
            );
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_multiple_include_patterns() {
        let parser = CliParser::new(vec![
            "--include".to_string(),
            "*.rs".to_string(),
            "-m".to_string(),
            "*.toml".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.matching.include_patterns,
                vec!["*.rs".to_string(), "*.toml".to_string()]
            );
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_multiple_exclude_patterns() {
        let parser = CliParser::new(vec![
            "--exclude".to_string(),
            "target".to_string(),
            "/X".to_string(),
            "node_modules".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.matching.exclude_patterns,
                vec!["target".to_string(), "node_modules".to_string()]
            );
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_gitignore_all_styles() {
        for flag in &["--gitignore", "-g", "/G", "/g"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.scan.respect_gitignore, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_include_with_complex_pattern() {
        let parser = CliParser::new(vec![
            "--include".to_string(),
            "src/**/*.rs".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.matching.include_patterns,
                vec!["src/**/*.rs".to_string()]
            );
        } else {
            panic!("解析失败");
        }
    }

    // ========================================================================
    // Configuration Validation Integration Tests
    // ========================================================================

    #[test]
    fn parse_silent_without_output_fails() {
        let parser = CliParser::new(vec!["--silent".to_string()]);
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn parse_nonexistent_path_fails() {
        let parser = CliParser::new(vec!["C:\\nonexistent\\path\\12345".to_string()]);
        let result = parser.parse();
        assert!(result.is_err(), "不存在的路径应该失败");
    }

    // ========================================================================
    // Rendering Option Tests
    // ========================================================================

    #[test]
    fn parse_full_path_all_styles() {
        for flag in &["--full-path", "-p", "/FP", "/fp"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(config.render.path_mode, PathMode::Full, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_size_all_styles() {
        for flag in &["--size", "-s", "/S"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.show_size, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_human_readable_all_styles() {
        for flag in &["--human-readable", "-H", "/HR", "/hr"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.human_readable, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_date_all_styles() {
        for flag in &["--date", "-d", "/DT", "/dt"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.show_date, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_disk_usage_with_batch() {
        for flag in &["--disk-usage", "-u", "/DU", "/du"] {
            let parser = CliParser::new(vec!["--batch".to_string(), flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.show_disk_usage, "测试 {flag}");
                assert!(config.batch_mode, "测试 {flag} 应启用 batch 模式");
            } else {
                panic!("解析 --batch {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_no_indent_all_styles() {
        for flag in &["--no-indent", "-i", "/NI", "/ni"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.no_indent, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_reverse_all_styles() {
        for flag in &["--reverse", "-r", "/R", "/r"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.reverse_sort, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_report_all_styles() {
        for flag in &["--report", "-e", "/RP", "/rp"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.show_report, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn parse_no_win_banner_all_styles() {
        for flag in &["--no-win-banner", "-N", "/NB", "/nb"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.no_win_banner, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    // ========================================================================
    // Help Text Tests
    // ========================================================================

    #[test]
    fn help_text_contains_all_options() {
        let help = help_text();
        assert!(help.contains("--help"));
        assert!(help.contains("--batch"));
        assert!(help.contains("--files"));
        assert!(help.contains("--ascii"));
        assert!(help.contains("--level"));
        assert!(help.contains("--output"));
        assert!(help.contains("--thread"));
        assert!(help.contains("--gitignore"));
        assert!(help.contains("--include"));
        assert!(help.contains("--exclude"));
        assert!(help.contains("--silent"));
        assert!(help.contains("--reverse"));
        assert!(help.contains("--all"));
        assert!(help.contains("--no-win-banner"));
        assert!(help.contains("--disk-usage"));
        assert!(help.contains("--human-readable"));
        assert!(help.contains("--date"));
        assert!(help.contains("--size"));
        assert!(help.contains("--full-path"));
        assert!(help.contains("--no-indent"));
        assert!(help.contains("--report"));
        assert!(help.contains("requires --batch"));
    }

    #[test]
    fn help_text_contains_usage() {
        let help = help_text();
        assert!(help.contains("Usage"));
        assert!(help.contains("treepp"));
        assert!(help.contains("PATH"));
        assert!(help.contains("OPTIONS"));
    }

    #[test]
    fn version_text_contains_required_info() {
        let version = version_text();
        assert!(version.contains("0.3.0"));
        assert!(version.contains("WaterRun"));
        assert!(version.contains("github.com"));
    }

    // ========================================================================
    // Path Position Tests
    // ========================================================================

    #[test]
    fn parse_multiple_paths_fails() {
        let parser = CliParser::new(vec!["path1".to_string(), "path2".to_string()]);
        let result = parser.parse();

        assert!(matches!(result, Err(CliError::MultiplePaths { .. })));

        if let Err(CliError::MultiplePaths { paths }) = result {
            assert_eq!(paths.len(), 2);
            assert_eq!(paths[0], "path1");
            assert_eq!(paths[1], "path2");
        }
    }

    #[test]
    fn parse_three_paths_fails() {
        let parser = CliParser::new(vec![
            "path1".to_string(),
            "path2".to_string(),
            "path3".to_string(),
        ]);
        let result = parser.parse();

        assert!(matches!(result, Err(CliError::MultiplePaths { .. })));

        if let Err(CliError::MultiplePaths { paths }) = result {
            assert_eq!(paths.len(), 3);
        }
    }

    #[test]
    fn parse_only_options_uses_default_path() {
        let parser = CliParser::new(vec!["/F".to_string()]);
        let result = parser.parse();

        assert!(result.is_ok(), "只有选项无路径应该成功（使用当前目录）");
    }

    #[test]
    fn parse_path_between_options() {
        let temp_dir = create_temp_dir();
        let parser = CliParser::new(vec![
            "/F".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
            "--ascii".to_string(),
        ]);

        let result = parser.parse();
        assert!(result.is_ok(), "路径在选项之间应该成功");
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn parse_empty_string_arg_fails() {
        let parser = CliParser::new(vec!["".to_string()]);
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn parse_whitespace_only_arg_fails() {
        let parser = CliParser::new(vec!["   ".to_string()]);
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn parse_dash_only_fails() {
        let parser = CliParser::new(vec!["-".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_double_dash_only_fails() {
        let parser = CliParser::new(vec!["--".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_slash_only_fails() {
        let parser = CliParser::new(vec!["/".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn parse_unicode_path() {
        let temp_dir = create_temp_dir();
        let unicode_dir = temp_dir.path().join("测试目录");
        std::fs::create_dir(&unicode_dir).expect("创建目录失败");

        let parser = CliParser::new(vec![
            unicode_dir.to_string_lossy().to_string(),
            "/F".to_string(),
        ]);
        let result = parser.parse();

        assert!(result.is_ok(), "Unicode 路径应该成功: {:?}", result);
    }

    #[test]
    fn parse_unicode_path_with_special_chars() {
        let temp_dir = create_temp_dir();
        let special_dir = temp_dir.path().join("テスト_フォルダ");
        std::fs::create_dir(&special_dir).expect("创建目录失败");

        let parser = CliParser::new(vec![special_dir.to_string_lossy().to_string()]);
        let result = parser.parse();

        assert!(result.is_ok(), "特殊 Unicode 路径应该成功: {:?}", result);
    }

    // ========================================================================
    // Complex Scenario Tests
    // ========================================================================

    #[test]
    fn parse_complex_command_line() {
        let temp_dir = create_temp_dir();
        let parser = parser_with_temp_dir(
            &temp_dir,
            vec![
                "/B", "/F", "-a", "--level", "5", "-s", "-H", "-r", "--include", "*.rs",
                "--exclude", "target", "-g", "--report", "-N", "--thread", "4",
                "--disk-usage",
            ],
        );

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.batch_mode);
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert_eq!(config.scan.max_depth, Some(5));
            assert!(config.render.show_size);
            assert!(config.render.human_readable);
            assert!(config.render.reverse_sort);
            assert_eq!(config.matching.include_patterns, vec!["*.rs"]);
            assert_eq!(config.matching.exclude_patterns, vec!["target"]);
            assert!(config.scan.respect_gitignore);
            assert!(config.render.show_report);
            assert!(config.render.no_win_banner);
            assert_eq!(config.scan.thread_count.get(), 4);
            assert!(config.render.show_disk_usage);
        } else {
            panic!("复杂命令行解析失败");
        }
    }

    #[test]
    fn parse_minimal_output_scenario() {
        let temp_dir = create_temp_dir();
        let output_file = temp_dir.path().join("output.json");

        let parser = parser_with_temp_dir(
            &temp_dir,
            vec!["--batch", "--output", output_file.to_str().unwrap(), "--silent", "/F"],
        );

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.batch_mode);
            assert!(config.output.silent);
            assert!(config.output.output_path.is_some());
            assert_eq!(config.output.format, OutputFormat::Json);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_all_rendering_options_combined() {
        let parser = CliParser::new(vec![
            "--ascii".to_string(),
            "--size".to_string(),
            "--human-readable".to_string(),
            "--date".to_string(),
            "--no-indent".to_string(),
            "--reverse".to_string(),
            "--report".to_string(),
            "--no-win-banner".to_string(),
            "--full-path".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert!(config.render.show_size);
            assert!(config.render.human_readable);
            assert!(config.render.show_date);
            assert!(config.render.no_indent);
            assert!(config.render.reverse_sort);
            assert!(config.render.show_report);
            assert!(config.render.no_win_banner);
            assert_eq!(config.render.path_mode, PathMode::Full);
        } else {
            panic!("解析失败");
        }
    }

    // ========================================================================
    // is_option_like Tests
    // ========================================================================

    #[test]
    fn is_option_like_identifies_options() {
        assert!(CliParser::is_option_like("-f"));
        assert!(CliParser::is_option_like("--files"));
        assert!(CliParser::is_option_like("/F"));
        assert!(CliParser::is_option_like("/?"));
        assert!(CliParser::is_option_like("-"));
        assert!(CliParser::is_option_like("--"));
        assert!(CliParser::is_option_like("/"));
    }

    #[test]
    fn is_option_like_identifies_non_options() {
        assert!(!CliParser::is_option_like("path"));
        assert!(!CliParser::is_option_like("C:\\dir"));
        assert!(!CliParser::is_option_like("file.txt"));
        assert!(!CliParser::is_option_like(""));
        assert!(!CliParser::is_option_like("123"));
        assert!(!CliParser::is_option_like("src/main.rs"));
    }

    // ========================================================================
    // from_env Tests
    // ========================================================================

    #[test]
    fn from_env_creates_parser() {
        let _parser = CliParser::from_env();
    }

    // ========================================================================
    // Path Explicit Set Tests
    // ========================================================================

    #[test]
    fn path_explicitly_set_when_specified() {
        let temp_dir = create_temp_dir();
        let path_str = temp_dir.path().to_string_lossy().to_string();

        let parser = CliParser::new(vec![path_str]);
        let result = parser.parse();

        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.path_explicitly_set);
        } else {
            panic!("解析应成功");
        }
    }

    #[test]
    fn path_not_explicitly_set_when_omitted() {
        let parser = CliParser::new(vec![]);
        let result = parser.parse();

        if let Ok(ParseResult::Config(config)) = result {
            assert!(!config.path_explicitly_set);
        } else {
            panic!("解析应成功");
        }
    }

    #[test]
    fn path_explicitly_set_with_dot() {
        let parser = CliParser::new(vec![".".to_string()]);
        let result = parser.parse();

        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.path_explicitly_set);
        } else {
            panic!("解析应成功");
        }
    }

    // ========================================================================
    // Batch Mode Tests
    // ========================================================================

    #[test]
    fn parse_batch_flag_cmd_style() {
        let parser = CliParser::new(vec!["/B".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.batch_mode);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_batch_flag_cmd_style_lowercase() {
        let parser = CliParser::new(vec!["/b".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.batch_mode);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_batch_flag_short_style() {
        let parser = CliParser::new(vec!["-b".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.batch_mode);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_batch_flag_long_style() {
        let parser = CliParser::new(vec!["--batch".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.batch_mode);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_thread_without_batch_fails() {
        let parser = CliParser::new(vec!["--thread".to_string(), "4".to_string()]);
        let result = parser.parse();
        assert!(result.is_err());

        if let Err(CliError::ConflictingOptions { opt_a, opt_b }) = result {
            assert!(opt_a.contains("thread"));
            assert!(opt_b.contains("batch"));
        } else {
            panic!("期望 ConflictingOptions 错误");
        }
    }

    #[test]
    fn parse_thread_with_batch_succeeds() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--thread".to_string(),
            "4".to_string(),
        ]);
        let result = parser.parse();
        assert!(result.is_ok());

        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.batch_mode);
            assert_eq!(config.scan.thread_count.get(), 4);
        }
    }

    #[test]
    fn parse_disk_usage_without_batch_fails() {
        let parser = CliParser::new(vec!["--disk-usage".to_string()]);
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn parse_disk_usage_with_batch_succeeds() {
        let parser = CliParser::new(vec!["--batch".to_string(), "--disk-usage".to_string()]);
        let result = parser.parse();
        assert!(result.is_ok());

        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.batch_mode);
            assert!(config.render.show_disk_usage);
        }
    }

    #[test]
    fn parse_json_output_without_batch_fails() {
        let parser = CliParser::new(vec!["--output".to_string(), "tree.json".to_string()]);
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn parse_json_output_with_batch_succeeds() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--output".to_string(),
            "tree.json".to_string(),
        ]);
        let result = parser.parse();
        assert!(result.is_ok());

        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.batch_mode);
            assert_eq!(config.output.format, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_txt_output_without_batch_succeeds() {
        let parser = CliParser::new(vec!["--output".to_string(), "tree.txt".to_string()]);
        let result = parser.parse();
        assert!(result.is_ok());

        if let Ok(ParseResult::Config(config)) = result {
            assert!(!config.batch_mode);
            assert_eq!(config.output.format, OutputFormat::Txt);
        }
    }

    #[test]
    fn parse_batch_with_multiple_options() {
        let temp_dir = create_temp_dir();
        let parser =
            parser_with_temp_dir(&temp_dir, vec!["/B", "/F", "/DU", "-t", "16", "-o", "tree.json"]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.batch_mode);
            assert!(config.scan.show_files);
            assert!(config.render.show_disk_usage);
            assert_eq!(config.scan.thread_count.get(), 16);
            assert_eq!(config.output.format, OutputFormat::Json);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_defaults_to_stream_mode() {
        let parser = CliParser::new(vec!["/F".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(!config.batch_mode);
        } else {
            panic!("解析失败");
        }
    }

    // ========================================================================
    // Additional Coverage Tests
    // ========================================================================

    #[test]
    fn parse_yaml_output_with_batch() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--output".to_string(),
            "tree.yaml".to_string(),
        ]);
        let result = parser.parse();
        assert!(result.is_ok());

        if let Ok(ParseResult::Config(config)) = result {
            assert_eq!(config.output.format, OutputFormat::Yaml);
        }
    }

    #[test]
    fn parse_toml_output_with_batch() {
        let parser = CliParser::new(vec![
            "--batch".to_string(),
            "--output".to_string(),
            "tree.toml".to_string(),
        ]);
        let result = parser.parse();
        assert!(result.is_ok());

        if let Ok(ParseResult::Config(config)) = result {
            assert_eq!(config.output.format, OutputFormat::Toml);
        }
    }

    #[test]
    fn parse_mixed_include_exclude_patterns() {
        let parser = CliParser::new(vec![
            "--include".to_string(),
            "*.rs".to_string(),
            "--exclude".to_string(),
            "target".to_string(),
            "-m".to_string(),
            "*.toml".to_string(),
            "/X".to_string(),
            ".git".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.matching.include_patterns.len(), 2);
            assert_eq!(config.matching.exclude_patterns.len(), 2);
            assert!(config.matching.include_patterns.contains(&"*.rs".to_string()));
            assert!(config.matching.include_patterns.contains(&"*.toml".to_string()));
            assert!(config.matching.exclude_patterns.contains(&"target".to_string()));
            assert!(config.matching.exclude_patterns.contains(&".git".to_string()));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_equals_syntax_empty_value() {
        let parser = CliParser::new(vec!["--level=".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn parse_float_level_fails() {
        let parser = CliParser::new(vec!["--level".to_string(), "3.14".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn parse_overflow_level() {
        let parser = CliParser::new(vec![
            "--level".to_string(),
            "99999999999999999999999999999".to_string(),
        ]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn parse_batch_order_independent() {
        let parser1 = CliParser::new(vec![
            "--batch".to_string(),
            "--thread".to_string(),
            "4".to_string(),
        ]);
        let parser2 = CliParser::new(vec![
            "--thread".to_string(),
            "4".to_string(),
            "--batch".to_string(),
        ]);

        let result1 = parser1.parse();
        let result2 = parser2.parse();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn parse_size_and_human_readable_combined() {
        let parser = CliParser::new(vec![
            "--size".to_string(),
            "--human-readable".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.render.show_size);
            assert!(config.render.human_readable);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_cmd_style_thread_lowercase() {
        let parser = CliParser::new(vec![
            "/b".to_string(),
            "/t".to_string(),
            "8".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.batch_mode);
            assert_eq!(config.scan.thread_count.get(), 8);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_with_path_at_end() {
        let temp_dir = create_temp_dir();
        let path_str = temp_dir.path().to_string_lossy().to_string();

        let parser = CliParser::new(vec!["/F".to_string(), "--ascii".to_string(), path_str]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert!(config.path_explicitly_set);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_with_path_at_beginning() {
        let temp_dir = create_temp_dir();
        let path_str = temp_dir.path().to_string_lossy().to_string();

        let parser = CliParser::new(vec![path_str, "/F".to_string(), "--ascii".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert!(config.path_explicitly_set);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_multiple_different_flags() {
        let parser = CliParser::new(vec![
            "/F".to_string(),
            "/A".to_string(),
            "/S".to_string(),
            "/HR".to_string(),
            "/R".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert!(config.render.show_size);
            assert!(config.render.human_readable);
            assert!(config.render.reverse_sort);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_all_flag_cmd_style() {
        let parser = CliParser::new(vec!["/AL".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_hidden);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_all_flag_cmd_style_lowercase() {
        let parser = CliParser::new(vec!["/al".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_hidden);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_all_flag_short_style() {
        let parser = CliParser::new(vec!["-k".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_hidden);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn parse_all_flag_long_style() {
        let parser = CliParser::new(vec!["--all".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_hidden);
        } else {
            panic!("解析失败");
        }
    }
}
