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
//! 更新于: 2025-01-05

#![forbid(unsafe_code)]

use std::fmt::Write as FmtWrite;
use std::path::Path;
use std::time::SystemTime;

#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::process::Command;

use crate::config::{CharsetMode, Config, PathMode};
use crate::scan::{EntryKind, ScanStats, TreeNode};

// ============================================================================
// 常量
// ============================================================================

/// Windows tree++ 样板信息目录路径
#[cfg(windows)]
const TREEPP_BANNER_DIR: &str = r"C:\__treepp__";

/// Windows tree++ 样板信息文件名
#[cfg(windows)]
const TREEPP_BANNER_FILE: &str = "tree++.txt";

/// 样板文件内容提示
#[cfg(windows)]
const TREEPP_BANNER_FILE_CONTENT: &str =
    "This file is used by tree++ to obtain Windows tree command banner information.";

/// 默认卷名 (回退值)
const DEFAULT_VOLUME_NAME: &str = "Local Disk";

/// 默认卷序列号 (回退值)
const DEFAULT_VOLUME_SERIAL: &str = "0000-0000";

/// 默认无子文件夹提示 (回退值)
const DEFAULT_NO_SUBFOLDER: &str = "No subfolders exist";

// ============================================================================
// Windows 样板信息
// ============================================================================

/// Windows tree 命令样板信息
///
/// 包含从 Windows 原生 tree 命令提取的样板信息。
///
/// # Examples
///
/// ```
/// use treepp::render::WinBanner;
///
/// let banner = WinBanner::default();
/// assert!(!banner.volume_line.is_empty());
/// assert!(!banner.serial_line.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct WinBanner {
    /// 卷信息行 (如 "卷 系统 的文件夹 PATH 列表")
    pub volume_line: String,
    /// 卷序列号行 (如 "卷序列号为 2810-11C7")
    pub serial_line: String,
    /// 无子文件夹提示 (如 "没有子文件夹")
    pub no_subfolder: String,
}

impl Default for WinBanner {
    fn default() -> Self {
        Self {
            volume_line: format!("卷 {} 的文件夹 PATH 列表", DEFAULT_VOLUME_NAME),
            serial_line: format!("卷序列号为 {}", DEFAULT_VOLUME_SERIAL),
            no_subfolder: DEFAULT_NO_SUBFOLDER.to_string(),
        }
    }
}

impl WinBanner {
    /// 从 Windows tree 命令获取样板信息
    ///
    /// 此方法会执行以下操作：
    /// 1. 确保 `C:\__treepp__` 目录存在
    /// 2. 确保 `tree++.txt` 文件存在
    /// 3. 执行 `tree /f` 命令获取输出
    /// 4. 解析输出提取样板信息
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::render::WinBanner;
    ///
    /// let banner = WinBanner::from_system();
    /// println!("Volume: {}", banner.volume_line);
    /// println!("Serial: {}", banner.serial_line);
    /// ```
    #[cfg(windows)]
    #[must_use]
    pub fn from_system() -> Self {
        Self::fetch_from_tree_command().unwrap_or_default()
    }

    /// 非 Windows 平台的回退实现
    #[cfg(not(windows))]
    #[must_use]
    pub fn from_system() -> Self {
        Self::default()
    }

