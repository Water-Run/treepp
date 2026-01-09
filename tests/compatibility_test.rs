//! tree++ 与 Windows 原生 tree 命令严格兼容性测试
//!
//! 本模块测试 tree++ 与 Windows 原生 tree 命令在相同参数下的输出**严格一致性**。
//!
//! # 测试范围
//!
//! - 无参数（仅显示目录）
//! - `/F` 参数（显示文件）
//! - `/A` 参数（ASCII 字符）
//! - `/F /A` 组合参数
//!
//! # 比较策略
//!
//! 输出必须**逐字节一致**（忽略行尾空白差异）
//!
//! 文件: tests/compatibility_test.rs
//! 作者: WaterRun
//! 更新于: 2026-01-09

use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

// ============================================================================
// 测试辅助结构与函数
// ============================================================================

/// 命令输出结果
#[derive(Debug, Clone)]
struct CommandOutput {
    stdout: String,
    #[allow(dead_code)]
    stderr: String,
    exit_code: Option<i32>,
}

impl CommandOutput {
    /// 从原生 tree 命令输出创建（GBK 解码）
    fn from_native_output(output: &Output) -> Self {
        let (stdout, _, _) = encoding_rs::GBK.decode(&output.stdout);
        let (stderr, _, _) = encoding_rs::GBK.decode(&output.stderr);
        Self {
            stdout: stdout.into_owned(),
            stderr: stderr.into_owned(),
            exit_code: output.status.code(),
        }
    }

    /// 从 treepp 命令输出创建（UTF-8 解码）
    fn from_treepp_output(output: &Output) -> Self {
        Self {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
        }
    }

    /// 规范化输出：移除行尾空白
    fn normalized_lines(&self) -> Vec<String> {
        self.stdout
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }
}

/// 执行原生 Windows tree 命令
fn run_native_tree(path: &Path, args: &[&str]) -> CommandOutput {
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "tree"]);
    cmd.args(args);
    cmd.arg(path);
    let output = cmd.output().expect("执行原生 tree 命令失败");
    CommandOutput::from_native_output(&output)
}

/// 执行 treepp 命令
fn run_treepp(path: &Path, args: &[&str]) -> CommandOutput {
    let treepp_path = get_treepp_path();
    let mut cmd = Command::new(&treepp_path);
    cmd.args(args);
    cmd.arg(path);
    let output = cmd.output().expect("执行 treepp 命令失败");
    CommandOutput::from_treepp_output(&output)
}

/// 获取 treepp 可执行文件路径
fn get_treepp_path() -> PathBuf {
    let debug_path = PathBuf::from("target/debug/treepp.exe");
    if debug_path.exists() {
        return debug_path;
    }
    let release_path = PathBuf::from("target/release/treepp.exe");
    if release_path.exists() {
        return release_path;
    }
    panic!("treepp 未构建，请先运行 cargo build");
}

/// 简洁差异报告（最多显示 5 处差异）
fn compact_diff(native: &CommandOutput, treepp: &CommandOutput, context: &str) {
    let nl = native.normalized_lines();
    let tl = treepp.normalized_lines();

    if nl == tl {
        return;
    }

    let mut report = format!("\n=== {} 不一致 ===\n", context);
    report.push_str(&format!("行数: native={}, treepp={}\n", nl.len(), tl.len()));

    let mut diff_count = 0;
    let max_lines = nl.len().max(tl.len());

    for i in 0..max_lines {
        let n = nl.get(i).map(|s| s.as_str()).unwrap_or("<缺失>");
        let t = tl.get(i).map(|s| s.as_str()).unwrap_or("<缺失>");

        if n != t {
            diff_count += 1;
            if diff_count <= 3 {
                report.push_str(&format!(
                    "L{}: N={:?}\n    T={:?}\n",
                    i + 1,
                    truncate(n, 60),
                    truncate(t, 60)
                ));
            }
        }
    }

    if diff_count > 3 {
        report.push_str(&format!("...及另外 {} 处差异\n", diff_count - 3));
    }

    panic!("{}", report);
}

/// 截断字符串
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

/// 验证退出码
fn assert_exit_codes(native: &CommandOutput, treepp: &CommandOutput, ctx: &str) {
    assert_eq!(
        native.exit_code, treepp.exit_code,
        "{}: 退出码不同 N={:?} T={:?}",
        ctx, native.exit_code, treepp.exit_code
    );
}

// ============================================================================
// 测试目录创建
// ============================================================================

