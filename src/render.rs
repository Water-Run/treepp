//! 渲染模块：将扫描 IR 渲染为文本树
//!
//! 本模块负责将 `scan` 模块产出的 `TreeNode` 渲染为可展示的文本格式，支持：
//!
//! - **树形风格**：ASCII (`/A`) 或 Unicode（默认）
//! - **无缩进模式**：仅使用空白缩进 (`/NI`)
//! - **路径显示**：相对名称（默认）或完整路径 (`/FP`)
//! - **元信息显示**：文件大小 (`/S`)、人类可读大小 (`/HR`)、修改日期 (`/DT`)、目录累计大小 (`/DU`)
//! - **统计报告**：末尾统计信息 (`/RP`)
//! - **Windows 样板**：默认显示系统卷信息，`/NB` 禁止
//!
//! 作者: WaterRun
//! 更新于: 2025-01-06

#![forbid(unsafe_code)]

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use std::time::SystemTime;

use crate::config::{CharsetMode, Config, PathMode};
use crate::error::RenderError;
use crate::scan::{EntryKind, ScanStats, TreeNode};

// ============================================================================
// 常量
// ============================================================================

/// Windows tree++ 样板信息目录路径
const TREEPP_BANNER_DIR: &str = r"C:\__tree++__";

/// Windows tree++ 样板信息文件名
const TREEPP_BANNER_FILE: &str = "tree++.txt";

/// 样板文件内容提示
const TREEPP_BANNER_FILE_CONTENT: &str = r#"This directory is automatically created by tree++ to align with the native Windows tree command's banner (boilerplate) output.

You may safely delete this directory. If you do not want tree++ to create it, use the /NB option when running tree++.

GitHub: https://github.com/Water-Run/treepp
"#;

/// 全局缓存的 Windows 样板信息
static CACHED_BANNER: OnceLock<Result<WinBanner, String>> = OnceLock::new();

// ============================================================================
// Windows 样板信息
// ============================================================================

/// Windows tree 命令样板信息
///
/// 包含从 Windows 原生 tree 命令提取的样板信息。
/// 通过在 `C:\__tree++__` 目录执行 `tree` 命令获取系统本地化的样板文本。
///
/// # 输出格式
///
/// Windows `tree` 命令输出固定为 4 行：
/// ```text
/// 卷 系统 的文件夹 PATH 列表      <- 第1行: volume_line
/// 卷序列号为 2810-11C7            <- 第2行: serial_line
/// C:.                             <- 第3行: 当前目录（忽略）
/// 没有子文件夹                    <- 第4行: no_subfolder
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinBanner {
    /// 卷信息行 (如 "卷 系统 的文件夹 PATH 列表")
    pub volume_line: String,
    /// 卷序列号行 (如 "卷序列号为 2810-11C7")
    pub serial_line: String,
    /// 无子文件夹提示 (如 "没有子文件夹")
    pub no_subfolder: String,
}

