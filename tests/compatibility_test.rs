//! Compatibility tests for tree++ against Windows native tree command.
//!
//! This module verifies that tree++ produces output strictly identical to the
//! Windows native `tree` command when using compatible parameters.
//!
//! # Test Coverage
//!
//! - No arguments (directories only)
//! - `/F` parameter (show files)
//! - `/A` parameter (ASCII characters)
//! - `/F /A` combined parameters
//!
//! # Comparison Strategy
//!
//! Output must be byte-for-byte identical, ignoring trailing whitespace differences.
//!
//! File: tests/compatibility_test.rs
//! Author: WaterRun
//! Date: 2026-01-12

use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

// ============================================================================
// Types
// ============================================================================

/// Captured output from a command execution.
///
/// Stores stdout, stderr, and exit code for comparison between
/// native tree and treepp commands.
///
/// # Examples
///
/// ```rust,ignore
/// let output = CommandOutput {
///     stdout: "directory listing".to_string(),
///     stderr: String::new(),
///     exit_code: Some(0),
/// };
/// assert_eq!(output.exit_code, Some(0));
/// ```
#[derive(Debug, Clone)]
struct CommandOutput {
    /// Standard output content.
    stdout: String,
    /// Standard error content.
    #[allow(dead_code)]
    stderr: String,
    /// Process exit code, if available.
    exit_code: Option<i32>,
}

impl CommandOutput {
    /// Creates a `CommandOutput` from native tree command output.
    ///
    /// Decodes stdout and stderr using GBK encoding, which is the default
    /// encoding for Windows command prompt on Chinese systems.
    ///
    /// # Arguments
    ///
    /// * `output` - Raw output from `std::process::Command`
    ///
    /// # Returns
    ///
    /// A new `CommandOutput` with GBK-decoded strings.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let output = Command::new("tree").output().unwrap();
    /// let cmd_output = CommandOutput::from_native_output(&output);
    /// ```
    fn from_native_output(output: &Output) -> Self {
        let (stdout, _, _) = encoding_rs::GBK.decode(&output.stdout);
        let (stderr, _, _) = encoding_rs::GBK.decode(&output.stderr);
        Self {
            stdout: stdout.into_owned(),
            stderr: stderr.into_owned(),
            exit_code: output.status.code(),
        }
    }

    /// Creates a `CommandOutput` from treepp command output.
    ///
    /// Decodes stdout and stderr as UTF-8, which is treepp's output encoding.
    ///
    /// # Arguments
    ///
    /// * `output` - Raw output from `std::process::Command`
    ///
    /// # Returns
    ///
    /// A new `CommandOutput` with UTF-8 decoded strings.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let output = Command::new("treepp").output().unwrap();
    /// let cmd_output = CommandOutput::from_treepp_output(&output);
    /// ```
    fn from_treepp_output(output: &Output) -> Self {
        Self {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
        }
    }

    /// Returns normalized output lines with trailing whitespace removed.
    ///
    /// This normalization allows comparison between outputs that may differ
    /// only in trailing whitespace.
    ///
    /// # Returns
    ///
    /// A vector of strings, each representing one line with trailing
    /// whitespace trimmed.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let output = CommandOutput {
    ///     stdout: "line1  \nline2\n".to_string(),
    ///     stderr: String::new(),
    ///     exit_code: Some(0),
    /// };
    /// let lines = output.normalized_lines();
    /// assert_eq!(lines, vec!["line1", "line2"]);
    /// ```
    fn normalized_lines(&self) -> Vec<String> {
        self.stdout
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }
}

// ============================================================================
// Command Execution Functions
// ============================================================================

/// Executes the native Windows tree command.
///
/// Runs `cmd /C tree` with the specified arguments and path.
///
/// # Arguments
///
/// * `path` - Directory path to run tree on
/// * `args` - Command line arguments to pass to tree
///
/// # Returns
///
/// Captured command output with GBK decoding applied.
///
/// # Panics
///
/// Panics if the tree command fails to execute.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = TempDir::new().unwrap();
/// let output = run_native_tree(dir.path(), &["/F"]);
/// assert_eq!(output.exit_code, Some(0));
/// ```
fn run_native_tree(path: &Path, args: &[&str]) -> CommandOutput {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "tree"]);
    cmd.args(args);
    cmd.arg(path);
    let output = cmd.output().expect("Failed to execute native tree command");
    CommandOutput::from_native_output(&output)
}

/// Executes the treepp command.
///
/// Runs treepp with the specified arguments and path.
///
/// # Arguments
///
/// * `path` - Directory path to run treepp on
/// * `args` - Command line arguments to pass to treepp
///
/// # Returns
///
/// Captured command output with UTF-8 decoding applied.
///
/// # Panics
///
/// Panics if the treepp command fails to execute.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = TempDir::new().unwrap();
/// let output = run_treepp(dir.path(), &["/F"]);
/// assert_eq!(output.exit_code, Some(0));
/// ```
fn run_treepp(path: &Path, args: &[&str]) -> CommandOutput {
    let treepp_path = get_treepp_path();
    let mut cmd = Command::new(&treepp_path);
    cmd.args(args);
    cmd.arg(path);
    let output = cmd.output().expect("Failed to execute treepp command");
    CommandOutput::from_treepp_output(&output)
}