fn create_empty_dir() -> TempDir {
    TempDir::new().expect("创建临时目录失败")
}

fn create_single_level_dirs() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("alpha")).unwrap();
    fs::create_dir(dir.path().join("beta")).unwrap();
    fs::create_dir(dir.path().join("gamma")).unwrap();
    dir
}

fn create_single_level_with_files() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("alpha")).unwrap();
    fs::create_dir(dir.path().join("beta")).unwrap();
    File::create(dir.path().join("file1.txt")).unwrap();
    File::create(dir.path().join("file2.txt")).unwrap();
    dir
}

fn create_nested_dirs() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("a/b/c")).unwrap();
    fs::create_dir(dir.path().join("a/d")).unwrap();
    fs::create_dir(dir.path().join("e")).unwrap();
    dir
}

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

fn create_with_empty_subdirs() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("empty1")).unwrap();
    fs::create_dir_all(dir.path().join("empty2/nested")).unwrap();
    fs::create_dir(dir.path().join("has_file")).unwrap();
    File::create(dir.path().join("has_file/f.txt")).unwrap();
    dir
}

fn create_project_like() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    File::create(dir.path().join("Cargo.toml")).unwrap();
    File::create(dir.path().join("README.md")).unwrap();
    File::create(dir.path().join("src/main.rs")).unwrap();
    File::create(dir.path().join("src/lib.rs")).unwrap();
    dir
}

fn create_deep_nested() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("a/b/c/d/e")).unwrap();
    File::create(dir.path().join("a/b/c/d/e/deep.txt")).unwrap();
    File::create(dir.path().join("a/b/mid.txt")).unwrap();
    dir
}

fn create_wide_structure() -> TempDir {
    let dir = TempDir::new().unwrap();
    for name in ["alpha", "beta", "gamma", "delta"] {
        fs::create_dir(dir.path().join(name)).unwrap();
        File::create(dir.path().join(name).join("file.txt")).unwrap();
    }
    dir
}

fn get_cargo_dir() -> Option<PathBuf> {
    let home = env::var("USERPROFILE").ok()?;
    let cargo_dir = PathBuf::from(home).join(".cargo");
    if cargo_dir.exists() && cargo_dir.is_dir() {
        Some(cargo_dir)
    } else {
        None
    }
}

// ============================================================================
// 无参数测试
// ============================================================================

#[test]
fn test_compat_empty_no_args() {
    let dir = create_empty_dir();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "空目录");
    compact_diff(&n, &t, "空目录-无参数");
}

#[test]
fn test_compat_single_dirs_no_args() {
    let dir = create_single_level_dirs();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "单层目录");
    compact_diff(&n, &t, "单层目录-无参数");
}

#[test]
fn test_compat_single_with_files_no_args() {
    let dir = create_single_level_with_files();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "单层带文件");
    compact_diff(&n, &t, "单层带文件-无参数");
}

#[test]
fn test_compat_nested_no_args() {
    let dir = create_nested_dirs();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "嵌套目录");
    compact_diff(&n, &t, "嵌套目录-无参数");
}

#[test]
fn test_compat_nested_files_no_args() {
    let dir = create_nested_with_files();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "嵌套带文件");
    compact_diff(&n, &t, "嵌套带文件-无参数");
}

#[test]
fn test_compat_empty_subdirs_no_args() {
    let dir = create_with_empty_subdirs();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "含空子目录");
    compact_diff(&n, &t, "含空子目录-无参数");
}

#[test]
fn test_compat_project_no_args() {
    let dir = create_project_like();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "项目结构");
    compact_diff(&n, &t, "项目结构-无参数");
}

#[test]
fn test_compat_deep_no_args() {
    let dir = create_deep_nested();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "深层嵌套");
    compact_diff(&n, &t, "深层嵌套-无参数");
}

#[test]
fn test_compat_wide_no_args() {
    let dir = create_wide_structure();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "宽结构");
    compact_diff(&n, &t, "宽结构-无参数");
}

// ============================================================================
// /F 参数测试
// ============================================================================

#[test]
fn test_compat_empty_f() {
    let dir = create_empty_dir();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "空目录/F");
    compact_diff(&n, &t, "空目录-/F");
}

#[test]
fn test_compat_single_dirs_f() {
    let dir = create_single_level_dirs();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "单层目录/F");
    compact_diff(&n, &t, "单层目录-/F");
}

#[test]
fn test_compat_single_with_files_f() {
    let dir = create_single_level_with_files();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "单层带文件/F");
    compact_diff(&n, &t, "单层带文件-/F");
}