impl WinBanner {
    /// 获取 Windows 样板信息（带缓存）
    ///
    /// 首次调用时从系统获取，后续调用返回缓存结果。
    ///
    /// # Errors
    ///
    /// 返回 `RenderError::BannerFetchFailed` 如果：
    /// - 无法创建样板目录
    /// - 无法执行 tree 命令
    /// - tree 输出行数不足
    pub fn get_cached() -> Result<&'static WinBanner, RenderError> {
        CACHED_BANNER
            .get_or_init(|| Self::fetch_from_system().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|reason| RenderError::BannerFetchFailed {
                reason: reason.clone(),
            })
    }

    /// 直接从系统获取样板信息（不使用缓存）
    ///
    /// 此方法会执行以下操作：
    /// 1. 确保 `C:\__tree++__` 目录存在
    /// 2. 确保 `tree++.txt` 文件存在
    /// 3. 在该目录下执行 `tree` 命令（无参数）
    /// 4. 提取输出的第1行、第2行、最后一行
    pub fn fetch() -> Result<Self, RenderError> {
        Self::fetch_from_system()
    }

    /// 从 tree 命令获取样板信息的内部实现
    fn fetch_from_system() -> Result<Self, RenderError> {
        let dir_path = Path::new(TREEPP_BANNER_DIR);
        let file_path = dir_path.join(TREEPP_BANNER_FILE);

        // 确保目录存在
        if !dir_path.exists() {
            fs::create_dir_all(dir_path).map_err(|e| RenderError::BannerFetchFailed {
                reason: format!("无法创建目录 {}: {}", TREEPP_BANNER_DIR, e),
            })?;
        }

        // 确保样板文件存在（用于标识此目录用途）
        if !file_path.exists() {
            fs::write(&file_path, TREEPP_BANNER_FILE_CONTENT).map_err(|e| {
                RenderError::BannerFetchFailed {
                    reason: format!("无法创建文件 {}: {}", file_path.display(), e),
                }
            })?;
        }

        // 在 C:\__tree++__ 目录下执行 tree 命令（无参数）
        let output = Command::new("cmd")
            .args(["/C", "tree"])
            .current_dir(dir_path)
            .output()
            .map_err(|e| RenderError::BannerFetchFailed {
                reason: format!("执行 tree 命令失败: {}", e),
            })?;

        if !output.status.success() {
            return Err(RenderError::BannerFetchFailed {
                reason: format!("tree 命令返回错误码: {:?}", output.status.code()),
            });
        }

        // 解码 GBK 输出
        let stdout = Self::decode_system_output(&output.stdout)?;

        Self::parse_tree_output(&stdout)
    }

    /// 解码系统输出（处理 Windows GBK/CP936 编码）
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

    /// 解析 tree 命令输出
    ///
    /// 直接提取第1行、第2行、最后一行，无需关键字匹配。
    fn parse_tree_output(output: &str) -> Result<Self, RenderError> {
        let lines: Vec<&str> = output.lines().collect();

        // tree 输出至少需要 4 行
        if lines.len() < 4 {
            return Err(RenderError::BannerFetchFailed {
                reason: format!(
                    "tree 输出行数不足，期望至少 4 行，实际 {} 行:\n{}",
                    lines.len(),
                    output
                ),
            });
        }

        // 第1行: 卷信息
        let volume_line = lines[0].trim().to_string();
        // 第2行: 序列号
        let serial_line = lines[1].trim().to_string();
        // 最后一行: 无子文件夹提示
        let no_subfolder = lines[lines.len() - 1].trim().to_string();

        Ok(Self {
            volume_line,
            serial_line,
            no_subfolder,
        })
    }

    /// 从字符串解析样板信息（用于测试）
    pub fn parse(output: &str) -> Result<Self, RenderError> {
        Self::parse_tree_output(output)
    }
}

// ============================================================================
// 渲染结果
// ============================================================================

/// 渲染结果
#[derive(Debug, Clone)]
pub struct RenderResult {
    /// 渲染后的文本内容
    pub content: String,
    /// 目录数量
    pub directory_count: usize,
    /// 文件数量
    pub file_count: usize,
}

// ============================================================================
// 树形连接符
// ============================================================================

/// 树形连接符集合
struct TreeChars {
    /// 分支连接符 (├─ 或 +--)
    branch: &'static str,
    /// 最后分支连接符 (└─ 或 \--)
    last_branch: &'static str,
    /// 垂直连接符 (│   或 |   )
    vertical: &'static str,
    /// 空白占位符
    space: &'static str,
}

impl TreeChars {
    /// 根据字符集模式创建连接符集合
    fn from_charset(charset: CharsetMode) -> Self {
        match charset {
            CharsetMode::Unicode => Self {
                branch: "├─",
                last_branch: "└─",
                vertical: "│   ",
                space: "    ",
            },
            CharsetMode::Ascii => Self {
                branch: "+--",
                last_branch: "\\--",
                vertical: "|   ",
                space: "    ",
            },
        }
    }
}