/// Locates the treepp executable.
///
/// Searches for treepp.exe in debug and release target directories.
///
/// # Returns
///
/// Path to the treepp executable.
///
/// # Panics
///
/// Panics if treepp.exe is not found in either location.
///
/// # Examples
///
/// ```rust,ignore
/// let path = get_treepp_path();
/// assert!(path.exists());
/// ```
fn get_treepp_path() -> PathBuf {
    let debug_path = PathBuf::from("target/debug/treepp.exe");
    if debug_path.exists() {
        return debug_path;
    }
    let release_path = PathBuf::from("target/release/treepp.exe");
    if release_path.exists() {
        return release_path;
    }
    panic!("treepp not built, please run cargo build first");
}

// ============================================================================
// Comparison and Assertion Functions
// ============================================================================

/// Reports differences between native and treepp output.
///
/// Compares normalized lines from both outputs and panics with a detailed
/// diff report if differences are found. Shows up to 3 differences.
///
/// # Arguments
///
/// * `native` - Output from native tree command
/// * `treepp` - Output from treepp command
/// * `context` - Description of the test case for error messages
///
/// # Panics
///
/// Panics if the outputs differ, with a formatted diff report.
///
/// # Examples
///
/// ```rust,ignore
/// let native = run_native_tree(path, &[]);
/// let treepp = run_treepp(path, &[]);
/// compact_diff(&native, &treepp, "empty directory");
/// ```
fn compact_diff(native: &CommandOutput, treepp: &CommandOutput, context: &str) {
    let native_lines = native.normalized_lines();
    let treepp_lines = treepp.normalized_lines();

    if native_lines == treepp_lines {
        return;
    }

    let mut report = format!("\n=== {} mismatch ===\n", context);
    report.push_str(&format!(
        "Line count: native={}, treepp={}\n",
        native_lines.len(),
        treepp_lines.len()
    ));

    let mut diff_count = 0;
    let max_lines = native_lines.len().max(treepp_lines.len());

    for i in 0..max_lines {
        let n = native_lines.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
        let t = treepp_lines.get(i).map(|s| s.as_str()).unwrap_or("<missing>");

        if n != t {
            diff_count += 1;
            if diff_count <= 3 {
                report.push_str(&format!(
                    "L{}: N={:?}\n    T={:?}\n",
                    i + 1,
                    truncate_string(n, 60),
                    truncate_string(t, 60)
                ));
            }
        }
    }

    if diff_count > 3 {
        report.push_str(&format!("...and {} more differences\n", diff_count - 3));
    }

    panic!("{}", report);
}

/// Truncates a string to a maximum length.
///
/// # Arguments
///
/// * `s` - String to truncate
/// * `max_len` - Maximum length before truncation
///
/// # Returns
///
/// Original string if within limit, or truncated string with "..." suffix.
///
/// # Examples
///
/// ```rust,ignore
/// assert_eq!(truncate_string("hello", 10), "hello");
/// assert_eq!(truncate_string("hello world", 5), "hello...");
/// ```
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Asserts that exit codes match between native and treepp.
///
/// # Arguments
///
/// * `native` - Output from native tree command
/// * `treepp` - Output from treepp command
/// * `context` - Description of the test case for error messages
///
/// # Panics
///
/// Panics if exit codes differ.
///
/// # Examples
///
/// ```rust,ignore
/// let native = run_native_tree(path, &[]);
/// let treepp = run_treepp(path, &[]);
/// assert_exit_codes(&native, &treepp, "empty directory");
/// ```
fn assert_exit_codes(native: &CommandOutput, treepp: &CommandOutput, context: &str) {
    assert_eq!(
        native.exit_code, treepp.exit_code,
        "{}: exit code mismatch N={:?} T={:?}",
        context, native.exit_code, treepp.exit_code
    );
}

// ============================================================================
// Test Directory Creation Functions
// ============================================================================

/// Creates an empty temporary directory.
///
/// # Returns
///
/// A `TempDir` that will be cleaned up when dropped.
///
/// # Panics
///
/// Panics if directory creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_empty_dir();
/// assert!(dir.path().exists());
/// ```
fn create_empty_dir() -> TempDir {
    TempDir::new().expect("Failed to create temporary directory")
}

/// Creates a directory with three subdirectories.
///
/// Structure:
/// - alpha/
/// - beta/
/// - gamma/
///
/// # Returns
///
/// A `TempDir` containing the directory structure.
///
/// # Panics
///
/// Panics if directory creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_single_level_dirs();
/// assert!(dir.path().join("alpha").exists());
/// ```
fn create_single_level_dirs() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("alpha")).unwrap();
    fs::create_dir(dir.path().join("beta")).unwrap();
    fs::create_dir(dir.path().join("gamma")).unwrap();
    dir
}

/// Creates a directory with subdirectories and files at root level.
///
/// Structure:
/// - alpha/
/// - beta/
/// - file1.txt
/// - file2.txt
///
/// # Returns
///
/// A `TempDir` containing the directory structure.
///
/// # Panics
///
/// Panics if creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_single_level_with_files();
/// assert!(dir.path().join("file1.txt").exists());
/// ```
fn create_single_level_with_files() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("alpha")).unwrap();
    fs::create_dir(dir.path().join("beta")).unwrap();
    File::create(dir.path().join("file1.txt")).unwrap();
    File::create(dir.path().join("file2.txt")).unwrap();
    dir
}

