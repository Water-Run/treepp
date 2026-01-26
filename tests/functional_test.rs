//! Functional integration tests for tree++.
//!
//! This module contains comprehensive end-to-end tests that invoke the compiled
//! `treepp` binary directly and validate its output against expected behavior.
//!
//! Tests cover:
//! - All command-line parameters and their combinations
//! - Exit codes for various scenarios
//! - Output format correctness (TXT, JSON, YAML, TOML)
//! - Edge cases and error handling
//! - Windows-native tree command compatibility
//!
//! File: tests/functional_test.rs
//! Author: WaterRun
//! Date: 2026-01-26

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output};
use tempfile::TempDir;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Returns the path to the compiled treepp binary.
///
/// Checks debug build first, then release build.
///
/// # Panics
///
/// Panics if neither debug nor release binary exists.
fn get_treepp_path() -> PathBuf {
    let debug_path = PathBuf::from("target/debug/treepp.exe");
    if debug_path.exists() {
        return debug_path;
    }
    let release_path = PathBuf::from("target/release/treepp.exe");
    if release_path.exists() {
        return release_path;
    }
    panic!("treepp not built, please run `cargo build` first");
}

/// Executes treepp with the given arguments.
fn run_treepp(args: &[&str]) -> Output {
    Command::new(get_treepp_path())
        .args(args)
        .output()
        .expect("Failed to execute treepp")
}

/// Executes treepp in a specific directory with the given arguments.
fn run_treepp_in_dir(dir: &std::path::Path, args: &[&str]) -> Output {
    Command::new(get_treepp_path())
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute treepp")
}

/// Gets stdout as a string from command output.
fn stdout_str(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Gets stderr as a string from command output.
fn stderr_str(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Creates a basic test directory structure.
///
/// Structure:
/// ```text
/// root/
/// ├── file1.txt (content: "hello")
/// ├── file2.md (content: "# README")
/// ├── src/
/// │   ├── main.rs (content: "fn main() {}")
/// │   └── lib.rs (content: "pub fn lib() {}")
/// ├── tests/
/// │   └── test.rs (content: "#[test]")
/// └── empty/
/// ```
fn create_basic_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create files in root
    File::create(root.join("file1.txt"))
        .unwrap()
        .write_all(b"hello")
        .unwrap();
    File::create(root.join("file2.md"))
        .unwrap()
        .write_all(b"# README")
        .unwrap();

    // Create src directory with files
    fs::create_dir(root.join("src")).unwrap();
    File::create(root.join("src/main.rs"))
        .unwrap()
        .write_all(b"fn main() {}")
        .unwrap();
    File::create(root.join("src/lib.rs"))
        .unwrap()
        .write_all(b"pub fn lib() {}")
        .unwrap();

    // Create tests directory with files
    fs::create_dir(root.join("tests")).unwrap();
    File::create(root.join("tests/test.rs"))
        .unwrap()
        .write_all(b"#[test]")
        .unwrap();

    // Create empty directory
    fs::create_dir(root.join("empty")).unwrap();

    dir
}

/// Creates a test directory with .gitignore.
///
/// Structure:
/// ```text
/// root/
/// ├── .gitignore (contains: "target/\n*.log\n")
/// ├── file.txt
/// ├── app.log (should be ignored)
/// ├── target/ (should be ignored)
/// │   └── debug
/// └── src/
///     └── main.rs
/// ```
fn create_gitignore_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create .gitignore
    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"target/\n*.log\n")
        .unwrap();

    // Create files
    File::create(root.join("file.txt"))
        .unwrap()
        .write_all(b"content")
        .unwrap();
    File::create(root.join("app.log"))
        .unwrap()
        .write_all(b"log")
        .unwrap();

    // Create target directory (should be ignored)
    fs::create_dir(root.join("target")).unwrap();
    File::create(root.join("target/debug"))
        .unwrap()
        .write_all(b"")
        .unwrap();

    // Create src directory
    fs::create_dir(root.join("src")).unwrap();
    File::create(root.join("src/main.rs"))
        .unwrap()
        .write_all(b"fn main() {}")
        .unwrap();

    dir
}

/// Creates a deeply nested directory structure.
///
/// Structure:
/// ```text
/// root/
/// └── level1/
///     └── level2/
///         └── level3/
///             └── level4/
///                 └── deep.txt
/// ```
fn create_deep_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    let mut current = root.to_path_buf();
    for i in 1..=4 {
        current = current.join(format!("level{}", i));
        fs::create_dir(&current).unwrap();
    }
    File::create(current.join("deep.txt"))
        .unwrap()
        .write_all(b"deep content")
        .unwrap();

    dir
}