// ============================================================================
// 格式化辅助函数
// ============================================================================

/// 格式化文件大小为人类可读形式
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

/// 格式化 SystemTime 为日期时间字符串
#[must_use]
pub fn format_datetime(time: &SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    let days = secs / 86400;
    let time_of_day = secs % 86400;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let (year, month, day) = days_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// 将天数转换为年月日
fn days_to_ymd(days: u64) -> (i32, u32, u32) {
    let mut remaining_days = days as i64;
    let mut year = 1970i32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let days_in_months: [i64; 12] = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u32;
    for days_in_month in days_in_months {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days as u32 + 1;

    (year, month, day)
}

/// 判断是否为闰年
const fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// 格式化条目名称
fn format_entry_name(node: &TreeNode, config: &Config) -> String {
    match config.render.path_mode {
        PathMode::Full => node.path.to_string_lossy().into_owned(),
        PathMode::Relative => node.name.clone(),
    }
}

/// 格式化条目元信息 (大小、日期)
fn format_entry_meta(node: &TreeNode, config: &Config) -> String {
    let mut parts = Vec::new();

    // 文件大小
    if config.render.show_size && node.kind == EntryKind::File {
        let size = node.metadata.size;
        let size_str = if config.render.human_readable {
            format_size_human(size)
        } else {
            size.to_string()
        };
        parts.push(size_str);
    }

    // 目录累计大小
    if config.render.show_disk_usage && node.kind == EntryKind::Directory {
        if let Some(usage) = node.disk_usage {
            let usage_str = if config.render.human_readable {
                format_size_human(usage)
            } else {
                usage.to_string()
            };
            parts.push(usage_str);
        }
    }

    // 修改日期
    if config.render.show_date {
        if let Some(ref modified) = node.metadata.modified {
            parts.push(format_datetime(modified));
        }
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("        {}", parts.join("  "))
    }
}

// ============================================================================
// 渲染器
// ============================================================================

/// 渲染树形结构为文本
#[must_use]
pub fn render(stats: &ScanStats, config: &Config) -> RenderResult {
    let mut output = String::new();
    let chars = TreeChars::from_charset(config.render.charset);

    let root_path = config.root_path.to_string_lossy();

    // Windows 样板头信息
    let banner = if config.render.no_win_banner {
        None
    } else {
        match WinBanner::get_cached() {
            Ok(b) => Some(b),
            Err(e) => {
                let _ = writeln!(output, "警告: {}", e);
                None
            }
        }
    };

    // 输出样板头
    if let Some(b) = &banner {
        output.push_str(&b.volume_line);
        output.push('\n');
        output.push_str(&b.serial_line);
        output.push('\n');
    }

    // 根路径
    output.push_str(&root_path);
    output.push('\n');

    // 渲染子节点
    if config.render.no_indent {
        render_children_no_indent(&mut output, &stats.tree, config, 0);
    } else {
        render_children(&mut output, &stats.tree, &chars, config, "");
    }

    // 无子目录提示
    if let Some(b) = &banner {
        if stats.directory_count == 0 {
            output.push('\n');
            output.push_str(&b.no_subfolder);
            output.push('\n');
        }
    }

    // gitignore 提示
    if config.scan.respect_gitignore {
        output.push('\n');
        output.push_str(".gitignore rules applied");
        output.push('\n');
    }

    // 统计报告
    if config.render.show_report {
        let time_str = format!(" in {:.3}s", stats.duration.as_secs_f64());

        output.push('\n');
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

    RenderResult {
        content: output,
        directory_count: stats.directory_count,
        file_count: stats.file_count,
    }
}

/// 渲染子节点 (带树形连接符)
fn render_children(
    output: &mut String,
    node: &TreeNode,
    chars: &TreeChars,
    config: &Config,
    prefix: &str,
) {
    let children = get_filtered_children(node, config);
    let count = children.len();

    for (i, child) in children.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last {
            chars.last_branch
        } else {
            chars.branch
        };

        let name = format_entry_name(child, config);
        let meta = format_entry_meta(child, config);

        let _ = writeln!(output, "{prefix}{connector}{name}{meta}");

        if !child.children.is_empty() {
            let new_prefix = if is_last {
                format!("{}{}", prefix, chars.space)
            } else {
                format!("{}{}", prefix, chars.vertical)
            };
            render_children(output, child, chars, config, &new_prefix);
        }
    }
}

/// 渲染子节点 (无树形连接符，仅缩进)
fn render_children_no_indent(output: &mut String, node: &TreeNode, config: &Config, depth: usize) {
    let children = get_filtered_children(node, config);
    let indent = "  ".repeat(depth);

    for child in &children {
        let name = format_entry_name(child, config);
        let meta = format_entry_meta(child, config);

        let _ = writeln!(output, "{indent}{name}{meta}");

        if !child.children.is_empty() {
            render_children_no_indent(output, child, config, depth + 1);
        }
    }
}

/// 获取过滤后的子节点列表
fn get_filtered_children<'a>(node: &'a TreeNode, config: &Config) -> Vec<&'a TreeNode> {
    node.children
        .iter()
        .filter(|c| config.scan.show_files || c.kind == EntryKind::Directory)
        .filter(|c| {
            !config.matching.prune_empty || c.kind != EntryKind::Directory || !c.is_empty_dir()
        })
        .collect()
}