    /// 从 tree 命令获取样板信息的内部实现
    #[cfg(windows)]
    fn fetch_from_tree_command() -> std::io::Result<Self> {
        let dir_path = Path::new(TREEPP_BANNER_DIR);
        let file_path = dir_path.join(TREEPP_BANNER_FILE);

        // 确保目录存在
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)?;
        }

        // 确保样板文件存在
        if !file_path.exists() {
            fs::write(&file_path, TREEPP_BANNER_FILE_CONTENT)?;
        }

        // 执行 tree /f 命令
        let output = Command::new("cmd")
            .args(["/C", "chcp 65001 >nul && tree /f"])
            .current_dir(dir_path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_tree_output(&stdout)
    }

    /// 解析 tree 命令输出
    fn parse_tree_output(output: &str) -> std::io::Result<Self> {
        let lines: Vec<&str> = output.lines().collect();

        if lines.len() < 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Tree output too short",
            ));
        }

        let volume_line = lines
            .iter()
            .find(|l| l.contains("卷") && l.contains("PATH"))
            .or_else(|| lines.iter().find(|l| l.contains("Volume") && l.contains("PATH")))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| format!("卷 {} 的文件夹 PATH 列表", DEFAULT_VOLUME_NAME));

        let serial_line = lines
            .iter()
            .find(|l| l.contains("卷序列号") || l.contains("Serial"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| format!("卷序列号为 {}", DEFAULT_VOLUME_SERIAL));

        let no_subfolder = lines
            .iter()
            .find(|l| l.contains("没有子文件夹") || l.contains("No subfolders"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| DEFAULT_NO_SUBFOLDER.to_string());

        Ok(Self {
            volume_line,
            serial_line,
            no_subfolder,
        })
    }

    /// 从字符串解析样板信息（用于测试）
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::render::WinBanner;
    ///
    /// let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n没有子文件夹";
    /// let banner = WinBanner::parse(output);
    /// assert!(banner.volume_line.contains("系统"));
    /// assert!(banner.serial_line.contains("2810-11C7"));
    /// ```
    #[must_use]
    pub fn parse(output: &str) -> Self {
        Self::parse_tree_output(output).unwrap_or_default()
    }
}

// ============================================================================
// 渲染结果
// ============================================================================

/// 渲染结果
///
/// 包含渲染后的文本内容和统计信息。
///
/// # Examples
///
/// ```
/// use treepp::render::RenderResult;
///
/// let result = RenderResult {
///     content: "tree output".to_string(),
///     directory_count: 5,
///     file_count: 10,
/// };
/// assert_eq!(result.directory_count, 5);
/// assert_eq!(result.file_count, 10);
/// ```
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
///
/// # Examples
///
/// ```
/// use treepp::render::format_size_human;
///
/// assert_eq!(format_size_human(0), "0 B");
/// assert_eq!(format_size_human(512), "512 B");
/// assert_eq!(format_size_human(1024), "1.0 KB");
/// assert_eq!(format_size_human(1536), "1.5 KB");
/// assert_eq!(format_size_human(1048576), "1.0 MB");
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

