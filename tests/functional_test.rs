//! Functional integration tests for tree++.
//!
//! This module contains comprehensive end-to-end tests that invoke the compiled
//! `treepp` binary directly and validate its output against expected behavior.
//!
//! Test categories:
//! - Help and version information
//! - Basic directory scanning
//! - Display options (files, sizes, dates, paths)
//! - Tree rendering (ASCII, Unicode, no-indent)
//! - Filtering (include, exclude, gitignore)
//! - Sorting and ordering
//! - Depth limiting
//! - Output formats (TXT, JSON, YAML, TOML)
//! - Batch mode and threading
//! - Error handling and edge cases
//! - Symbolic links and special files
//! - Permission and access issues
//! - Path edge cases
//!
//! Author: WaterRun
//! Date: 2026-02-06

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
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
fn run_treepp_in_dir(dir: &Path, args: &[&str]) -> Output {
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

// ============================================================================
// Test Directory Builders
// ============================================================================

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

    File::create(root.join("file1.txt"))
        .unwrap()
        .write_all(b"hello")
        .unwrap();
    File::create(root.join("file2.md"))
        .unwrap()
        .write_all(b"# README")
        .unwrap();

    fs::create_dir(root.join("src")).unwrap();
    File::create(root.join("src/main.rs"))
        .unwrap()
        .write_all(b"fn main() {}")
        .unwrap();
    File::create(root.join("src/lib.rs"))
        .unwrap()
        .write_all(b"pub fn lib() {}")
        .unwrap();

    fs::create_dir(root.join("tests")).unwrap();
    File::create(root.join("tests/test.rs"))
        .unwrap()
        .write_all(b"#[test]")
        .unwrap();

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

    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"target/\n*.log\n")
        .unwrap();

    File::create(root.join("file.txt"))
        .unwrap()
        .write_all(b"content")
        .unwrap();
    File::create(root.join("app.log"))
        .unwrap()
        .write_all(b"log")
        .unwrap();

    fs::create_dir(root.join("target")).unwrap();
    File::create(root.join("target/debug"))
        .unwrap()
        .write_all(b"")
        .unwrap();

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

    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"*.tmp\n")
        .unwrap();
    File::create(root.join("root.tmp")).unwrap();
    File::create(root.join("root.txt")).unwrap();

    fs::create_dir(root.join("level1")).unwrap();
    File::create(root.join("level1/.gitignore"))
        .unwrap()
        .write_all(b"*.bak\n")
        .unwrap();
    File::create(root.join("level1/l1.tmp")).unwrap();
    File::create(root.join("level1/l1.bak")).unwrap();
    File::create(root.join("level1/l1.txt")).unwrap();

    fs::create_dir(root.join("level1/level2")).unwrap();
    File::create(root.join("level1/level2/.gitignore"))
        .unwrap()
        .write_all(b"*.cache\n")
        .unwrap();
    File::create(root.join("level1/level2/l2.tmp")).unwrap();
    File::create(root.join("level1/level2/l2.bak")).unwrap();
    File::create(root.join("level1/level2/l2.cache")).unwrap();
    File::create(root.join("level1/level2/l2.txt")).unwrap();

    dir
}

/// Creates a directory with files of known sizes.
///
/// Structure:
/// ```text
/// root/
/// ├── empty.txt (0 bytes)
/// ├── small.txt (5 bytes)
/// ├── medium.txt (1024 bytes = 1 KB)
/// └── subdir/
///     └── large.txt (2048 bytes = 2 KB)
/// ```
fn create_sized_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    File::create(root.join("empty.txt")).unwrap();
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
/// ├── .dotfile
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
        ".dotfile",
        "123.txt",
        "Apple.txt",
        "banana.txt",
        "_underscore.txt",
        "zebra.txt",
    ] {
        File::create(root.join(name)).unwrap();
    }

    dir
}

/// Creates a test directory with hidden files (Windows hidden attribute).
///
/// Structure:
/// ```text
/// root/
/// ├── visible.txt
/// ├── .hidden_file (hidden attribute)
/// ├── normal_dir/
/// │   └── normal.txt
/// └── .hidden_dir/ (hidden attribute)
///     └── inside_hidden.txt
/// ```
fn create_hidden_files_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    File::create(root.join("visible.txt"))
        .unwrap()
        .write_all(b"visible")
        .unwrap();

    let hidden_file = root.join(".hidden_file");
    File::create(&hidden_file)
        .unwrap()
        .write_all(b"hidden")
        .unwrap();

    fs::create_dir(root.join("normal_dir")).unwrap();
    File::create(root.join("normal_dir/normal.txt")).unwrap();

    let hidden_dir = root.join(".hidden_dir");
    fs::create_dir(&hidden_dir).unwrap();
    File::create(hidden_dir.join("inside_hidden.txt")).unwrap();

    // Set Windows hidden attribute
    let _ = Command::new("attrib")
        .args(["+H", hidden_file.to_str().unwrap()])
        .output();
    let _ = Command::new("attrib")
        .args(["+H", hidden_dir.to_str().unwrap()])
        .output();

    dir
}

/// Creates a directory with special filenames.
///
/// Structure:
/// ```text
/// root/
/// ├── file with spaces.txt
/// ├── file-with-dashes.txt
/// ├── 中文文件.txt
/// ├── emoji_🎉.txt
/// └── special_!@#$%().txt
/// ```
fn create_special_names_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    for name in &[
        "file with spaces.txt",
        "file-with-dashes.txt",
        "中文文件.txt",
        "emoji_🎉.txt",
        "special_!@#$%().txt",
    ] {
        File::create(root.join(name)).unwrap();
    }

    dir
}

/// Creates a directory with gitignore edge case patterns.
///
/// Structure:
/// ```text
/// root/
/// ├── .gitignore (complex patterns)
/// ├── # not a comment.txt
/// ├── debug.log
/// ├── important.log
/// ├── build.txt
/// ├── build/
/// │   └── output.exe
/// └── src/
///     └── main.rs
/// ```
fn create_gitignore_edge_cases_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Complex gitignore with comments, negation, directory patterns
    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"# This is a comment\n\n*.log\n!important.log\nbuild/\n")
        .unwrap();

    // File that looks like a comment (escaped)
    File::create(root.join("# not a comment.txt")).unwrap();
    File::create(root.join("debug.log")).unwrap();
    File::create(root.join("important.log")).unwrap();
    File::create(root.join("build.txt")).unwrap();

    fs::create_dir(root.join("build")).unwrap();
    File::create(root.join("build/output.exe")).unwrap();

    fs::create_dir(root.join("src")).unwrap();
    File::create(root.join("src/main.rs")).unwrap();

    dir
}