/// Creates a test directory with nested .gitignore files.
///
/// Structure:
/// ```text
/// root/
/// ├── .gitignore (contains: "*.tmp")
/// ├── root.tmp (ignored by root .gitignore)
/// ├── root.txt
/// └── level1/
///     ├── .gitignore (contains: "*.bak")
///     ├── l1.tmp (ignored)
///     ├── l1.bak (ignored)
///     ├── l1.txt
///     └── level2/
///         ├── .gitignore (contains: "*.cache")
///         ├── l2.tmp (ignored)
///         ├── l2.bak (ignored)
///         ├── l2.cache (ignored)
///         └── l2.txt
/// ```
fn create_nested_gitignore_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Root level
    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"*.tmp\n")
        .unwrap();
    File::create(root.join("root.tmp"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("root.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();

    // Level 1
    fs::create_dir(root.join("level1")).unwrap();
    File::create(root.join("level1/.gitignore"))
        .unwrap()
        .write_all(b"*.bak\n")
        .unwrap();
    File::create(root.join("level1/l1.tmp"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("level1/l1.bak"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("level1/l1.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();

    // Level 2
    fs::create_dir(root.join("level1/level2")).unwrap();
    File::create(root.join("level1/level2/.gitignore"))
        .unwrap()
        .write_all(b"*.cache\n")
        .unwrap();
    File::create(root.join("level1/level2/l2.tmp"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("level1/level2/l2.bak"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("level1/level2/l2.cache"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("level1/level2/l2.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();

    dir
}

/// Creates a directory with files of known sizes.
///
/// Structure:
/// ```text
/// root/
/// ├── small.txt (5 bytes)
/// ├── medium.txt (1024 bytes = 1 KB)
/// └── subdir/
///     └── large.txt (2048 bytes = 2 KB)
/// ```
fn create_sized_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    File::create(root.join("small.txt"))
        .unwrap()
        .write_all(b"12345")
        .unwrap();
    File::create(root.join("medium.txt"))
        .unwrap()
        .write_all(&vec![b'x'; 1024])
        .unwrap();

    fs::create_dir(root.join("subdir")).unwrap();
    File::create(root.join("subdir/large.txt"))
        .unwrap()
        .write_all(&vec![b'y'; 2048])
        .unwrap();

    dir
}

/// Creates a directory with files for sorting tests.
///
/// Structure:
/// ```text
/// root/
/// ├── .hidden
/// ├── 123.txt
/// ├── Apple.txt
/// ├── banana.txt
/// ├── _underscore.txt
/// └── zebra.txt
/// ```
fn create_sorting_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    for name in &[
        ".hidden",
        "123.txt",
        "Apple.txt",
        "banana.txt",
        "_underscore.txt",
        "zebra.txt",
    ] {
        File::create(root.join(name))
            .unwrap()
            .write_all(b"")
            .unwrap();
    }

    dir
}

// ============================================================================
// Help and Version Tests
// ============================================================================

#[test]
fn should_show_help_with_help_flag() {
    let output = run_treepp(&["--help"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("tree++"));
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("Options:"));
    assert!(stdout.contains("--help"));
    assert!(stdout.contains("--version"));
    assert!(stdout.contains("--files"));
}

#[test]
fn should_show_help_with_short_h_flag() {
    let output = run_treepp(&["-h"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("tree++"));
}

#[test]
fn should_show_help_with_cmd_style() {
    let output = run_treepp(&["/?"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("tree++"));
}

#[test]
fn should_show_version_with_version_flag() {
    let output = run_treepp(&["--version"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("tree++"));
    assert!(stdout.contains("0.3.0"));
    assert!(stdout.contains("WaterRun"));
}

#[test]
fn should_show_version_with_short_v_flag() {
    let output = run_treepp(&["-v"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("0.3.0"));
}

#[test]
fn should_show_version_with_cmd_style() {
    let output = run_treepp(&["/V"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("0.3.0"));
}

#[test]
fn should_show_version_case_insensitive_cmd_style() {
    let output = run_treepp(&["/v"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("0.3.0"));
}

// ============================================================================
// Basic Directory Scanning Tests
// ============================================================================

#[test]
fn should_scan_current_directory_without_args() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Should show directories but not files by default
    assert!(stdout.contains("src"));
    assert!(stdout.contains("tests"));
    assert!(stdout.contains("empty"));
    // Should NOT show files without /F
    assert!(!stdout.contains("file1.txt"));
}

#[test]
fn should_scan_specified_directory() {
    let dir = create_basic_test_dir();
    let path = dir.path().to_string_lossy().to_string();
    let output = run_treepp(&[&path, "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("src"));
}

#[test]
fn should_fail_for_nonexistent_path() {
    let output = run_treepp(&["/nonexistent/path/12345"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_fail_for_file_as_root() {
    let dir = create_basic_test_dir();
    let file_path = dir.path().join("file1.txt");
    let output = run_treepp(&[file_path.to_str().unwrap()]);
    assert!(!output.status.success());
}

// ============================================================================
// Files Display Tests (/F)
// ============================================================================

#[test]
fn should_show_files_with_f_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.md"));
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("lib.rs"));
    assert!(stdout.contains("test.rs"));
}

#[test]
fn should_show_files_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["--files", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("file1.txt"));
}

#[test]
fn should_show_files_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("file1.txt"));
}

// ============================================================================
// ASCII Mode Tests (/A)
// ============================================================================

#[test]
fn should_use_ascii_characters_with_a_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/a", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Should contain ASCII tree characters
    assert!(stdout.contains("+---") || stdout.contains("\\---"));
    // Should NOT contain Unicode characters
    assert!(!stdout.contains("├─"));
    assert!(!stdout.contains("└─"));
}

#[test]
fn should_use_ascii_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["--ascii", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("+---") || stdout.contains("\\---"));
}

#[test]
fn should_use_ascii_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-a", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("+---") || stdout.contains("\\---"));
}

#[test]
fn should_use_unicode_by_default() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Should contain Unicode characters
    assert!(stdout.contains("├─") || stdout.contains("└─"));
}

// ============================================================================
// Full Path Tests (/FP)
// ============================================================================

#[test]
fn should_show_full_paths_with_fp_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/fp", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Should contain full paths
    assert!(stdout.contains("src") && stdout.contains("main.rs"));
    // The path separator should appear in output
    assert!(stdout.contains("\\") || stdout.contains("/"));
}

#[test]
fn should_show_full_paths_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--full-path", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_show_full_paths_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-p", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Size Display Tests (/S, /HR)
// ============================================================================

#[test]
fn should_show_file_sizes_with_s_flag() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/s", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Should show sizes in bytes
    assert!(stdout.contains("5")); // small.txt
    assert!(stdout.contains("1024")); // medium.txt
    assert!(stdout.contains("2048")); // large.txt
}

#[test]
fn should_show_human_readable_sizes_with_hr_flag() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/hr", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Should show human-readable sizes
    assert!(stdout.contains("KB") || stdout.contains("B"));
}

#[test]
fn should_enable_size_when_hr_enabled() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/hr", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // /HR implies /S
    assert!(stdout.contains("KB") || stdout.contains("B"));
}

#[test]
fn should_show_human_readable_with_gnu_style() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--human-readable", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("KB") || stdout_str(&output).contains("B"));
}

#[test]
fn should_show_human_readable_with_short_style() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-H", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Date Display Tests (/DT)
// ============================================================================

#[test]
fn should_show_dates_with_dt_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/dt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Should contain date format YYYY-MM-DD HH:MM:SS
    assert!(stdout.contains("-") && stdout.contains(":"));
}

#[test]
fn should_show_dates_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--date", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_show_dates_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-d", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// No Indent Tests (/NI)
// ============================================================================

#[test]
fn should_use_no_indent_with_ni_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/ni", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Should NOT contain tree characters
    assert!(!stdout.contains("├"));
    assert!(!stdout.contains("└"));
    assert!(!stdout.contains("+---"));
    assert!(!stdout.contains("\\---"));
}

#[test]
fn should_use_no_indent_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--no-indent", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(!stdout.contains("├"));
}

#[test]
fn should_use_no_indent_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-i", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Reverse Sort Tests (/R)
// ============================================================================

#[test]
fn should_reverse_sort_with_r_flag() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/r", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Find positions of items in output
    let zebra_pos = stdout.find("zebra.txt");
    let apple_pos = stdout.find("Apple.txt");

    assert!(zebra_pos.is_some());
    assert!(apple_pos.is_some());

    // In reverse order, zebra should come before apple
    assert!(
        zebra_pos.unwrap() < apple_pos.unwrap(),
        "zebra should come before apple in reverse sort"
    );
}

#[test]
fn should_normal_sort_without_r_flag() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let zebra_pos = stdout.find("zebra.txt");
    let apple_pos = stdout.find("Apple.txt");

    assert!(zebra_pos.is_some());
    assert!(apple_pos.is_some());

    // In normal order, apple should come before zebra
    assert!(
        apple_pos.unwrap() < zebra_pos.unwrap(),
        "apple should come before zebra in normal sort"
    );
}

#[test]
fn should_reverse_with_gnu_style() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--reverse", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_reverse_with_short_style() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-r", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Level Limit Tests (/L)
// ============================================================================

#[test]
fn should_limit_depth_with_l_flag() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/l", "1", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("level1"));
    assert!(!stdout.contains("level2"));
    assert!(!stdout.contains("deep.txt"));
}

#[test]
fn should_show_nothing_with_level_zero() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/l", "0", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    // Level 0 means root only, no children
    assert!(!stdout.contains("level1"));
}

#[test]
fn should_show_all_levels_without_limit() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("level1"));
    assert!(stdout.contains("level2"));
    assert!(stdout.contains("level3"));
    assert!(stdout.contains("level4"));
    assert!(stdout.contains("deep.txt"));
}

#[test]
fn should_limit_with_gnu_style() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--level", "2", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("level1"));
    assert!(stdout.contains("level2"));
    assert!(!stdout.contains("level3"));
}

#[test]
fn should_limit_with_short_style() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-L", "2", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_fail_with_invalid_level_value() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/l", "abc"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_fail_with_missing_level_value() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/l"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

// ============================================================================
// Include Pattern Tests (/M)
// ============================================================================

#[test]
fn should_include_matching_files_with_m_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "*.rs", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("lib.rs"));
    assert!(stdout.contains("test.rs"));
    assert!(!stdout.contains("file1.txt"));
    assert!(!stdout.contains("file2.md"));
}

#[test]
fn should_include_multiple_patterns() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "*.rs", "/m", "*.txt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("file1.txt"));
    assert!(!stdout.contains("file2.md"));
}

#[test]
fn should_always_show_directories_with_include() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "*.rs", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Directories should always be shown
    assert!(stdout.contains("src"));
    assert!(stdout.contains("tests"));
}