#[test]
fn test_compat_nested_f() {
    let dir = create_nested_dirs();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "嵌套目录/F");
    compact_diff(&n, &t, "嵌套目录-/F");
}

#[test]
fn test_compat_nested_files_f() {
    let dir = create_nested_with_files();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "嵌套带文件/F");
    compact_diff(&n, &t, "嵌套带文件-/F");
}

#[test]
fn test_compat_empty_subdirs_f() {
    let dir = create_with_empty_subdirs();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "含空子目录/F");
    compact_diff(&n, &t, "含空子目录-/F");
}

#[test]
fn test_compat_project_f() {
    let dir = create_project_like();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "项目结构/F");
    compact_diff(&n, &t, "项目结构-/F");
}

#[test]
fn test_compat_deep_f() {
    let dir = create_deep_nested();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "深层嵌套/F");
    compact_diff(&n, &t, "深层嵌套-/F");
}

#[test]
fn test_compat_wide_f() {
    let dir = create_wide_structure();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "宽结构/F");
    compact_diff(&n, &t, "宽结构-/F");
}

// ============================================================================
// /A 参数测试
// ============================================================================

#[test]
fn test_compat_empty_a() {
    let dir = create_empty_dir();
    let n = run_native_tree(dir.path(), &["/A"]);
    let t = run_treepp(dir.path(), &["/A"]);
    assert_exit_codes(&n, &t, "空目录/A");
    compact_diff(&n, &t, "空目录-/A");
}

#[test]
fn test_compat_single_dirs_a() {
    let dir = create_single_level_dirs();
    let n = run_native_tree(dir.path(), &["/A"]);
    let t = run_treepp(dir.path(), &["/A"]);
    assert_exit_codes(&n, &t, "单层目录/A");
    compact_diff(&n, &t, "单层目录-/A");
}

#[test]
fn test_compat_nested_a() {
    let dir = create_nested_dirs();
    let n = run_native_tree(dir.path(), &["/A"]);
    let t = run_treepp(dir.path(), &["/A"]);
    assert_exit_codes(&n, &t, "嵌套目录/A");
    compact_diff(&n, &t, "嵌套目录-/A");
}

#[test]
fn test_compat_project_a() {
    let dir = create_project_like();
    let n = run_native_tree(dir.path(), &["/A"]);
    let t = run_treepp(dir.path(), &["/A"]);
    assert_exit_codes(&n, &t, "项目结构/A");
    compact_diff(&n, &t, "项目结构-/A");
}

#[test]
fn test_compat_wide_a() {
    let dir = create_wide_structure();
    let n = run_native_tree(dir.path(), &["/A"]);
    let t = run_treepp(dir.path(), &["/A"]);
    assert_exit_codes(&n, &t, "宽结构/A");
    compact_diff(&n, &t, "宽结构-/A");
}

// ============================================================================
// /F /A 组合测试
// ============================================================================

#[test]
fn test_compat_empty_fa() {
    let dir = create_empty_dir();
    let n = run_native_tree(dir.path(), &["/F", "/A"]);
    let t = run_treepp(dir.path(), &["/F", "/A"]);
    assert_exit_codes(&n, &t, "空目录/F/A");
    compact_diff(&n, &t, "空目录-/F/A");
}

#[test]
fn test_compat_single_dirs_fa() {
    let dir = create_single_level_dirs();
    let n = run_native_tree(dir.path(), &["/F", "/A"]);
    let t = run_treepp(dir.path(), &["/F", "/A"]);
    assert_exit_codes(&n, &t, "单层目录/F/A");
    compact_diff(&n, &t, "单层目录-/F/A");
}

#[test]
fn test_compat_single_with_files_fa() {
    let dir = create_single_level_with_files();
    let n = run_native_tree(dir.path(), &["/F", "/A"]);
    let t = run_treepp(dir.path(), &["/F", "/A"]);
    assert_exit_codes(&n, &t, "单层带文件/F/A");
    compact_diff(&n, &t, "单层带文件-/F/A");
}

#[test]
fn test_compat_nested_fa() {
    let dir = create_nested_dirs();
    let n = run_native_tree(dir.path(), &["/F", "/A"]);
    let t = run_treepp(dir.path(), &["/F", "/A"]);
    assert_exit_codes(&n, &t, "嵌套目录/F/A");
    compact_diff(&n, &t, "嵌套目录-/F/A");
}

