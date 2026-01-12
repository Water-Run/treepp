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
//! 更新于: 2026-01-12

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

        // 第1行: 卷信息（只 trim 左侧，保留右侧空格）
        let volume_line = lines[0].trim_start().to_string();
        // 第2行: 序列号（只 trim 左侧，保留右侧空格）
        let serial_line = lines[1].trim_start().to_string();
        // 第4行: 无子文件夹提示（只 trim 左侧，保留右侧空格）
        let no_subfolder = lines[3].trim_start().to_string();

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

#[inline]
fn depth_within_limit(depth: usize, max_depth: Option<usize>) -> bool {
    max_depth.map_or(true, |m| depth <= m)
}

#[inline]
fn can_recurse(depth: usize, max_depth: Option<usize>) -> bool {
    max_depth.map_or(true, |m| depth < m)
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
    #[must_use]
    pub fn from_charset(charset: CharsetMode) -> Self {
        match charset {
            CharsetMode::Unicode => Self {
                branch: "├─",
                last_branch: "└─",
                vertical: "│  ",
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
    if config.render.show_disk_usage
        && node.kind == EntryKind::Directory
        && let Some(usage) = node.disk_usage
    {
        let usage_str = if config.render.human_readable {
            format_size_human(usage)
        } else {
            usage.to_string()
        };
        parts.push(usage_str);
    }

    // 修改日期
    if config.render.show_date
        && let Some(ref modified) = node.metadata.modified
    {
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

/// 流式渲染配置
#[derive(Debug, Clone)]
pub struct StreamRenderConfig {
    /// 字符集
    pub charset: CharsetMode,
    /// 是否禁用缩进
    pub no_indent: bool,
    /// 是否禁用 Windows Banner
    pub no_win_banner: bool,
    /// 是否显示报告
    pub show_report: bool,
    /// 是否显示文件
    pub show_files: bool,
    /// 路径显示模式
    pub path_mode: PathMode,
    /// 是否显示文件大小
    pub show_size: bool,
    /// 是否使用人类可读的大小格式
    pub human_readable: bool,
    /// 是否显示修改日期
    pub show_date: bool,
}

impl StreamRenderConfig {
    /// 从 `Config` 创建 `StreamRenderConfig`
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            charset: config.render.charset,
            no_indent: config.render.no_indent,
            no_win_banner: config.render.no_win_banner,
            show_report: config.render.show_report,
            show_files: config.scan.show_files,
            path_mode: config.render.path_mode,
            show_size: config.render.show_size,
            human_readable: config.render.human_readable,
            show_date: config.render.show_date,
        }
    }
}

/// 流式渲染器
///
/// 管理树形前缀状态，支持逐条目渲染。
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
    /// 每层的状态：(文件前缀, 该层最后渲染的是否为文件)
    level_state_stack: Vec<(Option<String>, bool)>,
    /// 是否刚添加过尾随行（用于避免重复添加）
    trailing_line_emitted: bool,
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
            level_state_stack: Vec::new(),
            trailing_line_emitted: false,
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
    #[must_use]
    pub fn render_entry(&mut self, entry: &StreamEntry) -> String {
        // 记录文件前缀用于尾随行，并更新当前层的"最后是否为文件"状态
        if entry.is_file {
            let file_prefix = self.build_file_prefix(entry.has_more_dirs);
            if let Some(last) = self.level_state_stack.last_mut() {
                last.0 = Some(file_prefix);
                last.1 = true; // 最后渲染的是文件
            }
        } else {
            // 渲染目录时，更新状态：最后渲染的不是文件
            if let Some(last) = self.level_state_stack.last_mut() {
                last.1 = false;
            }
        }

        if self.config.no_indent {
            return self.render_entry_no_indent(entry);
        }

        let mut output = String::new();

        // 检查是否需要插入分隔行（从文件切换到目录）
        if self.config.show_files && self.last_was_file && !entry.is_file {
            let prefix = self.build_prefix();
            let _ = writeln!(output, "{}{}", prefix, self.chars.vertical);
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

        // 文件使用竖线+空格或纯空格（完整宽度）
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

    /// 构建当前文件的完整前缀（用于尾随行对齐）
    fn build_file_prefix(&self, has_more_dirs: bool) -> String {
        let mut prefix = self.build_prefix();
        if has_more_dirs {
            prefix.push_str(self.chars.vertical);
        } else {
            prefix.push_str(self.chars.space);
        }
        prefix
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
        match self.config.path_mode {
            PathMode::Full => path.to_string_lossy().into_owned(),
            PathMode::Relative => name.to_string(),
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

    /// 进入子目录前调用
    pub fn push_level(&mut self, has_more_siblings: bool) {
        self.prefix_stack.push(has_more_siblings);
        self.level_state_stack.push((None, false)); // (文件前缀, 最后是否为文件)
        self.last_was_file = false;
        self.trailing_line_emitted = false;
    }

    /// 离开子目录后调用（恢复前缀栈）
    ///
    /// 返回需要输出的尾随行（如果该目录最后渲染的是文件的话）
    #[must_use]
    pub fn pop_level(&mut self) -> Option<String> {
        let level_state = self.level_state_stack.pop();
        self.last_was_file = false;

        // 如果刚添加过尾随行，不再添加
        if self.trailing_line_emitted {
            let _ = self.prefix_stack.pop();
            return None;
        }

        // 只有当：1) 有文件前缀，且 2) 该层最后渲染的是文件，才输出尾随行
        let result = if let Some((file_prefix, last_was_file)) = level_state {
            if self.config.show_files && file_prefix.is_some() && last_was_file && !self.config.no_indent {
                self.trailing_line_emitted = true;
                file_prefix
            } else {
                None
            }
        } else {
            None
        };

        let _ = self.prefix_stack.pop();
        result
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

    /// 检查当前是否在根级别（没有进入任何子目录）
    #[must_use]
    pub fn is_at_root_level(&self) -> bool {
        self.prefix_stack.is_empty()
    }

    /// 检查根级别是否有内容被渲染
    #[must_use]
    pub fn root_has_content(&self) -> bool {
        self.last_was_file || !self.prefix_stack.is_empty()
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
        render_children_no_indent(&mut output, &stats.tree, config, 1);
    } else {
        let mut last_file_prefix: Option<String> = None;
        render_children(&mut output, &stats.tree, &chars, config, "", 1, &mut last_file_prefix);
    }

    // 无子目录提示（当目录没有子目录时显示，不考虑文件）
    if !tree_has_subdirectories(&stats.tree) {
        // 如果有文件但无目录，先输出占位行
        let has_files = stats
            .tree
            .children
            .iter()
            .any(|c| c.kind == EntryKind::File);
        if has_files && config.scan.show_files {
            // 输出与文件行同等缩进的占位行
            let _ = writeln!(output, "{}", chars.space);
        }

        if let Some(b) = &banner {
            if !b.no_subfolder.is_empty() {
                output.push_str(&b.no_subfolder);
                output.push('\n');
            }
        }
        // 只有在没有子目录时才输出末尾空行
        output.push('\n');
    }

    // 统计报告
    if config.render.show_report {
        let time_str = format!(" in {:.3}s", stats.duration.as_secs_f64());

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

    let output = remove_trailing_pipe_only_line(output);

    RenderResult {
        content: output,
        directory_count: stats.directory_count,
        file_count: stats.file_count,
    }
}

/// 如果最后一行仅包含竖线和空白字符（且至少有一个竖线），则移除该行
fn remove_trailing_pipe_only_line(mut output: String) -> String {
    // 移除末尾的换行符后查找最后一行
    let trimmed = output.trim_end_matches('\n');
    if let Some(last_newline_pos) = trimmed.rfind('\n') {
        let last_line = &trimmed[last_newline_pos + 1..];

        // 检查最后一行是否包含竖线
        let has_pipe = last_line.chars().any(|c| c == '|' || c == '│');

        // 检查最后一行是否只包含竖线和空白
        let only_pipes_and_whitespace = !last_line.is_empty()
            && last_line
            .chars()
            .all(|c| c == '|' || c == '│' || c.is_whitespace());

        // 只有当行包含竖线时才移除
        if has_pipe && only_pipes_and_whitespace {
            output.truncate(last_newline_pos + 1);
        }
    }
    output
}

/// 渲染子节点 (带树形连接符)
///
/// `last_file_prefix` 用于追踪最近渲染的文件行前缀，以便在目录渲染完成后输出正确对齐的尾随行
fn render_children(
    output: &mut String,
    node: &TreeNode,
    chars: &TreeChars,
    config: &Config,
    prefix: &str,
    depth: usize,
    last_file_prefix: &mut Option<String>,
) {
    if !depth_within_limit(depth, config.scan.max_depth) {
        return;
    }

    let (files, dirs): (Vec<_>, Vec<_>) = get_filtered_children(node, config)
        .into_iter()
        .partition(|c| c.kind == EntryKind::File);

    let has_dirs = !dirs.is_empty();
    let has_files = !files.is_empty();

    // 渲染文件
    if has_files {
        let file_prefix = if has_dirs {
            format!("{}{}", prefix, chars.vertical)
        } else {
            format!("{}{}", prefix, chars.space)
        };

        for file in &files {
            if !depth_within_limit(depth, config.scan.max_depth) {
                continue;
            }

            let name = format_entry_name(file, config);
            let meta = format_entry_meta(file, config);
            let _ = writeln!(output, "{}{}{}", file_prefix, name, meta);
        }

        // 更新最后文件前缀
        *last_file_prefix = Some(format!("{}{}", prefix, chars.space).replace("│", " "));
    }

    // 文件和目录之间的分隔行
    if has_files && has_dirs {
        let file_prefix = if has_dirs {
            format!("{}{}", prefix, chars.vertical)
        } else {
            format!("{}{}", prefix, chars.space)
        };
        let _ = writeln!(output, "{}", file_prefix);
    }

    // 渲染目录
    let dir_count = dirs.len();
    for (i, dir) in dirs.iter().enumerate() {
        if !depth_within_limit(depth, config.scan.max_depth) {
            continue;
        }

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
        if !dir.children.is_empty() && can_recurse(depth, config.scan.max_depth) {
            let new_prefix = if is_last {
                format!("{}{}", prefix, chars.space)
            } else {
                format!("{}{}", prefix, chars.vertical)
            };

            // 递归调用
            render_children(output, dir, chars, config, &new_prefix, depth + 1, last_file_prefix);

            // 子目录渲染完成后，如果有文件内容且不是最后一个目录，输出尾随行
            if !is_last && config.scan.show_files {
                if let Some(trailing) = last_file_prefix.as_ref() {
                    let _ = writeln!(output, "{}", trailing);
                }
            }
        }
    }
}

/// 渲染子节点 (无树形连接符，仅缩进)
fn render_children_no_indent(output: &mut String, node: &TreeNode, config: &Config, depth: usize) {
    if !depth_within_limit(depth, config.scan.max_depth) {
        return;
    }

    let (files, dirs): (Vec<_>, Vec<_>) = get_filtered_children(node, config)
        .into_iter()
        .partition(|c| c.kind == EntryKind::File);

    let indent = "  ".repeat(depth);

    for file in &files {
        if !depth_within_limit(depth, config.scan.max_depth) {
            continue;
        }
        let name = format_entry_name(file, config);
        let meta = format_entry_meta(file, config);
        let _ = writeln!(output, "{}{}{}", indent, name, meta);
    }

    for dir in &dirs {
        if !depth_within_limit(depth, config.scan.max_depth) {
            continue;
        }
        let name = format_entry_name(dir, config);
        let meta = format_entry_meta(dir, config);
        let _ = writeln!(output, "{}{}{}", indent, name, meta);

        if !dir.children.is_empty() && can_recurse(depth, config.scan.max_depth) {
            render_children_no_indent(output, dir, config, depth + 1);
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

/// 仅渲染树结构（不含 banner 和统计信息）
pub fn render_tree_only(node: &TreeNode, config: &Config) -> String {
    let mut output = String::new();
    let chars = TreeChars::from_charset(config.render.charset);

    let root_name = format_entry_name(node, config);
    let root_meta = format_entry_meta(node, config);
    let _ = writeln!(output, "{root_name}{root_meta}");

    if config.render.no_indent {
        render_children_no_indent(&mut output, node, config, 1);
    } else {
        let mut last_file_prefix: Option<String> = None;
        render_children(&mut output, node, &chars, config, "", 1, &mut last_file_prefix);
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
        let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n没有子文件夹 ";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹 ");
    }

    #[test]
    fn test_win_banner_parse_with_trailing_empty_lines() {
        let output = "卷 系统 的文件夹 PATH 列表\n卷序列号为 2810-11C7\nC:.\n没有子文件夹 \n\n";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹 ");
    }

    #[test]
    fn test_win_banner_parse_with_trailing_whitespace() {
        let output =
            "卷 系统 的文件夹 PATH 列表  \n  卷序列号为 2810-11C7\nC:.\n没有子文件夹  \n";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "卷 系统 的文件夹 PATH 列表  ");
        assert_eq!(banner.serial_line, "卷序列号为 2810-11C7");
        assert_eq!(banner.no_subfolder, "没有子文件夹  ");
    }

    #[test]
    fn test_win_banner_parse_english_locale() {
        let output = "Folder PATH listing for volume OS\nVolume serial number is ABCD-1234\nC:.\nNo subfolders exist ";
        let banner = WinBanner::parse(output).expect("解析应成功");

        assert_eq!(banner.volume_line, "Folder PATH listing for volume OS");
        assert_eq!(banner.serial_line, "Volume serial number is ABCD-1234");
        assert_eq!(banner.no_subfolder, "No subfolders exist ");
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
        let output_c = "卷 系统 的文件夹 PATH 列表\n卷序列号为 1234-5678\nC:.\n没有子文件夹";
        let output_d = "卷 数据 的文件夹 PATH 列表\n卷序列号为 ABCD-EF01\nD:.\n没有子文件夹";

        let banner_c = WinBanner::parse(output_c).expect("C盘解析应成功");
        let banner_d = WinBanner::parse(output_d).expect("D盘解析应成功");

        assert_eq!(banner_c.no_subfolder, "没有子文件夹");
        assert_eq!(banner_d.no_subfolder, "没有子文件夹");
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

        let local_now = Local::now();
        let expected_date = local_now.format("%Y-%m-%d").to_string();

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
        assert_eq!(chars.vertical, "│  ");
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
        config.scan.show_files = true;
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

        assert!(report.contains("5 directory"));
        assert!(report.contains("20 files"));
    }

    #[test]
    fn test_stream_render_config_from_config() {
        let mut config = Config::default();
        config.render.charset = CharsetMode::Ascii;
        config.render.show_size = true;
        config.render.human_readable = true;
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let render_config = StreamRenderConfig::from_config(&config);

        assert_eq!(render_config.charset, CharsetMode::Ascii);
        assert!(render_config.show_size);
        assert!(render_config.human_readable);
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
        config.path_explicitly_set = false;

        let result = render(&stats, &config);

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
    fn test_render_file_uses_indent_not_branch() {
        use crate::scan::sort_tree;

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

        sort_tree(&mut root, false);

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

        let lines: Vec<&str> = result.content.lines().collect();
        let file_line = lines.iter().find(|l| l.contains("file.txt")).unwrap();
        assert!(!file_line.contains("├"), "文件不应该使用├");
        assert!(!file_line.contains("└"), "文件不应该使用└");

        let dir_line = lines.iter().find(|l| l.contains("dir")).unwrap();
        assert!(
            dir_line.contains("├") || dir_line.contains("└"),
            "目录应该使用分支符"
        );
    }

    // ------------------------------------------------------------------------
    // disk_usage 与 show_files 独立性渲染测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_render_disk_usage_without_show_files() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut subdir = TreeNode::new(
            PathBuf::from("root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        subdir.children.push(TreeNode::new(
            PathBuf::from("root/subdir/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 1024,
                ..Default::default()
            },
        ));
        subdir.disk_usage = Some(1024);

        root.children.push(subdir);
        root.disk_usage = Some(1024);

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.batch_mode = true;
        config.scan.show_files = false;
        config.render.show_disk_usage = true;
        config.render.show_size = true;

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 1,
            file_count: 0,
        };

        let result = render(&stats, &config);

        assert!(result.content.contains("1024") || result.content.contains("1.0 KB"));
        assert!(!result.content.contains("file.txt"));
        assert!(result.content.contains("subdir"));
    }

    #[test]
    fn test_get_filtered_children_excludes_files_when_show_files_false() {
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

        let mut config = Config::default();
        config.scan.show_files = false;

        let filtered = get_filtered_children(&root, &config);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "subdir");
    }

    #[test]
    fn test_get_filtered_children_includes_files_when_show_files_true() {
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

        let mut config = Config::default();
        config.scan.show_files = true;

        let filtered = get_filtered_children(&root, &config);

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_render_disk_usage_respects_display_depth() {
        let mut dir2 = TreeNode::new(
            PathBuf::from("root/dir1/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir2.children.push(TreeNode::new(
            PathBuf::from("root/dir1/dir2/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 5,
                ..Default::default()
            },
        ));

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(dir2);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);

        root.compute_disk_usage();
        let directory_count = root.count_directories();
        let file_count = root.count_files();
        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(1),
            directory_count,
            file_count,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.batch_mode = true;
        config.render.no_win_banner = true;
        config.render.show_disk_usage = true;
        config.render.show_size = true;
        config.render.human_readable = false;
        config.scan.show_files = false;
        config.scan.max_depth = Some(1);

        let rendered = render(&stats, &config).content;

        assert!(rendered.contains("dir1"));
        assert!(!rendered.contains("dir2"));
        assert!(!rendered.contains("file.txt"));
        assert!(rendered.contains("5"));
    }

    #[test]
    fn test_render_files_then_dirs_with_separator() {
        use crate::scan::sort_tree;

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(TreeNode::new(
            PathBuf::from("root/dir1/file_a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        dir1.children.push(TreeNode::new(
            PathBuf::from("root/dir1/file_b.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut dir2 = TreeNode::new(
            PathBuf::from("root/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir2.children.push(TreeNode::new(
            PathBuf::from("root/dir2/file_c.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);
        root.children.push(dir2);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 2,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let dir1_idx = lines.iter().position(|l| l.contains("dir1")).unwrap();

        assert!(lines[dir1_idx + 1].contains("file_a.txt"));
        assert!(lines[dir1_idx + 2].contains("file_b.txt"));

        let separator_line = lines[dir1_idx + 3];
        assert!(
            separator_line.trim() == "|" || separator_line.trim().is_empty(),
            "dir1 渲染完后应有尾随空行，实际: '{}'",
            separator_line
        );

        assert!(
            lines[dir1_idx + 4].contains("dir2"),
            "尾随空行后应是 dir2，实际: '{}'",
            lines[dir1_idx + 4]
        );
    }

    #[test]
    fn test_render_no_separator_for_last_dir() {
        use crate::scan::sort_tree;

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(TreeNode::new(
            PathBuf::from("root/dir1/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 1,
            file_count: 1,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let dir1_idx = lines.iter().position(|l| l.contains("dir1")).unwrap();
        assert!(lines[dir1_idx + 1].contains("file.txt"));

        if lines.len() > dir1_idx + 2 {
            let after_file = lines[dir1_idx + 2];
            assert!(
                !after_file.starts_with("|")
                    && !after_file.contains("├")
                    && !after_file.contains("│"),
                "最后目录使用空格前缀的尾随空行，实际: '{}'",
                after_file
            );
        }
    }

    #[test]
    fn test_render_nested_dirs_with_files_ascii() {
        use crate::scan::sort_tree;

        let mut subdir = TreeNode::new(
            PathBuf::from("root/dir1/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        subdir.children.push(TreeNode::new(
            PathBuf::from("root/dir1/subdir/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(TreeNode::new(
            PathBuf::from("root/dir1/file1.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        dir1.children.push(subdir);

        let mut dir2 = TreeNode::new(
            PathBuf::from("root/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir2.children.push(TreeNode::new(
            PathBuf::from("root/dir2/file3.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);
        root.children.push(dir2);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 3,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);

        assert!(result.content.contains("dir1"));
        assert!(result.content.contains("file1.txt"));
        assert!(result.content.contains("subdir"));
        assert!(result.content.contains("file2.txt"));
        assert!(result.content.contains("dir2"));
        assert!(result.content.contains("file3.txt"));

        let lines: Vec<&str> = result.content.lines().collect();
        let dir2_idx = lines.iter().position(|l| l.contains("\\---dir2")).unwrap();

        let before_dir2 = lines[dir2_idx - 1];
        assert!(
            before_dir2.starts_with("|"),
            "dir2 前应有 | 占位行，实际: '{}'",
            before_dir2
        );
    }

    #[test]
    fn test_render_empty_dir_no_extra_separator() {
        use crate::scan::sort_tree;

        let dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        let dir2 = TreeNode::new(
            PathBuf::from("root/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);
        root.children.push(dir2);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 2,
            file_count: 0,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = false;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let dir1_idx = lines.iter().position(|l| l.contains("dir1")).unwrap();
        let dir2_idx = lines.iter().position(|l| l.contains("dir2")).unwrap();

        assert_eq!(dir2_idx - dir1_idx, 1, "空目录之间不应有占位行");
    }

    #[test]
    fn test_stream_renderer_pop_level_returns_trailing_line() {
        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);

        let entry = StreamEntry {
            path: PathBuf::from("file.txt"),
            name: "file.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&entry);

        let trailing = renderer.pop_level();
        assert!(trailing.is_some());
        let trailing_str = trailing.unwrap();
        assert!(
            trailing_str.len() >= 3,
            "尾随行应保留完整宽度（至少3字符），实际长度: {}",
            trailing_str.len()
        );
    }

    #[test]
    fn test_stream_renderer_pop_level_no_trailing_for_dir_only() {
        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);

        let entry = StreamEntry {
            path: PathBuf::from("subdir"),
            name: "subdir".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&entry);

        let trailing = renderer.pop_level();
        assert!(trailing.is_none());
    }

    #[test]
    fn test_stream_renderer_trailing_line_for_last_dir() {
        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(false);

        let entry = StreamEntry {
            path: PathBuf::from("file.txt"),
            name: "file.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&entry);

        let trailing = renderer.pop_level();
        assert!(trailing.is_some());
        let trailing_str = trailing.unwrap();
        assert!(
            !trailing_str.contains("│"),
            "最后目录的尾随行不应包含竖线，实际: '{}'",
            trailing_str
        );
        assert!(
            trailing_str.chars().all(|c| c == ' '),
            "最后目录的尾随行应全是空格，实际: '{}'",
            trailing_str
        );
    }

    #[test]
    fn test_render_no_trailing_for_pure_directory_structure() {
        use crate::scan::sort_tree;

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        let subdir = TreeNode::new(
            PathBuf::from("root/dir1/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(subdir);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 2,
            file_count: 0,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = false;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let dir1_idx = lines.iter().position(|l| l.contains("dir1")).unwrap();
        let subdir_idx = lines.iter().position(|l| l.contains("subdir")).unwrap();

        assert_eq!(subdir_idx - dir1_idx, 1, "纯目录结构不应有尾随空行");
    }

    #[test]
    fn test_trailing_line_alignment_ascii() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v2.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(api);

        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(docs);
        root.children.push(src);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 3,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let v2_idx = lines.iter().position(|l| l.contains("v2.md")).unwrap();
        let v2_line = lines[v2_idx];
        let trailing_line = lines[v2_idx + 1];

        let v2_prefix_len = v2_line.find("v2.md").unwrap();
        let v2_prefix = &v2_line[..v2_prefix_len];

        assert_eq!(
            trailing_line.len(),
            v2_prefix.len(),
            "尾随行长度应与文件前缀相同。v2前缀: '{}', 尾随行: '{}'",
            v2_prefix,
            trailing_line
        );
    }

    #[test]
    fn test_trailing_line_alignment_unicode() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v2.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(api);

        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(docs);
        root.children.push(src);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 3,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let v2_idx = lines.iter().position(|l| l.contains("v2.md")).unwrap();
        let v2_line = lines[v2_idx];
        let trailing_line = lines[v2_idx + 1];

        // 使用字符数而非字节长度进行比较
        let v2_prefix_char_count = v2_line.chars().take_while(|c| *c != 'v').count();
        let trailing_char_count = trailing_line.chars().count();

        assert_eq!(
            trailing_char_count,
            v2_prefix_char_count,
            "尾随行字符数应与文件前缀字符数相同。v2前缀: '{}' ({} 字符), 尾随行: '{}' ({} 字符)",
            &v2_line.chars().take(v2_prefix_char_count).collect::<String>(),
            v2_prefix_char_count,
            trailing_line,
            trailing_char_count
        );
    }

    #[test]
    fn test_no_trailing_line_when_last_is_directory() {
        use crate::scan::sort_tree;

        let grandchild = TreeNode::new(
            PathBuf::from("root/parent/child1/grandchild"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut child1 = TreeNode::new(
            PathBuf::from("root/parent/child1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        child1.children.push(grandchild);

        let child2 = TreeNode::new(
            PathBuf::from("root/parent/child2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut parent = TreeNode::new(
            PathBuf::from("root/parent"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        parent.children.push(TreeNode::new(
            PathBuf::from("root/parent/file1.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        parent.children.push(TreeNode::new(
            PathBuf::from("root/parent/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        parent.children.push(child1);
        parent.children.push(child2);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(parent);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 2,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let child2_idx = lines.iter().position(|l| l.contains("child2")).unwrap();

        if child2_idx + 1 < lines.len() {
            let after_child2 = lines[child2_idx + 1];
            let is_trailing_whitespace =
                !after_child2.is_empty() && after_child2.chars().all(|c| c.is_whitespace());
            assert!(
                !is_trailing_whitespace,
                "最后一个目录后不应有尾随空行，但发现: '{}'",
                after_child2
            );
        }
    }

    #[test]
    fn test_only_files_trailing_line_alignment() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/b.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/c.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 0,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let c_idx = lines.iter().position(|l| l.contains("c.txt")).unwrap();
        let c_line = lines[c_idx];

        let c_prefix_len = c_line.find("c.txt").unwrap();

        if c_idx + 1 < lines.len() {
            let next_line = lines[c_idx + 1];
            if next_line.chars().all(|c| c.is_whitespace()) && !next_line.is_empty() {
                assert_eq!(
                    next_line.len(),
                    c_prefix_len,
                    "尾随行长度应与文件前缀相同。c前缀长度: {}, 尾随行: '{}' (长度: {})",
                    c_prefix_len,
                    next_line,
                    next_line.len()
                );
            }
        }
    }

    #[test]
    fn test_no_duplicate_trailing_lines_for_sibling_dirs() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v2.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(TreeNode::new(
            PathBuf::from("root/docs/guide.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        docs.children.push(api);

        let mut lib = TreeNode::new(
            PathBuf::from("root/src/lib"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        lib.children.push(TreeNode::new(
            PathBuf::from("root/src/lib/mod.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        src.children.push(lib);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/README.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(docs);
        root.children.push(src);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 6,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        for i in 0..lines.len().saturating_sub(1) {
            let current_is_trailing = !lines[i].is_empty()
                && lines[i]
                .chars()
                .all(|c| c.is_whitespace() || c == '│' || c == '|');
            let next_is_trailing = !lines[i + 1].is_empty()
                && lines[i + 1]
                .chars()
                .all(|c| c.is_whitespace() || c == '│' || c == '|');

            assert!(
                !(current_is_trailing && next_is_trailing),
                "发现连续尾随行在第 {} 和 {} 行:\n'{}'\n'{}'",
                i + 1,
                i + 2,
                lines[i],
                lines[i + 1]
            );
        }
    }

    #[test]
    fn test_trailing_line_exact_prefix_match_ascii() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v2.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(TreeNode::new(
            PathBuf::from("root/docs/guide.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        docs.children.push(api);

        let mut lib = TreeNode::new(
            PathBuf::from("root/src/lib"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        lib.children.push(TreeNode::new(
            PathBuf::from("root/src/lib/mod.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        src.children.push(lib);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/README.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(docs);
        root.children.push(src);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 6,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        for i in 1..lines.len() {
            let current = lines[i];
            let previous = lines[i - 1];

            let is_trailing =
                !current.is_empty() && current.chars().all(|c| c.is_whitespace() || c == '|');

            if is_trailing {
                let prev_prefix_len = previous
                    .chars()
                    .take_while(|c| *c == '|' || *c == '+' || *c == '\\' || *c == '-' || *c == ' ')
                    .count();

                assert_eq!(
                    current.len(),
                    prev_prefix_len,
                    "尾随行长度应与上一行前缀长度一致。\n上一行: '{}' (前缀长度: {})\n尾随行: '{}' (长度: {})",
                    previous,
                    prev_prefix_len,
                    current,
                    current.len()
                );
            }
        }
    }

    #[test]
    fn test_trailing_line_exact_prefix_match_unicode() {
        use crate::scan::sort_tree;

        let mut api = TreeNode::new(
            PathBuf::from("root/docs/api"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v1.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        api.children.push(TreeNode::new(
            PathBuf::from("root/docs/api/v2.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut docs = TreeNode::new(
            PathBuf::from("root/docs"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        docs.children.push(TreeNode::new(
            PathBuf::from("root/docs/guide.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        docs.children.push(api);

        let mut lib = TreeNode::new(
            PathBuf::from("root/src/lib"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        lib.children.push(TreeNode::new(
            PathBuf::from("root/src/lib/mod.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        src.children.push(lib);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/README.md"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(docs);
        root.children.push(src);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 6,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        for i in 1..lines.len() {
            let current = lines[i];
            let previous = lines[i - 1];

            // 检测尾随行：只包含空格和树形符号
            let is_trailing = !current.is_empty()
                && current
                .chars()
                .all(|c| c.is_whitespace() || c == '│' || c == '├' || c == '└' || c == '─');

            if is_trailing {
                // 使用字符数比较，而非字节长度
                let prev_prefix_char_count = previous
                    .chars()
                    .take_while(|c| {
                        *c == '│' || *c == '├' || *c == '└' || *c == '─' || *c == ' '
                    })
                    .count();
                let trailing_char_count = current.chars().count();

                assert_eq!(
                    trailing_char_count,
                    prev_prefix_char_count,
                    "尾随行字符数应与上一行前缀字符数一致。\n上一行: '{}' (前缀 {} 字符)\n尾随行: '{}' ({} 字符)",
                    previous,
                    prev_prefix_char_count,
                    current,
                    trailing_char_count
                );
            }
        }
    }

    #[test]
    fn test_file_prefix_consistency_in_nested_dirs() {
        use crate::scan::sort_tree;

        let mut c = TreeNode::new(
            PathBuf::from("root/a/b/c"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        c.children.push(TreeNode::new(
            PathBuf::from("root/a/b/c/file1.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        c.children.push(TreeNode::new(
            PathBuf::from("root/a/b/c/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut b = TreeNode::new(
            PathBuf::from("root/a/b"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        b.children.push(c);
        b.children.push(TreeNode::new(
            PathBuf::from("root/a/b/file3.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut a = TreeNode::new(
            PathBuf::from("root/a"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        a.children.push(b);
        a.children.push(TreeNode::new(
            PathBuf::from("root/a/file4.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut d = TreeNode::new(
            PathBuf::from("root/d"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        d.children.push(TreeNode::new(
            PathBuf::from("root/d/file5.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(a);
        root.children.push(d);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 5,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            println!("L{:02}: '{}'", i + 1, line);
        }

        for i in 0..lines.len().saturating_sub(1) {
            let current = lines[i];
            let next = lines[i + 1];

            let current_is_file = current.contains(".txt");
            let next_is_trailing =
                !next.is_empty() && next.chars().all(|c| c.is_whitespace() || c == '|');

            if current_is_file && next_is_trailing {
                let current_prefix_len = current
                    .chars()
                    .take_while(|c| *c == '|' || *c == '+' || *c == '\\' || *c == '-' || *c == ' ')
                    .count();

                assert_eq!(
                    next.len(),
                    current_prefix_len,
                    "尾随行应与文件行前缀长度一致。\n文件行: '{}' (前缀长度: {})\n尾随行: '{}' (长度: {})",
                    current,
                    current_prefix_len,
                    next,
                    next.len()
                );
            }
        }
    }

    #[test]
    fn test_stream_renderer_build_file_prefix() {
        let config = Config::default();
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        renderer.push_level(true);

        let prefix = renderer.build_file_prefix(true);
        assert_eq!(prefix, "│  │  ", "有后续兄弟时应使用 vertical");

        renderer.push_level(false);

        let prefix = renderer.build_file_prefix(false);
        assert!(prefix.contains("    "), "没有后续兄弟时应使用 space");
    }

    #[test]
    fn test_trailing_line_after_deeply_nested_files() {
        use crate::scan::sort_tree;

        let mut deep = TreeNode::new(
            PathBuf::from("root/dir1/subdir/deep"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        deep.children.push(TreeNode::new(
            PathBuf::from("root/dir1/subdir/deep/file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut subdir = TreeNode::new(
            PathBuf::from("root/dir1/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        subdir.children.push(deep);

        let mut dir1 = TreeNode::new(
            PathBuf::from("root/dir1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir1.children.push(subdir);

        let mut dir2 = TreeNode::new(
            PathBuf::from("root/dir2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        dir2.children.push(TreeNode::new(
            PathBuf::from("root/dir2/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(dir1);
        root.children.push(dir2);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 2,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let file_idx = lines
            .iter()
            .position(|l| l.contains("file.txt") && !l.contains("file2"))
            .unwrap();
        let file_line = lines[file_idx];

        if file_idx + 1 < lines.len() {
            let trailing = lines[file_idx + 1];
            if !trailing.is_empty() && trailing.chars().all(|c| c.is_whitespace() || c == '|') {
                let file_prefix_len = file_line
                    .chars()
                    .take_while(|c| *c == '|' || *c == '+' || *c == '\\' || *c == '-' || *c == ' ')
                    .count();

                assert_eq!(
                    trailing.len(),
                    file_prefix_len,
                    "深层嵌套后的尾随行应与文件行前缀一致"
                );
            }
        }
    }

    #[test]
    fn test_trailing_line_prefix_matches_previous_line() {
        use crate::scan::sort_tree;

        // 构建测试树
        let mut src = TreeNode::new(
            PathBuf::from("root/src"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        src.children.push(TreeNode::new(
            PathBuf::from("root/src/main.rs"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut tmp = TreeNode::new(
            PathBuf::from("root/tmp"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        tmp.children.push(TreeNode::new(
            PathBuf::from("root/tmp/a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(src);
        root.children.push(tmp);

        sort_tree(&mut root, false);

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Unicode;
        config.scan.show_files = true;

        let output = render_tree_only(&root, &config);
        let lines: Vec<&str> = output.lines().collect();

        // 找到所有尾随行（纯空白行或只有树形符号的行）
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                continue;
            }

            // 检测尾随行：只包含空格和│的行
            let trimmed = line.trim_end();
            if trimmed.chars().all(|c| c == ' ' || c == '│') && !trimmed.is_empty() {
                // 这是尾随行，检查其前缀是否与上一行匹配
                let prev_line = lines[i - 1];
                let trailing_len = trimmed.len();

                // 上一行的前缀（相同长度）应该与尾随行完全一致
                let prev_prefix: String = prev_line.chars().take(trailing_len).collect();

                assert_eq!(
                    trimmed, prev_prefix,
                    "尾随行前缀不匹配\n第{}行(尾随行): {:?}\n第{}行(上一行前缀): {:?}",
                    i + 1, trimmed,
                    i, prev_prefix
                );
            }
        }
    }

    #[test]
    fn test_remove_trailing_pipe_only_line_at_end() {
        use crate::scan::sort_tree;

        // 构建与图示相同的树结构
        let grandchild = TreeNode::new(
            PathBuf::from("root/parent/child1/grandchild"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut child1 = TreeNode::new(
            PathBuf::from("root/parent/child1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        child1.children.push(grandchild);

        let child2 = TreeNode::new(
            PathBuf::from("root/parent/child2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut parent = TreeNode::new(
            PathBuf::from("root/parent"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        parent.children.push(TreeNode::new(
            PathBuf::from("root/parent/file1.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        parent.children.push(TreeNode::new(
            PathBuf::from("root/parent/file2.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        parent.children.push(child1);
        parent.children.push(child2);

        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(parent);

        sort_tree(&mut root, false);

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 4,
            file_count: 2,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        // 获取最后一个非空行
        let last_non_empty = lines.iter().rev().find(|l| !l.is_empty());

        if let Some(last_line) = last_non_empty {
            // 最后一行不应仅包含竖线和空白
            let only_pipes = last_line.chars().all(|c| c == '|' || c == '│' || c.is_whitespace());
            assert!(
                !only_pipes,
                "最后一行不应仅包含竖线，实际最后一行: '{}'",
                last_line
            );
        }
    }

    #[test]
    fn test_remove_trailing_pipe_only_line_helper() {
        // 含竖线的行应被移除
        let input = "line1\nline2\n|   \n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\n");

        // 实际内容不应被移除
        let input = "line1\nline2\nchild2\n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\nchild2\n");

        // Unicode 竖线应被移除
        let input = "line1\nline2\n│   \n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\n");

        // 纯空白行应保留（不含竖线）
        let input = "line1\nline2\n    \n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\n    \n");

        // 空输入
        let input = "".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "");

        // 混合竖线和空格应被移除
        let input = "line1\nline2\n|   |   \n".to_string();
        let result = remove_trailing_pipe_only_line(input);
        assert_eq!(result, "line1\nline2\n");
    }

    #[test]
    fn test_stream_renderer_no_trailing_when_last_child_is_dir() {
        // 测试场景：目录包含文件和子目录，但最后一个子条目是目录
        // 期望：pop_level 不应返回尾随行

        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        // 进入 parent 目录
        renderer.push_level(false);

        // 渲染 file1.txt
        let file1 = StreamEntry {
            path: PathBuf::from("file1.txt"),
            name: "file1.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: false,
            is_file: true,
            has_more_dirs: true,
        };
        let _ = renderer.render_entry(&file1);

        // 渲染 file2.txt
        let file2 = StreamEntry {
            path: PathBuf::from("file2.txt"),
            name: "file2.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: false,
            is_file: true,
            has_more_dirs: true,
        };
        let _ = renderer.render_entry(&file2);

        // 渲染 child1 目录
        let child1 = StreamEntry {
            path: PathBuf::from("child1"),
            name: "child1".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: false,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&child1);

        // 进入 child1，渲染 grandchild，离开 child1
        renderer.push_level(true);
        let grandchild = StreamEntry {
            path: PathBuf::from("grandchild"),
            name: "grandchild".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 2,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&grandchild);
        let _ = renderer.pop_level(); // 离开 child1

        // 渲染 child2 目录（最后一个子条目）
        let child2 = StreamEntry {
            path: PathBuf::from("child2"),
            name: "child2".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&child2);

        // 进入并离开 child2（空目录）
        renderer.push_level(false);
        let child2_trailing = renderer.pop_level();
        assert!(
            child2_trailing.is_none(),
            "空目录 child2 不应有尾随行"
        );

        // 离开 parent 目录
        let parent_trailing = renderer.pop_level();

        // 关键断言：parent 的最后一个子条目是 child2（目录），不是文件
        // 所以不应该有尾随行
        assert!(
            parent_trailing.is_none(),
            "parent 的最后子条目是目录，不应有尾随行，但得到: {:?}",
            parent_trailing
        );
    }

    #[test]
    fn test_stream_renderer_trailing_when_last_child_is_file() {
        // 测试场景：目录的最后一个子条目是文件
        // 期望：pop_level 应返回尾随行

        let mut config = Config::default();
        config.scan.show_files = true;
        let render_config = StreamRenderConfig::from_config(&config);
        let mut renderer = StreamRenderer::new(render_config);

        // 进入目录
        renderer.push_level(true); // 有后续兄弟

        // 渲染 subdir 目录
        let subdir = StreamEntry {
            path: PathBuf::from("subdir"),
            name: "subdir".to_string(),
            kind: EntryKind::Directory,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: false,
            is_file: false,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&subdir);

        // 渲染 file.txt（最后一个子条目）
        let file = StreamEntry {
            path: PathBuf::from("file.txt"),
            name: "file.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 1,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };
        let _ = renderer.render_entry(&file);

        // 离开目录
        let trailing = renderer.pop_level();

        // 关键断言：最后一个子条目是文件，应该有尾随行
        assert!(
            trailing.is_some(),
            "最后子条目是文件，应有尾随行"
        );
    }

    #[test]
    fn test_render_files_only_has_trailing_space_line() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/b.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/c.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let stats = ScanStats {
            tree: root,
            duration: Duration::from_millis(100),
            directory_count: 0,
            file_count: 3,
        };

        let mut config = Config::with_root(PathBuf::from("root"));
        config.render.no_win_banner = true;
        config.render.charset = CharsetMode::Ascii;
        config.scan.show_files = true;

        let result = render(&stats, &config);
        let lines: Vec<&str> = result.content.lines().collect();

        let c_idx = lines.iter().position(|l| l.contains("c.txt")).unwrap();
        assert!(c_idx + 1 < lines.len(), "c.txt 后应有尾随行");
        assert_eq!(lines[c_idx + 1], "    ", "尾随行应为4个空格");
    }
}