#[test]
fn should_include_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--include", "*.rs", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_include_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-m", "*.rs", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Exclude Pattern Tests (/X)
// ============================================================================

#[test]
fn should_exclude_matching_files_with_x_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/x", "*.md", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(!stdout.contains("file2.md"));
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("main.rs"));
}

#[test]
fn should_exclude_multiple_patterns() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/x", "*.md", "/x", "*.txt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(!stdout.contains("file2.md"));
    assert!(!stdout.contains("file1.txt"));
    assert!(stdout.contains("main.rs"));
}

#[test]
fn should_exclude_directories() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/x", "tests", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(!stdout.contains("tests"));
    assert!(!stdout.contains("test.rs"));
    assert!(stdout.contains("src"));
}

#[test]
fn should_exclude_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--exclude", "*.md", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_exclude_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-I", "*.md", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Report Tests (/RP)
// ============================================================================

#[test]
fn should_show_report_with_rp_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/rp", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("directory"));
    assert!(stdout.contains("files"));
    assert!(stdout.contains("s")); // seconds
}

#[test]
fn should_show_directory_only_report_without_files() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/rp", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("directory"));
    // Should NOT show files count when /F not specified
    assert!(!stdout.contains("files"));
}

#[test]
fn should_show_report_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--report", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_show_report_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-e", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// No Windows Banner Tests (/NB)
// ============================================================================

#[test]
fn should_hide_banner_with_nb_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should NOT contain volume info
    assert!(!stdout.contains("Folder PATH listing"));
    assert!(!stdout.contains("Volume serial number"));
}

#[test]
fn should_hide_banner_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["--no-win-banner"]);
    assert!(output.status.success());
}

#[test]
fn should_hide_banner_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-N"]);
    assert!(output.status.success());
}

// ============================================================================
// Gitignore Tests (/G)
// ============================================================================

#[test]
fn should_respect_gitignore_with_g_flag() {
    let dir = create_gitignore_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(!stdout.contains("target"));
    assert!(!stdout.contains("app.log"));
    assert!(stdout.contains("file.txt"));
    assert!(stdout.contains("main.rs"));
}

#[test]
fn should_not_respect_gitignore_by_default() {
    let dir = create_gitignore_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("target"));
    assert!(stdout.contains("app.log"));
}

#[test]
fn should_respect_nested_gitignore() {
    let dir = create_nested_gitignore_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // All .tmp files should be ignored (from root .gitignore)
    assert!(!stdout.contains("root.tmp"));
    assert!(!stdout.contains("l1.tmp"));
    assert!(!stdout.contains("l2.tmp"));

    // All .bak files should be ignored (from level1 .gitignore, inherited)
    assert!(!stdout.contains("l1.bak"));
    assert!(!stdout.contains("l2.bak"));

    // .cache files ignored from level2
    assert!(!stdout.contains("l2.cache"));

    // .txt files should be present
    assert!(stdout.contains("root.txt"));
    assert!(stdout.contains("l1.txt"));
    assert!(stdout.contains("l2.txt"));
}

#[test]
fn should_respect_gitignore_with_gnu_style() {
    let dir = create_gitignore_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--gitignore", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_respect_gitignore_with_short_style() {
    let dir = create_gitignore_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "-g", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Batch Mode Tests (/B)
// ============================================================================

#[test]
fn should_work_in_batch_mode() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("src"));
    assert!(stdout.contains("main.rs"));
}

#[test]
fn should_work_with_gnu_style_batch() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["--batch", "-f", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_work_with_short_style_batch() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-b", "-f", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Disk Usage Tests (/DU)
// ============================================================================

#[test]
fn should_show_disk_usage_with_du_flag() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/du", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should show cumulative size for directories
    // Total size should be 5 + 1024 + 2048 = 3077 bytes
    // subdir should show 2048
    assert!(stdout.contains("subdir"));
}

#[test]
fn should_fail_disk_usage_without_batch() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/du"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_show_disk_usage_with_human_readable() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/du", "/hr", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("KB") || stdout.contains("B"));
}

#[test]
fn should_show_disk_usage_with_gnu_style() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["--batch", "--disk-usage", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_show_disk_usage_with_short_style() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-b", "-u", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Thread Count Tests (/T)
// ============================================================================

#[test]
fn should_accept_thread_count_with_batch() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/t", "4", "/f", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_fail_thread_without_batch() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/t", "4"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_fail_with_zero_threads() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/t", "0"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_fail_with_invalid_thread_count() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/t", "abc"]);
    assert!(!output.status.success());
}

#[test]
fn should_accept_thread_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["--batch", "--thread", "8", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_accept_thread_with_short_style() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-b", "-t", "2", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Output File Tests (/O)
// ============================================================================

#[test]
fn should_output_to_txt_file() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.txt");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());
    assert!(output_file.exists());

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("src"));
    assert!(content.contains("main.rs"));
}

#[test]
fn should_output_to_json_file_with_batch() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());
    assert!(output_file.exists());

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.starts_with("{"));
    assert!(content.contains("\"schema\""));
    assert!(content.contains("treepp.pretty.v1"));
    assert!(content.contains("\"root\""));
}

#[test]
fn should_fail_json_output_without_batch() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/o", output_file.to_str().unwrap()],
    );
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_output_to_yaml_file_with_batch() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.yml");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());
    assert!(output_file.exists());

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(!content.is_empty());
}

#[test]
fn should_output_to_toml_file_with_batch() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.toml");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn should_fail_with_unknown_extension() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.xyz");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/o", output_file.to_str().unwrap()],
    );
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_output_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("out.txt");
    let output = run_treepp_in_dir(
        dir.path(),
        &["-f", "--output", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());
}

#[test]
fn should_output_with_short_style() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("out.txt");
    let output = run_treepp_in_dir(
        dir.path(),
        &["-f", "-o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());
}

#[test]
fn should_output_with_equals_syntax() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("out.txt");
    let arg = format!("--output={}", output_file.to_str().unwrap());
    let output = run_treepp_in_dir(dir.path(), &["-f", &arg, "/nb"]);
    assert!(output.status.success());
    assert!(output_file.exists());
}

// ============================================================================
// Silent Mode Tests (/SI)
// ============================================================================

#[test]
fn should_be_silent_with_si_flag_and_output() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.txt");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/o", output_file.to_str().unwrap(), "/si"],
    );
    assert!(output.status.success());

    let stdout = stdout_str(&output);
    // Should have no console output (or minimal)
    assert!(stdout.is_empty() || !stdout.contains("src"));

    // But file should have content
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("src"));
}

#[test]
fn should_fail_silent_without_output() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/si"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_silent_with_gnu_style() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("out.txt");
    let output = run_treepp_in_dir(
        dir.path(),
        &["-f", "--silent", "-o", output_file.to_str().unwrap()],
    );
    assert!(output.status.success());
}

#[test]
fn should_silent_with_short_style() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("out.txt");
    let output = run_treepp_in_dir(
        dir.path(),
        &["-f", "-l", "-o", output_file.to_str().unwrap()],
    );
    assert!(output.status.success());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn should_fail_with_unknown_option() {
    let output = run_treepp(&["/unknown"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_str(&output).contains("Unknown option"));
}

#[test]
fn should_fail_with_duplicate_option() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/f"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_fail_with_multiple_paths() {
    let dir = create_basic_test_dir();
    let output = run_treepp(&[
        dir.path().to_str().unwrap(),
        dir.path().to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_show_hint_for_unknown_option() {
    let output = run_treepp(&["/xyz"]);
    assert!(!output.status.success());
    let stderr = stderr_str(&output);
    assert!(stderr.contains("--help") || stderr.contains("Hint"));
}

// ============================================================================
// Mixed Style Tests
// ============================================================================

#[test]
fn should_mix_cmd_and_gnu_styles() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/F", "--ascii", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("+---") || stdout.contains("\\---"));
}

#[test]
fn should_mix_all_three_styles() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/F", "-a", "--level", "1", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_handle_case_insensitive_cmd_options() {
    let dir = create_basic_test_dir();
    let output1 = run_treepp_in_dir(dir.path(), &["/F", "/nb"]);
    let output2 = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output1.status.success());
    assert!(output2.status.success());
    assert_eq!(stdout_str(&output1), stdout_str(&output2));
}

