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
//! - **流式渲染**：`StreamRenderer` 支持边扫描边渲染
//!
//! 文件: src/render.rs
//! 作者: WaterRun
//! 更新于: 2026-01-08

#![forbid(unsafe_code)]

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use crate::config::{CharsetMode, Config, PathMode};
use crate::error::RenderError;
use crate::scan::{EntryKind, EntryMetadata, ScanStats, StreamEntry, TreeNode};

// ============================================================================
// 常量
// ============================================================================

/// Windows tree++ 样板信息文件名
const TREEPP_BANNER_FILE: &str = "tree++.txt";

/// 样板文件内容提示
const TREEPP_BANNER_FILE_CONTENT: &str = r#"This directory is automatically created by tree++ to align with the native Windows tree command's banner (boilerplate) output.

You may safely delete this directory. If you do not want tree++ to create it, use the /NB option when running tree++.

GitHub: https://github.com/Water-Run/treepp
"#;

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
    /// 获取指定盘符的 Windows 样板信息
    ///
    /// # 参数
    ///
    /// * `drive` - 盘符（如 'C', 'D'）
    ///
    /// # Errors
    ///
    /// 返回 `RenderError::BannerFetchFailed` 如果：
    /// - 无法创建样板目录
    /// - 无法执行 tree 命令
    /// - tree 输出解析失败
    pub fn fetch_for_drive(drive: char) -> Result<Self, RenderError> {
        let drive = drive.to_ascii_uppercase();
        let banner_dir = format!(r"{}:\__tree++__", drive);
        let dir_path = Path::new(&banner_dir);
        let file_path = dir_path.join(TREEPP_BANNER_FILE);

        // 确保目录存在
        if !dir_path.exists() {
            fs::create_dir_all(dir_path).map_err(|e| RenderError::BannerFetchFailed {
                reason: format!("无法创建目录 {}: {}", banner_dir, e),
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

        // 在样板目录下执行 tree 命令（无参数）
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
    /// `__tree++__` 目录由程序创建，只包含一个 `tree++.txt` 文件，
    /// 因此在该目录执行 `tree` 命令的输出是固定的 4 行格式：
    ///
    /// ```text
    /// 卷 系统 的文件夹 PATH 列表      <- 第1行: volume_line
    /// 卷序列号为 2810-11C7            <- 第2行: serial_line
    /// X:.                             <- 第3行: 当前目录（忽略）
    /// 没有子文件夹                    <- 第4行: no_subfolder
    /// ```
    fn parse_tree_output(output: &str) -> Result<Self, RenderError> {
        let lines: Vec<&str> = output.lines().collect();

        // tree 输出固定为 4 行
        if lines.len() < 4 {
            return Err(RenderError::BannerFetchFailed {
                reason: format!(
                    "tree 输出行数不足，期望 4 行，实际 {} 行:\n{}",
                    lines.len(),
                    output
                ),
            });
        }

        // 第1行: 卷信息
        let volume_line = lines[0].trim().to_string();
        // 第2行: 序列号
        let serial_line = lines[1].trim().to_string();
        // 第4行: 无子文件夹提示（固定位置）
        let no_subfolder = lines[3].trim().to_string();

        Ok(Self {
            volume_line,
            serial_line,
            no_subfolder,
        })
    }

    /// 从字符串解析样板信息（用于测试）
    #[cfg(test)]
    pub fn parse(output: &str) -> Result<Self, RenderError> {
        Self::parse_tree_output(output)
    }
}

// ============================================================================
// 根路径格式化
// ============================================================================

/// 从规范化路径中提取盘符
///
/// # Errors
///
/// 如果无法从路径中提取盘符，返回 `RenderError::InvalidPath`
fn extract_drive_letter(root_path: &Path) -> Result<char, RenderError> {
    use std::path::Component;

    if let Some(Component::Prefix(prefix)) = root_path.components().next() {
        let prefix_str = prefix.as_os_str().to_string_lossy();
        let chars: Vec<char> = prefix_str.chars().collect();

        // 普通格式 "C:"
        if chars.len() >= 2 && chars[1] == ':' {
            return Ok(chars[0].to_ascii_uppercase());
        }

        // 长路径格式 "\\?\C:"
        if prefix_str.starts_with(r"\\?\") && chars.len() >= 6 && chars[5] == ':' {
            return Ok(chars[4].to_ascii_uppercase());
        }
    }

    Err(RenderError::InvalidPath {
        path: root_path.to_path_buf(),
        reason: "Unable to extract drive letter".to_string(),
    })
}

/// 格式化根路径以匹配 Windows tree 命令的显示风格
///
/// - 当用户未显式指定路径时，显示为 `D:.` 格式
/// - 当用户显式指定路径时，显示为完整大写路径
///
/// # Errors
///
/// 如果未显式指定路径且无法提取盘符，返回 `RenderError::InvalidPath`
pub fn format_root_path_display(
    root_path: &Path,
    path_explicitly_set: bool,
) -> Result<String, RenderError> {
    if path_explicitly_set {
        // 显式指定路径：显示完整大写路径
        Ok(root_path.to_string_lossy().to_uppercase())
    } else {
        // 未显式指定路径：显示为 "X:." 格式
        let drive = extract_drive_letter(root_path)?;
        Ok(format!("{}:.", drive))
    }
}

/// 检查树中是否有子目录（不含根节点自身）
///
/// 用于判断是否显示"没有子文件夹"提示。
/// 只检查直接子节点中是否有目录，不考虑文件。
#[must_use]
fn tree_has_subdirectories(node: &TreeNode) -> bool {
    node.children
        .iter()
        .any(|child| child.kind == EntryKind::Directory)
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
#[derive(Debug, Clone, Copy)]
pub struct TreeChars {
    /// 分支连接符 (├─ 或 +---)
    pub branch: &'static str,
    /// 最后分支连接符 (└─ 或 \---)
    pub last_branch: &'static str,
    /// 垂直连接符后续行 (│   或 |   )
    pub vertical: &'static str,
    /// 空白占位符（用于最后一个分支后的子项）
    pub space: &'static str,
}

impl TreeChars {
    /// 根据字符集模式创建连接符集合
    ///
    /// 格式与 Windows tree 命令保持一致：
    /// - Unicode: ├─, └─, │   (│后3空格), 4空格
    /// - ASCII: +---, \---, |   (|后3空格), 4空格
    #[must_use]
    pub fn from_charset(charset: CharsetMode) -> Self {
        match charset {
            CharsetMode::Unicode => Self {
                branch: "├─",
                last_branch: "└─",
                vertical: "│   ",
                space: "    ",
            },
            CharsetMode::Ascii => Self {
                branch: "+---",
                last_branch: "\\---",
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

/// 格式化 SystemTime 为本地时间日期字符串
///
/// 将 UTC 时间转换为本地时区时间并格式化输出。
///
/// # Examples
///
/// ```
/// use std::time::SystemTime;
/// use treepp::render::format_datetime;
///
/// let now = SystemTime::now();
/// let formatted = format_datetime(&now);
/// // 输出格式: "2025-01-06 15:30:45"
/// assert!(formatted.len() == 19);
/// ```
#[must_use]
pub fn format_datetime(time: &SystemTime) -> String {
    use chrono::{DateTime, Local};

    let datetime: DateTime<Local> = (*time).into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 格式化条目名称
fn format_entry_name(node: &TreeNode, config: &Config) -> String {
    let name = match config.render.path_mode {
        PathMode::Full => node.path.to_string_lossy().into_owned(),
        PathMode::Relative => node.name.clone(),
    };

    if config.render.quote_names {
        format!("\"{}\"", name)
    } else {
        name
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
    if config.render.show_disk_usage && node.kind == EntryKind::Directory
        && let Some(usage) = node.disk_usage {
            let usage_str = if config.render.human_readable {
                format_size_human(usage)
            } else {
                usage.to_string()
            };
            parts.push(usage_str);
        }

    // 修改日期
    if config.render.show_date
        && let Some(ref modified) = node.metadata.modified {
            parts.push(format_datetime(modified));
        }

    if parts.is_empty() {
        String::new()
    } else {
        format!("        {}", parts.join("  "))
    }
}

// ============================================================================
// 流式渲染器
// ============================================================================

/// 流式渲染配置（从 Config 提取渲染相关配置）
#[derive(Debug, Clone)]
pub struct StreamRenderConfig {
    /// 字符集模式
    pub charset: CharsetMode,
    /// 路径显示模式
    pub path_mode: PathMode,
    /// 是否显示文件大小
    pub show_size: bool,
    /// 是否以人类可读格式显示大小
    pub human_readable: bool,
    /// 是否显示最后修改日期
    pub show_date: bool,
    /// 是否显示目录累计大小（流式模式下不支持）
    pub show_disk_usage: bool,
    /// 是否用双引号包裹文件名
    pub quote_names: bool,
    /// 是否不显示树形连接线（仅缩进）
    pub no_indent: bool,
    /// 是否显示统计报告
    pub show_report: bool,
    /// 是否隐藏 Windows 样板信息
    pub no_win_banner: bool,
    /// 是否显示文件
    pub show_files: bool,
}

impl StreamRenderConfig {
    /// 从完整配置创建流式渲染配置
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            charset: config.render.charset,
            path_mode: config.render.path_mode,
            show_size: config.render.show_size,
            human_readable: config.render.human_readable,
            show_date: config.render.show_date,
            show_disk_usage: config.render.show_disk_usage,
            quote_names: config.render.quote_names,
            no_indent: config.render.no_indent,
            show_report: config.render.show_report,
            no_win_banner: config.render.no_win_banner,
            show_files: config.scan.show_files,
        }
    }
}

/// 流式渲染器
///
/// 管理树形前缀状态，支持逐条目渲染。
/// 严格遵循 Windows 原生 tree 的输出格式：
/// - 文件使用缩进（不使用分支符）
/// - 目录使用分支符（├─ 或 └─）
#[derive(Debug)]
pub struct StreamRenderer {
    /// 前缀栈：记录每层是否还有后续目录（true = 有更多目录）
    prefix_stack: Vec<bool>,
    /// 树形字符集
    chars: TreeChars,
    /// 渲染配置
    config: StreamRenderConfig,
    /// 上一个条目是否为文件（用于插入空行）
    last_was_file: bool,
    /// 当前深度是否有文件输出过
    depth_has_files: Vec<bool>,
}

impl StreamRenderer {
    /// 创建新的流式渲染器
    #[must_use]
    pub fn new(config: StreamRenderConfig) -> Self {
        let chars = TreeChars::from_charset(config.charset);
        Self {
            prefix_stack: Vec::new(),
            chars,
            config,
            last_was_file: false,
            depth_has_files: vec![false],
        }
    }

    /// 渲染 Banner 和根路径头部
    #[must_use]
    pub fn render_header(&self, root_path: &Path, path_explicitly_set: bool) -> String {
        let mut output = String::new();

        // 获取盘符用于 banner
        let drive = extract_drive_letter(root_path).ok();

        // Windows 样板头信息
        let banner = if self.config.no_win_banner {
            None
        } else if let Some(d) = drive {
            match WinBanner::fetch_for_drive(d) {
                Ok(b) => Some(b),
                Err(e) => {
                    let _ = writeln!(output, "Warning: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 输出样板头
        if let Some(b) = &banner {
            output.push_str(&b.volume_line);
            output.push('\n');
            output.push_str(&b.serial_line);
            output.push('\n');
        }

        // 根路径
        let root_display = match format_root_path_display(root_path, path_explicitly_set) {
            Ok(s) => s,
            Err(e) => {
                let _ = writeln!(output, "Warning: {}", e);
                root_path.to_string_lossy().to_uppercase()
            }
        };
        output.push_str(&root_display);
        output.push('\n');

        output
    }

    /// 渲染单个条目为一行文本
    ///
    /// 根据条目类型选择不同的渲染方式：
    /// - 文件：使用缩进前缀
    /// - 目录：使用分支连接符
    #[must_use]
    pub fn render_entry(&mut self, entry: &StreamEntry) -> String {
        if self.config.no_indent {
            return self.render_entry_no_indent(entry);
        }

        let mut output = String::new();

        // 检查是否需要插入空行（从文件切换到目录）
        if self.last_was_file && !entry.is_file {
            let prefix = self.build_prefix();
            let separator = self.chars.vertical.trim_end();
            let _ = writeln!(output, "{}{}", prefix, separator);
        }

        if entry.is_file {
            output.push_str(&self.render_file_entry(entry));
        } else {
            output.push_str(&self.render_dir_entry(entry));
        }

        self.last_was_file = entry.is_file;
        output
    }

    /// 渲染文件条目（使用缩进，不使用分支符）
    fn render_file_entry(&self, entry: &StreamEntry) -> String {
        let mut line = String::new();

        // 构建前缀
        let prefix = self.build_prefix();
        line.push_str(&prefix);

        // 文件使用竖线+空格或纯空格
        if entry.has_more_dirs {
            line.push_str(self.chars.vertical);
        } else {
            line.push_str(self.chars.space);
        }

        // 添加名称
        let name = self.format_name(&entry.name, &entry.path);
        line.push_str(&name);

        // 添加元信息
        let meta = self.format_meta(&entry.metadata, entry.kind);
        line.push_str(&meta);

        line
    }

    /// 渲染目录条目（使用分支符）
    fn render_dir_entry(&self, entry: &StreamEntry) -> String {
        let mut line = String::new();

        // 构建前缀
        let prefix = self.build_prefix();
        line.push_str(&prefix);

        // 目录使用分支连接符
        let connector = if entry.is_last {
            self.chars.last_branch
        } else {
            self.chars.branch
        };
        line.push_str(connector);

        // 添加名称
        let name = self.format_name(&entry.name, &entry.path);
        line.push_str(&name);

        // 添加元信息
        let meta = self.format_meta(&entry.metadata, entry.kind);
        line.push_str(&meta);

        line
    }

    /// 渲染无树形连接符的条目（仅缩进）
    fn render_entry_no_indent(&mut self, entry: &StreamEntry) -> String {
        let mut line = String::new();

        // 缩进
        let indent = "  ".repeat(entry.depth);
        line.push_str(&indent);

        // 名称
        let name = self.format_name(&entry.name, &entry.path);
        line.push_str(&name);

        // 元信息
        let meta = self.format_meta(&entry.metadata, entry.kind);
        line.push_str(&meta);

        self.last_was_file = entry.is_file;
        line
    }

    /// 构建当前前缀字符串
    fn build_prefix(&self) -> String {
        let mut prefix = String::new();
        for &has_more in &self.prefix_stack {
            if has_more {
                prefix.push_str(self.chars.vertical);
            } else {
                prefix.push_str(self.chars.space);
            }
        }
        prefix
    }

    /// 格式化名称
    fn format_name(&self, name: &str, path: &Path) -> String {
        let display_name = match self.config.path_mode {
            PathMode::Full => path.to_string_lossy().into_owned(),
            PathMode::Relative => name.to_string(),
        };

        if self.config.quote_names {
            format!("\"{}\"", display_name)
        } else {
            display_name
        }
    }

    /// 格式化元信息
    fn format_meta(&self, metadata: &EntryMetadata, kind: EntryKind) -> String {
        let mut parts = Vec::new();

        // 文件大小
        if self.config.show_size && kind == EntryKind::File {
            let size_str = if self.config.human_readable {
                format_size_human(metadata.size)
            } else {
                metadata.size.to_string()
            };
            parts.push(size_str);
        }

        // 修改日期
        if self.config.show_date {
            if let Some(ref modified) = metadata.modified {
                parts.push(format_datetime(modified));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("        {}", parts.join("  "))
        }
    }

    /// 进入子目录前调用（更新前缀栈）
    ///
    /// # Arguments
    ///
    /// * `has_more_siblings` - 当前目录是否还有后续兄弟目录
    pub fn push_level(&mut self, has_more_siblings: bool) {
        self.prefix_stack.push(has_more_siblings);
        self.last_was_file = false;
        self.depth_has_files.push(false);
    }

    /// 离开子目录后调用（恢复前缀栈）
    pub fn pop_level(&mut self) {
        self.prefix_stack.pop();
        self.depth_has_files.pop();
        self.last_was_file = false;
    }

    /// 渲染统计报告
    #[must_use]
    pub fn render_report(
        &self,
        directory_count: usize,
        file_count: usize,
        duration: Duration,
    ) -> String {
        let mut output = String::new();

        if self.config.show_report {
            let time_str = format!(" in {:.3}s", duration.as_secs_f64());

            output.push('\n');
            if self.config.show_files {
                let _ = writeln!(
                    output,
                    "{} directory, {} files{}",
                    directory_count, file_count, time_str
                );
            } else {
                let _ = writeln!(output, "{} directory{}", directory_count, time_str);
            }
        }

        output
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

    // 获取盘符用于 banner
    let drive = extract_drive_letter(&config.root_path).ok();

    // Windows 样板头信息
    let banner = if config.render.no_win_banner {
        None
    } else if let Some(d) = drive {
        match WinBanner::fetch_for_drive(d) {
            Ok(b) => Some(b),
            Err(e) => {
                let _ = writeln!(output, "Warning: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 输出样板头
    if let Some(b) = &banner {
        output.push_str(&b.volume_line);
        output.push('\n');
        output.push_str(&b.serial_line);
        output.push('\n');
    }

    // 根路径（使用新的格式化函数）
    let root_display = match format_root_path_display(&config.root_path, config.path_explicitly_set)
    {
        Ok(s) => s,
        Err(e) => {
            // 格式化失败时回退到原始显示并输出警告
            let _ = writeln!(output, "Warning: {}", e);
            config.root_path.to_string_lossy().to_uppercase()
        }
    };
    output.push_str(&root_display);
    output.push('\n');

    // 渲染子节点
    if config.render.no_indent {
        render_children_no_indent(&mut output, &stats.tree, config, 0);
    } else {
        render_children(&mut output, &stats.tree, &chars, config, "");
    }

    // 无子目录提示（当目录没有子目录时显示，不考虑文件）
    if !tree_has_subdirectories(&stats.tree)
        && let Some(b) = &banner
            && !b.no_subfolder.is_empty() {
                output.push('\n');
                output.push_str(&b.no_subfolder);
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
///
/// 渲染规则：
/// - 文件：使用缩进（│   或空格），不使用分支符
/// - 目录：使用分支符（├─ 或 └─）
/// - 根据 dirs_first 配置决定渲染顺序
fn render_children(
    output: &mut String,
    node: &TreeNode,
    chars: &TreeChars,
    config: &Config,
    prefix: &str,
) {
    // 分离文件和目录（保持排序后的相对顺序）
    let (files, dirs): (Vec<_>, Vec<_>) = get_filtered_children(node, config)
        .into_iter()
        .partition(|c| c.kind == EntryKind::File);

    let has_files = !files.is_empty();
    let has_dirs = !dirs.is_empty();

    if config.render.dirs_first {
        // 目录优先模式：先目录后文件

        // 渲染目录
        let dir_count = dirs.len();
        for (i, dir) in dirs.iter().enumerate() {
            // 如果后面还有文件，目录都不是最后一个
            let is_last = i == dir_count - 1 && !has_files;
            let connector = if is_last {
                chars.last_branch
            } else {
                chars.branch
            };

            let name = format_entry_name(dir, config);
            let meta = format_entry_meta(dir, config);
            let _ = writeln!(output, "{}{}{}{}", prefix, connector, name, meta);

            // 递归渲染子目录
            if !dir.children.is_empty() {
                let new_prefix = if is_last {
                    format!("{}{}", prefix, chars.space)
                } else {
                    format!("{}{}", prefix, chars.vertical)
                };
                render_children(output, dir, chars, config, &new_prefix);
            }
        }

        // 目录和文件之间的空行
        if has_dirs && has_files {
            let _ = writeln!(output, "{}{}", prefix, chars.vertical.trim_end());
        }

        // 渲染文件（文件在最后，没有后续目录）
        for file in &files {
            let file_prefix = format!("{}{}", prefix, chars.space);

            let name = format_entry_name(file, config);
            let meta = format_entry_meta(file, config);
            let _ = writeln!(output, "{}{}{}", file_prefix, name, meta);
        }
    } else {
        // 文件优先模式（默认，与原生 Windows tree 一致）

        // 渲染文件（使用缩进，不使用分支符）
        for file in &files {
            let file_prefix = if has_dirs {
                format!("{}{}", prefix, chars.vertical)
            } else {
                format!("{}{}", prefix, chars.space)
            };

            let name = format_entry_name(file, config);
            let meta = format_entry_meta(file, config);
            let _ = writeln!(output, "{}{}{}", file_prefix, name, meta);
        }

        // 文件和目录之间的空行
        if has_files && has_dirs {
            let _ = writeln!(output, "{}{}", prefix, chars.vertical.trim_end());
        }

        // 渲染目录（使用分支符）
        let dir_count = dirs.len();
        for (i, dir) in dirs.iter().enumerate() {
            let is_last = i == dir_count - 1;
            let connector = if is_last {
                chars.last_branch
            } else {
                chars.branch
            };

            let name = format_entry_name(dir, config);
            let meta = format_entry_meta(dir, config);
            let _ = writeln!(output, "{}{}{}{}", prefix, connector, name, meta);

            // 递归渲染子目录
            if !dir.children.is_empty() {
                let new_prefix = if is_last {
                    format!("{}{}", prefix, chars.space)
                } else {
                    format!("{}{}", prefix, chars.vertical)
                };
                render_children(output, dir, chars, config, &new_prefix);
            }
        }
    }
}

/// 渲染子节点 (无树形连接符，仅缩进)
fn render_children_no_indent(output: &mut String, node: &TreeNode, config: &Config, depth: usize) {
    // 分离文件和目录
    let (files, dirs): (Vec<_>, Vec<_>) = get_filtered_children(node, config)
        .into_iter()
        .partition(|c| c.kind == EntryKind::File);

    let indent = "  ".repeat(depth);

    if config.render.dirs_first {
        // 目录优先：先渲染目录
        for dir in &dirs {
            let name = format_entry_name(dir, config);
            let meta = format_entry_meta(dir, config);
            let _ = writeln!(output, "{}{}{}", indent, name, meta);

            if !dir.children.is_empty() {
                render_children_no_indent(output, dir, config, depth + 1);
            }
        }

        // 再渲染文件
        for file in &files {
            let name = format_entry_name(file, config);
            let meta = format_entry_meta(file, config);
            let _ = writeln!(output, "{}{}{}", indent, name, meta);
        }
    } else {
        // 文件优先（默认）：先渲染文件
        for file in &files {
            let name = format_entry_name(file, config);
            let meta = format_entry_meta(file, config);
            let _ = writeln!(output, "{}{}{}", indent, name, meta);
        }

        // 再渲染目录
        for dir in &dirs {
            let name = format_entry_name(dir, config);
            let meta = format_entry_meta(dir, config);
            let _ = writeln!(output, "{}{}{}", indent, name, meta);

            if !dir.children.is_empty() {
                render_children_no_indent(output, dir, config, depth + 1);
            }
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
    use crate::config::SortKey;
    use crate::scan::sort_tree;

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
    fn test_win_banner_parse_with_trailing_empty_lines() {
        // tree 命令输出末尾可能有空行，但前 4 行是固定的
        let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n没有子文件夹\n\n";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹");
    }

    #[test]
    fn test_win_banner_parse_with_trailing_whitespace() {
        let output = "卷 系统 的文件夹 PATH 列表  \n  卷序列号为 2810-11C7\nC:.\n没有子文件夹  \n";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹");
    }

    #[test]
    fn test_win_banner_parse_english_locale() {
        let output = "Folder PATH listing for volume OS\nVolume serial number is ABCD-1234\nC:.\nNo subfolders exist";
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
        // 即使有超过 4 行，也只取前 4 行的固定位置
        let output = "line1\nline2\nline3\nline4\nline5\nline6";
        let banner = WinBanner::parse(output).expect("多于4行也应解析成功");

        assert_eq!(banner.volume_line, "line1");
        assert_eq!(banner.serial_line, "line2");
        assert_eq!(banner.no_subfolder, "line4");
    }

    #[test]
    fn test_win_banner_empty_input() {
        let output = "";
        let result = WinBanner::parse(output);
        assert!(result.is_err());
    }

    #[test]
    fn test_win_banner_parse_different_drives() {
        // 验证不同盘符的输出格式相同
        let output_c = "卷 系统 的文件夹 PATH 列表\n卷序列号为 1234-5678\nC:.\n没有子文件夹";
        let output_d = "卷 数据 的文件夹 PATH 列表\n卷序列号为 ABCD-EF01\nD:.\n没有子文件夹";

        let banner_c = WinBanner::parse(output_c).expect("C盘解析应成功");
        let banner_d = WinBanner::parse(output_d).expect("D盘解析应成功");

        assert_eq!(banner_c.no_subfolder, "没有子文件夹");
        assert_eq!(banner_d.no_subfolder, "没有子文件夹");
        // 卷名和序列号会不同
        assert_ne!(banner_c.volume_line, banner_d.volume_line);
        assert_ne!(banner_c.serial_line, banner_d.serial_line);
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
    fn test_format_datetime_format() {
        use std::time::SystemTime;

        let now = SystemTime::now();
        let formatted = format_datetime(&now);

        // 验证格式: YYYY-MM-DD HH:MM:SS (19 字符)
        assert_eq!(formatted.len(), 19);
        assert_eq!(&formatted[4..5], "-");
        assert_eq!(&formatted[7..8], "-");
        assert_eq!(&formatted[10..11], " ");
        assert_eq!(&formatted[13..14], ":");
        assert_eq!(&formatted[16..17], ":");
    }

    #[test]
    fn test_format_datetime_returns_local_time() {
        use chrono::Local;
        use std::time::SystemTime;

        let now = SystemTime::now();
        let formatted = format_datetime(&now);

        // 获取当前本地时间进行比较
        let local_now = Local::now();
        let expected_date = local_now.format("%Y-%m-%d").to_string();

        // 日期部分应该匹配本地时间
        assert!(
            formatted.starts_with(&expected_date),
            "格式化时间 {} 应以本地日期 {} 开头",
            formatted,
            expected_date
        );
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
        assert_eq!(chars.space, "    ");
    }

    #[test]
    fn test_tree_chars_ascii() {
        let chars = TreeChars::from_charset(CharsetMode::Ascii);
        assert_eq!(chars.branch, "+---");
        assert_eq!(chars.last_branch, "\\---");
        assert_eq!(chars.vertical, "|   ");
        assert_eq!(chars.space, "    ");
    }

    // ========================================================================
    // StreamRenderer 测试
    // ========================================================================

    #[test]
    fn test_stream_renderer_new() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let renderer = StreamRenderer::new(render_config);

        assert!(renderer.prefix_stack.is_empty());
    }

    #[test]
    fn test_stream_renderer_render_entry_basic() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("test.txt"));
        assert!(line.contains("└─"));
    }

    #[test]
    fn test_stream_renderer_render_entry_not_last() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: false,
            is_file: false,
            has_more_dirs: true,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("├─"));
    }

    #[test]
    fn test_stream_renderer_render_entry_ascii() {
        let mut config = Config::default();
        config.render.charset = CharsetMode::Ascii;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("\\---"));
    }

    #[test]
    fn test_stream_renderer_render_entry_no_indent() {
        let mut config = Config::default();
        config.render.no_indent = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 2,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(!line.contains("├"));
        assert!(!line.contains("└"));
        assert!(line.starts_with("    "));
    }

    #[test]
    fn test_stream_renderer_render_entry_with_size() {
        let mut config = Config::default();
        config.render.show_size = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata {
                size: 1024,
                ..Default::default()
            },
            depth: 0,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("1024"));
    }

    #[test]
    fn test_stream_renderer_render_entry_with_human_readable_size() {
        let mut config = Config::default();
        config.render.show_size = true;
        config.render.human_readable = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata {
                size: 1024,
                ..Default::default()
            },
            depth: 0,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("1.0 KB"));
    }

    #[test]
    fn test_stream_renderer_render_entry_with_quote() {
        let mut config = Config::default();
        config.render.quote_names = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("\"test.txt\""));
    }

    #[test]
    fn test_stream_renderer_push_pop_level() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        assert!(renderer.prefix_stack.is_empty());

        renderer.push_level(true);
        assert_eq!(renderer.prefix_stack.len(), 1);
        assert_eq!(renderer.prefix_stack[0], true);

        renderer.push_level(false);
        assert_eq!(renderer.prefix_stack.len(), 2);
        assert_eq!(renderer.prefix_stack[1], false);

        renderer.pop_level();
        assert_eq!(renderer.prefix_stack.len(), 1);

        renderer.pop_level();
        assert!(renderer.prefix_stack.is_empty());
    }

    #[test]
    fn test_stream_renderer_prefix_with_siblings() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);

        let entry = StreamEntry {
            path: PathBuf::from("nested.txt"),
            name: "nested.txt".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(line.contains("│"));
    }

    #[test]
    fn test_stream_renderer_prefix_without_siblings() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(false);

        let entry = StreamEntry {
            path: PathBuf::from("nested.txt"),
            name: "nested.txt".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };

        let line = renderer.render_entry(&entry);
        assert!(!line.contains("│"));
        assert!(line.starts_with("    "));
    }

    #[test]
    fn test_stream_renderer_render_report() {
        let mut config = Config::default();
        config.render.show_report = true;
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let renderer = StreamRenderer::new(render_config);

        let report = renderer.render_report(5, 20, Duration::from_millis(100));

        assert!(report.contains("5 directory"));
        assert!(report.contains("20 files"));
        assert!(report.contains("0.100s"));
    }

    #[test]
    fn test_stream_renderer_render_report_dirs_only() {
        let mut config = Config::default();
        config.render.show_report = true;
        config.scan.show_files = false;
        let render_config = StreamRenderConfig::from_config(&config);
        let renderer = StreamRenderer::new(render_config);

        let report = renderer.render_report(5, 0, Duration::from_millis(50));

        assert!(report.contains("5 directory"));
        assert!(!report.contains("files"));
    }

    #[test]
    fn test_stream_renderer_render_report_with_gitignore() {
        let mut config = Config::default();
        config.render.show_report = true;
        config.scan.respect_gitignore = true;
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let renderer = StreamRenderer::new(render_config);

        let report = renderer.render_report(5, 20, Duration::from_millis(100));

        // render_report 只负责统计信息，不负责 gitignore 提示
        // gitignore 提示是在主 render 函数中处理的
        assert!(report.contains("5 directory"));
        assert!(report.contains("20 files"));
    }

    #[test]
    fn test_stream_render_config_from_config() {
        let mut config = Config::default();
        config.render.charset = CharsetMode::Ascii;
        config.render.show_size = true;
        config.render.human_readable = true;
        config.render.quote_names = true;
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let render_config = StreamRenderConfig::from_config(&config);

        assert_eq!(render_config.charset, CharsetMode::Ascii);
        assert!(render_config.show_size);
        assert!(render_config.human_readable);
        assert!(render_config.quote_names);
        assert!(render_config.show_files);
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
        // 禁用路径显式设置标志，避免大写转换问题
        config.path_explicitly_set = false;

        let result = render(&stats, &config);

        // 由于 test_root 不是有效 Windows 路径，会触发错误回退
        // 检查内容中包含相关信息即可
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

    #[test]
    fn test_format_entry_name_with_quote() {
        let node = TreeNode::new(
            PathBuf::from("test.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        );

        let mut config = Config::with_root(PathBuf::from("."));
        config.render.quote_names = false;

        let name = format_entry_name(&node, &config);
        assert_eq!(name, "test.txt");

        config.render.quote_names = true;
        let name = format_entry_name(&node, &config);
        assert_eq!(name, "\"test.txt\"");
    }

    #[test]
    fn test_format_entry_name_full_path_with_quote() {
        let node = TreeNode::new(
            PathBuf::from("/path/to/test.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        );

        let mut config = Config::with_root(PathBuf::from("."));
        config.render.path_mode = PathMode::Full;
        config.render.quote_names = true;

        let name = format_entry_name(&node, &config);
        assert!(name.starts_with('"'));
        assert!(name.ends_with('"'));
        assert!(name.contains("test.txt"));
    }

    #[test]
    fn test_render_with_quote() {
        let tree = create_test_tree();
        let stats = create_test_stats(tree);

        let mut config = Config::with_root(PathBuf::from("test_root"));
        config.render.no_win_banner = true;
        config.render.quote_names = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(result.content.contains("\"src\""));
        assert!(result.content.contains("\"main.rs\""));
    }

    #[test]
    fn test_render_with_dirs_first() {
        use crate::config::SortKey;
        use crate::scan::sort_tree;

        let mut tree = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        tree.children.push(TreeNode::new(
            PathBuf::from("root/z_file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        tree.children.push(TreeNode::new(
            PathBuf::from("root/a_dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        // 关键：使用 dirs_first=true 进行排序
        sort_tree(&mut tree, SortKey::Name, false, true);

        let stats = ScanStats {
            tree,
            duration: Duration::from_millis(100),
            directory_count: 1,
            file_count: 1,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.dirs_first = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        // 目录应该在文件之前
        let a_dir_pos = result.content.find("a_dir").unwrap();
        let z_file_pos = result.content.find("z_file.txt").unwrap();
        assert!(a_dir_pos < z_file_pos, "目录应该在文件之前");
    }

    // ========================================================================
    // format_root_path_display 测试
    // ========================================================================

    #[test]
    fn test_format_root_path_display_explicitly_set() {
        let path = Path::new(r"D:\Users\Test\Project");
        let result = format_root_path_display(path, true).unwrap();
        assert_eq!(result, r"D:\USERS\TEST\PROJECT");
    }

    #[test]
    fn test_format_root_path_display_not_explicitly_set() {
        let path = Path::new(r"C:\Some\Path");
        let result = format_root_path_display(path, false).unwrap();
        assert_eq!(result, "C:.");
    }

    #[test]
    fn test_format_root_path_display_lowercase_drive() {
        let path = Path::new(r"d:\test");
        let result = format_root_path_display(path, false).unwrap();
        assert_eq!(result, "D:.");
    }

    #[test]
    fn test_format_root_path_display_explicitly_set_uppercase() {
        let path = Path::new(r"c:\Users\Test");
        let result = format_root_path_display(path, true).unwrap();
        assert_eq!(result, r"C:\USERS\TEST");
    }

    #[test]
    fn test_extract_drive_letter_normal_path() {
        let path = Path::new(r"C:\Windows");
        let drive = extract_drive_letter(path).unwrap();
        assert_eq!(drive, 'C');
    }

    #[test]
    fn test_extract_drive_letter_lowercase() {
        let path = Path::new(r"d:\data");
        let drive = extract_drive_letter(path).unwrap();
        assert_eq!(drive, 'D');
    }

    #[test]
    fn test_extract_drive_letter_relative_path_fails() {
        let path = Path::new("relative/path");
        let result = extract_drive_letter(path);
        assert!(result.is_err());
    }

    // ========================================================================
    // tree_has_subdirectories 测试
    // ========================================================================

    #[test]
    fn test_tree_has_subdirectories_with_subdir() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        assert!(tree_has_subdirectories(&root));
    }

    #[test]
    fn test_tree_has_subdirectories_only_files() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        assert!(!tree_has_subdirectories(&root));
    }

    #[test]
    fn test_tree_has_subdirectories_empty() {
        let root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        assert!(!tree_has_subdirectories(&root));
    }

    #[test]
    fn test_tree_has_subdirectories_mixed() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        assert!(tree_has_subdirectories(&root));
    }

    #[test]
    fn test_render_files_before_dirs() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/z_dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/a_file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        // 先排序
        sort_tree(&mut root, SortKey::Name, false, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 1,
            file_count: 1,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        // 文件应该在目录之前
        let file_pos = result.content.find("a_file.txt").unwrap();
        let dir_pos = result.content.find("z_dir").unwrap();
        assert!(file_pos < dir_pos, "文件应该在目录之前");
    }

    #[test]
    fn test_render_file_uses_indent_not_branch() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        sort_tree(&mut root, SortKey::Name, false, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 1,
            file_count: 1,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        // 文件行不应该有分支符
        let lines: Vec<&str> = result.content.lines().collect();
        let file_line = lines.iter().find(|l| l.contains("file.txt")).unwrap();
        assert!(!file_line.contains("├"), "文件不应该使用├");
        assert!(!file_line.contains("└"), "文件不应该使用└");

        // 目录行应该有分支符
        let dir_line = lines.iter().find(|l| l.contains("dir")).unwrap();
        assert!(
            dir_line.contains("├") || dir_line.contains("└"),
            "目录应该使用分支符"
        );
    }
}