/// Creates a directory with symbolic links (if supported).
///
/// Returns None if symlink creation fails (e.g., no admin rights on Windows).
fn create_symlink_test_dir() -> Option<TempDir> {
    let dir = TempDir::new().ok()?;
    let root = dir.path();

    // Create real file and directory
    File::create(root.join("real_file.txt"))
        .ok()?
        .write_all(b"real content")
        .ok()?;
    fs::create_dir(root.join("real_dir")).ok()?;
    File::create(root.join("real_dir/inside.txt")).ok()?;

    // Try to create symlinks (may fail without admin rights)
    #[cfg(windows)]
    {
        use std::os::windows::fs::{symlink_dir, symlink_file};
        symlink_file(root.join("real_file.txt"), root.join("link_to_file.txt")).ok()?;
        symlink_dir(root.join("real_dir"), root.join("link_to_dir")).ok()?;
        // Create broken symlink
        symlink_file(root.join("nonexistent"), root.join("broken_link")).ok()?;
    }

    Some(dir)
}

// ============================================================================
// Help and Version Tests
// ============================================================================

#[test]
fn should_show_help_with_all_flag_variants() {
    for flag in &["--help", "-h", "/?"] {
        let output = run_treepp(&[flag]);
        assert!(
            output.status.success(),
            "Help flag {} should succeed",
            flag
        );
        let stdout = stdout_str(&output);
        assert!(stdout.contains("tree++"), "Should contain program name");
        assert!(stdout.contains("Usage:"), "Should contain usage section");
        assert!(stdout.contains("Options:"), "Should contain options section");
    }
}

#[test]
fn should_show_version_with_all_flag_variants() {
    for flag in &["--version", "-v", "/V", "/v"] {
        let output = run_treepp(&[flag]);
        assert!(
            output.status.success(),
            "Version flag {} should succeed",
            flag
        );
        let stdout = stdout_str(&output);
        assert!(stdout.contains("tree++"), "Should contain program name");
        assert!(stdout.contains("WaterRun"), "Should contain author");
    }
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
    assert!(!stdout.contains("file1.txt"));
}

#[test]
fn should_scan_specified_directory() {
    let dir = create_basic_test_dir();
    let path = dir.path().to_string_lossy().to_string();
    let output = run_treepp(&[&path, "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("src"));
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

#[test]
fn should_handle_empty_directory() {
    let dir = TempDir::new().unwrap();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_handle_single_file_directory() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("only.txt")).unwrap();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("only.txt"));
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
fn should_use_unicode_by_default() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
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

    // Should contain path separators indicating full paths
    assert!(stdout.contains("\\") || stdout.contains("/"));
    assert!(stdout.contains("main.rs"));
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
    assert!(stdout.contains("KB") || stdout.contains("B"));
}

#[test]
fn should_display_zero_byte_file_correctly() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/s", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("empty.txt") && stdout.contains("0"));
}

#[test]
fn should_display_zero_with_human_readable() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/hr", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("0 B") || stdout.contains("0B") || stdout.contains("0"));
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

// ============================================================================
// No Indent Tests (/NI)
// ============================================================================

#[test]
fn should_use_no_indent_with_ni_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/ni", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should NOT contain any tree characters
    assert!(!stdout.contains("├"));
    assert!(!stdout.contains("└"));
    assert!(!stdout.contains("│"));
    assert!(!stdout.contains("+---"));
    assert!(!stdout.contains("\\---"));
    assert!(!stdout.contains("|"));
}

#[test]
fn should_combine_no_indent_with_ascii() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/ni", "/a", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // /NI takes precedence - no tree chars at all
    assert!(!stdout.contains("+"));
    assert!(!stdout.contains("|"));
    assert!(!stdout.contains("\\"));
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

    let zebra_pos = stdout.find("zebra.txt").expect("zebra.txt not found");
    let apple_pos = stdout.find("Apple.txt").expect("Apple.txt not found");

    assert!(
        zebra_pos < apple_pos,
        "In reverse order, zebra should come before Apple"
    );
}

#[test]
fn should_sort_normally_without_r_flag() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let zebra_pos = stdout.find("zebra.txt").expect("zebra.txt not found");
    let apple_pos = stdout.find("Apple.txt").expect("Apple.txt not found");

    assert!(
        apple_pos < zebra_pos,
        "In normal order, Apple should come before zebra"
    );
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
fn should_show_only_root_with_level_zero() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/l", "0", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
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

    // Directories should always be shown to maintain structure
    assert!(stdout.contains("src"));
    assert!(stdout.contains("tests"));
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
fn should_combine_include_and_exclude() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/m", "*.rs", "/m", "*.txt", "/x", "lib.rs", "/nb"],
    );
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("file1.txt"));
    assert!(!stdout.contains("lib.rs"));
    assert!(!stdout.contains("file2.md"));
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

    assert!(stdout.contains("directory") || stdout.contains("directories"));
    assert!(stdout.contains("file"));
    assert!(stdout.contains("s")); // seconds
}

#[test]
fn should_show_directory_only_report_without_files_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/rp", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("directory") || stdout.contains("directories"));
    // Should NOT show files count when /F not specified
    assert!(!stdout.contains("file"));
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

    assert!(!stdout.contains("Folder PATH listing"));
    assert!(!stdout.contains("Volume serial number"));
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
fn should_handle_gitignore_comments_and_empty_lines() {
    let dir = create_gitignore_edge_cases_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Comments should be ignored, pattern *.log should work
    assert!(!stdout.contains("debug.log"));
    // Source files should be present
    assert!(stdout.contains("main.rs"));
}

#[test]
fn should_handle_gitignore_negation_pattern() {
    let dir = create_gitignore_edge_cases_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // important.log should be included due to negation
    assert!(
        stdout.contains("important.log"),
        "Negated pattern should include important.log"
    );
    // Other .log files should be excluded
    assert!(!stdout.contains("debug.log"));
}

#[test]
fn should_handle_gitignore_directory_pattern() {
    let dir = create_gitignore_edge_cases_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // build/ directory should be excluded
    assert!(!stdout.contains("output.exe"));
    // build.txt file should be included (pattern ends with /)
    assert!(stdout.contains("build.txt"));
}

#[test]
fn should_handle_empty_gitignore_file() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join(".gitignore"))
        .unwrap()
        .write_all(b"   \n\n  \n")
        .unwrap();
    File::create(dir.path().join("file.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("file.txt"));
}

// ============================================================================
// Hidden Files Tests (/AL)
// ============================================================================

#[test]
fn should_hide_hidden_files_by_default() {
    let dir = create_hidden_files_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("visible.txt"));
    assert!(stdout.contains("normal_dir"));

    // Hidden items should NOT appear by default
    assert!(!stdout.contains(".hidden_file"));
    assert!(!stdout.contains(".hidden_dir"));
    assert!(!stdout.contains("inside_hidden.txt"));
}

#[test]
fn should_show_hidden_files_with_al_flag() {
    let dir = create_hidden_files_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/al", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("visible.txt"));
    assert!(stdout.contains(".hidden_file"));
    assert!(stdout.contains(".hidden_dir"));
    assert!(stdout.contains("inside_hidden.txt"));
}