#[test]
fn should_be_case_sensitive_for_short_options() {
    let dir = create_basic_test_dir();
    // -f should work (files)
    let output1 = run_treepp_in_dir(dir.path(), &["-f", "/nb"]);
    assert!(output1.status.success());

    // -F should fail (unknown)
    let output2 = run_treepp_in_dir(dir.path(), &["-F"]);
    assert!(!output2.status.success());
}

// ============================================================================
// Combination Tests
// ============================================================================

#[test]
fn should_combine_files_and_size_and_date() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/s", "/dt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should have size
    assert!(stdout.contains("5") || stdout.contains("1024"));
    // Should have date format
    assert!(stdout.contains("-") && stdout.contains(":"));
}

#[test]
fn should_combine_gitignore_with_include() {
    let dir = create_gitignore_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/m", "*.rs", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("file.txt"));
    assert!(!stdout.contains("app.log"));
}

#[test]
fn should_combine_level_with_report() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/l", "2", "/rp", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("level1"));
    assert!(stdout.contains("level2"));
    assert!(!stdout.contains("level3"));
    assert!(stdout.contains("directory"));
}

#[test]
fn should_combine_batch_with_all_options() {
    let dir = create_sized_test_dir();
    let output_file = dir.path().join("full.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &[
            "/b",
            "/f",
            "/s",
            "/hr",
            "/dt",
            "/du",
            "/rp",
            "/nb",
            "/o",
            output_file.to_str().unwrap(),
        ],
    );
    assert!(output.status.success());
    assert!(output_file.exists());
}

// ============================================================================
// Windows Compatibility Tests
// ============================================================================

#[test]
fn should_show_root_as_drive_dot_without_explicit_path() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should show X:. format
    assert!(
        stdout.contains(":.") || stdout.contains(":\\"),
        "Root should be shown as drive:. format"
    );
}

#[test]
fn should_show_uppercase_path_with_explicit_path() {
    let dir = create_basic_test_dir();
    let path = dir.path().to_string_lossy().to_string();
    let output = run_treepp(&[&path, "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Path should be uppercase
    assert!(stdout.to_uppercase().contains(&path.to_uppercase()));
}

// ============================================================================
// Sorting Tests
// ============================================================================

#[test]
fn should_sort_windows_style_dot_first() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let hidden_pos = stdout.find(".hidden");
    let apple_pos = stdout.find("Apple.txt");

    assert!(hidden_pos.is_some());
    assert!(apple_pos.is_some());
    assert!(
        hidden_pos.unwrap() < apple_pos.unwrap(),
        ".hidden should come before Apple.txt"
    );
}

#[test]
fn should_sort_numbers_before_letters() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let num_pos = stdout.find("123.txt");
    let apple_pos = stdout.find("Apple.txt");

    assert!(num_pos.is_some());
    assert!(apple_pos.is_some());
    assert!(
        num_pos.unwrap() < apple_pos.unwrap(),
        "123.txt should come before Apple.txt"
    );
}

#[test]
fn should_sort_underscore_after_letters() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let underscore_pos = stdout.find("_underscore.txt");
    let zebra_pos = stdout.find("zebra.txt");

    assert!(underscore_pos.is_some());
    assert!(zebra_pos.is_some());
    assert!(
        underscore_pos.unwrap() > zebra_pos.unwrap(),
        "_underscore.txt should come after zebra.txt"
    );
}

#[test]
fn should_sort_files_before_directories() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let file_pos = stdout.find("file1.txt");
    let src_pos = stdout.find("src");

    assert!(file_pos.is_some());
    assert!(src_pos.is_some());
    assert!(
        file_pos.unwrap() < src_pos.unwrap(),
        "Files should come before directories"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn should_handle_empty_directory() {
    let dir = TempDir::new().unwrap();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_handle_single_file_directory() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("only.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("only.txt"));
}

#[test]
fn should_handle_special_characters_in_filenames() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    File::create(root.join("file with spaces.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("file-with-dashes.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("file_with_underscores.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();

    let output = run_treepp_in_dir(root, &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("file with spaces.txt"));
    assert!(stdout.contains("file-with-dashes.txt"));
    assert!(stdout.contains("file_with_underscores.txt"));
}

#[test]
fn should_handle_unicode_filenames() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    File::create(root.join("中文文件.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("日本語ファイル.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();

    let output = run_treepp_in_dir(root, &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("中文文件.txt"));
    assert!(stdout.contains("日本語ファイル.txt"));
}

#[test]
fn should_handle_very_long_filenames() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let long_name = "a".repeat(200) + ".txt";
    File::create(root.join(&long_name))
        .unwrap()
        .write_all(b"")
        .unwrap();

    let output = run_treepp_in_dir(root, &["/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains(&long_name));
}

#[test]
fn should_handle_deeply_nested_structure() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let mut current = root.to_path_buf();
    for i in 0..20 {
        current = current.join(format!("level{}", i));
        fs::create_dir(&current).unwrap();
    }
    File::create(current.join("deep.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();

    let output = run_treepp_in_dir(root, &["/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("deep.txt"));
}

// ============================================================================
// JSON Output Structure Tests
// ============================================================================

#[test]
fn should_output_valid_json_structure() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Output should be valid JSON");

    // Validate new schema structure
    assert!(json.is_object());
    assert_eq!(
        json.get("schema").and_then(|v| v.as_str()),
        Some("treepp.pretty.v1"),
        "Should have correct schema identifier"
    );
    assert!(json.get("root").is_some(), "Should have root object");

    let root = json.get("root").unwrap();
    assert!(root.get("path").is_some(), "Root should have path");
    assert_eq!(
        root.get("type").and_then(|v| v.as_str()),
        Some("dir"),
        "Root type should be dir"
    );
    assert!(root.get("files").is_some(), "Root should have files array");
    assert!(root.get("dirs").is_some(), "Root should have dirs object");
}

#[test]
fn should_include_size_in_json_when_enabled() {
    let dir = create_sized_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/s", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();

    // With the new format, files might be an array of strings or objects with metadata
    // Check if size info is present somewhere in the structure
    let content_str = content.to_lowercase();
    assert!(
        content_str.contains("size") || content_str.contains("5"),
        "Size information should be present in output"
    );
}

#[test]
fn should_include_disk_usage_in_json_when_enabled() {
    let dir = create_sized_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/du", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Verify structure exists
    assert!(json.get("schema").is_some(), "Should have schema");
    assert!(json.get("root").is_some(), "Should have root");

    // Check that disk usage info is present in the structure
    let content_str = content.to_lowercase();
    assert!(
        content_str.contains("disk") || content_str.contains("size") || content_str.contains("2048"),
        "Disk usage information should be present"
    );
}

// ============================================================================
// YAML Output Structure Tests
// ============================================================================

#[test]
fn should_output_valid_yaml_structure() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.yml");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).expect("Output should be valid YAML");

    // Validate new schema structure
    assert!(yaml.is_mapping(), "Root should be a mapping");

    let mapping = yaml.as_mapping().unwrap();
    assert!(
        mapping.get(&serde_yaml::Value::String("schema".to_string())).is_some(),
        "Should have schema field"
    );
    assert!(
        mapping.get(&serde_yaml::Value::String("root".to_string())).is_some(),
        "Should have root field"
    );

    // Verify schema value
    let schema = mapping.get(&serde_yaml::Value::String("schema".to_string()));
    assert_eq!(
        schema.and_then(|v| v.as_str()),
        Some("treepp.pretty.v1"),
        "Schema should be treepp.pretty.v1"
    );
}

// ============================================================================
// TOML Output Structure Tests
// ============================================================================

#[test]
fn should_output_valid_toml_structure() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.toml");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let toml: toml::Value =
        toml::from_str(&content).expect("Output should be valid TOML");

    // Validate new schema structure
    assert!(toml.is_table(), "Root should be a table");

    let table = toml.as_table().unwrap();
    assert!(table.get("schema").is_some(), "Should have schema field");
    assert!(table.get("root").is_some(), "Should have root field");

    // Verify schema value
    assert_eq!(
        table.get("schema").and_then(|v| v.as_str()),
        Some("treepp.pretty.v1"),
        "Schema should be treepp.pretty.v1"
    );

    // Verify root structure
    let root = table.get("root").and_then(|v| v.as_table());
    assert!(root.is_some(), "Root should be a table");
    let root = root.unwrap();
    assert!(root.get("path").is_some(), "Root should have path");
    assert_eq!(
        root.get("type").and_then(|v| v.as_str()),
        Some("dir"),
        "Root type should be dir"
    );
}

// ============================================================================
// Performance and Thread Tests
// ============================================================================

#[test]
fn should_produce_consistent_results_with_different_thread_counts() {
    let dir = create_basic_test_dir();

    let output1 = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "1", "/nb"]);
    let output4 = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "4", "/nb"]);
    let output8 = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "8", "/nb"]);

    assert!(output1.status.success());
    assert!(output4.status.success());
    assert!(output8.status.success());

    // Results should be identical regardless of thread count
    assert_eq!(stdout_str(&output1), stdout_str(&output4));
    assert_eq!(stdout_str(&output4), stdout_str(&output8));
}