#[test]
fn test_compat_nested_files_fa() {
    let dir = create_nested_with_files();
    let n = run_native_tree(dir.path(), &["/F", "/A"]);
    let t = run_treepp(dir.path(), &["/F", "/A"]);
    assert_exit_codes(&n, &t, "嵌套带文件/F/A");
    compact_diff(&n, &t, "嵌套带文件-/F/A");
}

#[test]
fn test_compat_project_fa() {
    let dir = create_project_like();
    let n = run_native_tree(dir.path(), &["/F", "/A"]);
    let t = run_treepp(dir.path(), &["/F", "/A"]);
    assert_exit_codes(&n, &t, "项目结构/F/A");
    compact_diff(&n, &t, "项目结构-/F/A");
}

#[test]
fn test_compat_deep_fa() {
    let dir = create_deep_nested();
    let n = run_native_tree(dir.path(), &["/F", "/A"]);
    let t = run_treepp(dir.path(), &["/F", "/A"]);
    assert_exit_codes(&n, &t, "深层嵌套/F/A");
    compact_diff(&n, &t, "深层嵌套-/F/A");
}

#[test]
fn test_compat_wide_fa() {
    let dir = create_wide_structure();
    let n = run_native_tree(dir.path(), &["/F", "/A"]);
    let t = run_treepp(dir.path(), &["/F", "/A"]);
    assert_exit_codes(&n, &t, "宽结构/F/A");
    compact_diff(&n, &t, "宽结构-/F/A");
}

// ============================================================================
// 参数变体测试
// ============================================================================

#[test]
fn test_compat_lowercase_f() {
    let dir = create_project_like();
    let n = run_native_tree(dir.path(), &["/f"]);
    let t = run_treepp(dir.path(), &["/f"]);
    assert_exit_codes(&n, &t, "小写/f");
    compact_diff(&n, &t, "小写-/f");
}

#[test]
fn test_compat_lowercase_a() {
    let dir = create_project_like();
    let n = run_native_tree(dir.path(), &["/a"]);
    let t = run_treepp(dir.path(), &["/a"]);
    assert_exit_codes(&n, &t, "小写/a");
    compact_diff(&n, &t, "小写-/a");
}

#[test]
fn test_compat_reversed_af() {
    let dir = create_project_like();
    let n = run_native_tree(dir.path(), &["/A", "/F"]);
    let t = run_treepp(dir.path(), &["/A", "/F"]);
    assert_exit_codes(&n, &t, "反序/A/F");
    compact_diff(&n, &t, "反序-/A/F");
}

// ============================================================================
// 真实目录测试（.cargo）
// ============================================================================

#[test]
fn test_compat_cargo_no_args() {
    let Some(cargo) = get_cargo_dir() else {
        eprintln!("跳过: .cargo 不存在");
        return;
    };
    let n = run_native_tree(&cargo, &[]);
    let t = run_treepp(&cargo, &[]);
    assert_exit_codes(&n, &t, ".cargo");
    compact_diff(&n, &t, ".cargo-无参数");
}

#[test]
fn test_compat_cargo_f() {
    let Some(cargo) = get_cargo_dir() else {
        eprintln!("跳过: .cargo 不存在");
        return;
    };
    let n = run_native_tree(&cargo, &["/F"]);
    let t = run_treepp(&cargo, &["/F"]);
    assert_exit_codes(&n, &t, ".cargo/F");
    compact_diff(&n, &t, ".cargo-/F");
}

#[test]
fn test_compat_cargo_a() {
    let Some(cargo) = get_cargo_dir() else {
        eprintln!("跳过: .cargo 不存在");
        return;
    };
    let n = run_native_tree(&cargo, &["/A"]);
    let t = run_treepp(&cargo, &["/A"]);
    assert_exit_codes(&n, &t, ".cargo/A");
    compact_diff(&n, &t, ".cargo-/A");
}

#[test]
fn test_compat_cargo_fa() {
    let Some(cargo) = get_cargo_dir() else {
        eprintln!("跳过: .cargo 不存在");
        return;
    };
    let n = run_native_tree(&cargo, &["/F", "/A"]);
    let t = run_treepp(&cargo, &["/F", "/A"]);
    assert_exit_codes(&n, &t, ".cargo/F/A");
    compact_diff(&n, &t, ".cargo-/F/A");
}

// ============================================================================
// 边界条件测试
// ============================================================================

#[test]
fn test_compat_single_file() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("only.txt")).unwrap();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "单文件");
    compact_diff(&n, &t, "单文件-/F");
}