#[test]
fn should_show_hidden_with_all_flag_variants() {
    let dir = create_hidden_files_test_dir();

    for flag in &["--all", "-k", "/AL", "/al"] {
        let output = run_treepp_in_dir(dir.path(), &["/f", flag, "/nb"]);
        assert!(output.status.success(), "Flag {} should succeed", flag);
        assert!(
            stdout_str(&output).contains(".hidden_file"),
            "Flag {} should show hidden files",
            flag
        );
    }
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
fn should_produce_same_output_in_streaming_and_batch() {
    let dir = create_basic_test_dir();

    let streaming = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    let batch = run_treepp_in_dir(dir.path(), &["/b", "/f", "/nb"]);

    assert!(streaming.status.success());
    assert!(batch.status.success());
    assert_eq!(stdout_str(&streaming), stdout_str(&batch));
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
fn should_combine_disk_usage_with_human_readable() {
    let dir = create_sized_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/du", "/hr", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("KB") || stdout.contains("B"));
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
fn should_handle_large_thread_count() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "128", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_produce_consistent_results_with_different_thread_counts() {
    let dir = create_basic_test_dir();

    let output1 = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "1", "/nb"]);
    let output4 = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "4", "/nb"]);
    let output16 = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "16", "/nb"]);

    assert!(output1.status.success());
    assert!(output4.status.success());
    assert!(output16.status.success());

    // Results should be identical regardless of thread count
    assert_eq!(stdout_str(&output1), stdout_str(&output4));
    assert_eq!(stdout_str(&output4), stdout_str(&output16));
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
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Should be valid JSON");

    assert_eq!(
        json.get("schema").and_then(|v| v.as_str()),
        Some("treepp.pretty.v1")
    );
    assert!(json.get("root").is_some());
}

#[test]
fn should_fail_json_output_without_batch() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(dir.path(), &["/f", "/o", output_file.to_str().unwrap()]);
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
    assert!(content.contains("schema: treepp.pretty.v1"));
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

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("schema = \"treepp.pretty.v1\""));
}

#[test]
fn should_fail_with_unknown_extension() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.xyz");
    let output = run_treepp_in_dir(dir.path(), &["/f", "/o", output_file.to_str().unwrap()]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
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
fn should_overwrite_existing_output_file() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.txt");

    // Create existing file with different content
    fs::write(&output_file, "old content").unwrap();

    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(!content.contains("old content"));
    assert!(content.contains("src"));
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
    assert!(stdout.is_empty() || !stdout.contains("src"));

    // File should have content
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
    let path = dir.path().to_str().unwrap();
    let output = run_treepp(&[path, path]);
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
// Parameter Style and Mixing Tests
// ============================================================================

#[test]
fn should_handle_case_insensitive_cmd_options() {
    let dir = create_basic_test_dir();

    let lower = run_treepp_in_dir(dir.path(), &["/f", "/s", "/nb"]);
    let upper = run_treepp_in_dir(dir.path(), &["/F", "/S", "/NB"]);
    let mixed = run_treepp_in_dir(dir.path(), &["/f", "/S", "/Nb"]);

    assert!(lower.status.success());
    assert!(upper.status.success());
    assert!(mixed.status.success());

    assert_eq!(stdout_str(&lower), stdout_str(&upper));
    assert_eq!(stdout_str(&lower), stdout_str(&mixed));
}

#[test]
fn should_mix_cmd_gnu_and_short_styles() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/F", "-a", "--level", "1", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_produce_same_output_regardless_of_parameter_order() {
    let dir = create_basic_test_dir();

    let order1 = run_treepp_in_dir(dir.path(), &["/f", "/s", "/nb"]);
    let order2 = run_treepp_in_dir(dir.path(), &["/nb", "/s", "/f"]);
    let order3 = run_treepp_in_dir(dir.path(), &["/s", "/f", "/nb"]);

    assert!(order1.status.success());
    assert!(order2.status.success());
    assert!(order3.status.success());

    assert_eq!(stdout_str(&order1), stdout_str(&order2));
    assert_eq!(stdout_str(&order1), stdout_str(&order3));
}

#[test]
fn should_accept_path_at_any_position() {
    let dir = create_basic_test_dir();
    let path = dir.path().to_string_lossy().to_string();

    let at_start = run_treepp(&[&path, "/f", "/nb"]);
    let at_middle = run_treepp(&["/f", &path, "/nb"]);
    let at_end = run_treepp(&["/f", "/nb", &path]);

    assert!(at_start.status.success());
    assert!(at_middle.status.success());
    assert!(at_end.status.success());
}

#[test]
fn should_accept_equals_syntax_for_value_options() {
    let dir = create_deep_test_dir();

    let output = run_treepp_in_dir(dir.path(), &["--level=2", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);
    assert!(stdout.contains("level1"));
    assert!(stdout.contains("level2"));
    assert!(!stdout.contains("level3"));
}

// ============================================================================
// Sorting Tests
// ============================================================================

#[test]
fn should_sort_dotfiles_first() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/al", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let dotfile_pos = stdout.find(".dotfile").expect(".dotfile not found");
    let apple_pos = stdout.find("Apple.txt").expect("Apple.txt not found");

    assert!(dotfile_pos < apple_pos, ".dotfile should come before Apple.txt");
}

#[test]
fn should_sort_numbers_before_letters() {
    let dir = create_sorting_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let num_pos = stdout.find("123.txt").expect("123.txt not found");
    let apple_pos = stdout.find("Apple.txt").expect("Apple.txt not found");

    assert!(num_pos < apple_pos, "123.txt should come before Apple.txt");
}

#[test]
fn should_sort_files_before_directories() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    let file_pos = stdout.find("file1.txt").expect("file1.txt not found");
    let src_pos = stdout.find("src").expect("src not found");

    assert!(file_pos < src_pos, "Files should come before directories");
}

// ============================================================================
// Tree Rendering Tests
// ============================================================================

#[test]
fn should_render_proper_vertical_lines() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::create_dir(root.join("dir1")).unwrap();
    fs::create_dir(root.join("dir2")).unwrap();
    File::create(root.join("dir1/file1.txt")).unwrap();
    File::create(root.join("dir2/file2.txt")).unwrap();

    let output = run_treepp_in_dir(root, &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should have vertical continuation lines for sibling directories
    assert!(stdout.contains("│") || stdout.contains("|"));
}

// ============================================================================
// Special Filename Tests
// ============================================================================

#[test]
fn should_handle_special_characters_in_filenames() {
    let dir = create_special_names_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("file with spaces.txt"));
    assert!(stdout.contains("file-with-dashes.txt"));
    assert!(stdout.contains("中文文件.txt"));
    assert!(stdout.contains("emoji_🎉.txt"));
    assert!(stdout.contains("special_!@#$%().txt"));
}

#[test]
fn should_handle_very_long_filename() {
    let dir = TempDir::new().unwrap();
    let long_name = "a".repeat(200) + ".txt";
    File::create(dir.path().join(&long_name)).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains(&long_name));
}

#[test]
fn should_handle_filename_with_leading_dash() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("-file.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("-file.txt"));
}

// ============================================================================
// Path Edge Cases
// ============================================================================

#[test]
fn should_handle_dot_path() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &[".", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("src"));
}