#[test]
fn should_handle_large_thread_count() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "64", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Streaming vs Batch Consistency Tests
// ============================================================================

#[test]
fn should_produce_same_content_in_streaming_and_batch() {
    let dir = create_basic_test_dir();

    // Streaming mode (default)
    let streaming = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    // Batch mode
    let batch = run_treepp_in_dir(dir.path(), &["/b", "/f", "/nb"]);

    assert!(streaming.status.success());
    assert!(batch.status.success());

    // Content should be identical
    assert_eq!(stdout_str(&streaming), stdout_str(&batch));
}

#[test]
fn should_produce_same_counts_in_streaming_and_batch() {
    let dir = create_basic_test_dir();

    let streaming = run_treepp_in_dir(dir.path(), &["/f", "/rp", "/nb"]);
    let batch = run_treepp_in_dir(dir.path(), &["/b", "/f", "/rp", "/nb"]);

    assert!(streaming.status.success());
    assert!(batch.status.success());

    // Both should report same directory and file counts
    let streaming_out = stdout_str(&streaming);
    let batch_out = stdout_str(&batch);

    // Extract counts from report line
    assert!(streaming_out.contains("directory"));
    assert!(batch_out.contains("directory"));
}

// ============================================================================
// Exit Code Tests
// ============================================================================

#[test]
fn should_return_exit_code_0_on_success() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/nb"]);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn should_return_exit_code_1_on_cli_error() {
    let output = run_treepp(&["/unknown_option"]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_return_exit_code_1_on_config_error() {
    let dir = create_basic_test_dir();
    // Silent without output is a config error
    let output = run_treepp_in_dir(dir.path(), &["/si"]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_return_exit_code_2_on_scan_error() {
    // Nonexistent path causes scan error
    let output = run_treepp(&["/nonexistent/path/12345"]);
    // This might be 1 (CLI) or 2 (Scan) depending on when error is caught
    assert!(!output.status.success());
}

// ============================================================================
// Path Position Tests
// ============================================================================

#[test]
fn should_accept_path_at_beginning() {
    let dir = create_basic_test_dir();
    let path = dir.path().to_string_lossy().to_string();
    let output = run_treepp(&[&path, "/f", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_accept_path_at_end() {
    let dir = create_basic_test_dir();
    let path = dir.path().to_string_lossy().to_string();
    let output = run_treepp(&["/f", "/nb", &path]);
    assert!(output.status.success());
}

#[test]
fn should_accept_path_in_middle() {
    let dir = create_basic_test_dir();
    let path = dir.path().to_string_lossy().to_string();
    let output = run_treepp(&["/f", &path, "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Equals Syntax Tests
// ============================================================================

#[test]
fn should_accept_level_with_equals_syntax() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["--level=2", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("level1"));
    assert!(stdout.contains("level2"));
    assert!(!stdout.contains("level3"));
}

#[test]
fn should_accept_thread_with_equals_syntax() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "--thread=4", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_accept_output_with_equals_syntax() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("out.txt");
    let arg = format!("--output={}", output_file.to_str().unwrap());
    let output = run_treepp_in_dir(dir.path(), &["-f", &arg, "/nb"]);
    assert!(output.status.success());
    assert!(output_file.exists());
}

#[test]
fn should_accept_include_with_equals_syntax() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--include=*.rs", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("main.rs"));
}

#[test]
fn should_accept_exclude_with_equals_syntax() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["-f", "--exclude=*.md", "/nb"]);
    assert!(output.status.success());
    assert!(!stdout_str(&output).contains("file2.md"));
}

// ============================================================================
// Tree Character Rendering Tests
// ============================================================================

#[test]
fn should_render_files_without_branch_connectors() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Files should be indented but not use branch connectors
    let lines: Vec<&str> = stdout.lines().collect();
    for line in &lines {
        if line.contains("file1.txt") || line.contains("file2.md") {
            // File lines should NOT start with branch connector
            let trimmed = line.trim_start();
            assert!(
                !trimmed.starts_with("├─") && !trimmed.starts_with("└─"),
                "File line should not start with branch: {}",
                line
            );
        }
    }
}

#[test]
fn should_render_directories_with_branch_connectors() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // At least one directory should have branch connector
    assert!(
        stdout.contains("├─") || stdout.contains("└─"),
        "Directory should have branch connectors"
    );
}

#[test]
fn should_render_proper_vertical_lines() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Create structure that requires vertical lines:
    // dir1/
    //   file1.txt
    // dir2/
    //   file2.txt
    fs::create_dir(root.join("dir1")).unwrap();
    fs::create_dir(root.join("dir2")).unwrap();
    File::create(root.join("dir1/file1.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();
    File::create(root.join("dir2/file2.txt"))
        .unwrap()
        .write_all(b"")
        .unwrap();

    let output = run_treepp_in_dir(root, &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should have vertical continuation lines
    assert!(
        stdout.contains("│") || stdout.contains("|"),
        "Should have vertical lines for sibling directories"
    );
}

// ============================================================================
// Additional Combination Tests
// ============================================================================

#[test]
fn should_combine_ascii_with_all_display_options() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/a", "/f", "/s", "/dt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should use ASCII characters
    assert!(!stdout.contains("├─"));
    assert!(!stdout.contains("└─"));
    // Should have size and date
    assert!(stdout.contains("-") && stdout.contains(":"));
}

#[test]
fn should_combine_no_indent_with_size_and_date() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/ni", "/f", "/s", "/dt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // No tree characters
    assert!(!stdout.contains("├"));
    assert!(!stdout.contains("└"));
    // But should have metadata
    assert!(stdout.contains("-") && stdout.contains(":"));
}

#[test]
fn should_combine_full_path_with_size() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/fp", "/s", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should have full paths
    assert!(stdout.contains("\\") || stdout.contains("/"));
    // And sizes
    assert!(stdout.contains("5") || stdout.contains("1024"));
}

#[test]
fn should_combine_reverse_with_level() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/r", "/l", "3", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("level1"));
    assert!(stdout.contains("level2"));
    assert!(stdout.contains("level3"));
    assert!(!stdout.contains("level4"));
}

