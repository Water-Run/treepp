//! Error handling module: defines unified error types for tree++.
//!
//! This module provides a hierarchical error type system covering:
//!
//! - **CLI parsing errors**: argument format, conflicts, unknown options
//! - **Configuration errors**: re-exported from `config` module for API consistency
//! - **Scan errors**: filesystem access, permissions, path not found
//! - **Match errors**: glob pattern syntax, gitignore rule parsing
//! - **Render errors**: output formatting anomalies
//! - **Output errors**: file writing, serialization failures
//!
//! All error types implement `std::error::Error` with proper error chain support.
//!
//! File: src/error.rs
//! Author: WaterRun
//! Date: 2026-01-12

#![forbid(unsafe_code)]

use std::io;
use std::path::PathBuf;
use thiserror::Error;

pub use crate::config::ConfigError;

/// Top-level error type for tree++.
///
/// Aggregates all sub-module errors as the unified error return type for the
/// program's main entry point. Supports automatic conversion from all sub-error
/// types via the `From` trait.
///
/// # Examples
///
/// ```
/// use treepp::error::{TreeppError, CliError};
///
/// fn example_cli_error() -> Result<(), TreeppError> {
///     Err(CliError::UnknownOption {
///         option: "/Z".to_string(),
///     }.into())
/// }
///
/// let err = example_cli_error().unwrap_err();
/// assert!(err.to_string().contains("/Z"));
/// ```
///
/// ```
/// use treepp::error::{TreeppError, ScanError};
/// use std::path::PathBuf;
///
/// let scan_err = ScanError::PathNotFound {
///     path: PathBuf::from("/missing"),
/// };
/// let treepp_err: TreeppError = scan_err.into();
/// assert!(matches!(treepp_err, TreeppError::Scan(_)));
/// ```
#[derive(Debug, Error)]
pub enum TreeppError {
    /// CLI parsing error.
    #[error(transparent)]
    Cli(#[from] CliError),

    /// Configuration validation error.
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// Directory scan error.
    #[error(transparent)]
    Scan(#[from] ScanError),

    /// Pattern matching error.
    #[error(transparent)]
    Match(#[from] MatchError),

    /// Rendering error.
    #[error(transparent)]
    Render(#[from] RenderError),

    /// Output error.
    #[error(transparent)]
    Output(#[from] OutputError),
}

/// Result type alias for tree++ operations.
///
/// A convenience alias for `Result<T, TreeppError>` used throughout the crate.
///
/// # Examples
///
/// ```
/// use treepp::error::TreeppResult;
///
/// fn operation() -> TreeppResult<i32> {
///     Ok(42)
/// }
///
/// assert_eq!(operation().unwrap(), 42);
/// ```
pub type TreeppResult<T> = Result<T, TreeppError>;

/// CLI argument parsing errors.
///
/// Represents errors that occur during command-line argument parsing, including:
/// - Unknown options
/// - Missing required argument values
/// - Invalid argument value formats
/// - Option conflicts
///
/// # Examples
///
/// ```
/// use treepp::error::CliError;
///
/// let err = CliError::MissingValue {
///     option: "--level".to_string(),
/// };
/// assert!(err.to_string().contains("--level"));
/// assert!(err.to_string().contains("requires a value"));
/// ```
///
/// ```
/// use treepp::error::CliError;
///
/// let err1 = CliError::UnknownOption { option: "/Z".to_string() };
/// let err2 = CliError::UnknownOption { option: "/Z".to_string() };
/// assert_eq!(err1, err2);
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CliError {
    /// Unknown option was provided.
    #[error("Unknown option: {option}")]
    UnknownOption {
        /// The unrecognized option name.
        option: String,
    },

    /// Option is missing its required value.
    #[error("Option {option} requires a value.")]
    MissingValue {
        /// The option name.
        option: String,
    },

    /// Option value has invalid format.
    #[error("Invalid value '{value}' for option {option}: {reason}")]
    InvalidValue {
        /// The option name.
        option: String,
        /// The provided value.
        value: String,
        /// The reason for invalidity.
        reason: String,
    },

    /// Option was specified more than once.
    #[error("Option {option} was specified more than once.")]
    DuplicateOption {
        /// The option name.
        option: String,
    },

    /// Two options conflict with each other.
    #[error("Option conflict: {opt_a} and {opt_b} cannot be used together.")]
    ConflictingOptions {
        /// First conflicting option.
        opt_a: String,
        /// Second conflicting option.
        opt_b: String,
    },

    /// Multiple paths were specified when only one is allowed.
    #[error("Only one path can be specified, but multiple were provided: {paths:?}")]
    MultiplePaths {
        /// All discovered paths.
        paths: Vec<String>,
    },

    /// Path argument could not be parsed.
    #[error("Failed to parse path argument: {arg}")]
    InvalidPath {
        /// The original argument.
        arg: String,
    },

    /// Generic parsing error.
    #[error("Argument parsing failed: {message}")]
    ParseError {
        /// Error message.
        message: String,
    },
}

/// Directory scanning errors.
///
/// Represents errors that occur during directory traversal, including filesystem
/// access issues, permission problems, and path resolution failures.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::error::ScanError;
///
/// let err = ScanError::PathNotFound {
///     path: PathBuf::from("C:\\nonexistent"),
/// };
/// assert!(err.to_string().contains("nonexistent"));
/// assert!(err.to_string().contains("Path not found"));
/// ```
///
/// ```
/// use std::path::PathBuf;
/// use treepp::error::ScanError;
///
/// let err = ScanError::PermissionDenied {
///     path: PathBuf::from("/protected"),
/// };
/// assert!(err.to_string().contains("Permission denied"));
/// ```
#[derive(Debug, Error)]
pub enum ScanError {
    /// The specified path does not exist.
    #[error("Path not found: {path}")]
    PathNotFound {
        /// The non-existent path.
        path: PathBuf,
    },

    /// The specified path is not a directory.
    #[error("Path is not a directory: {path}")]
    NotADirectory {
        /// The non-directory path.
        path: PathBuf,
    },

    /// Permission was denied for the path.
    #[error("Permission denied: {path}")]
    PermissionDenied {
        /// The inaccessible path.
        path: PathBuf,
    },

    /// Failed to read directory contents.
    #[error("Failed to read directory: {path}")]
    ReadDirFailed {
        /// The directory path.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// Failed to retrieve file metadata.
    #[error("Failed to retrieve metadata: {path}")]
    MetadataFailed {
        /// The file path.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// Failed to canonicalize path.
    #[error("Failed to canonicalize path: {path}")]
    CanonicalizeFailed {
        /// The original path.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// Error during directory walk traversal.
    #[error("Directory walk error: {message}")]
    WalkError {
        /// Error message.
        message: String,
        /// Related path, if available.
        path: Option<PathBuf>,
    },
}

impl ScanError {
    /// Creates an appropriate scan error from an IO error and path.
    ///
    /// Automatically selects the appropriate error variant based on the IO error kind.
    ///
    /// # Arguments
    ///
    /// * `err` - The IO error to convert.
    /// * `path` - The path associated with the error.
    ///
    /// # Returns
    ///
    /// A `ScanError` variant appropriate for the IO error kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{self, ErrorKind};
    /// use std::path::PathBuf;
    /// use treepp::error::ScanError;
    ///
    /// let io_err = io::Error::new(ErrorKind::NotFound, "not found");
    /// let scan_err = ScanError::from_io_error(io_err, PathBuf::from("/missing"));
    /// assert!(matches!(scan_err, ScanError::PathNotFound { .. }));
    /// ```
    ///
    /// ```
    /// use std::io::{self, ErrorKind};
    /// use std::path::PathBuf;
    /// use treepp::error::ScanError;
    ///
    /// let io_err = io::Error::new(ErrorKind::PermissionDenied, "denied");
    /// let scan_err = ScanError::from_io_error(io_err, PathBuf::from("/protected"));
    /// assert!(matches!(scan_err, ScanError::PermissionDenied { .. }));
    /// ```
    ///
    /// ```
    /// use std::io::{self, ErrorKind};
    /// use std::path::PathBuf;
    /// use treepp::error::ScanError;
    ///
    /// let io_err = io::Error::new(ErrorKind::Other, "other error");
    /// let scan_err = ScanError::from_io_error(io_err, PathBuf::from("/path"));
    /// assert!(matches!(scan_err, ScanError::ReadDirFailed { .. }));
    /// ```
    #[must_use]
    pub fn from_io_error(err: io::Error, path: PathBuf) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::PathNotFound { path },
            io::ErrorKind::PermissionDenied => Self::PermissionDenied { path },
            _ => Self::ReadDirFailed { path, source: err },
        }
    }
}

impl From<walkdir::Error> for ScanError {
    fn from(err: walkdir::Error) -> Self {
        let path = err.path().map(PathBuf::from);
        if let Some(io_err) = err.io_error() {
            match io_err.kind() {
                io::ErrorKind::PermissionDenied => {
                    if let Some(p) = path {
                        return Self::PermissionDenied { path: p };
                    }
                }
                io::ErrorKind::NotFound => {
                    if let Some(p) = path {
                        return Self::PathNotFound { path: p };
                    }
                }
                _ => {}
            }
        }
        Self::WalkError {
            message: err.to_string(),
            path,
        }
    }
}

/// Pattern matching errors.
///
/// Represents errors in pattern matching and gitignore rule parsing, including
/// invalid glob patterns and gitignore file parsing failures.
///
/// # Examples
///
/// ```
/// use treepp::error::MatchError;
///
/// let err = MatchError::InvalidPattern {
///     pattern: "[invalid".to_string(),
///     reason: "unclosed bracket".to_string(),
/// };
/// assert!(err.to_string().contains("[invalid"));
/// assert!(err.to_string().contains("Invalid pattern"));
/// ```
///
/// ```
/// use treepp::error::MatchError;
///
/// let err1 = MatchError::InvalidPattern {
///     pattern: "*.txt".to_string(),
///     reason: "test".to_string(),
/// };
/// let err2 = MatchError::InvalidPattern {
///     pattern: "*.txt".to_string(),
///     reason: "test".to_string(),
/// };
/// assert_eq!(err1, err2);
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MatchError {
    /// Invalid glob pattern syntax.
    #[error("Invalid pattern '{pattern}': {reason}")]
    InvalidPattern {
        /// The invalid pattern string.
        pattern: String,
        /// The reason for invalidity.
        reason: String,
    },

    /// Failed to parse gitignore file.
    #[error("Failed to parse .gitignore: {path}")]
    GitignoreParseError {
        /// The gitignore file path.
        path: PathBuf,
        /// Error details.
        detail: String,
    },

    /// Failed to build gitignore rules.
    #[error("Failed to build gitignore rules: {reason}")]
    GitignoreBuildError {
        /// The reason for failure.
        reason: String,
    },
}

impl MatchError {
    /// Creates a `MatchError` from a glob pattern error.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The invalid pattern string.
    /// * `reason` - The reason for the error.
    ///
    /// # Returns
    ///
    /// A `MatchError::InvalidPattern` variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::error::MatchError;
    ///
    /// let err = MatchError::from_glob_error("[bad", "unclosed bracket");
    /// assert!(matches!(err, MatchError::InvalidPattern { .. }));
    /// ```
    ///
    /// ```
    /// use treepp::error::MatchError;
    ///
    /// let err = MatchError::from_glob_error("**[", "syntax error");
    /// assert!(err.to_string().contains("**["));
    /// ```
    #[must_use]
    pub fn from_glob_error(pattern: &str, reason: &str) -> Self {
        Self::InvalidPattern {
            pattern: pattern.to_string(),
            reason: reason.to_string(),
        }
    }
}

impl From<glob::PatternError> for MatchError {
    fn from(err: glob::PatternError) -> Self {
        Self::InvalidPattern {
            pattern: err.msg.to_string(),
            reason: format!("position {}", err.pos),
        }
    }
}

impl From<ignore::Error> for MatchError {
    fn from(err: ignore::Error) -> Self {
        Self::GitignoreBuildError {
            reason: err.to_string(),
        }
    }
}

/// Rendering errors.
///
/// Represents errors that occur during tree structure rendering. These errors
/// are relatively rare and primarily handle edge cases.
///
/// # Examples
///
/// ```
/// use treepp::error::RenderError;
///
/// let err = RenderError::FormatError {
///     context: "date formatting".to_string(),
///     detail: "timestamp out of range".to_string(),
/// };
/// assert!(err.to_string().contains("date formatting"));
/// assert!(err.to_string().contains("timestamp out of range"));
/// ```
///
/// ```
/// use treepp::error::RenderError;
///
/// let err = RenderError::InvalidUtf8Path {
///     path_lossy: "some\u{FFFD}path".to_string(),
/// };
/// assert!(err.to_string().contains("invalid UTF-8"));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// Formatting error during rendering.
    #[error("Formatting error ({context}): {detail}")]
    FormatError {
        /// Error context.
        context: String,
        /// Error details.
        detail: String,
    },

    /// Path contains invalid UTF-8 characters.
    #[error("Encoding error: path contains invalid UTF-8 characters")]
    InvalidUtf8Path {
        /// The path with lossy conversion applied.
        path_lossy: String,
    },

    /// Failed to fetch Windows tree banner.
    #[error("Failed to fetch Windows tree banner: {reason}")]
    BannerFetchFailed {
        /// The reason for failure.
        reason: String,
    },

    /// Invalid path for rendering.
    #[error("Invalid path '{path}': {reason}")]
    InvalidPath {
        /// The path.
        path: PathBuf,
        /// The reason.
        reason: String,
    },
}

/// Output errors.
///
/// Represents errors that occur during result output, including file writing
/// and serialization failures.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::error::OutputError;
///
/// let err = OutputError::SerializationFailed {
///     format: "JSON".to_string(),
///     reason: "circular reference".to_string(),
/// };
/// assert!(err.to_string().contains("JSON"));
/// assert!(err.to_string().contains("circular reference"));
/// ```
///
/// ```
/// use treepp::error::OutputError;
///
/// let err = OutputError::json_error("invalid structure");
/// assert!(err.to_string().contains("JSON"));
/// ```
#[derive(Debug, Error)]
pub enum OutputError {
    /// Failed to create output file.
    #[error("Failed to create output file: {path}")]
    FileCreateFailed {
        /// Target file path.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// Failed to write to file.
    #[error("Failed to write file: {path}")]
    WriteFailed {
        /// Target file path.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// Serialization failed.
    #[error("{format} serialization failed: {reason}")]
    SerializationFailed {
        /// Output format name.
        format: String,
        /// Failure reason.
        reason: String,
    },

    /// Failed to write to stdout.
    #[error("Failed to write to stdout")]
    StdoutFailed {
        /// The underlying IO error.
        #[source]
        source: io::Error,
    },

    /// Invalid output path.
    #[error("Invalid output path: {path} ({reason})")]
    InvalidOutputPath {
        /// Output path.
        path: PathBuf,
        /// The reason.
        reason: String,
    },
}

impl OutputError {
    /// Creates a JSON serialization error.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for the serialization failure.
    ///
    /// # Returns
    ///
    /// An `OutputError::SerializationFailed` with format set to "JSON".
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::error::OutputError;
    ///
    /// let err = OutputError::json_error("invalid structure");
    /// assert!(err.to_string().contains("JSON"));
    /// assert!(err.to_string().contains("invalid structure"));
    /// ```
    #[must_use]
    pub fn json_error(reason: impl Into<String>) -> Self {
        Self::SerializationFailed {
            format: "JSON".to_string(),
            reason: reason.into(),
        }
    }

    /// Creates a YAML serialization error.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for the serialization failure.
    ///
    /// # Returns
    ///
    /// An `OutputError::SerializationFailed` with format set to "YAML".
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::error::OutputError;
    ///
    /// let err = OutputError::yaml_error("indentation error");
    /// assert!(err.to_string().contains("YAML"));
    /// assert!(err.to_string().contains("indentation error"));
    /// ```
    #[must_use]
    pub fn yaml_error(reason: impl Into<String>) -> Self {
        Self::SerializationFailed {
            format: "YAML".to_string(),
            reason: reason.into(),
        }
    }

    /// Creates a TOML serialization error.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for the serialization failure.
    ///
    /// # Returns
    ///
    /// An `OutputError::SerializationFailed` with format set to "TOML".
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::error::OutputError;
    ///
    /// let err = OutputError::toml_error("key-value error");
    /// assert!(err.to_string().contains("TOML"));
    /// assert!(err.to_string().contains("key-value error"));
    /// ```
    #[must_use]
    pub fn toml_error(reason: impl Into<String>) -> Self {
        Self::SerializationFailed {
            format: "TOML".to_string(),
            reason: reason.into(),
        }
    }
}

impl From<io::Error> for OutputError {
    fn from(err: io::Error) -> Self {
        Self::StdoutFailed { source: err }
    }
}

/// Converts a path to a displayable string.
///
/// Handles paths that may contain invalid UTF-8 by using lossy conversion,
/// ensuring output is always possible.
///
/// # Arguments
///
/// * `path` - The path to convert.
///
/// # Returns
///
/// A `String` representation of the path.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use treepp::error::path_display;
///
/// let path = Path::new("C:\\Users\\test");
/// assert_eq!(path_display(path), "C:\\Users\\test");
/// ```
///
/// ```
/// use std::path::Path;
/// use treepp::error::path_display;
///
/// let path = Path::new("/home/user/file.txt");
/// assert!(path_display(path).contains("file.txt"));
/// ```
#[must_use]
pub fn path_display(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Determines whether a scan error is recoverable.
///
/// Some scan errors (such as permission denied on a single file) should not
/// interrupt the entire traversal. This function identifies such recoverable
/// errors.
///
/// # Arguments
///
/// * `err` - The scan error to check.
///
/// # Returns
///
/// `true` if the error is recoverable and processing can continue with other
/// items, `false` otherwise.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::error::{ScanError, is_recoverable};
///
/// let err = ScanError::PermissionDenied {
///     path: PathBuf::from("/protected"),
/// };
/// assert!(is_recoverable(&err));
/// ```
///
/// ```
/// use std::path::PathBuf;
/// use treepp::error::{ScanError, is_recoverable};
///
/// let err = ScanError::NotADirectory {
///     path: PathBuf::from("/test"),
/// };
/// assert!(!is_recoverable(&err));
/// ```
///
/// ```
/// use treepp::error::{ScanError, is_recoverable};
///
/// let err = ScanError::WalkError {
///     message: "test error".to_string(),
///     path: None,
/// };
/// assert!(!is_recoverable(&err));
/// ```
#[must_use]
pub const fn is_recoverable(err: &ScanError) -> bool {
    matches!(
        err,
        ScanError::PermissionDenied { .. }
            | ScanError::MetadataFailed { .. }
            | ScanError::PathNotFound { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn treepp_error_converts_from_cli_error() {
        let cli_err = CliError::UnknownOption {
            option: "/X".to_string(),
        };
        let treepp_err: TreeppError = cli_err.into();

        assert!(matches!(treepp_err, TreeppError::Cli(_)));
        assert!(treepp_err.to_string().contains("/X"));
    }

    #[test]
    fn treepp_error_converts_from_scan_error() {
        let scan_err = ScanError::PathNotFound {
            path: PathBuf::from("/missing"),
        };
        let treepp_err: TreeppError = scan_err.into();

        assert!(matches!(treepp_err, TreeppError::Scan(_)));
    }

    #[test]
    fn treepp_error_converts_from_match_error() {
        let match_err = MatchError::InvalidPattern {
            pattern: "[bad".to_string(),
            reason: "unclosed".to_string(),
        };
        let treepp_err: TreeppError = match_err.into();

        assert!(matches!(treepp_err, TreeppError::Match(_)));
    }

    #[test]
    fn treepp_error_converts_from_render_error() {
        let render_err = RenderError::FormatError {
            context: "test".to_string(),
            detail: "detail".to_string(),
        };
        let treepp_err: TreeppError = render_err.into();

        assert!(matches!(treepp_err, TreeppError::Render(_)));
    }

    #[test]
    fn treepp_error_converts_from_output_error() {
        let output_err = OutputError::SerializationFailed {
            format: "JSON".to_string(),
            reason: "test".to_string(),
        };
        let treepp_err: TreeppError = output_err.into();

        assert!(matches!(treepp_err, TreeppError::Output(_)));
    }

    #[test]
    fn cli_error_unknown_option_formats_correctly() {
        let err = CliError::UnknownOption {
            option: "--unknown".to_string(),
        };
        assert!(err.to_string().contains("--unknown"));
        assert!(err.to_string().contains("Unknown option:"));
    }

    #[test]
    fn cli_error_missing_value_formats_correctly() {
        let err = CliError::MissingValue {
            option: "--level".to_string(),
        };
        assert!(err.to_string().contains("--level"));
        assert!(err.to_string().contains("requires a value."));
    }

    #[test]
    fn cli_error_invalid_value_formats_correctly() {
        let err = CliError::InvalidValue {
            option: "--thread".to_string(),
            value: "abc".to_string(),
            reason: "必须是正整数".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("--thread"));
        assert!(msg.contains("abc"));
        assert!(msg.contains("必须是正整数"));
    }

    #[test]
    fn cli_error_duplicate_option_formats_correctly() {
        let err = CliError::DuplicateOption {
            option: "--level".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("--level"));
        assert!(msg.contains("more than once"));
    }

    #[test]
    fn cli_error_conflicting_options_formats_correctly() {
        let err = CliError::ConflictingOptions {
            opt_a: "--silent".to_string(),
            opt_b: "--verbose".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("--silent"));
        assert!(msg.contains("--verbose"));
        assert!(msg.contains("cannot be used together"));
    }

    #[test]
    fn cli_error_multiple_paths_formats_correctly() {
        let err = CliError::MultiplePaths {
            paths: vec!["path1".to_string(), "path2".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("path1"));
        assert!(msg.contains("path2"));
    }

    #[test]
    fn cli_error_parse_error_formats_correctly() {
        let err = CliError::ParseError {
            message: "unexpected token".to_string(),
        };
        assert!(err.to_string().contains("unexpected token"));
    }

    #[test]
    fn cli_errors_compare_equal_when_identical() {
        let err1 = CliError::UnknownOption {
            option: "/Z".to_string(),
        };
        let err2 = CliError::UnknownOption {
            option: "/Z".to_string(),
        };
        let err3 = CliError::UnknownOption {
            option: "/Y".to_string(),
        };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn scan_error_from_io_not_found_creates_path_not_found() {
        let io_err = io::Error::new(ErrorKind::NotFound, "file not found");
        let path = PathBuf::from("/test/path");
        let scan_err = ScanError::from_io_error(io_err, path.clone());

        assert!(matches!(scan_err, ScanError::PathNotFound { path: p } if p == path));
    }

    #[test]
    fn scan_error_from_io_permission_denied_creates_permission_denied() {
        let io_err = io::Error::new(ErrorKind::PermissionDenied, "access denied");
        let path = PathBuf::from("/protected");
        let scan_err = ScanError::from_io_error(io_err, path.clone());

        assert!(matches!(scan_err, ScanError::PermissionDenied { path: p } if p == path));
    }

    #[test]
    fn scan_error_from_io_other_creates_read_dir_failed() {
        let io_err = io::Error::new(ErrorKind::Other, "some error");
        let path = PathBuf::from("/some/path");
        let scan_err = ScanError::from_io_error(io_err, path.clone());

        assert!(matches!(scan_err, ScanError::ReadDirFailed { path: p, .. } if p == path));
    }

    #[test]
    fn scan_error_path_not_found_formats_correctly() {
        let err = ScanError::PathNotFound {
            path: PathBuf::from("C:\\missing\\dir"),
        };
        let msg = err.to_string();
        assert!(msg.contains("Path not found:"));
        assert!(msg.contains("C:\\missing\\dir"));
    }

    #[test]
    fn scan_error_permission_denied_formats_correctly() {
        let err = ScanError::PermissionDenied {
            path: PathBuf::from("/root/secret"),
        };
        let msg = err.to_string();
        assert!(msg.contains("Permission denied"));
    }

    #[test]
    fn scan_error_not_a_directory_formats_correctly() {
        let err = ScanError::NotADirectory {
            path: PathBuf::from("/some/file.txt"),
        };
        let msg = err.to_string();
        assert!(msg.contains("not a directory"));
    }

    #[test]
    fn scan_error_canonicalize_failed_formats_correctly() {
        let err = ScanError::CanonicalizeFailed {
            path: PathBuf::from("/invalid/../path"),
            source: io::Error::new(ErrorKind::NotFound, "not found"),
        };
        let msg = err.to_string();
        assert!(msg.contains("canonicalize"));
    }

    #[test]
    fn scan_error_walk_error_formats_correctly() {
        let err = ScanError::WalkError {
            message: "walk failed".to_string(),
            path: Some(PathBuf::from("/test")),
        };
        let msg = err.to_string();
        assert!(msg.contains("walk failed"));
    }

    #[test]
    fn match_error_from_glob_error_creates_invalid_pattern() {
        let err = MatchError::from_glob_error("[invalid", "未闭合的括号");

        match err {
            MatchError::InvalidPattern { pattern, reason } => {
                assert_eq!(pattern, "[invalid");
                assert!(reason.contains("未闭合的括号"));
            }
            _ => panic!("Expected InvalidPattern variant"),
        }
    }

    #[test]
    fn match_error_invalid_pattern_formats_correctly() {
        let err = MatchError::InvalidPattern {
            pattern: "**[".to_string(),
            reason: "语法错误".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("**["));
        assert!(msg.contains("Invalid pattern"));
    }

    #[test]
    fn match_error_gitignore_parse_error_formats_correctly() {
        let err = MatchError::GitignoreParseError {
            path: PathBuf::from(".gitignore"),
            detail: "第 5 行语法错误".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains(".gitignore"));
    }

    #[test]
    fn match_error_gitignore_build_error_formats_correctly() {
        let err = MatchError::GitignoreBuildError {
            reason: "invalid rule".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid rule"));
    }

    #[test]
    fn match_errors_compare_equal_when_identical() {
        let err1 = MatchError::InvalidPattern {
            pattern: "*.txt".to_string(),
            reason: "test".to_string(),
        };
        let err2 = MatchError::InvalidPattern {
            pattern: "*.txt".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(err1, err2);
    }

    #[test]
    fn render_error_format_error_formats_correctly() {
        let err = RenderError::FormatError {
            context: "大小格式化".to_string(),
            detail: "数值溢出".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("大小格式化"));
        assert!(msg.contains("数值溢出"));
    }

    #[test]
    fn render_error_invalid_utf8_path_formats_correctly() {
        let err = RenderError::InvalidUtf8Path {
            path_lossy: "some\u{FFFD}path".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid UTF-8"));
    }

    #[test]
    fn render_error_banner_fetch_failed_formats_correctly() {
        let err = RenderError::BannerFetchFailed {
            reason: "command failed".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("command failed"));
    }

    #[test]
    fn render_error_invalid_path_formats_correctly() {
        let err = RenderError::InvalidPath {
            path: PathBuf::from("/bad/path"),
            reason: "missing drive letter".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("missing drive letter"));
    }

    #[test]
    fn render_errors_compare_equal_when_identical() {
        let err1 = RenderError::FormatError {
            context: "test".to_string(),
            detail: "detail".to_string(),
        };
        let err2 = RenderError::FormatError {
            context: "test".to_string(),
            detail: "detail".to_string(),
        };
        assert_eq!(err1, err2);
    }

    #[test]
    fn output_error_json_error_creates_correct_variant() {
        let err = OutputError::json_error("无效结构");
        match err {
            OutputError::SerializationFailed { format, reason } => {
                assert_eq!(format, "JSON");
                assert!(reason.contains("无效结构"));
            }
            _ => panic!("Expected SerializationFailed variant"),
        }
    }

    #[test]
    fn output_error_yaml_error_creates_correct_variant() {
        let err = OutputError::yaml_error("缩进错误");
        match err {
            OutputError::SerializationFailed { format, reason } => {
                assert_eq!(format, "YAML");
                assert!(reason.contains("缩进错误"));
            }
            _ => panic!("Expected SerializationFailed variant"),
        }
    }

    #[test]
    fn output_error_toml_error_creates_correct_variant() {
        let err = OutputError::toml_error("键值对错误");
        match err {
            OutputError::SerializationFailed { format, reason } => {
                assert_eq!(format, "TOML");
                assert!(reason.contains("键值对错误"));
            }
            _ => panic!("Expected SerializationFailed variant"),
        }
    }

    #[test]
    fn output_error_converts_from_io_error() {
        let io_err = io::Error::new(ErrorKind::BrokenPipe, "pipe broken");
        let output_err: OutputError = io_err.into();

        assert!(matches!(output_err, OutputError::StdoutFailed { .. }));
    }

    #[test]
    fn output_error_file_create_failed_formats_correctly() {
        let err = OutputError::FileCreateFailed {
            path: PathBuf::from("output.json"),
            source: io::Error::new(ErrorKind::PermissionDenied, "denied"),
        };
        let msg = err.to_string();
        assert!(msg.contains("output.json"));
        assert!(msg.contains("Failed to create output file:"));
    }

    #[test]
    fn output_error_write_failed_formats_correctly() {
        let err = OutputError::WriteFailed {
            path: PathBuf::from("output.txt"),
            source: io::Error::new(ErrorKind::Other, "disk full"),
        };
        let msg = err.to_string();
        assert!(msg.contains("output.txt"));
        assert!(msg.contains("Failed to write file"));
    }

    #[test]
    fn output_error_invalid_output_path_formats_correctly() {
        let err = OutputError::InvalidOutputPath {
            path: PathBuf::from("/invalid/path"),
            reason: "directory does not exist".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Invalid output path"));
    }

    #[test]
    fn path_display_handles_valid_utf8_path() {
        let path = std::path::Path::new("C:\\Users\\test\\file.txt");
        let display = path_display(path);
        assert!(display.contains("test"));
        assert!(display.contains("file.txt"));
    }

    #[test]
    fn path_display_handles_simple_path() {
        let path = std::path::Path::new("/home/user");
        let display = path_display(path);
        assert!(display.contains("user"));
    }

    #[test]
    fn is_recoverable_returns_true_for_permission_denied() {
        let err = ScanError::PermissionDenied {
            path: PathBuf::from("/test"),
        };
        assert!(is_recoverable(&err));
    }

    #[test]
    fn is_recoverable_returns_true_for_metadata_failed() {
        let err = ScanError::MetadataFailed {
            path: PathBuf::from("/test"),
            source: io::Error::new(ErrorKind::Other, "test"),
        };
        assert!(is_recoverable(&err));
    }

    #[test]
    fn is_recoverable_returns_true_for_path_not_found() {
        let err = ScanError::PathNotFound {
            path: PathBuf::from("/test"),
        };
        assert!(is_recoverable(&err));
    }

    #[test]
    fn is_recoverable_returns_false_for_not_a_directory() {
        let err = ScanError::NotADirectory {
            path: PathBuf::from("/test"),
        };
        assert!(!is_recoverable(&err));
    }

    #[test]
    fn is_recoverable_returns_false_for_walk_error() {
        let err = ScanError::WalkError {
            message: "test error".to_string(),
            path: None,
        };
        assert!(!is_recoverable(&err));
    }

    #[test]
    fn is_recoverable_returns_false_for_read_dir_failed() {
        let err = ScanError::ReadDirFailed {
            path: PathBuf::from("/test"),
            source: io::Error::new(ErrorKind::Other, "test"),
        };
        assert!(!is_recoverable(&err));
    }

    #[test]
    fn is_recoverable_returns_false_for_canonicalize_failed() {
        let err = ScanError::CanonicalizeFailed {
            path: PathBuf::from("/test"),
            source: io::Error::new(ErrorKind::Other, "test"),
        };
        assert!(!is_recoverable(&err));
    }

    #[test]
    fn scan_error_read_dir_failed_preserves_source() {
        let io_err = io::Error::new(ErrorKind::Other, "underlying error");
        let err = ScanError::ReadDirFailed {
            path: PathBuf::from("/test"),
            source: io_err,
        };

        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }

    #[test]
    fn scan_error_metadata_failed_preserves_source() {
        let io_err = io::Error::new(ErrorKind::Other, "metadata error");
        let err = ScanError::MetadataFailed {
            path: PathBuf::from("/test"),
            source: io_err,
        };

        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }

    #[test]
    fn scan_error_canonicalize_failed_preserves_source() {
        let io_err = io::Error::new(ErrorKind::NotFound, "not found");
        let err = ScanError::CanonicalizeFailed {
            path: PathBuf::from("/test"),
            source: io_err,
        };

        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }

    #[test]
    fn output_error_file_create_failed_preserves_source() {
        let io_err = io::Error::new(ErrorKind::PermissionDenied, "no permission");
        let err = OutputError::FileCreateFailed {
            path: PathBuf::from("test.txt"),
            source: io_err,
        };

        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }

    #[test]
    fn output_error_write_failed_preserves_source() {
        let io_err = io::Error::new(ErrorKind::Other, "write error");
        let err = OutputError::WriteFailed {
            path: PathBuf::from("test.txt"),
            source: io_err,
        };

        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }

    #[test]
    fn output_error_stdout_failed_preserves_source() {
        let io_err = io::Error::new(ErrorKind::BrokenPipe, "broken");
        let err = OutputError::StdoutFailed { source: io_err };

        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }

    #[test]
    fn treepp_result_type_alias_works_correctly() {
        fn test_ok() -> TreeppResult<i32> {
            Ok(42)
        }

        fn test_err() -> TreeppResult<i32> {
            Err(CliError::UnknownOption {
                option: "/X".to_string(),
            }
                .into())
        }

        assert_eq!(test_ok().unwrap(), 42);
        assert!(test_err().is_err());
    }

    #[test]
    fn cli_error_invalid_path_formats_correctly() {
        let err = CliError::InvalidPath {
            arg: ":::invalid:::".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains(":::invalid:::"));
    }

    #[test]
    fn cli_error_clone_produces_equal_value() {
        let err = CliError::UnknownOption {
            option: "/Z".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn match_error_clone_produces_equal_value() {
        let err = MatchError::InvalidPattern {
            pattern: "*.txt".to_string(),
            reason: "test".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn render_error_clone_produces_equal_value() {
        let err = RenderError::FormatError {
            context: "test".to_string(),
            detail: "detail".to_string(),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn scan_error_walk_error_with_none_path_formats_correctly() {
        let err = ScanError::WalkError {
            message: "generic walk error".to_string(),
            path: None,
        };
        let msg = err.to_string();
        assert!(msg.contains("generic walk error"));
    }

    #[test]
    fn scan_error_walk_error_with_some_path_stores_path() {
        let err = ScanError::WalkError {
            message: "walk error".to_string(),
            path: Some(PathBuf::from("/some/path")),
        };
        match err {
            ScanError::WalkError { path, .. } => {
                assert!(path.is_some());
                assert_eq!(path.unwrap(), PathBuf::from("/some/path"));
            }
            _ => panic!("Expected WalkError variant"),
        }
    }
}