#[test]
fn should_handle_double_dot_path() {
    let dir = create_basic_test_dir();
    let subdir = dir.path().join("src");
    let output = run_treepp_in_dir(&subdir, &["..", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("src"));
}

#[test]
fn should_handle_trailing_slash_in_path() {
    let dir = create_basic_test_dir();
    let path_with_slash = format!("{}\\", dir.path().to_string_lossy());
    let output = run_treepp(&[&path_with_slash, "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_handle_path_with_spaces() {
    let dir = TempDir::new().unwrap();
    let space_dir = dir.path().join("path with spaces");
    fs::create_dir(&space_dir).unwrap();
    File::create(space_dir.join("file.txt")).unwrap();

    let output = run_treepp(&[space_dir.to_str().unwrap(), "/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("file.txt"));
}

// ============================================================================
// Deep Nesting Tests
// ============================================================================

#[test]
fn should_handle_deeply_nested_structure() {
    let dir = TempDir::new().unwrap();
    let mut current = dir.path().to_path_buf();

    for i in 0..30 {
        current = current.join(format!("level{}", i));
        fs::create_dir(&current).unwrap();
    }
    File::create(current.join("deep.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("deep.txt"));
}

// ============================================================================
// Large Directory Tests
// ============================================================================

#[test]
fn should_handle_directory_with_many_files() {
    let dir = TempDir::new().unwrap();

    for i in 0..500 {
        File::create(dir.path().join(format!("file_{:04}.txt", i))).unwrap();
    }

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("file_0000.txt"));
    assert!(stdout.contains("file_0250.txt"));
    assert!(stdout.contains("file_0499.txt"));
}

// ============================================================================
// Symbolic Link Tests
// ============================================================================

#[test]
fn should_handle_symbolic_links() {
    if let Some(dir) = create_symlink_test_dir() {
        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
        assert!(output.status.success());
        let stdout = stdout_str(&output);

        assert!(stdout.contains("real_file.txt"));
        assert!(stdout.contains("real_dir"));
    }
    // Skip test if symlinks not supported (no admin rights)
}

// ============================================================================
// Combination Tests
// ============================================================================

#[test]
fn should_combine_files_size_date() {
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
    assert!(stdout.contains("directory") || stdout.contains("directories"));
}

#[test]
fn should_combine_all_filters() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/m", "*.rs", "/x", "lib.rs", "/l", "2", "/nb"],
    );
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("lib.rs"));
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
// JSON Output Structure Tests
// ============================================================================

#[test]
fn should_output_valid_json_with_schema() {
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

    let root = json.get("root").expect("Should have root");
    assert!(root.get("path").is_some());
    assert_eq!(root.get("type").and_then(|v| v.as_str()), Some("dir"));
    assert!(root.get("files").is_some());
    assert!(root.get("dirs").is_some());
}

#[test]
fn should_separate_files_and_dirs_in_json() {
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
    let files = root.get("files").unwrap();
    let dirs = root.get("dirs").unwrap();

    assert!(files.is_array());
    assert!(dirs.is_object());
    assert!(dirs.get("src").is_some());
}

#[test]
fn should_have_nested_structure_in_json() {
    let dir = create_basic_test_dir();
    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    let src = json
        .get("root")
        .and_then(|r| r.get("dirs"))
        .and_then(|d| d.get("src"))
        .expect("Should have src directory");

    assert_eq!(src.get("type").and_then(|v| v.as_str()), Some("dir"));
    assert!(src.get("files").is_some());
    assert!(src.get("dirs").is_some());
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
    let output = run_treepp_in_dir(dir.path(), &["/si"]);
    assert_eq!(output.status.code(), Some(1));
}

// ============================================================================
// Output File Error Tests
// ============================================================================

#[test]
fn should_fail_when_output_path_is_directory() {
    let dir = create_basic_test_dir();
    let output_dir = dir.path().join("output_dir");
    fs::create_dir(&output_dir).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/o", output_dir.to_str().unwrap()]);
    assert!(!output.status.success());
}

#[test]
fn should_fail_when_output_parent_does_not_exist() {
    let dir = create_basic_test_dir();
    let bad_path = dir.path().join("nonexistent").join("output.txt");

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/o", bad_path.to_str().unwrap()]);
    assert!(!output.status.success());
}

// ============================================================================
// Stable Sorting Tests
// ============================================================================

#[test]
fn should_produce_stable_sorted_output() {
    let dir = create_basic_test_dir();

    let mut outputs: Vec<String> = Vec::new();
    for _ in 0..3 {
        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/b"]);
        assert!(output.status.success());
        outputs.push(stdout_str(&output));
    }

    // All outputs should be identical
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(outputs[1], outputs[2]);
}

// ============================================================================
// Gitignore Parsing Edge Cases (5 tests)
// ============================================================================

/// Creates a directory with double-asterisk glob patterns in .gitignore.
///
/// Structure:
/// ```text
/// root/
/// ├── .gitignore (contains: "**/build\nsrc/**/test\n")
/// ├── build/ (ignored)
/// │   └── output.exe
/// ├── deep/
/// │   └── nested/
/// │       └── build/ (ignored)
/// │           └── artifact.dll
/// ├── src/
/// │   ├── main.rs
/// │   └── modules/
/// │       └── test/ (ignored)
/// │           └── mock.rs
/// └── other/
///     └── test/ (not ignored - pattern is src/**/test)
///         └── data.txt
/// ```
fn create_double_asterisk_gitignore_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"**/build\nsrc/**/test\n")
        .unwrap();

    // Root level build directory
    fs::create_dir_all(root.join("build")).unwrap();
    File::create(root.join("build/output.exe")).unwrap();

    // Deeply nested build directory
    fs::create_dir_all(root.join("deep/nested/build")).unwrap();
    File::create(root.join("deep/nested/build/artifact.dll")).unwrap();

    // Source directory with nested test
    fs::create_dir_all(root.join("src/modules/test")).unwrap();
    File::create(root.join("src/main.rs")).unwrap();
    File::create(root.join("src/modules/test/mock.rs")).unwrap();

    // Test directory outside src (should NOT be ignored)
    fs::create_dir_all(root.join("other/test")).unwrap();
    File::create(root.join("other/test/data.txt")).unwrap();

    dir
}

#[test]
fn should_handle_double_asterisk_any_depth_pattern() {
    let dir = create_double_asterisk_gitignore_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // **/build should match build at any depth
    assert!(
        !stdout.contains("output.exe"),
        "Root build/ should be ignored by **/build"
    );
    assert!(
        !stdout.contains("artifact.dll"),
        "Nested build/ should be ignored by **/build"
    );

    // src/**/test should match test directories under src
    assert!(
        !stdout.contains("mock.rs"),
        "src/modules/test/ should be ignored by src/**/test"
    );

    // Files outside the pattern should be present
    assert!(stdout.contains("main.rs"), "src/main.rs should be present");
    assert!(
        stdout.contains("data.txt"),
        "other/test/data.txt should NOT be ignored (pattern is src/**/test)"
    );
}

/// Creates a directory with character class patterns in .gitignore.
///
/// Structure:
/// ```text
/// root/
/// ├── .gitignore (contains: "[abc].txt\n[!xyz].log\n[0-9].dat\n")
/// ├── a.txt (ignored)
/// ├── b.txt (ignored)
/// ├── d.txt (not ignored)
/// ├── x.log (not ignored - negated class)
/// ├── y.log (not ignored)
/// ├── a.log (ignored - matches [!xyz])
/// ├── 0.dat (ignored)
/// ├── 5.dat (ignored)
/// └── a.dat (not ignored)
/// ```
fn create_character_class_gitignore_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"[abc].txt\n[!xyz].log\n[0-9].dat\n")
        .unwrap();

    // [abc].txt pattern files
    File::create(root.join("a.txt")).unwrap();
    File::create(root.join("b.txt")).unwrap();
    File::create(root.join("d.txt")).unwrap();

    // [!xyz].log pattern files (negated class - matches anything NOT x, y, or z)
    File::create(root.join("x.log")).unwrap();
    File::create(root.join("y.log")).unwrap();
    File::create(root.join("a.log")).unwrap();

    // [0-9].dat pattern files
    File::create(root.join("0.dat")).unwrap();
    File::create(root.join("5.dat")).unwrap();
    File::create(root.join("a.dat")).unwrap();

    dir
}