/// Creates a nested directory structure without files.
///
/// Structure:
/// - a/
///   - b/
///     - c/
///   - d/
/// - e/
///
/// # Returns
///
/// A `TempDir` containing the nested structure.
///
/// # Panics
///
/// Panics if creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_nested_dirs();
/// assert!(dir.path().join("a/b/c").exists());
/// ```
fn create_nested_dirs() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("a/b/c")).unwrap();
    fs::create_dir(dir.path().join("a/d")).unwrap();
    fs::create_dir(dir.path().join("e")).unwrap();
    dir
}

/// Creates a nested directory structure with files.
///
/// Structure:
/// - Cargo.toml
/// - src/
///   - main.rs
///   - lib.rs
///   - utils/
///     - helper.rs
/// - tests/
///   - test.rs
///
/// # Returns
///
/// A `TempDir` containing the project-like structure.
///
/// # Panics
///
/// Panics if creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_nested_with_files();
/// assert!(dir.path().join("src/main.rs").exists());
/// ```
fn create_nested_with_files() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src/utils")).unwrap();
    fs::create_dir(dir.path().join("tests")).unwrap();
    File::create(dir.path().join("Cargo.toml")).unwrap();
    File::create(dir.path().join("src/main.rs")).unwrap();
    File::create(dir.path().join("src/lib.rs")).unwrap();
    File::create(dir.path().join("src/utils/helper.rs")).unwrap();
    File::create(dir.path().join("tests/test.rs")).unwrap();
    dir
}

/// Creates a structure with empty subdirectories.
///
/// Structure:
/// - empty1/
/// - empty2/
///   - nested/
/// - has_file/
///   - f.txt
///
/// # Returns
///
/// A `TempDir` containing mixed empty and non-empty directories.
///
/// # Panics
///
/// Panics if creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_with_empty_subdirs();
/// assert!(dir.path().join("empty1").exists());
/// ```
fn create_with_empty_subdirs() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("empty1")).unwrap();
    fs::create_dir_all(dir.path().join("empty2/nested")).unwrap();
    fs::create_dir(dir.path().join("has_file")).unwrap();
    File::create(dir.path().join("has_file/f.txt")).unwrap();
    dir
}

/// Creates a simple project-like structure.
///
/// Structure:
/// - Cargo.toml
/// - README.md
/// - src/
///   - main.rs
///   - lib.rs
///
/// # Returns
///
/// A `TempDir` containing the project structure.
///
/// # Panics
///
/// Panics if creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_project_like();
/// assert!(dir.path().join("Cargo.toml").exists());
/// ```
fn create_project_like() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    File::create(dir.path().join("Cargo.toml")).unwrap();
    File::create(dir.path().join("README.md")).unwrap();
    File::create(dir.path().join("src/main.rs")).unwrap();
    File::create(dir.path().join("src/lib.rs")).unwrap();
    dir
}

/// Creates a deeply nested directory structure.
///
/// Structure:
/// - a/
///   - b/
///     - mid.txt
///     - c/
///       - d/
///         - e/
///           - deep.txt
///
/// # Returns
///
/// A `TempDir` containing the deep structure.
///
/// # Panics
///
/// Panics if creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_deep_nested();
/// assert!(dir.path().join("a/b/c/d/e/deep.txt").exists());
/// ```
fn create_deep_nested() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("a/b/c/d/e")).unwrap();
    File::create(dir.path().join("a/b/c/d/e/deep.txt")).unwrap();
    File::create(dir.path().join("a/b/mid.txt")).unwrap();
    dir
}

/// Creates a wide directory structure with multiple siblings.
///
/// Structure:
/// - alpha/
///   - file.txt
/// - beta/
///   - file.txt
/// - gamma/
///   - file.txt
/// - delta/
///   - file.txt
///
/// # Returns
///
/// A `TempDir` containing the wide structure.
///
/// # Panics
///
/// Panics if creation fails.
///
/// # Examples
///
/// ```rust,ignore
/// let dir = create_wide_structure();
/// assert!(dir.path().join("alpha/file.txt").exists());
/// ```
fn create_wide_structure() -> TempDir {
    let dir = TempDir::new().unwrap();
    for name in ["alpha", "beta", "gamma", "delta"] {
        fs::create_dir(dir.path().join(name)).unwrap();
        File::create(dir.path().join(name).join("file.txt")).unwrap();
    }
    dir
}

/// Returns the path to the user's .cargo directory if it exists.
///
/// # Returns
///
/// `Some(PathBuf)` if .cargo exists, `None` otherwise.
///
/// # Examples
///
/// ```rust,ignore
/// if let Some(cargo_dir) = get_cargo_dir() {
///     println!("Cargo directory: {:?}", cargo_dir);
/// }
/// ```
fn get_cargo_dir() -> Option<PathBuf> {
    let home = env::var("USERPROFILE").ok()?;
    let cargo_dir = PathBuf::from(home).join(".cargo");
    if cargo_dir.exists() && cargo_dir.is_dir() {
        Some(cargo_dir)
    } else {
        None
    }
}

/// Creates a directory with only dot-prefixed files.
///
/// Structure:
/// - .config
/// - .env
/// - .settings
///
/// # Returns
///
/// A `TempDir` containing only hidden files.
///
/// # Panics
///
/// Panics if creation fails.
fn create_only_dotfiles() -> TempDir {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join(".config")).unwrap();
    File::create(dir.path().join(".env")).unwrap();
    File::create(dir.path().join(".settings")).unwrap();
    dir
}