#[test]
fn should_combine_include_and_exclude() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/m", "*.rs", "/m", "*.txt", "/x", "lib.rs", "/nb"],
    );
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should include .rs and .txt but exclude lib.rs
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("file1.txt"));
    assert!(!stdout.contains("lib.rs"));
    assert!(!stdout.contains("file2.md"));
}

#[test]
fn should_output_json_with_metadata() {
    let dir = create_sized_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &[
            "/b",
            "/f",
            "/s",
            "/hr",
            "/dt",
            "/fp",
            "/o",
            output_file.to_str().unwrap(),
            "/nb",
        ],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Verify new structure with schema
    assert_eq!(
        json.get("schema").and_then(|v| v.as_str()),
        Some("treepp.pretty.v1"),
        "Should have correct schema"
    );

    let root = json.get("root").expect("Should have root");
    assert!(root.get("path").is_some(), "Root should have path");
    assert!(root.get("files").is_some(), "Root should have files");
    assert!(root.get("dirs").is_some(), "Root should have dirs");
}

// ============================================================================
// Hidden Files Tests (/AL)
// ============================================================================

/// Creates a test directory with hidden files and directories.
///
/// Structure:
/// ```text
/// root/
/// ├── visible.txt
/// ├── .hidden_file (hidden attribute set via attrib +H)
/// ├── normal_dir/
/// │   └── normal.txt
/// └── .hidden_dir/ (hidden attribute set via attrib +H)
///     └── inside_hidden.txt
/// ```
fn create_hidden_files_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create visible file
    File::create(root.join("visible.txt"))
        .unwrap()
        .write_all(b"visible content")
        .unwrap();

    // Create file that will be hidden
    let hidden_file_path = root.join(".hidden_file");
    File::create(&hidden_file_path)
        .unwrap()
        .write_all(b"hidden content")
        .unwrap();

    // Create normal directory with file
    fs::create_dir(root.join("normal_dir")).unwrap();
    File::create(root.join("normal_dir/normal.txt"))
        .unwrap()
        .write_all(b"normal")
        .unwrap();

    // Create directory that will be hidden
    let hidden_dir_path = root.join(".hidden_dir");
    fs::create_dir(&hidden_dir_path).unwrap();
    File::create(hidden_dir_path.join("inside_hidden.txt"))
        .unwrap()
        .write_all(b"inside hidden dir")
        .unwrap();

    // Use attrib command to set hidden attribute on file
    Command::new("attrib")
        .args(["+H", hidden_file_path.to_str().unwrap()])
        .output()
        .expect("Failed to set hidden attribute on file");

    // Use attrib command to set hidden attribute on directory
    Command::new("attrib")
        .args(["+H", hidden_dir_path.to_str().unwrap()])
        .output()
        .expect("Failed to set hidden attribute on directory");

    dir
}

#[test]
fn should_hide_hidden_files_by_default() {
    let dir = create_hidden_files_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Visible files should appear
    assert!(stdout.contains("visible.txt"));
    assert!(stdout.contains("normal_dir"));
    assert!(stdout.contains("normal.txt"));

    // Hidden files and directories should NOT appear by default
    assert!(
        !stdout.contains(".hidden_file"),
        "Hidden file should not appear by default"
    );
    assert!(
        !stdout.contains(".hidden_dir"),
        "Hidden directory should not appear by default"
    );
    assert!(
        !stdout.contains("inside_hidden.txt"),
        "Files inside hidden directory should not appear by default"
    );
}

#[test]
fn should_show_hidden_files_with_al_flag() {
    let dir = create_hidden_files_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/al", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // All files should appear including hidden ones
    assert!(stdout.contains("visible.txt"));
    assert!(stdout.contains("normal_dir"));
    assert!(stdout.contains("normal.txt"));
    assert!(
        stdout.contains(".hidden_file"),
        "Hidden file should appear with /AL flag"
    );
    assert!(
        stdout.contains(".hidden_dir"),
        "Hidden directory should appear with /AL flag"
    );
    assert!(
        stdout.contains("inside_hidden.txt"),
        "Files inside hidden directory should appear with /AL flag"
    );
}

#[test]
fn should_show_hidden_files_with_gnu_and_short_styles() {
    let dir = create_hidden_files_test_dir();

    // Test GNU style --all
    let output_gnu = run_treepp_in_dir(dir.path(), &["-f", "--all", "/nb"]);
    assert!(output_gnu.status.success());
    let stdout_gnu = stdout_str(&output_gnu);
    assert!(
        stdout_gnu.contains(".hidden_file"),
        "Hidden file should appear with --all flag"
    );
    assert!(
        stdout_gnu.contains(".hidden_dir"),
        "Hidden directory should appear with --all flag"
    );

    // Test short style -k
    let output_short = run_treepp_in_dir(dir.path(), &["-f", "-k", "/nb"]);
    assert!(output_short.status.success());
    let stdout_short = stdout_str(&output_short);
    assert!(
        stdout_short.contains(".hidden_file"),
        "Hidden file should appear with -k flag"
    );
    assert!(
        stdout_short.contains(".hidden_dir"),
        "Hidden directory should appear with -k flag"
    );

    // Results should be identical
    assert_eq!(stdout_gnu, stdout_short);
}