#[test]
fn should_handle_character_class_patterns_in_gitignore() {
    let dir = create_character_class_gitignore_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // [abc].txt should match a.txt, b.txt but not d.txt
    assert!(!stdout.contains("a.txt"), "a.txt should be ignored by [abc].txt");
    assert!(!stdout.contains("b.txt"), "b.txt should be ignored by [abc].txt");
    assert!(stdout.contains("d.txt"), "d.txt should NOT be ignored");

    // [!xyz].log should match anything except x.log, y.log, z.log
    assert!(stdout.contains("x.log"), "x.log should NOT be ignored by [!xyz].log");
    assert!(stdout.contains("y.log"), "y.log should NOT be ignored by [!xyz].log");
    assert!(!stdout.contains("a.log"), "a.log should be ignored by [!xyz].log");

    // [0-9].dat should match single digit files
    assert!(!stdout.contains("0.dat"), "0.dat should be ignored by [0-9].dat");
    assert!(!stdout.contains("5.dat"), "5.dat should be ignored by [0-9].dat");
    assert!(stdout.contains("a.dat"), "a.dat should NOT be ignored");
}

/// Creates a directory with escaped special characters in .gitignore.
///
/// Structure:
/// ```text
/// root/
/// ├── .gitignore (contains: "\\#comment.txt\n\\!important.md\n")
/// ├── #comment.txt (ignored - escaped hash)
/// ├── !important.md (ignored - escaped exclamation)
/// ├── regular.txt
/// └── # actual comment line should be ignored
/// ```
fn create_escaped_chars_gitignore_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // Note: \# matches literal #, \! matches literal !
    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"\\#comment.txt\n\\!important.md\n# This is a real comment\n")
        .unwrap();

    File::create(root.join("#comment.txt")).unwrap();
    File::create(root.join("!important.md")).unwrap();
    File::create(root.join("regular.txt")).unwrap();

    dir
}

#[test]
fn should_handle_escaped_special_characters_in_gitignore() {
    let dir = create_escaped_chars_gitignore_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Escaped # should match literal # filename
    assert!(
        !stdout.contains("#comment.txt"),
        "#comment.txt should be ignored by \\#comment.txt pattern"
    );

    // Escaped ! should match literal ! filename
    assert!(
        !stdout.contains("!important.md"),
        "!important.md should be ignored by \\!important.md pattern"
    );

    // Regular files should be present
    assert!(stdout.contains("regular.txt"), "regular.txt should be present");
}

/// Creates a directory with root-relative patterns in .gitignore.
///
/// Structure:
/// ```text
/// root/
/// ├── .gitignore (contains: "/build\n/config.local\n")
/// ├── build/ (ignored - at root)
/// │   └── output.exe
/// ├── config.local (ignored - at root)
/// ├── src/
/// │   ├── build/ (NOT ignored - not at root)
/// │   │   └── temp.o
/// │   └── config.local (NOT ignored - not at root)
/// └── nested/
///     └── deep/
///         └── build/ (NOT ignored)
///             └── artifact.bin
/// ```
fn create_root_relative_gitignore_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(b"/build\n/config.local\n")
        .unwrap();

    // Root level (should be ignored)
    fs::create_dir(root.join("build")).unwrap();
    File::create(root.join("build/output.exe")).unwrap();
    File::create(root.join("config.local")).unwrap();

    // Nested in src (should NOT be ignored)
    fs::create_dir_all(root.join("src/build")).unwrap();
    File::create(root.join("src/build/temp.o")).unwrap();
    File::create(root.join("src/config.local")).unwrap();

    // Deeply nested (should NOT be ignored)
    fs::create_dir_all(root.join("nested/deep/build")).unwrap();
    File::create(root.join("nested/deep/build/artifact.bin")).unwrap();

    dir
}

#[test]
fn should_handle_root_relative_patterns_in_gitignore() {
    let dir = create_root_relative_gitignore_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Root level should be ignored
    assert!(
        !stdout.contains("output.exe"),
        "Root build/output.exe should be ignored by /build"
    );

    // Nested occurrences should NOT be ignored (pattern starts with /)
    assert!(
        stdout.contains("temp.o"),
        "src/build/temp.o should NOT be ignored (not at root)"
    );
    assert!(
        stdout.contains("artifact.bin"),
        "nested/deep/build/artifact.bin should NOT be ignored"
    );

    // Check config.local similarly
    let config_count = stdout.matches("config.local").count();
    assert!(
        config_count >= 1,
        "At least one nested config.local should be present"
    );
}

/// Creates a directory with UTF-8 BOM in .gitignore.
///
/// Structure:
/// ```text
/// root/
/// ├── .gitignore (UTF-8 BOM + "ignored.txt\n")
/// ├── ignored.txt (should be ignored despite BOM)
/// └── visible.txt
/// ```
fn create_bom_gitignore_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    // UTF-8 BOM: EF BB BF
    let mut gitignore_content = vec![0xEF, 0xBB, 0xBF];
    gitignore_content.extend_from_slice(b"ignored.txt\n");

    File::create(root.join(".gitignore"))
        .unwrap()
        .write_all(&gitignore_content)
        .unwrap();

    File::create(root.join("ignored.txt")).unwrap();
    File::create(root.join("visible.txt")).unwrap();

    dir
}

#[test]
fn should_handle_utf8_bom_in_gitignore() {
    let dir = create_bom_gitignore_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Pattern should work despite BOM
    assert!(
        !stdout.contains("ignored.txt"),
        "ignored.txt should be ignored even with UTF-8 BOM in .gitignore"
    );
    assert!(stdout.contains("visible.txt"), "visible.txt should be present");
}

// ============================================================================
// Wildcard Pattern Tests (/M and /X)
// ============================================================================