/// Creates a directory with files having various extensions.
///
/// Structure:
/// - file.txt
/// - file.rs
/// - file.md
/// - file.json
/// - file.toml
/// - no_extension
///
/// # Returns
///
/// A `TempDir` containing files with different extensions.
///
/// # Panics
///
/// Panics if creation fails.
fn create_various_extensions() -> TempDir {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("file.txt")).unwrap();
    File::create(dir.path().join("file.rs")).unwrap();
    File::create(dir.path().join("file.md")).unwrap();
    File::create(dir.path().join("file.json")).unwrap();
    File::create(dir.path().join("file.toml")).unwrap();
    File::create(dir.path().join("no_extension")).unwrap();
    dir
}

/// Creates a symmetrical tree structure.
///
/// Structure:
/// - left/
///   - a/
///   - b/
/// - right/
///   - a/
///   - b/
///
/// # Returns
///
/// A `TempDir` containing the symmetrical structure.
///
/// # Panics
///
/// Panics if creation fails.
fn create_symmetrical_tree() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("left/a")).unwrap();
    fs::create_dir(dir.path().join("left/b")).unwrap();
    fs::create_dir_all(dir.path().join("right/a")).unwrap();
    fs::create_dir(dir.path().join("right/b")).unwrap();
    dir
}

/// Creates a directory with mixed directory and file siblings.
///
/// Structure:
/// - aaa_dir/
/// - aaa_file.txt
/// - bbb_dir/
/// - bbb_file.txt
/// - ccc_dir/
/// - ccc_file.txt
///
/// # Returns
///
/// A `TempDir` with interleaved directories and files.
///
/// # Panics
///
/// Panics if creation fails.
fn create_interleaved_dir_file() -> TempDir {
    let dir = TempDir::new().unwrap();
    for prefix in ["aaa", "bbb", "ccc"] {
        fs::create_dir(dir.path().join(format!("{}_dir", prefix))).unwrap();
        File::create(dir.path().join(format!("{}_file.txt", prefix))).unwrap();
    }
    dir
}

/// Creates a structure with single-character names.
///
/// Structure:
/// - a/
/// - b/
/// - c/
/// - x.txt
/// - y.txt
/// - z.txt
///
/// # Returns
///
/// A `TempDir` with single-character named items.
///
/// # Panics
///
/// Panics if creation fails.
fn create_single_char_names() -> TempDir {
    let dir = TempDir::new().unwrap();
    for c in ['a', 'b', 'c'] {
        fs::create_dir(dir.path().join(c.to_string())).unwrap();
    }
    for c in ['x', 'y', 'z'] {
        File::create(dir.path().join(format!("{}.txt", c))).unwrap();
    }
    dir
}