#[test]
fn test_compat_single_subdir() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("only")).unwrap();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "单子目录");
    compact_diff(&n, &t, "单子目录");
}

#[test]
fn test_compat_hidden_files() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join(".hidden")).unwrap();
    File::create(dir.path().join(".gitignore")).unwrap();
    File::create(dir.path().join("normal.txt")).unwrap();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "隐藏文件");
    compact_diff(&n, &t, "隐藏文件-/F");
}

#[test]
fn test_compat_special_chars() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("file with spaces.txt")).unwrap();
    File::create(dir.path().join("file-dash.txt")).unwrap();
    File::create(dir.path().join("file_underscore.txt")).unwrap();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "特殊字符");
    compact_diff(&n, &t, "特殊字符-/F");
}

#[test]
fn test_compat_sorting_alpha() {
    let dir = TempDir::new().unwrap();
    for name in ["zebra.txt", "apple.txt", "mango.txt"] {
        File::create(dir.path().join(name)).unwrap();
    }
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "字母排序");
    compact_diff(&n, &t, "字母排序-/F");
}

#[test]
fn test_compat_sorting_mixed_case() {
    let dir = TempDir::new().unwrap();
    for name in ["AAA.txt", "aaa.txt", "BBB.txt", "bbb.txt"] {
        File::create(dir.path().join(name)).unwrap();
    }
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "大小写混合");
    compact_diff(&n, &t, "大小写混合-/F");
}

#[test]
fn test_compat_sorting_numeric() {
    let dir = TempDir::new().unwrap();
    for name in ["1.txt", "10.txt", "2.txt", "20.txt"] {
        File::create(dir.path().join(name)).unwrap();
    }
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "数字排序");
    compact_diff(&n, &t, "数字排序-/F");
}

#[test]
fn test_compat_dir_file_order() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("dir_a")).unwrap();
    File::create(dir.path().join("file_a.txt")).unwrap();
    fs::create_dir(dir.path().join("dir_b")).unwrap();
    File::create(dir.path().join("file_b.txt")).unwrap();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "目录文件顺序");
    compact_diff(&n, &t, "目录文件顺序-/F");
}

#[test]
fn test_compat_unicode_names() {
    let dir = TempDir::new().unwrap();
    File::create(dir.path().join("中文.txt")).unwrap();
    File::create(dir.path().join("日本語.txt")).unwrap();
    File::create(dir.path().join("normal.txt")).unwrap();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "Unicode文件名");
    compact_diff(&n, &t, "Unicode-/F");
}

#[test]
fn test_compat_long_name() {
    let dir = TempDir::new().unwrap();
    let long = "a".repeat(100) + ".txt";
    File::create(dir.path().join(&long)).unwrap();
    File::create(dir.path().join("short.txt")).unwrap();
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "长文件名");
    compact_diff(&n, &t, "长文件名-/F");
}

#[test]
fn test_compat_deep_empty() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("a/b/c/d/e")).unwrap();
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "深层空目录");
    compact_diff(&n, &t, "深层空目录");
}

// ============================================================================
// 压力测试
// ============================================================================

#[test]
fn test_compat_many_files() {
    let dir = TempDir::new().unwrap();
    for i in 0..30 {
        File::create(dir.path().join(format!("file_{:03}.txt", i))).unwrap();
    }
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "多文件");
    compact_diff(&n, &t, "多文件-/F");
}

#[test]
fn test_compat_many_dirs() {
    let dir = TempDir::new().unwrap();
    for i in 0..20 {
        fs::create_dir(dir.path().join(format!("dir_{:03}", i))).unwrap();
    }
    let n = run_native_tree(dir.path(), &[]);
    let t = run_treepp(dir.path(), &[]);
    assert_exit_codes(&n, &t, "多目录");
    compact_diff(&n, &t, "多目录");
}

#[test]
fn test_compat_mixed_many() {
    let dir = TempDir::new().unwrap();
    for i in 0..10 {
        let sub = dir.path().join(format!("dir_{:02}", i));
        fs::create_dir(&sub).unwrap();
        for j in 0..3 {
            File::create(sub.join(format!("f{}.txt", j))).unwrap();
        }
    }
    let n = run_native_tree(dir.path(), &["/F"]);
    let t = run_treepp(dir.path(), &["/F"]);
    assert_exit_codes(&n, &t, "混合多");
    compact_diff(&n, &t, "混合多-/F");
}