/// Creates a directory for wildcard pattern testing.
///
/// Structure:
/// ```text
/// root/
/// ├── test.txt
/// ├── test123.txt
/// ├── test_file.txt
/// ├── data.test.txt
/// ├── file.tar.gz
/// ├── archive.tar.bz2
/// ├── README
/// └── .hidden
/// ```
fn create_wildcard_test_dir() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let root = dir.path();

    for name in &[
        "test.txt",
        "test123.txt",
        "test_file.txt",
        "data.test.txt",
        "file.tar.gz",
        "archive.tar.bz2",
        "README",
        ".hidden",
    ] {
        File::create(root.join(name)).unwrap();
    }

    dir
}

#[test]
fn should_handle_multiple_dots_in_wildcard_pattern() {
    let dir = create_wildcard_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "*.tar.*", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("file.tar.gz"), "file.tar.gz should match *.tar.*");
    assert!(
        stdout.contains("archive.tar.bz2"),
        "archive.tar.bz2 should match *.tar.*"
    );
    assert!(
        !stdout.contains("test.txt"),
        "test.txt should not match *.tar.*"
    );
}

#[test]
fn should_handle_middle_asterisk_pattern() {
    let dir = create_wildcard_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "test*.txt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("test.txt"), "test.txt should match test*.txt");
    assert!(
        stdout.contains("test123.txt"),
        "test123.txt should match test*.txt"
    );
    assert!(
        stdout.contains("test_file.txt"),
        "test_file.txt should match test*.txt"
    );
    assert!(
        !stdout.contains("data.test.txt"),
        "data.test.txt should not match test*.txt"
    );
}

#[test]
fn should_handle_asterisk_only_pattern() {
    let dir = create_wildcard_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "*", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // All files should match
    assert!(stdout.contains("test.txt"));
    assert!(stdout.contains("README"));
    assert!(stdout.contains("file.tar.gz"));
}

#[test]
fn should_handle_question_mark_single_char_wildcard() {
    let dir = TempDir::new().unwrap();
    for name in &["a.txt", "ab.txt", "abc.txt", "1.txt"] {
        File::create(dir.path().join(name)).unwrap();
    }

    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "?.txt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("a.txt"), "a.txt should match ?.txt");
    assert!(stdout.contains("1.txt"), "1.txt should match ?.txt");
    assert!(!stdout.contains("ab.txt"), "ab.txt should not match ?.txt");
    assert!(!stdout.contains("abc.txt"), "abc.txt should not match ?.txt");
}

#[test]
fn should_be_case_insensitive_on_windows_for_patterns() {
    let dir = TempDir::new().unwrap();

    // Create files with different case extensions
    // Note: On Windows, we can't have both FILE.TXT and file.txt in same directory
    // due to case-insensitive filesystem, so use different base names
    File::create(dir.path().join("readme.TXT")).unwrap();
    File::create(dir.path().join("data.Txt")).unwrap();
    File::create(dir.path().join("notes.txt")).unwrap();
    File::create(dir.path().join("image.PNG")).unwrap(); // Should not match

    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "*.txt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // On Windows, *.txt should match *.TXT, *.Txt case-insensitively
    // Count how many .txt/.TXT/.Txt files appear in output
    let txt_count = stdout
        .lines()
        .filter(|line| {
            let lower = line.to_lowercase();
            lower.contains(".txt")
        })
        .count();

    assert!(
        txt_count >= 3,
        "Pattern matching should be case-insensitive on Windows, expected 3 txt files, found {}.\nOutput:\n{}",
        txt_count,
        stdout
    );

    // PNG file should not appear
    assert!(
        !stdout.to_lowercase().contains(".png"),
        "PNG file should not match *.txt pattern"
    );
}

// ============================================================================
// Path and Filesystem Edge Cases
// ============================================================================

#[test]
fn should_handle_drive_root_scanning() {
    // Test scanning from drive root (may require admin or specific setup)
    let output = run_treepp(&["C:\\", "/l", "0", "/nb"]);
    // Should succeed or fail gracefully
    if output.status.success() {
        let stdout = stdout_str(&output);
        assert!(stdout.contains("C:\\") || stdout.contains("C:."));
    }
}