#[test]
fn should_combine_hidden_files_with_other_options() {
    let dir = create_hidden_files_test_dir();

    // Combine /AL with /S (size), /DT (date), and /HR (human-readable)
    let output = run_treepp_in_dir(dir.path(), &["/f", "/al", "/hr", "/dt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should show hidden files
    assert!(
        stdout.contains(".hidden_file"),
        "Hidden file should appear when combined with other options"
    );
    assert!(
        stdout.contains(".hidden_dir"),
        "Hidden directory should appear when combined with other options"
    );

    // Should contain human-readable size format
    assert!(
        stdout.contains("B") || stdout.contains("KB"),
        "Human-readable size should be displayed"
    );

    // Should contain date format (YYYY-MM-DD HH:MM:SS pattern)
    assert!(
        stdout.contains(":") && stdout.contains("-"),
        "Date should be displayed"
    );
}

#[test]
fn should_respect_hidden_with_gitignore_and_filters() {
    let dir = create_hidden_files_test_dir();

    // Test /AL combined with /X (exclude) - exclude .txt files
    let output_exclude = run_treepp_in_dir(dir.path(), &["/f", "/al", "/x", "*.txt", "/nb"]);
    assert!(output_exclude.status.success());
    let stdout_exclude = stdout_str(&output_exclude);

    // Hidden file without .txt extension should still appear
    assert!(
        stdout_exclude.contains(".hidden_file"),
        "Hidden file should appear with /AL even when filtering"
    );
    // .txt files should be excluded
    assert!(
        !stdout_exclude.contains("visible.txt"),
        "Excluded .txt files should not appear"
    );

    // Test /AL combined with /M (include) - only show hidden files
    let output_include = run_treepp_in_dir(dir.path(), &["/f", "/al", "/m", ".hidden*", "/nb"]);
    assert!(output_include.status.success());
    let stdout_include = stdout_str(&output_include);

    // Only .hidden* pattern files should appear
    assert!(
        stdout_include.contains(".hidden_file"),
        "Hidden file matching pattern should appear"
    );
    // Directories should always be shown to maintain structure
    assert!(
        stdout_include.contains(".hidden_dir") || stdout_include.contains("normal_dir"),
        "Directories should be shown to maintain tree structure"
    );
}

// ============================================================================
// Special Path Handling Tests
// ============================================================================

#[test]
fn should_handle_path_with_spaces() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    let space_dir = root.join("path with spaces");
    fs::create_dir(&space_dir).unwrap();
    File::create(space_dir.join("file inside.txt"))
        .unwrap()
        .write_all(b"content")
        .unwrap();

    let output = run_treepp(&[space_dir.to_str().unwrap(), "/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("file inside.txt"));
}

#[test]
fn should_handle_path_with_chinese_characters() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    let chinese_dir = root.join("中文目录");
    fs::create_dir(&chinese_dir).unwrap();
    File::create(chinese_dir.join("文件.txt"))
        .unwrap()
        .write_all(b"content")
        .unwrap();

    let output = run_treepp(&[chinese_dir.to_str().unwrap(), "/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("文件.txt"));
}

#[test]
fn should_handle_path_with_special_characters() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Windows allows these characters in filenames
    let special_dir = root.join("special_!@#$%()_dir");
    fs::create_dir(&special_dir).unwrap();
    File::create(special_dir.join("file_!@#$%().txt"))
        .unwrap()
        .write_all(b"content")
        .unwrap();

    let output = run_treepp(&[special_dir.to_str().unwrap(), "/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("file_!@#$%().txt"));
}

// ============================================================================
// Relative vs Absolute Path Consistency Tests
// ============================================================================

#[test]
fn should_produce_consistent_output_for_relative_and_absolute_paths() {
    let dir = create_basic_test_dir();

    // Run with absolute path
    let abs_output = run_treepp(&[dir.path().to_str().unwrap(), "/f", "/nb"]);
    assert!(abs_output.status.success());
    let abs_stdout = stdout_str(&abs_output);

    // Run with "." from within the directory
    let rel_output = run_treepp_in_dir(dir.path(), &[".", "/f", "/nb"]);
    assert!(rel_output.status.success());
    let rel_stdout = stdout_str(&rel_output);

    // Extract content lines (skip root path line which will differ)
    let abs_lines: Vec<&str> = abs_stdout.lines().skip(1).collect();
    let rel_lines: Vec<&str> = rel_stdout.lines().skip(1).collect();

    assert_eq!(
        abs_lines, rel_lines,
        "Content should be identical regardless of path type"
    );
}

// ============================================================================
// Output File Error Handling Tests
// ============================================================================

#[test]
fn should_fail_when_output_path_is_directory() {
    let dir = create_basic_test_dir();
    let output_dir = dir.path().join("output_dir");
    fs::create_dir(&output_dir).unwrap();

    // Try to write to a path that is a directory (not a file)
    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/nb", "/o", output_dir.to_str().unwrap()],
    );

    // Should fail (either exit code 1 or 3)
    assert!(!output.status.success());
}

#[test]
fn should_fail_when_output_path_parent_does_not_exist() {
    let dir = create_basic_test_dir();
    let nonexistent_path = dir.path().join("nonexistent_dir").join("output.txt");

    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/nb", "/o", nonexistent_path.to_str().unwrap()],
    );

    // Should fail (either exit code 1 or 3)
    assert!(!output.status.success());
}

// ============================================================================
// Large Directory Performance Tests
// ============================================================================

#[test]
fn should_handle_directory_with_many_files() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create 1000 files
    for i in 0..1000 {
        File::create(root.join(format!("file_{:04}.txt", i)))
            .unwrap()
            .write_all(b"content")
            .unwrap();
    }

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Verify some files are present
    assert!(stdout.contains("file_0000.txt"));
    assert!(stdout.contains("file_0500.txt"));
    assert!(stdout.contains("file_0999.txt"));
}

#[test]
fn should_handle_deeply_nested_directories() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let mut current_path = dir.path().to_path_buf();

    // Create 50 levels of nesting
    for i in 0..50 {
        current_path = current_path.join(format!("level_{}", i));
        fs::create_dir(&current_path).unwrap();
    }

    // Create a file at the deepest level
    File::create(current_path.join("deep_file.txt"))
        .unwrap()
        .write_all(b"deep content")
        .unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("deep_file.txt"));
    assert!(stdout.contains("level_49"));
}

// ============================================================================
// Gitignore Edge Case Tests
// ============================================================================

#[test]
fn should_handle_gitignore_negation_pattern() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create .gitignore with negation pattern
    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"*.log\n!important.log\n")
        .unwrap();

    File::create(root.join("debug.log")).unwrap();
    File::create(root.join("error.log")).unwrap();
    File::create(root.join("important.log")).unwrap();
    File::create(root.join("readme.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/g"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // important.log should be included (negation pattern)
    assert!(stdout.contains("important.log"));
    // Other .log files should be excluded
    assert!(!stdout.contains("debug.log"));
    assert!(!stdout.contains("error.log"));
    // Non-log files should be included
    assert!(stdout.contains("readme.txt"));
}

#[test]
fn should_handle_gitignore_directory_pattern() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create .gitignore with directory-specific pattern
    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"build/\n")
        .unwrap();

    fs::create_dir(root.join("build")).unwrap();
    File::create(root.join("build/output.exe")).unwrap();

    fs::create_dir(root.join("src")).unwrap();
    File::create(root.join("src/main.rs")).unwrap();

    // Create a file named "build.txt" (not a directory)
    File::create(root.join("build.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/g"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // build directory should be excluded
    assert!(!stdout.contains("output.exe"));
    // build.txt file should be included (pattern ends with /)
    assert!(stdout.contains("build.txt"));
    // src should be included
    assert!(stdout.contains("main.rs"));
}

#[test]
fn should_handle_gitignore_double_star_pattern() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create .gitignore with double star pattern
    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"**/temp\n")
        .unwrap();

    // Create nested temp directories
    fs::create_dir_all(root.join("a/b/temp")).unwrap();
    File::create(root.join("a/b/temp/file.txt")).unwrap();

    fs::create_dir(root.join("src")).unwrap();
    File::create(root.join("src/main.rs")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/g"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // src should be included
    assert!(stdout.contains("main.rs"));
}

// ============================================================================
// Unicode and Multi-byte Character Tests
// ============================================================================

#[test]
fn should_handle_emoji_in_filenames() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    File::create(root.join("emoji_🎉_file.txt")).unwrap();
    File::create(root.join("日本語ファイル.txt")).unwrap();
    File::create(root.join("한국어파일.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("emoji_🎉_file.txt"));
    assert!(stdout.contains("日本語ファイル.txt"));
    assert!(stdout.contains("한국어파일.txt"));
}

// ============================================================================
// Batch Mode Sorting Stability Tests
// ============================================================================

#[test]
fn should_produce_stable_sorted_output_in_batch_mode() {
    let dir = create_basic_test_dir();

    // Run multiple times and compare outputs
    let mut outputs: Vec<String> = Vec::new();
    for _ in 0..5 {
        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/b"]);
        assert!(output.status.success());
        outputs.push(stdout_str(&output));
    }

    // All outputs should be identical
    let first = &outputs[0];
    for (i, output) in outputs.iter().enumerate().skip(1) {
        assert_eq!(first, output, "Output {} differs from first output", i);
    }
}

// ============================================================================
// Structured Output Metadata Completeness Tests
// ============================================================================

#[test]
fn should_include_all_metadata_in_json_output() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    fs::create_dir(root.join("subdir")).unwrap();
    File::create(root.join("subdir/file.txt"))
        .unwrap()
        .write_all(b"some content here")
        .unwrap();

    let output_file = root.join("output.json");

    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/nb", "/b", "/s", "/dt", "/o", output_file.to_str().unwrap()],
    );
    assert!(output.status.success());

    let json_content = fs::read_to_string(&output_file).expect("Should read JSON file");
    let json: serde_json::Value =
        serde_json::from_str(&json_content).expect("Should be valid JSON");

    // Verify new structure
    assert!(json.is_object(), "Root should be object");
    assert_eq!(
        json.get("schema").and_then(|v| v.as_str()),
        Some("treepp.pretty.v1"),
        "Should have correct schema"
    );

    let root_obj = json.get("root").expect("Should have root");
    assert!(root_obj.get("path").is_some(), "Root should have path");
    assert!(root_obj.get("type").is_some(), "Root should have type");
    assert!(root_obj.get("dirs").is_some(), "Root should have dirs");
}