/// 仅渲染树形文本，不包含 banner 和统计信息
#[must_use]
pub fn render_tree_only(node: &TreeNode, config: &Config) -> String {
    let mut output = String::new();
    let chars = TreeChars::from_charset(config.render.charset);

    let root_name = format_entry_name(node, config);
    let root_meta = format_entry_meta(node, config);
    let _ = writeln!(output, "{root_name}{root_meta}");

    if config.render.no_indent {
        render_children_no_indent(&mut output, node, config, 0);
    } else {
        render_children(&mut output, node, &chars, config, "");
    }

    output
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::EntryMetadata;
    use std::path::PathBuf;
    use std::time::Duration;

    /// 创建测试用的简单树结构
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

    // ========================================================================
    // WinBanner 测试
    // ========================================================================

    #[test]
    fn test_win_banner_parse_valid_4_lines() {
        let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n没有子文件夹";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹");
    }

    #[test]
    fn test_win_banner_parse_with_trailing_whitespace() {
        let output =
            "卷 系统 的文件夹 PATH 列表  \n  卷序列号为 2810-11C7\nC:.\n没有子文件夹  \n";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹");
    }

    #[test]
    fn test_win_banner_parse_english_locale() {
        let output =
            "Folder PATH listing for volume OS\nVolume serial number is ABCD-1234\nC:.\nNo subfolders exist";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "Folder PATH listing for volume OS");
        assert_eq!(banner.serial_line, "Volume serial number is ABCD-1234");
        assert_eq!(banner.no_subfolder, "No subfolders exist");
    }

    #[test]
    fn test_win_banner_parse_too_few_lines() {
        let output = "只有一行";
        let result = WinBanner::parse(output);
        assert!(result.is_err());

        let output = "第一行\n第二行";
        let result = WinBanner::parse(output);
        assert!(result.is_err());

        let output = "第一行\n第二行\n第三行";
        let result = WinBanner::parse(output);
        assert!(result.is_err());
    }

    #[test]
    fn test_win_banner_parse_exactly_4_lines() {
        let output = "line1\nline2\nline3\nline4";
        let banner = WinBanner::parse(output).expect("4行应该解析成功");

        assert_eq!(banner.volume_line, "line1");
        assert_eq!(banner.serial_line, "line2");
        assert_eq!(banner.no_subfolder, "line4");
    }

    #[test]
    fn test_win_banner_parse_more_than_4_lines() {
        // 如果目录有子目录，输出会更多行，最后一行可能是目录名
        let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n├─subdir\n└─another";
        let banner = WinBanner::parse(output).expect("多行应该解析成功");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "└─another"); // 最后一行
    }

    #[test]
    fn test_win_banner_empty_input() {
        let output = "";
        let result = WinBanner::parse(output);
        assert!(result.is_err());
    }

    // ========================================================================
    // format_size_human 测试
    // ========================================================================

    #[test]
    fn test_format_size_human() {
        assert_eq!(format_size_human(0), "0 B");
        assert_eq!(format_size_human(512), "512 B");
        assert_eq!(format_size_human(1023), "1023 B");
        assert_eq!(format_size_human(1024), "1.0 KB");
        assert_eq!(format_size_human(1536), "1.5 KB");
        assert_eq!(format_size_human(1048576), "1.0 MB");
        assert_eq!(format_size_human(1073741824), "1.0 GB");
        assert_eq!(format_size_human(1099511627776), "1.0 TB");
    }

    // ========================================================================
    // format_datetime 测试
    // ========================================================================

    #[test]
    fn test_format_datetime() {
        use std::time::UNIX_EPOCH;

        let epoch = UNIX_EPOCH;
        let formatted = format_datetime(&epoch);
        assert_eq!(formatted, "1970-01-01 00:00:00");
    }

    // ========================================================================
    // days_to_ymd 测试
    // ========================================================================

    #[test]
    fn test_days_to_ymd() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        assert_eq!(days_to_ymd(1), (1970, 1, 2));
        assert_eq!(days_to_ymd(31), (1970, 2, 1));
        assert_eq!(days_to_ymd(365), (1971, 1, 1));
    }

    // ========================================================================
    // is_leap_year 测试
    // ========================================================================

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(1970));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }

    // ========================================================================
    // TreeChars 测试
    // ========================================================================

    #[test]
    fn test_tree_chars_unicode() {
        let chars = TreeChars::from_charset(CharsetMode::Unicode);
        assert_eq!(chars.branch, "├─");
        assert_eq!(chars.last_branch, "└─");
        assert_eq!(chars.vertical, "│   ");
    }

    #[test]
    fn test_tree_chars_ascii() {
        let chars = TreeChars::from_charset(CharsetMode::Ascii);
        assert_eq!(chars.branch, "+--");
        assert_eq!(chars.last_branch, "\\--");
        assert_eq!(chars.vertical, "|   ");
    }

    // ========================================================================
    // render 测试
    // ========================================================================

    #[test]
    fn test_render_basic() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(result.content.contains("test_root"));
        assert!(result.content.contains("src"));
        assert!(result.content.contains("main.rs"));
        assert!(result.content.contains("Cargo.toml"));
        assert_eq!(result.directory_count, 1);
        assert_eq!(result.file_count, 3);
    }

    #[test]
    fn test_render_ascii() {
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
    fn test_render_no_indent() {
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
    fn test_render_with_size() {
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
    fn test_render_human_readable_size() {
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
    fn test_render_with_report() {
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
    fn test_render_directories_only() {
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

    #[test]
    fn test_render_tree_only() {
        let tree = create_test_tree();

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.scan.show_files = true;

        let result = render_tree_only(&tree, &config);

        assert!(result.contains("test_root"));
        assert!(result.contains("src"));
        assert!(result.contains("main.rs"));
    }

    #[test]
    fn test_render_result_struct() {
        let result = RenderResult {
            content: "test".to_string(),
            directory_count: 5,
            file_count: 10,
        };
        assert_eq!(result.content, "test");
        assert_eq!(result.directory_count, 5);
        assert_eq!(result.file_count, 10);
    }
}