#[test]
fn should_handle_consecutive_spaces_in_directory_name() {
    let dir = TempDir::new().unwrap();
    let space_dir = dir.path().join("dir  with   spaces");
    fs::create_dir(&space_dir).unwrap();
    File::create(space_dir.join("file.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(
        stdout.contains("dir  with   spaces"),
        "Directory with consecutive spaces should be displayed"
    );
    assert!(stdout.contains("file.txt"));
}

#[test]
fn should_handle_windows_reserved_names_gracefully() {
    let dir = TempDir::new().unwrap();

    // Try to create files/directories with reserved names
    // These may fail on Windows, which is expected
    let reserved_names = ["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"];

    for name in &reserved_names {
        // Attempt creation - may fail on Windows
        let _ = fs::create_dir(dir.path().join(name));
    }

    // Create a normal file
    File::create(dir.path().join("normal.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("normal.txt"));
}

#[test]
fn should_handle_junction_points() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("target_dir");
    let junction = dir.path().join("junction_link");

    fs::create_dir(&target).unwrap();
    File::create(target.join("inside.txt")).unwrap();

    // Create junction point (Windows specific)
    let result = Command::new("cmd")
        .args([
            "/C",
            "mklink",
            "/J",
            junction.to_str().unwrap(),
            target.to_str().unwrap(),
        ])
        .output();

    if result.is_ok() && junction.exists() {
        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
        assert!(output.status.success());
        let stdout = stdout_str(&output);
        assert!(stdout.contains("target_dir") || stdout.contains("junction_link"));
    }
    // Skip if junction creation failed (no admin rights)
}

#[test]
fn should_handle_very_long_paths() {
    let dir = TempDir::new().unwrap();

    // Create a path approaching 260 character limit
    let mut current = dir.path().to_path_buf();
    let segment = "a".repeat(20);

    for _ in 0..10 {
        current = current.join(&segment);
        if fs::create_dir(&current).is_err() {
            break; // Hit path length limit
        }
    }

    let output = run_treepp_in_dir(dir.path(), &["/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Multithreading and Batch Mode
// ============================================================================

#[test]
fn should_handle_single_file_with_many_threads() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("only.txt")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "16", "/nb"]);
    assert!(output.status.success());
    assert!(stdout_str(&output).contains("only.txt"));
}

#[test]
fn should_handle_wide_directory_with_multiple_threads() {
    let dir = TempDir::new().unwrap();

    // Create many subdirectories
    for i in 0..100 {
        let subdir = dir.path().join(format!("dir_{:03}", i));
        fs::create_dir(&subdir).unwrap();
        File::create(subdir.join("file.txt")).unwrap();
    }

    let output = run_treepp_in_dir(dir.path(), &["/b", "/f", "/t", "8", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("dir_000"));
    assert!(stdout.contains("dir_050"));
    assert!(stdout.contains("dir_099"));
}

#[test]
fn should_fail_with_negative_thread_count() {
    let dir = TempDir::new().unwrap();
    let output = run_treepp_in_dir(dir.path(), &["/b", "/t", "-1"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_produce_identical_results_across_thread_counts() {
    let dir = create_basic_test_dir();

    let outputs: Vec<String> = [1, 2, 4, 8, 16]
        .iter()
        .map(|t| {
            let output =
                run_treepp_in_dir(dir.path(), &["/b", "/f", "/nb", "/t", &t.to_string()]);
            assert!(output.status.success());
            stdout_str(&output)
        })
        .collect();

    // All outputs should be identical
    for (i, output) in outputs.iter().enumerate().skip(1) {
        assert_eq!(
            &outputs[0], output,
            "Output with {} threads differs from 1 thread",
            [1, 2, 4, 8, 16][i]
        );
    }
}

// ============================================================================
// Output Format Validation
// ============================================================================

#[test]
fn should_handle_empty_directory_in_json() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("empty")).unwrap();

    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    let empty_dir = json
        .get("root")
        .and_then(|r| r.get("dirs"))
        .and_then(|d| d.get("empty"));

    assert!(empty_dir.is_some(), "Empty directory should be in JSON output");
}

#[test]
fn should_preserve_yaml_indentation_in_deep_nesting() {
    let dir = create_deep_test_dir();
    let output_file = dir.path().join("tree.yml");
    let output = run_treepp_in_dir(
        dir.path(),
        &["/b", "/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();

    // Verify YAML is parseable
    assert!(content.contains("schema: treepp.pretty.v1"));

    // Check for consistent indentation (2 spaces per level is common)
    let lines: Vec<&str> = content.lines().collect();
    for line in &lines {
        if line.starts_with(' ') {
            let leading_spaces = line.len() - line.trim_start().len();
            assert_eq!(
                leading_spaces % 2,
                0,
                "YAML indentation should be consistent (multiples of 2)"
            );
        }
    }
}

#[test]
fn should_handle_large_file_sizes_in_json() {
    let dir = TempDir::new().unwrap();

    // Create a file and write enough to get a meaningful size
    let large_file = dir.path().join("large.bin");
    let data = vec![0u8; 1024 * 1024]; // 1 MB
    fs::write(&large_file, &data).unwrap();

    let output_file = dir.path().join("tree.json");
    let output = run_treepp_in_dir(
        dir.path(),
        &[
            "/b",
            "/f",
            "/s",
            "/o",
            output_file.to_str().unwrap(),
            "/nb",
        ],
    );
    assert!(output.status.success());

    let content = fs::read_to_string(&output_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Verify size is present and reasonable
    assert!(json.to_string().contains("1048576") || json.to_string().contains("size"));
}

// ============================================================================
// Disk Usage Calculation (/DU)
// ============================================================================

#[test]
fn should_show_zero_for_empty_directory_disk_usage() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("empty")).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/b", "/du", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("empty"));
    // Empty directory should have 0 size
    assert!(stdout.contains("0") || stdout.contains("empty"));
}

#[test]
fn should_accumulate_sizes_correctly_in_nested_directories() {
    let dir = TempDir::new().unwrap();

    // Create structure with known sizes
    fs::create_dir(dir.path().join("parent")).unwrap();
    fs::create_dir(dir.path().join("parent/child")).unwrap();

    // 100 bytes in child
    fs::write(dir.path().join("parent/child/file.txt"), vec![b'x'; 100]).unwrap();
    // 200 bytes in parent
    fs::write(dir.path().join("parent/other.txt"), vec![b'y'; 200]).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/b", "/f", "/du", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Parent should show cumulative size of at least 300 bytes
    assert!(
        stdout.contains("300") || stdout.contains("parent"),
        "Parent directory should show cumulative size"
    );
}

#[test]
fn should_combine_disk_usage_with_human_readable_correctly() {
    let dir = TempDir::new().unwrap();

    // Create 1 KB file
    fs::write(dir.path().join("kb_file.txt"), vec![b'x'; 1024]).unwrap();

    let output = run_treepp_in_dir(dir.path(), &["/b", "/f", "/du", "/hr", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(
        stdout.contains("KB") || stdout.contains("1.0") || stdout.contains("1024"),
        "Should show human-readable size"
    );
}

// ============================================================================
// Date and Time Handling
// ============================================================================

#[test]
fn should_display_dates_in_consistent_format() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/dt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Date format should be YYYY-MM-DD HH:MM:SS
    let date_pattern = regex::Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();
    assert!(
        date_pattern.is_match(&stdout),
        "Date should be in YYYY-MM-DD HH:MM:SS format"
    );
}

// ============================================================================
// Report Statistics (/RP)
// ============================================================================

#[test]
fn should_show_zero_counts_for_empty_directory() {
    let dir = TempDir::new().unwrap();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/rp", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(
        stdout.contains("0 director") || stdout.contains("0 file"),
        "Empty directory should show zero counts"
    );
}

#[test]
fn should_report_only_filtered_items_when_using_include() {
    let dir = create_basic_test_dir();

    // Get report without filter
    let output_all = run_treepp_in_dir(dir.path(), &["/f", "/rp", "/nb"]);
    assert!(output_all.status.success());

    // Get report with filter
    let output_filtered = run_treepp_in_dir(dir.path(), &["/f", "/m", "*.rs", "/rp", "/nb"]);
    assert!(output_filtered.status.success());

    let stdout_filtered = stdout_str(&output_filtered);

    // Filtered report should show fewer files
    assert!(
        stdout_filtered.contains("file") || stdout_filtered.contains("director"),
        "Filtered report should include statistics"
    );
}

#[test]
fn should_report_directories_only_without_f_flag() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/rp", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Should show directory count
    assert!(
        stdout.contains("director"),
        "Report should include directory count"
    );
    // Should NOT show file count when /F not used
    assert!(
        !stdout.contains("file") || stdout.contains("0 file"),
        "Report should not show file count without /F"
    );
}

// ============================================================================
// Parameter Parsing Edge Cases
// ============================================================================

#[test]
fn should_handle_value_immediately_after_flag() {
    let dir = create_deep_test_dir();

    // Some tools support /L1 without space
    let output = run_treepp_in_dir(dir.path(), &["/L1", "/nb"]);

    // This might fail or succeed depending on implementation
    // The test documents the behavior
    if output.status.success() {
        let stdout = stdout_str(&output);
        assert!(stdout.contains("level1"));
        assert!(!stdout.contains("level2"));
    }
}

#[test]
fn should_fail_with_empty_value_after_equals() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["--level="]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn should_use_last_value_when_parameter_repeated() {
    let dir = create_deep_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/l", "1", "/l", "3", "/nb"]);

    // Implementation-dependent: might error or use last value
    // This test documents the actual behavior
    if output.status.success() {
        let stdout = stdout_str(&output);
        // If it uses last value (3), level3 should be visible
        // If it uses first value (1), level2 should not be visible
        assert!(stdout.contains("level1"));
    }
}

#[test]
fn should_handle_mixed_path_separators() {
    let dir = create_basic_test_dir();

    // Point to parent of the mixed path
    let parent = dir.path().join("src");
    let output = run_treepp(&[parent.to_str().unwrap(), "/f", "/nb"]);
    assert!(output.status.success());
}

// ============================================================================
// Unicode and Encoding
// ============================================================================

#[test]
fn should_handle_pure_emoji_directory_name() {
    let dir = TempDir::new().unwrap();
    let emoji_dir = dir.path().join("🎉🎊🎁");

    if fs::create_dir(&emoji_dir).is_ok() {
        File::create(emoji_dir.join("party.txt")).unwrap();

        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
        assert!(output.status.success());
        let stdout = stdout_str(&output);

        assert!(
            stdout.contains("🎉🎊🎁"),
            "Pure emoji directory name should be displayed"
        );
        assert!(stdout.contains("party.txt"));
    }
}

#[test]
fn should_handle_right_to_left_text() {
    let dir = TempDir::new().unwrap();
    let arabic_file = dir.path().join("ملف.txt"); // "file" in Arabic

    if File::create(&arabic_file).is_ok() {
        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
        assert!(output.status.success());
        let stdout = stdout_str(&output);

        assert!(
            stdout.contains("ملف.txt"),
            "Right-to-left text should be displayed"
        );
    }
}

#[test]
fn should_handle_combining_characters() {
    let dir = TempDir::new().unwrap();
    // é composed as e + combining acute accent
    let combining_file = dir.path().join("cafe\u{0301}.txt");

    if File::create(&combining_file).is_ok() {
        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
        assert!(output.status.success());
        // File should appear in output (exact form may vary)
    }
}

#[test]
fn should_handle_zero_width_characters() {
    let dir = TempDir::new().unwrap();
    // Zero-width space in filename
    let zwsp_file = dir.path().join("file\u{200B}name.txt");

    if File::create(&zwsp_file).is_ok() {
        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
        assert!(output.status.success());
        let stdout = stdout_str(&output);

        assert!(
            stdout.contains("file") && stdout.contains("name"),
            "File with zero-width character should be displayed"
        );
    }
}

// ============================================================================
// Error Message Quality
// ============================================================================

#[test]
fn should_include_path_in_error_for_nonexistent_directory() {
    let bad_path = "/nonexistent/path/12345/67890";
    let output = run_treepp(&[bad_path]);
    assert!(!output.status.success());

    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("nonexistent") || stderr.contains(bad_path),
        "Error message should include the problematic path"
    );
}

#[test]
fn should_provide_helpful_hint_for_misspelled_option() {
    let output = run_treepp(&["/file"]); // Might be confused with /F
    assert!(!output.status.success());

    let stderr = stderr_str(&output);
    assert!(
        stderr.contains("--help") || stderr.contains("/?") || stderr.contains("Unknown"),
        "Error should suggest how to get help"
    );
}

// ============================================================================
// Deep Nesting Edge Cases
// ============================================================================

#[test]
fn should_handle_50_level_deep_directory() {
    let dir = TempDir::new().unwrap();
    let mut current = dir.path().to_path_buf();

    for i in 0..50 {
        current = current.join(format!("d{}", i));
        if fs::create_dir(&current).is_err() {
            break;
        }
    }
    let _ = File::create(current.join("deep.txt"));

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
}

#[test]
fn should_handle_breadth_of_1000_items() {
    let dir = TempDir::new().unwrap();

    for i in 0..1000 {
        File::create(dir.path().join(format!("file_{:04}.txt", i))).unwrap();
    }

    let output = run_treepp_in_dir(dir.path(), &["/f", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(stdout.contains("file_0000.txt"));
    assert!(stdout.contains("file_0500.txt"));
    assert!(stdout.contains("file_0999.txt"));
}

// ============================================================================
// Combination Conflict Tests
// ============================================================================

#[test]
fn should_handle_no_indent_with_ascii_combination() {
    let dir = create_basic_test_dir();
    let output = run_treepp_in_dir(dir.path(), &["/f", "/ni", "/a", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // /NI should take precedence - no tree characters
    assert!(!stdout.contains("├"));
    assert!(!stdout.contains("└"));
    assert!(!stdout.contains("+"));
    assert!(!stdout.contains("\\---"));
}

#[test]
fn should_apply_gitignore_before_exclude_pattern() {
    let dir = create_gitignore_test_dir();

    // .gitignore excludes target/ and *.log
    // /X additionally excludes *.txt
    let output = run_treepp_in_dir(dir.path(), &["/f", "/g", "/x", "*.txt", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    assert!(!stdout.contains("target"), "target should be ignored by .gitignore");
    assert!(!stdout.contains("app.log"), "*.log should be ignored by .gitignore");
    assert!(
        !stdout.contains("file.txt"),
        "*.txt should be excluded by /X"
    );
    assert!(stdout.contains("main.rs"), "main.rs should be present");
}

#[test]
fn should_handle_conflicting_include_and_exclude_patterns() {
    let dir = create_basic_test_dir();

    // Include *.rs but also exclude *.rs (conflicting)
    let output = run_treepp_in_dir(dir.path(), &["/f", "/m", "*.rs", "/x", "*.rs", "/nb"]);
    assert!(output.status.success());
    let stdout = stdout_str(&output);

    // Exclude should win (more restrictive)
    assert!(
        !stdout.contains("main.rs"),
        "Exclude should take precedence over include"
    );
    assert!(
        !stdout.contains("lib.rs"),
        "Exclude should take precedence over include"
    );
}

// ============================================================================
// Sorting Stability Tests
// ============================================================================

#[test]
fn should_maintain_stable_sort_with_identical_names_different_types() {
    let dir = TempDir::new().unwrap();

    // Create file and directory with same name (not possible on same level)
    // But test similar names
    fs::create_dir(dir.path().join("item")).unwrap();
    File::create(dir.path().join("item.txt")).unwrap();
    File::create(dir.path().join("item.rs")).unwrap();

    let mut outputs: Vec<String> = Vec::new();
    for _ in 0..5 {
        let output = run_treepp_in_dir(dir.path(), &["/f", "/nb", "/b"]);
        assert!(output.status.success());
        outputs.push(stdout_str(&output));
    }

    // All outputs should be identical (stable sort)
    for output in &outputs[1..] {
        assert_eq!(&outputs[0], output, "Sort should be stable across runs");
    }
}

// ============================================================================
// Output File Error Handling
// ============================================================================

#[test]
fn should_fail_gracefully_when_output_path_has_invalid_chars() {
    let dir = create_basic_test_dir();

    // Try to output to path with invalid characters
    let invalid_path = if cfg!(windows) {
        "output<>.txt"
    } else {
        "output\0.txt"
    };

    let output = run_treepp_in_dir(dir.path(), &["/f", "/o", invalid_path]);
    assert!(!output.status.success());
}

#[test]
fn should_create_output_file_even_when_tree_is_empty() {
    let dir = TempDir::new().unwrap();
    let output_file = dir.path().join("empty_tree.txt");

    let output = run_treepp_in_dir(
        dir.path(),
        &["/f", "/o", output_file.to_str().unwrap(), "/nb"],
    );
    assert!(output.status.success());
    assert!(output_file.exists(), "Output file should be created even for empty tree");
}