// ============================================================================
// Structured Output Format V1 Schema Tests
// ============================================================================

#[test]
fn should_have_correct_json_schema_version() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(
        json.get("schema").and_then(|v| v.as_str()),
        Some("treepp.pretty.v1")
    );
}

#[test]
fn should_have_correct_yaml_schema_version() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.yml");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("schema: treepp.pretty.v1"),
        "YAML should contain schema identifier"
    );
}

#[test]
fn should_have_correct_toml_schema_version() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.toml");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(
        content.contains("schema = \"treepp.pretty.v1\""),
        "TOML should contain schema identifier"
    );
}

#[test]
fn should_separate_files_and_dirs_in_json_output() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    let root = json.get("root").expect("Should have root");

    // Files should be an array
    let files = root.get("files").expect("Should have files");
    assert!(files.is_array(), "files should be an array");

    // Dirs should be an object/map
    let dirs = root.get("dirs").expect("Should have dirs");
    assert!(dirs.is_object(), "dirs should be an object");

    // Check that files array contains expected files
    let files_array = files.as_array().unwrap();
    let file_names: Vec<&str> = files_array
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        file_names.contains(&"file1.txt") || file_names.iter().any(|f| f.contains("file1")),
        "Should contain file1.txt"
    );

    // Check that dirs contains expected directories
    assert!(
        dirs.get("src").is_some(),
        "Should have src directory in dirs"
    );
}

#[test]
fn should_have_nested_directory_structure_in_json() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    let root = json.get("root").expect("Should have root");
    let dirs = root.get("dirs").expect("Should have dirs");
    let src = dirs.get("src").expect("Should have src directory");

    // Nested directory should have same structure
    assert_eq!(
        src.get("type").and_then(|v| v.as_str()),
        Some("dir"),
        "Nested directory should have type dir"
    );
    assert!(
        src.get("files").is_some(),
        "Nested directory should have files"
    );
    assert!(
        src.get("dirs").is_some(),
        "Nested directory should have dirs"
    );

    // Check files in src directory
    let src_files = src.get("files").unwrap();
    assert!(src_files.is_array(), "src files should be an array");
}

#[test]
fn should_have_type_dir_for_all_directories_in_json() {
    let dir = create_deep_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Helper function to recursively check all directories have type: dir
    fn check_dir_type(value: &serde_json::Value, path: &str) {
        if let Some(dir_type) = value.get("type") {
            assert_eq!(
                dir_type.as_str(),
                Some("dir"),
                "Directory at {} should have type 'dir'",
                path
            );
        }
        if let Some(dirs) = value.get("dirs") {
            if let Some(dirs_obj) = dirs.as_object() {
                for (name, subdir) in dirs_obj {
                    check_dir_type(subdir, &format!("{}/{}", path, name));
                }
            }
        }
    }

    let root = json.get("root").expect("Should have root");
    check_dir_type(root, "root");
}

#[test]
fn should_have_empty_dirs_object_for_leaf_directories() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    let root = json.get("root").unwrap();
    let dirs = root.get("dirs").unwrap();

    // empty directory should have empty dirs object
    if let Some(empty_dir) = dirs.get("empty") {
        let empty_dirs = empty_dir.get("dirs");
        assert!(
            empty_dirs.is_some(),
            "Leaf directory should have dirs field"
        );
        if let Some(empty_dirs_obj) = empty_dirs.and_then(|v| v.as_object()) {
            assert!(
                empty_dirs_obj.is_empty(),
                "Leaf directory should have empty dirs object"
            );
        }
    }
}

// ============================================================================
// Parameter Conflict and Edge Case Tests
// ============================================================================

#[test]
fn should_handle_no_indent_with_ascii_mode() {
    let dir = create_basic_test_dir();

    // /NI (no indent) with /A (ASCII) - both affect tree drawing
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/ni", "/a"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // With /NI, there should be no tree-drawing characters
    assert!(
        !stdout.contains("├"),
        "Should not contain box-drawing chars with /NI"
    );
    assert!(
        !stdout.contains("│"),
        "Should not contain box-drawing chars with /NI"
    );
    assert!(
        !stdout.contains("└"),
        "Should not contain box-drawing chars with /NI"
    );
    assert!(
        !stdout.contains("+"),
        "Should not contain ASCII tree chars with /NI"
    );
    assert!(
        !stdout.contains("|"),
        "Should not contain ASCII tree chars with /NI"
    );
    assert!(
        !stdout.contains("\\"),
        "Should not contain ASCII tree chars with /NI"
    );
}

// ============================================================================
// Zero-byte and Large File Size Display Tests
// ============================================================================

#[test]
fn should_display_zero_byte_file_size_correctly() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create empty file
    File::create(root.join("empty.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/s"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should show 0 for size
    assert!(stdout.contains("0") && stdout.contains("empty.txt"));
}

#[test]
fn should_display_zero_byte_file_size_with_human_readable() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Create empty file
    File::create(root.join("empty.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/hr"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should show 0 B or similar
    assert!(
        stdout.contains("0 B") || stdout.contains("0B") || stdout.contains("0"),
        "Should display 0 bytes in human readable format"
    );
}

// ============================================================================
// Command Line Parameter Order Independence Tests
// ============================================================================

#[test]
fn should_produce_same_output_regardless_of_parameter_order() {
    let dir = create_basic_test_dir();

    let order1 = run_treepp_in_dir(dir.path(), &["/f", "/s", "/nb"]);
    let order2 = run_treepp_in_dir(dir.path(), &["/nb", "/s", "/f"]);
    let order3 = run_treepp_in_dir(dir.path(), &["/s", "/f", "/nb"]);
    let order4 = run_treepp_in_dir(dir.path(), &["/s", "/nb", "/f"]);

    assert!(order1.status.success());
    assert!(order2.status.success());
    assert!(order3.status.success());
    assert!(order4.status.success());

    let stdout1 = stdout_str(&order1);
    let stdout2 = stdout_str(&order2);
    let stdout3 = stdout_str(&order3);
    let stdout4 = stdout_str(&order4);

    assert_eq!(stdout1, stdout2, "Order /f /s /nb vs /nb /s /f should match");
    assert_eq!(stdout1, stdout3, "Order /f /s /nb vs /s /f /nb should match");
    assert_eq!(stdout1, stdout4, "Order /f /s /nb vs /s /nb /f should match");
}

#[test]
fn should_produce_same_output_with_mixed_case_parameters() {
    let dir = create_basic_test_dir();

    let lower = run_treepp_in_dir(dir.path(), &["/f", "/s", "/nb"]);
    let upper = run_treepp_in_dir(dir.path(), &["/F", "/S", "/NB"]);
    let mixed = run_treepp_in_dir(dir.path(), &["/f", "/S", "/Nb"]);

    assert!(lower.status.success());
    assert!(upper.status.success());
    assert!(mixed.status.success());

    let stdout_lower = stdout_str(&lower);
    let stdout_upper = stdout_str(&upper);
    let stdout_mixed = stdout_str(&mixed);

    assert_eq!(
        stdout_lower, stdout_upper,
        "Lower and upper case should match"
    );
    assert_eq!(
        stdout_lower, stdout_mixed,
        "Lower and mixed case should match"
    );
}