/// 格式化 SystemTime 为日期时间字符串
///
/// # Examples
///
/// ```
/// use std::time::SystemTime;
/// use treepp::render::format_datetime;
///
/// let now = SystemTime::now();
/// let formatted = format_datetime(&now);
/// // 格式: "YYYY-MM-DD HH:MM:SS"
/// assert!(formatted.len() >= 19);
/// ```
#[must_use]
pub fn format_datetime(time: &SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    // 简单的日期时间计算（不考虑时区，使用 UTC）
    // 更精确的实现应使用 chrono 等库，但为保持依赖最小化，这里使用简单实现
    let days = secs / 86400;
    let time_of_day = secs % 86400;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // 从 1970-01-01 计算日期
    let (year, month, day) = days_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// 将天数转换为年月日
fn days_to_ymd(days: u64) -> (i32, u32, u32) {
    // 简化的日期计算，从 1970-01-01 开始
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
///
/// 这是主要的渲染入口函数，根据配置将 `TreeNode` 渲染为文本树。
///
/// # 参数
///
/// * `stats` - 扫描统计结果，包含根节点和统计信息
/// * `config` - 渲染配置
///
/// # 返回值
///
/// 返回 `RenderResult`，包含渲染后的文本和统计信息。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::scan;
/// use treepp::render::render;
///
/// let config = Config::with_root(PathBuf::from(".")).validate().unwrap();
/// let stats = scan(&config).expect("扫描失败");
/// let result = render(&stats, &config);
/// println!("{}", result.content);
/// ```
#[must_use]
pub fn render(stats: &ScanStats, config: &Config) -> RenderResult {
    let mut output = String::new();
    let chars = TreeChars::from_charset(config.render.charset);

    // 获取根路径显示
    let root_path = config.root_path.to_string_lossy();

    // Windows 样板头信息
    if !config.render.no_win_banner {
        let banner = WinBanner::from_system();
        output.push_str(&banner.volume_line);
        output.push('\n');
        output.push_str(&banner.serial_line);
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

    // 无子目录提示（仅在启用 banner 且无子目录时显示）
    if !config.render.no_win_banner && stats.directory_count == 0 {
        let banner = WinBanner::from_system();
        output.push('\n');
        output.push_str(&banner.no_subfolder);
        output.push('\n');
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
///
/// 根据配置过滤子节点：
/// - 如果不显示文件，则只保留目录
/// - 如果启用 prune，则排除空目录
fn get_filtered_children<'a>(node: &'a TreeNode, config: &Config) -> Vec<&'a TreeNode> {
    node.children
        .iter()
        .filter(|c| config.scan.show_files || c.kind == EntryKind::Directory)
        .filter(|c| !config.matching.prune_empty || c.kind != EntryKind::Directory || !c.is_empty_dir())
        .collect()
}

/// 仅渲染树形文本，不包含 banner 和统计信息
///
/// 此函数用于需要纯净树形输出的场景。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::scan;
/// use treepp::render::render_tree_only;
///
/// let config = Config::with_root(PathBuf::from(".")).validate().unwrap();
/// let stats = scan(&config).expect("扫描失败");
/// let tree_text = render_tree_only(&stats.tree, &config);
/// println!("{}", tree_text);
/// ```
#[must_use]
pub fn render_tree_only(node: &TreeNode, config: &Config) -> String {
    let mut output = String::new();
    let chars = TreeChars::from_charset(config.render.charset);

    // 根节点名称
    let root_name = format_entry_name(node, config);
    let root_meta = format_entry_meta(node, config);
    let _ = writeln!(output, "{root_name}{root_meta}");

    // 渲染子节点
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

        // 添加子目录
        let mut src = TreeNode::new(
            PathBuf::from("test_root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        // 添加文件
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

        // 根目录文件
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

    /// 创建测试用的 ScanStats
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

    #[test]
    fn test_format_datetime() {
        use std::time::UNIX_EPOCH;

        // 测试 UNIX 纪元
        let epoch = UNIX_EPOCH;
        let formatted = format_datetime(&epoch);
        assert_eq!(formatted, "1970-01-01 00:00:00");
    }

    #[test]
    fn test_win_banner_default() {
        let banner = WinBanner::default();
        assert!(!banner.volume_line.is_empty());
        assert!(!banner.serial_line.is_empty());
        assert!(!banner.no_subfolder.is_empty());
    }

    #[test]
    fn test_win_banner_parse() {
        let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n没有子文件夹";
        let banner = WinBanner::parse(output);
        assert!(banner.volume_line.contains("系统"));
        assert!(banner.serial_line.contains("2810-11C7"));
    }

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

        // 无树形连接符
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

        // 检查是否包含大小信息
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

        // 检查是否包含人类可读大小
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

        // 不应包含文件
        assert!(!result.content.contains("main.rs"));
        assert!(!result.content.contains("Cargo.toml"));
        // 应包含目录
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

    #[test]
    fn test_days_to_ymd() {
        // 1970-01-01
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        // 1970-01-02
        assert_eq!(days_to_ymd(1), (1970, 1, 2));
        // 1970-02-01
        assert_eq!(days_to_ymd(31), (1970, 2, 1));
        // 1971-01-01
        assert_eq!(days_to_ymd(365), (1971, 1, 1));
    }

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(1970));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
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