// ============================================================================
// Tests: No Arguments
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_match_native_output_for_empty_directory() {
        let dir = create_empty_dir();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "空目录");
        compact_diff(&native, &treepp, "空目录-无参数");
    }

    #[test]
    fn should_match_native_output_for_single_level_directories() {
        let dir = create_single_level_dirs();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "单层目录");
        compact_diff(&native, &treepp, "单层目录-无参数");
    }

    #[test]
    fn should_match_native_output_for_single_level_with_files_no_args() {
        let dir = create_single_level_with_files();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "单层带文件");
        compact_diff(&native, &treepp, "单层带文件-无参数");
    }

    #[test]
    fn should_match_native_output_for_nested_directories() {
        let dir = create_nested_dirs();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "嵌套目录");
        compact_diff(&native, &treepp, "嵌套目录-无参数");
    }

    #[test]
    fn should_match_native_output_for_nested_with_files_no_args() {
        let dir = create_nested_with_files();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "嵌套带文件");
        compact_diff(&native, &treepp, "嵌套带文件-无参数");
    }

    #[test]
    fn should_match_native_output_for_empty_subdirectories() {
        let dir = create_with_empty_subdirs();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "含空子目录");
        compact_diff(&native, &treepp, "含空子目录-无参数");
    }

    #[test]
    fn should_match_native_output_for_project_structure() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "项目结构");
        compact_diff(&native, &treepp, "项目结构-无参数");
    }

    #[test]
    fn should_match_native_output_for_deep_nesting() {
        let dir = create_deep_nested();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "深层嵌套");
        compact_diff(&native, &treepp, "深层嵌套-无参数");
    }

    #[test]
    fn should_match_native_output_for_wide_structure() {
        let dir = create_wide_structure();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "宽结构");
        compact_diff(&native, &treepp, "宽结构-无参数");
    }

    #[test]
    fn should_match_native_output_for_symmetrical_tree() {
        let dir = create_symmetrical_tree();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "对称结构");
        compact_diff(&native, &treepp, "对称结构-无参数");
    }

    // ========================================================================
    // Tests: /F Parameter
    // ========================================================================

    #[test]
    fn should_match_native_output_for_empty_directory_with_f_flag() {
        let dir = create_empty_dir();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "空目录/F");
        compact_diff(&native, &treepp, "空目录-/F");
    }

    #[test]
    fn should_match_native_output_for_single_level_directories_with_f_flag() {
        let dir = create_single_level_dirs();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "单层目录/F");
        compact_diff(&native, &treepp, "单层目录-/F");
    }

    #[test]
    fn should_match_native_output_for_single_level_with_files_f_flag() {
        let dir = create_single_level_with_files();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "单层带文件/F");
        compact_diff(&native, &treepp, "单层带文件-/F");
    }

    #[test]
    fn should_match_native_output_for_nested_directories_with_f_flag() {
        let dir = create_nested_dirs();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "嵌套目录/F");
        compact_diff(&native, &treepp, "嵌套目录-/F");
    }

    #[test]
    fn should_match_native_output_for_nested_with_files_f_flag() {
        let dir = create_nested_with_files();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "嵌套带文件/F");
        compact_diff(&native, &treepp, "嵌套带文件-/F");
    }

    #[test]
    fn should_match_native_output_for_empty_subdirectories_with_f_flag() {
        let dir = create_with_empty_subdirs();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "含空子目录/F");
        compact_diff(&native, &treepp, "含空子目录-/F");
    }

    #[test]
    fn should_match_native_output_for_project_structure_with_f_flag() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "项目结构/F");
        compact_diff(&native, &treepp, "项目结构-/F");
    }

    #[test]
    fn should_match_native_output_for_deep_nesting_with_f_flag() {
        let dir = create_deep_nested();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "深层嵌套/F");
        compact_diff(&native, &treepp, "深层嵌套-/F");
    }

    #[test]
    fn should_match_native_output_for_wide_structure_with_f_flag() {
        let dir = create_wide_structure();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "宽结构/F");
        compact_diff(&native, &treepp, "宽结构-/F");
    }

    #[test]
    fn should_match_native_output_for_various_extensions_with_f_flag() {
        let dir = create_various_extensions();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "多种扩展名/F");
        compact_diff(&native, &treepp, "多种扩展名-/F");
    }

    #[test]
    fn should_match_native_output_for_interleaved_dir_file_with_f_flag() {
        let dir = create_interleaved_dir_file();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "交错目录文件/F");
        compact_diff(&native, &treepp, "交错目录文件-/F");
    }

    // ========================================================================
    // Tests: /A Parameter
    // ========================================================================

    #[test]
    fn should_match_native_output_for_empty_directory_with_a_flag() {
        let dir = create_empty_dir();
        let native = run_native_tree(dir.path(), &["/A"]);
        let treepp = run_treepp(dir.path(), &["/A"]);
        assert_exit_codes(&native, &treepp, "空目录/A");
        compact_diff(&native, &treepp, "空目录-/A");
    }

    #[test]
    fn should_match_native_output_for_single_level_directories_with_a_flag() {
        let dir = create_single_level_dirs();
        let native = run_native_tree(dir.path(), &["/A"]);
        let treepp = run_treepp(dir.path(), &["/A"]);
        assert_exit_codes(&native, &treepp, "单层目录/A");
        compact_diff(&native, &treepp, "单层目录-/A");
    }

    #[test]
    fn should_match_native_output_for_nested_directories_with_a_flag() {
        let dir = create_nested_dirs();
        let native = run_native_tree(dir.path(), &["/A"]);
        let treepp = run_treepp(dir.path(), &["/A"]);
        assert_exit_codes(&native, &treepp, "嵌套目录/A");
        compact_diff(&native, &treepp, "嵌套目录-/A");
    }

    #[test]
    fn should_match_native_output_for_project_structure_with_a_flag() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &["/A"]);
        let treepp = run_treepp(dir.path(), &["/A"]);
        assert_exit_codes(&native, &treepp, "项目结构/A");
        compact_diff(&native, &treepp, "项目结构-/A");
    }

    #[test]
    fn should_match_native_output_for_wide_structure_with_a_flag() {
        let dir = create_wide_structure();
        let native = run_native_tree(dir.path(), &["/A"]);
        let treepp = run_treepp(dir.path(), &["/A"]);
        assert_exit_codes(&native, &treepp, "宽结构/A");
        compact_diff(&native, &treepp, "宽结构-/A");
    }

    #[test]
    fn should_match_native_output_for_deep_nesting_with_a_flag() {
        let dir = create_deep_nested();
        let native = run_native_tree(dir.path(), &["/A"]);
        let treepp = run_treepp(dir.path(), &["/A"]);
        assert_exit_codes(&native, &treepp, "深层嵌套/A");
        compact_diff(&native, &treepp, "深层嵌套-/A");
    }

    // ========================================================================
    // Tests: /F /A Combined
    // ========================================================================

    #[test]
    fn should_match_native_output_for_empty_directory_with_fa_flags() {
        let dir = create_empty_dir();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "空目录/F/A");
        compact_diff(&native, &treepp, "空目录-/F/A");
    }

    #[test]
    fn should_match_native_output_for_single_level_directories_with_fa_flags() {
        let dir = create_single_level_dirs();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "单层目录/F/A");
        compact_diff(&native, &treepp, "单层目录-/F/A");
    }

    #[test]
    fn should_match_native_output_for_single_level_with_files_fa_flags() {
        let dir = create_single_level_with_files();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "单层带文件/F/A");
        compact_diff(&native, &treepp, "单层带文件-/F/A");
    }

    #[test]
    fn should_match_native_output_for_nested_directories_with_fa_flags() {
        let dir = create_nested_dirs();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "嵌套目录/F/A");
        compact_diff(&native, &treepp, "嵌套目录-/F/A");
    }

    #[test]
    fn should_match_native_output_for_nested_with_files_fa_flags() {
        let dir = create_nested_with_files();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "嵌套带文件/F/A");
        compact_diff(&native, &treepp, "嵌套带文件-/F/A");
    }

    #[test]
    fn should_match_native_output_for_project_structure_with_fa_flags() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "项目结构/F/A");
        compact_diff(&native, &treepp, "项目结构-/F/A");
    }

    #[test]
    fn should_match_native_output_for_deep_nesting_with_fa_flags() {
        let dir = create_deep_nested();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "深层嵌套/F/A");
        compact_diff(&native, &treepp, "深层嵌套-/F/A");
    }

    #[test]
    fn should_match_native_output_for_wide_structure_with_fa_flags() {
        let dir = create_wide_structure();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "宽结构/F/A");
        compact_diff(&native, &treepp, "宽结构-/F/A");
    }

    #[test]
    fn should_match_native_output_for_only_dotfiles_with_fa_flags() {
        let dir = create_only_dotfiles();
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "仅点文件/F/A");
        compact_diff(&native, &treepp, "仅点文件-/F/A");
    }

    // ========================================================================
    // Tests: Argument Variants
    // ========================================================================

    #[test]
    fn should_match_native_output_for_lowercase_f_flag() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &["/f"]);
        let treepp = run_treepp(dir.path(), &["/f"]);
        assert_exit_codes(&native, &treepp, "小写/f");
        compact_diff(&native, &treepp, "小写-/f");
    }

    #[test]
    fn should_match_native_output_for_lowercase_a_flag() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &["/a"]);
        let treepp = run_treepp(dir.path(), &["/a"]);
        assert_exit_codes(&native, &treepp, "小写/a");
        compact_diff(&native, &treepp, "小写-/a");
    }

    #[test]
    fn should_match_native_output_for_reversed_af_flag_order() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &["/A", "/F"]);
        let treepp = run_treepp(dir.path(), &["/A", "/F"]);
        assert_exit_codes(&native, &treepp, "反序/A/F");
        compact_diff(&native, &treepp, "反序-/A/F");
    }

    #[test]
    fn should_match_native_output_for_mixed_case_flags() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &["/f", "/A"]);
        let treepp = run_treepp(dir.path(), &["/f", "/A"]);
        assert_exit_codes(&native, &treepp, "混合大小写/f/A");
        compact_diff(&native, &treepp, "混合大小写-/f/A");
    }

    // ========================================================================
    // Tests: Real Directory (.cargo)
    // ========================================================================

    #[test]
    fn should_match_native_output_for_cargo_directory() {
        let Some(cargo) = get_cargo_dir() else {
            eprintln!("Skip: .cargo does not exist");
            return;
        };
        let native = run_native_tree(&cargo, &[]);
        let treepp = run_treepp(&cargo, &[]);
        assert_exit_codes(&native, &treepp, ".cargo");
        compact_diff(&native, &treepp, ".cargo-无参数");
    }

    #[test]
    fn should_match_native_output_for_cargo_directory_with_f_flag() {
        let Some(cargo) = get_cargo_dir() else {
            eprintln!("Skip: .cargo does not exist");
            return;
        };
        let native = run_native_tree(&cargo, &["/F"]);
        let treepp = run_treepp(&cargo, &["/F"]);
        assert_exit_codes(&native, &treepp, ".cargo/F");
        compact_diff(&native, &treepp, ".cargo-/F");
    }

    #[test]
    fn should_match_native_output_for_cargo_directory_with_a_flag() {
        let Some(cargo) = get_cargo_dir() else {
            eprintln!("Skip: .cargo does not exist");
            return;
        };
        let native = run_native_tree(&cargo, &["/A"]);
        let treepp = run_treepp(&cargo, &["/A"]);
        assert_exit_codes(&native, &treepp, ".cargo/A");
        compact_diff(&native, &treepp, ".cargo-/A");
    }

    #[test]
    fn should_match_native_output_for_cargo_directory_with_fa_flags() {
        let Some(cargo) = get_cargo_dir() else {
            eprintln!("Skip: .cargo does not exist");
            return;
        };
        let native = run_native_tree(&cargo, &["/F", "/A"]);
        let treepp = run_treepp(&cargo, &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, ".cargo/F/A");
        compact_diff(&native, &treepp, ".cargo-/F/A");
    }

    // ========================================================================
    // Tests: Edge Cases
    // ========================================================================

    #[test]
    fn should_match_native_output_for_single_file_only() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("only.txt")).unwrap();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "单文件");
        compact_diff(&native, &treepp, "单文件-/F");
    }

    #[test]
    fn should_match_native_output_for_single_subdirectory() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("only")).unwrap();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "单子目录");
        compact_diff(&native, &treepp, "单子目录");
    }

    #[test]
    fn should_match_native_output_for_hidden_files() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join(".hidden")).unwrap();
        File::create(dir.path().join(".gitignore")).unwrap();
        File::create(dir.path().join("normal.txt")).unwrap();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "隐藏文件");
        compact_diff(&native, &treepp, "隐藏文件-/F");
    }

    #[test]
    fn should_match_native_output_for_special_characters_in_names() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("file with spaces.txt")).unwrap();
        File::create(dir.path().join("file-dash.txt")).unwrap();
        File::create(dir.path().join("file_underscore.txt")).unwrap();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "特殊字符");
        compact_diff(&native, &treepp, "特殊字符-/F");
    }

    #[test]
    fn should_match_native_output_for_alphabetic_sorting() {
        let dir = TempDir::new().unwrap();
        for name in ["zebra.txt", "apple.txt", "mango.txt"] {
            File::create(dir.path().join(name)).unwrap();
        }
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "字母排序");
        compact_diff(&native, &treepp, "字母排序-/F");
    }

    #[test]
    fn should_match_native_output_for_mixed_case_sorting() {
        let dir = TempDir::new().unwrap();
        for name in ["AAA.txt", "aaa.txt", "BBB.txt", "bbb.txt"] {
            File::create(dir.path().join(name)).unwrap();
        }
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "大小写混合");
        compact_diff(&native, &treepp, "大小写混合-/F");
    }

    #[test]
    fn should_match_native_output_for_numeric_sorting() {
        let dir = TempDir::new().unwrap();
        for name in ["1.txt", "10.txt", "2.txt", "20.txt"] {
            File::create(dir.path().join(name)).unwrap();
        }
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "数字排序");
        compact_diff(&native, &treepp, "数字排序-/F");
    }

    #[test]
    fn should_match_native_output_for_directory_file_ordering() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("dir_a")).unwrap();
        File::create(dir.path().join("file_a.txt")).unwrap();
        fs::create_dir(dir.path().join("dir_b")).unwrap();
        File::create(dir.path().join("file_b.txt")).unwrap();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "目录文件顺序");
        compact_diff(&native, &treepp, "目录文件顺序-/F");
    }

    #[test]
    fn should_match_native_output_for_unicode_filenames() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("中文.txt")).unwrap();
        File::create(dir.path().join("日本語.txt")).unwrap();
        File::create(dir.path().join("normal.txt")).unwrap();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "Unicode文件名");
        compact_diff(&native, &treepp, "Unicode-/F");
    }

    #[test]
    fn should_match_native_output_for_long_filenames() {
        let dir = TempDir::new().unwrap();
        let long_name = "a".repeat(100) + ".txt";
        File::create(dir.path().join(&long_name)).unwrap();
        File::create(dir.path().join("short.txt")).unwrap();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "长文件名");
        compact_diff(&native, &treepp, "长文件名-/F");
    }

    #[test]
    fn should_match_native_output_for_deep_empty_directories() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("a/b/c/d/e")).unwrap();
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "深层空目录");
        compact_diff(&native, &treepp, "深层空目录");
    }

    #[test]
    fn should_match_native_output_for_single_char_names() {
        let dir = create_single_char_names();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "单字符名/F");
        compact_diff(&native, &treepp, "单字符名-/F");
    }

    #[test]
    fn should_match_native_output_for_files_with_multiple_dots() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("file.backup.txt")).unwrap();
        File::create(dir.path().join("archive.tar.gz")).unwrap();
        File::create(dir.path().join("...weird")).unwrap();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "多点文件名/F");
        compact_diff(&native, &treepp, "多点文件名-/F");
    }

    // ========================================================================
    // Tests: Stress Tests
    // ========================================================================

    #[test]
    fn should_match_native_output_for_many_files() {
        let dir = TempDir::new().unwrap();
        for i in 0..30 {
            File::create(dir.path().join(format!("file_{:03}.txt", i))).unwrap();
        }
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "多文件");
        compact_diff(&native, &treepp, "多文件-/F");
    }

    #[test]
    fn should_match_native_output_for_many_directories() {
        let dir = TempDir::new().unwrap();
        for i in 0..20 {
            fs::create_dir(dir.path().join(format!("dir_{:03}", i))).unwrap();
        }
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "多目录");
        compact_diff(&native, &treepp, "多目录");
    }

    #[test]
    fn should_match_native_output_for_mixed_large_structure() {
        let dir = TempDir::new().unwrap();
        for i in 0..10 {
            let sub = dir.path().join(format!("dir_{:02}", i));
            fs::create_dir(&sub).unwrap();
            for j in 0..3 {
                File::create(sub.join(format!("f{}.txt", j))).unwrap();
            }
        }
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "混合多");
        compact_diff(&native, &treepp, "混合多-/F");
    }

    #[test]
    fn should_match_native_output_for_deeply_nested_with_many_files() {
        let dir = TempDir::new().unwrap();
        let deep = dir.path().join("a/b/c/d");
        fs::create_dir_all(&deep).unwrap();
        for i in 0..5 {
            File::create(deep.join(format!("file{}.txt", i))).unwrap();
        }
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);
        assert_exit_codes(&native, &treepp, "深层多文件/F");
        compact_diff(&native, &treepp, "深层多文件-/F");
    }

    #[test]
    fn should_match_native_output_for_wide_and_deep_combined() {
        let dir = TempDir::new().unwrap();
        for i in 0..5 {
            let branch = dir.path().join(format!("branch{}", i));
            fs::create_dir_all(branch.join("sub/deep")).unwrap();
            File::create(branch.join("sub/deep/file.txt")).unwrap();
        }
        let native = run_native_tree(dir.path(), &["/F", "/A"]);
        let treepp = run_treepp(dir.path(), &["/F", "/A"]);
        assert_exit_codes(&native, &treepp, "宽深结合/F/A");
        compact_diff(&native, &treepp, "宽深结合-/F/A");
    }

    #[test]
    fn should_match_native_output_for_many_empty_nested_dirs() {
        let dir = TempDir::new().unwrap();
        for i in 0..8 {
            fs::create_dir_all(dir.path().join(format!("empty{}/nested/deep", i))).unwrap();
        }
        let native = run_native_tree(dir.path(), &[]);
        let treepp = run_treepp(dir.path(), &[]);
        assert_exit_codes(&native, &treepp, "多空嵌套");
        compact_diff(&native, &treepp, "多空嵌套");
    }

    // ========================================================================
    // Tests: Path Argument Variations
    // ========================================================================

    #[test]
    fn should_match_native_output_for_dot_path() {
        let dir = create_project_like();
        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", "."]);
        native_cmd.current_dir(dir.path());
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.arg(".");
        treepp_cmd.current_dir(dir.path());
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "点路径");
        compact_diff(&native, &treepp, "点路径");
    }

    #[test]
    fn should_match_native_output_for_dot_path_with_f_flag() {
        let dir = create_project_like();
        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", ".", "/F"]);
        native_cmd.current_dir(dir.path());
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.args([".", "/F"]);
        treepp_cmd.current_dir(dir.path());
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "点路径/F");
        compact_diff(&native, &treepp, "点路径-/F");
    }

    #[test]
    fn should_match_native_output_for_no_path_argument() {
        let dir = create_project_like();
        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree"]);
        native_cmd.current_dir(dir.path());
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.current_dir(dir.path());
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "无路径参数");
        compact_diff(&native, &treepp, "无路径参数");
    }

    #[test]
    fn should_match_native_output_for_no_path_with_f_flag() {
        let dir = create_project_like();
        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", "/F"]);
        native_cmd.current_dir(dir.path());
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.arg("/F");
        treepp_cmd.current_dir(dir.path());
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "无路径/F");
        compact_diff(&native, &treepp, "无路径-/F");
    }

    #[test]
    fn should_match_native_output_for_path_before_flags() {
        let dir = create_project_like();
        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree"]);
        native_cmd.arg(dir.path());
        native_cmd.arg("/F");
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.arg(dir.path());
        treepp_cmd.arg("/F");
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "路径在前/F");
        compact_diff(&native, &treepp, "路径在前-/F");
    }

    #[test]
    fn should_match_native_output_for_path_between_flags() {
        let dir = create_project_like();
        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", "/F"]);
        native_cmd.arg(dir.path());
        native_cmd.arg("/A");
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.arg("/F");
        treepp_cmd.arg(dir.path());
        treepp_cmd.arg("/A");
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "路径在中/F/A");
        compact_diff(&native, &treepp, "路径在中-/F/A");
    }

    #[test]
    fn should_match_native_output_for_absolute_path() {
        let dir = create_project_like();
        let native = run_native_tree(dir.path(), &["/F"]);
        let treepp = run_treepp(dir.path(), &["/F"]);

        assert_exit_codes(&native, &treepp, "绝对路径/F");
        compact_diff(&native, &treepp, "绝对路径-/F");
    }
    #[test]
    fn should_match_native_output_for_relative_subdirectory() {
        let dir = create_nested_with_files();
        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", "src", "/F"]);
        native_cmd.current_dir(dir.path());
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.args(["src", "/F"]);
        treepp_cmd.current_dir(dir.path());
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "相对子目录/F");
        compact_diff(&native, &treepp, "相对子目录-/F");
    }

    #[test]
    fn should_match_native_output_for_parent_directory() {
        let dir = create_nested_with_files();
        let subdir = dir.path().join("src");

        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", ".."]);
        native_cmd.current_dir(&subdir);
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.arg("..");
        treepp_cmd.current_dir(&subdir);
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "父目录");
        compact_diff(&native, &treepp, "父目录");
    }

    #[test]
    fn should_match_native_output_for_parent_directory_with_flags() {
        let dir = create_nested_with_files();
        let subdir = dir.path().join("src");

        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", "..", "/F", "/A"]);
        native_cmd.current_dir(&subdir);
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.args(["..", "/F", "/A"]);
        treepp_cmd.current_dir(&subdir);
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "父目录/F/A");
        compact_diff(&native, &treepp, "父目录-/F/A");
    }

    #[test]
    fn should_match_native_output_for_path_with_trailing_slash() {
        let dir = create_project_like();
        let path_with_slash = format!("{}\\", dir.path().display());

        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", &path_with_slash, "/F"]);
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.args([&path_with_slash, "/F"]);
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "尾斜杠路径/F");
        compact_diff(&native, &treepp, "尾斜杠路径-/F");
    }

    #[test]
    fn should_match_native_output_for_forward_slash_path() {
        let dir = create_project_like();
        let path_str = dir.path().to_string_lossy().replace('\\', "/");

        let mut native_cmd = Command::new("cmd");
        native_cmd.args(["/C", "tree", &path_str, "/F"]);
        let native_output = native_cmd.output().expect("Failed to execute native tree");
        let native = CommandOutput::from_native_output(&native_output);

        let treepp_path = get_treepp_path();
        let mut treepp_cmd = Command::new(&treepp_path);
        treepp_cmd.args([&path_str, "/F"]);
        let treepp_output = treepp_cmd.output().expect("Failed to execute treepp");
        let treepp = CommandOutput::from_treepp_output(&treepp_output);

        assert_exit_codes(&native, &treepp, "正斜杠路径/F");
        compact_diff(&native, &treepp, "正斜杠路径-/F");
    }
}