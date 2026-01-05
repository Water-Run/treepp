//! 扫描模块：目录树扫描引擎与统一 IR
//!
//! 本模块负责目录树的遍历与构建，提供：
//!
//! - **统一 IR**：`TreeNode` 与 `EntryKind` 表示目录树结构
//! - **扫描统计**：`ScanStats` 记录扫描结果与耗时
//! - **双模式扫描**：单线程 `walk` 与多线程 `parallel` 模式，输出保证一致
//! - **过滤功能**：include/exclude 通配、ignore-case、level 限制、prune 空目录
//! - **gitignore 支持**：分层叠加 `.gitignore` 规则
//! - **确定性排序**：按 `SortKey` 排序，支持逆序
//!
//! 作者: WaterRun
//! 更新于: 2025-01-05

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use glob::Pattern;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

use crate::config::{Config, SortKey};
use crate::error::{MatchError, ScanError, TreeppResult};

// ============================================================================
// 类型定义
// ============================================================================

/// 文件系统条目类型
///
/// 区分目录与文件两种基本类型。
///
/// # Examples
///
/// ```
/// use treepp::scan::EntryKind;
///
/// let dir = EntryKind::Directory;
/// let file = EntryKind::File;
/// assert_ne!(dir, file);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// 目录
    Directory,
    /// 文件
    File,
}

impl EntryKind {
    /// 从文件系统元数据判断条目类型
    #[must_use]
    pub fn from_metadata(meta: &Metadata) -> Self {
        if meta.is_dir() {
            Self::Directory
        } else {
            Self::File
        }
    }
}

/// 条目元数据
///
/// 存储文件/目录的附加信息，用于显示和排序。
///
/// # Examples
///
/// ```
/// use treepp::scan::EntryMetadata;
///
/// let meta = EntryMetadata::default();
/// assert_eq!(meta.size, 0);
/// assert!(meta.modified.is_none());
/// ```
#[derive(Debug, Clone, Default)]
pub struct EntryMetadata {
    /// 文件大小（字节），目录为 0
    pub size: u64,
    /// 最后修改时间
    pub modified: Option<SystemTime>,
    /// 创建时间
    pub created: Option<SystemTime>,
}

impl EntryMetadata {
    /// 从文件系统元数据创建
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs;
    /// use treepp::scan::EntryMetadata;
    ///
    /// let meta = fs::metadata(".").unwrap();
    /// let entry_meta = EntryMetadata::from_fs_metadata(&meta);
    /// ```
    #[must_use]
    pub fn from_fs_metadata(meta: &Metadata) -> Self {
        Self {
            size: if meta.is_file() { meta.len() } else { 0 },
            modified: meta.modified().ok(),
            created: meta.created().ok(),
        }
    }
}

/// 目录树节点
///
/// 表示目录树中的单个条目，可递归包含子节点。
/// 这是扫描输出的统一中间表示（IR）。
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
///
/// let node = TreeNode::new(
///     PathBuf::from("src"),
///     EntryKind::Directory,
///     EntryMetadata::default(),
/// );
/// assert_eq!(node.name, "src");
/// assert!(node.children.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// 条目名称（不含路径）
    pub name: String,
    /// 完整路径
    pub path: PathBuf,
    /// 条目类型
    pub kind: EntryKind,
    /// 元数据
    pub metadata: EntryMetadata,
    /// 子节点（仅目录有效）
    pub children: Vec<TreeNode>,
    /// 目录累计大小（用于 disk_usage 显示）
    pub disk_usage: Option<u64>,
}

impl TreeNode {
    /// 创建新的叶子节点
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let node = TreeNode::new(
    ///     PathBuf::from("main.rs"),
    ///     EntryKind::File,
    ///     EntryMetadata { size: 1024, ..Default::default() },
    /// );
    /// assert_eq!(node.name, "main.rs");
    /// assert_eq!(node.metadata.size, 1024);
    /// ```
    #[must_use]
    pub fn new(path: PathBuf, kind: EntryKind, metadata: EntryMetadata) -> Self {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        Self {
            name,
            path,
            kind,
            metadata,
            children: Vec::new(),
            disk_usage: None,
        }
    }

    /// 创建带子节点的目录节点
    #[must_use]
    pub fn with_children(
        path: PathBuf,
        kind: EntryKind,
        metadata: EntryMetadata,
        children: Vec<Self>,
    ) -> Self {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        Self {
            name,
            path,
            kind,
            metadata,
            children,
            disk_usage: None,
        }
    }

    /// 递归统计目录数量（不含根节点自身）
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let mut root = TreeNode::new(PathBuf::from("."), EntryKind::Directory, EntryMetadata::default());
    /// root.children.push(TreeNode::new(PathBuf::from("src"), EntryKind::Directory, EntryMetadata::default()));
    /// assert_eq!(root.count_directories(), 1);
    /// ```
    #[must_use]
    pub fn count_directories(&self) -> usize {
        self.children
            .iter()
            .map(|c| {
                if c.kind == EntryKind::Directory {
                    1 + c.count_directories()
                } else {
                    0
                }
            })
            .sum()
    }

    /// 递归统计文件数量
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let mut root = TreeNode::new(PathBuf::from("."), EntryKind::Directory, EntryMetadata::default());
    /// root.children.push(TreeNode::new(PathBuf::from("main.rs"), EntryKind::File, EntryMetadata::default()));
    /// assert_eq!(root.count_files(), 1);
    /// ```
    #[must_use]
    pub fn count_files(&self) -> usize {
        let self_count = if self.kind == EntryKind::File { 1 } else { 0 };
        self_count + self.children.iter().map(Self::count_files).sum::<usize>()
    }

    /// 递归计算目录累计大小
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let mut root = TreeNode::new(PathBuf::from("."), EntryKind::Directory, EntryMetadata::default());
    /// root.children.push(TreeNode::new(
    ///     PathBuf::from("file.txt"),
    ///     EntryKind::File,
    ///     EntryMetadata { size: 100, ..Default::default() },
    /// ));
    /// root.compute_disk_usage();
    /// assert_eq!(root.disk_usage, Some(100));
    /// ```
    pub fn compute_disk_usage(&mut self) -> u64 {
        if self.kind == EntryKind::File {
            return self.metadata.size;
        }

        let total: u64 = self
            .children
            .iter_mut()
            .map(|c| c.compute_disk_usage())
            .sum();

        self.disk_usage = Some(total);
        total
    }

    /// 检查是否为空目录（无子节点或仅含空子目录）
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let empty = TreeNode::new(PathBuf::from("empty"), EntryKind::Directory, EntryMetadata::default());
    /// assert!(empty.is_empty_dir());
    ///
    /// let mut with_file = TreeNode::new(PathBuf::from("src"), EntryKind::Directory, EntryMetadata::default());
    /// with_file.children.push(TreeNode::new(PathBuf::from("main.rs"), EntryKind::File, EntryMetadata::default()));
    /// assert!(!with_file.is_empty_dir());
    /// ```
    #[must_use]
    pub fn is_empty_dir(&self) -> bool {
        if self.kind != EntryKind::Directory {
            return false;
        }
        self.children.is_empty() || self.children.iter().all(Self::is_empty_dir)
    }

    /// 修剪空目录
    ///
    /// 递归移除所有空目录节点。
    pub fn prune_empty_dirs(&mut self) {
        // 先递归处理子节点
        for child in &mut self.children {
            child.prune_empty_dirs();
        }
        // 移除空目录
        self.children.retain(|c| {
            if c.kind == EntryKind::Directory {
                !c.is_empty_dir()
            } else {
                true
            }
        });
    }
}

/// 扫描统计结果
///
/// 包含扫描产出的目录树、耗时统计和条目计数。
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use std::time::Duration;
/// use treepp::scan::{ScanStats, TreeNode, EntryKind, EntryMetadata};
///
/// let tree = TreeNode::new(PathBuf::from("."), EntryKind::Directory, EntryMetadata::default());
/// let stats = ScanStats {
///     tree,
///     duration: Duration::from_millis(100),
///     directory_count: 5,
///     file_count: 20,
/// };
/// assert_eq!(stats.directory_count, 5);
/// ```
#[derive(Debug)]
pub struct ScanStats {
    /// 根节点
    pub tree: TreeNode,
    /// 扫描耗时
    pub duration: Duration,
    /// 目录总数（不含根）
    pub directory_count: usize,
    /// 文件总数
    pub file_count: usize,
}

// ============================================================================
// 匹配规则
// ============================================================================

/// 编译后的匹配规则集
struct CompiledRules {
    /// 包含模式
    include_patterns: Vec<Pattern>,
    /// 排除模式
    exclude_patterns: Vec<Pattern>,
    /// 是否忽略大小写（已在编译时处理）
    ignore_case: bool,
}

impl CompiledRules {
    /// 从配置编译匹配规则
    fn compile(config: &Config) -> Result<Self, MatchError> {
        let ignore_case = config.matching.ignore_case;

        let include_patterns = config
            .matching
            .include_patterns
            .iter()
            .map(|p| compile_pattern(p, ignore_case))
            .collect::<Result<Vec<_>, _>>()?;

        let exclude_patterns = config
            .matching
            .exclude_patterns
            .iter()
            .map(|p| compile_pattern(p, ignore_case))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            include_patterns,
            exclude_patterns,
            ignore_case,
        })
    }

    /// 检查名称是否应被包含
    fn should_include(&self, name: &str, is_dir: bool) -> bool {
        // 目录始终包含（除非被排除）
        if is_dir {
            return true;
        }

        // 无 include 模式时包含所有
        if self.include_patterns.is_empty() {
            return true;
        }

        let match_name = if self.ignore_case {
            name.to_lowercase()
        } else {
            name.to_string()
        };

        self.include_patterns.iter().any(|p| p.matches(&match_name))
    }

    /// 检查名称是否应被排除
    fn should_exclude(&self, name: &str) -> bool {
        if self.exclude_patterns.is_empty() {
            return false;
        }

        let match_name = if self.ignore_case {
            name.to_lowercase()
        } else {
            name.to_string()
        };

        self.exclude_patterns.iter().any(|p| p.matches(&match_name))
    }
}

/// 编译单个通配符模式
fn compile_pattern(pattern: &str, ignore_case: bool) -> Result<Pattern, MatchError> {
    let pat = if ignore_case {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };

    Pattern::new(&pat).map_err(|e| MatchError::InvalidPattern {
        pattern: pattern.to_string(),
        reason: e.msg.to_string(),
    })
}

// ============================================================================
// Gitignore 支持
// ============================================================================

/// 加载指定目录的 .gitignore 规则
fn load_gitignore(dir: &Path) -> Option<Gitignore> {
    let gitignore_path = dir.join(".gitignore");
    if !gitignore_path.exists() {
        return None;
    }

    let mut builder = GitignoreBuilder::new(dir);
    if builder.add(&gitignore_path).is_some() {
        return None;
    }

    builder.build().ok()
}

/// 检查路径是否被 gitignore 忽略
fn is_ignored_by_gitignore(gitignore: Option<&Gitignore>, path: &Path, is_dir: bool) -> bool {
    gitignore
        .map(|gi| gi.matched(path, is_dir).is_ignore())
        .unwrap_or(false)
}

// ============================================================================
// 排序
// ============================================================================

/// 对树节点进行确定性排序（递归）
fn sort_tree(node: &mut TreeNode, sort_key: SortKey, reverse: bool) {
    // 排序子节点
    node.children.sort_by(|a, b| {
        // 目录在前，文件在后
        let kind_order = match (a.kind, b.kind) {
            (EntryKind::Directory, EntryKind::File) => Ordering::Less,
            (EntryKind::File, EntryKind::Directory) => Ordering::Greater,
            _ => Ordering::Equal,
        };

        if kind_order != Ordering::Equal {
            return kind_order;
        }

        // 按指定键排序
        let cmp = match sort_key {
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::Size => {
                let size_a = a.disk_usage.unwrap_or(a.metadata.size);
                let size_b = b.disk_usage.unwrap_or(b.metadata.size);
                size_a.cmp(&size_b)
            }
            SortKey::Mtime => a.metadata.modified.cmp(&b.metadata.modified),
            SortKey::Ctime => a.metadata.created.cmp(&b.metadata.created),
        };

        if reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });

    // 递归排序子节点的子节点
    for child in &mut node.children {
        sort_tree(child, sort_key, reverse);
    }
}

// ============================================================================
// 扫描上下文
// ============================================================================

/// 扫描上下文，包含所有扫描配置
struct ScanContext {
    /// 是否显示文件
    show_files: bool,
    /// 最大递归深度
    max_depth: Option<usize>,
    /// 是否遵循 gitignore
    respect_gitignore: bool,
    /// 编译后的匹配规则
    rules: CompiledRules,
    /// 排序键
    sort_key: SortKey,
    /// 是否逆序
    reverse: bool,
    /// 是否修剪空目录
    prune_empty: bool,
    /// 是否需要大小信息
    needs_size: bool,
}

impl ScanContext {
    /// 从配置创建扫描上下文
    fn from_config(config: &Config) -> Result<Self, MatchError> {
        Ok(Self {
            show_files: config.scan.show_files,
            max_depth: config.scan.max_depth,
            respect_gitignore: config.scan.respect_gitignore,
            rules: CompiledRules::compile(config)?,
            sort_key: config.render.sort_key,
            reverse: config.render.reverse_sort,
            prune_empty: config.matching.prune_empty,
            needs_size: config.needs_size_info(),
        })
    }

    /// 检查条目是否应被过滤
    fn should_filter(&self, name: &str, is_dir: bool) -> bool {
        // 排除检查
        if self.rules.should_exclude(name) {
            return true;
        }

        // 包含检查（仅对文件）
        if !is_dir && !self.rules.should_include(name, is_dir) {
            return true;
        }

        // 文件显示检查
        if !is_dir && !self.show_files {
            return true;
        }

        false
    }
}

// ============================================================================
// 单线程扫描
// ============================================================================

/// 单线程递归扫描
fn walk_recursive(
    path: &Path,
    depth: usize,
    ctx: &ScanContext,
    parent_gitignore: Option<&Gitignore>,
) -> Option<TreeNode> {
    // 深度限制检查
    if let Some(max) = ctx.max_depth {
        if depth > max {
            return None;
        }
    }

    let meta = fs::metadata(path).ok()?;
    let kind = EntryKind::from_metadata(&meta);
    let metadata = EntryMetadata::from_fs_metadata(&meta);

    // 加载当前目录的 gitignore（如果启用）
    let current_gitignore = if ctx.respect_gitignore && kind == EntryKind::Directory {
        load_gitignore(path)
    } else {
        None
    };

    // 合并 gitignore：优先使用当前目录的，否则继承父目录
    let effective_gitignore = current_gitignore.as_ref().or(parent_gitignore);

    let mut node = TreeNode::new(path.to_path_buf(), kind, metadata);

    if kind == EntryKind::Directory {
        let entries = match fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return Some(node),
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            let entry_name = entry_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            let entry_meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let is_dir = entry_meta.is_dir();

            // gitignore 检查
            if ctx.respect_gitignore {
                if is_ignored_by_gitignore(effective_gitignore, &entry_path, is_dir) {
                    continue;
                }
            }

            // 过滤检查
            if ctx.should_filter(&entry_name, is_dir) {
                continue;
            }

            // 递归处理
            if let Some(child) =
                walk_recursive(&entry_path, depth + 1, ctx, effective_gitignore)
            {
                node.children.push(child);
            }
        }
    }

    Some(node)
}

/// 单线程扫描入口
///
/// 使用深度优先递归遍历目录树。
///
/// # Errors
///
/// 返回 `ScanError` 如果根路径不存在或无法访问。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::scan_walk;
///
/// let config = Config::with_root(PathBuf::from("."));
/// let stats = scan_walk(&config).expect("扫描失败");
/// println!("目录: {}, 文件: {}", stats.directory_count, stats.file_count);
/// ```
pub fn scan_walk(config: &Config) -> TreeppResult<ScanStats> {
    let start = Instant::now();

    // 验证根路径
    if !config.root_path.exists() {
        return Err(ScanError::PathNotFound {
            path: config.root_path.clone(),
        }
            .into());
    }

    if !config.root_path.is_dir() {
        return Err(ScanError::NotADirectory {
            path: config.root_path.clone(),
        }
            .into());
    }

    // 创建扫描上下文
    let ctx = ScanContext::from_config(config)?;

    // 执行扫描
    let mut tree = walk_recursive(&config.root_path, 0, &ctx, None).ok_or_else(|| {
        ScanError::ReadDirFailed {
            path: config.root_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::Other, "无法读取根目录"),
        }
    })?;

    // 计算目录累计大小（如需要）
    if ctx.needs_size {
        tree.compute_disk_usage();
    }

    // 修剪空目录（如需要）
    if ctx.prune_empty {
        tree.prune_empty_dirs();
    }

    // 排序
    sort_tree(&mut tree, ctx.sort_key, ctx.reverse);

    let duration = start.elapsed();
    let directory_count = tree.count_directories();
    let file_count = tree.count_files();

    Ok(ScanStats {
        tree,
        duration,
        directory_count,
        file_count,
    })
}

// ============================================================================
// 多线程扫描
// ============================================================================

/// 多线程分治扫描（内部函数）
fn parallel_scan_dir(
    path: &Path,
    depth: usize,
    ctx: &ScanContext,
    parent_gitignore: Option<&Gitignore>,
) -> Option<TreeNode> {
    // 深度限制检查
    if let Some(max) = ctx.max_depth {
        if depth > max {
            return None;
        }
    }

    let meta = fs::metadata(path).ok()?;
    let kind = EntryKind::from_metadata(&meta);
    let metadata = EntryMetadata::from_fs_metadata(&meta);

    if kind != EntryKind::Directory {
        return Some(TreeNode::new(path.to_path_buf(), kind, metadata));
    }

    // 加载当前目录的 gitignore
    let current_gitignore = if ctx.respect_gitignore {
        load_gitignore(path)
    } else {
        None
    };
    let effective_gitignore = current_gitignore.as_ref().or(parent_gitignore);

    // 读取目录条目
    let entries: Vec<_> = fs::read_dir(path).ok()?.flatten().collect();

    // 分离子目录和文件
    let mut subdirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries {
        let entry_path = entry.path();
        let entry_name = entry_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        let entry_meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = entry_meta.is_dir();

        // gitignore 检查
        if ctx.respect_gitignore {
            if is_ignored_by_gitignore(effective_gitignore, &entry_path, is_dir) {
                continue;
            }
        }

        // 过滤检查
        if ctx.should_filter(&entry_name, is_dir) {
            continue;
        }

        if is_dir {
            subdirs.push(entry_path);
        } else {
            let file_metadata = EntryMetadata::from_fs_metadata(&entry_meta);
            files.push(TreeNode::new(entry_path, EntryKind::File, file_metadata));
        }
    }

    // 并行处理子目录
    // 注意：gitignore 需要序列化传递，这里我们为每个子目录重新加载
    let subdir_trees: Vec<TreeNode> = subdirs
        .into_par_iter()
        .filter_map(|subdir| {
            // 重新加载父级 gitignore（因为不能跨线程传递引用）
            let parent_gi = if ctx.respect_gitignore {
                load_gitignore(path)
            } else {
                None
            };
            parallel_scan_dir(&subdir, depth + 1, ctx, parent_gi.as_ref())
        })
        .collect();

    // 合并结果
    let mut children = subdir_trees;
    children.extend(files);

    Some(TreeNode::with_children(
        path.to_path_buf(),
        EntryKind::Directory,
        metadata,
        children,
    ))
}

/// 多线程扫描入口
///
/// 使用 rayon 分治策略并行扫描目录树。
/// 对于小目录或单核系统，内部可能回退到单线程模式。
///
/// # 保证
///
/// 输出结果与 `scan_walk` 在结构上完全一致（经过相同的排序后）。
///
/// # Errors
///
/// 返回 `ScanError` 如果根路径不存在、线程池创建失败等。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::scan_parallel;
///
/// let config = Config::with_root(PathBuf::from("."));
/// let stats = scan_parallel(&config).expect("扫描失败");
/// println!("耗时: {:?}", stats.duration);
/// ```
pub fn scan_parallel(config: &Config) -> TreeppResult<ScanStats> {
    let start = Instant::now();

    // 验证根路径
    if !config.root_path.exists() {
        return Err(ScanError::PathNotFound {
            path: config.root_path.clone(),
        }
            .into());
    }

    if !config.root_path.is_dir() {
        return Err(ScanError::NotADirectory {
            path: config.root_path.clone(),
        }
            .into());
    }

    // 创建扫描上下文
    let ctx = ScanContext::from_config(config)?;

    // 创建线程池
    let thread_count = config.scan.thread_count.get();
    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .map_err(|e| ScanError::WalkError {
            message: format!("线程池创建失败: {}", e),
            path: Some(config.root_path.clone()),
        })?;

    // 在线程池中执行扫描
    let root_path = config.root_path.clone();
    let mut tree = pool.install(|| parallel_scan_dir(&root_path, 0, &ctx, None)).ok_or_else(
        || ScanError::ReadDirFailed {
            path: config.root_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::Other, "无法读取根目录"),
        },
    )?;

    // 计算目录累计大小（如需要）
    if ctx.needs_size {
        tree.compute_disk_usage();
    }

    // 修剪空目录（如需要）
    if ctx.prune_empty {
        tree.prune_empty_dirs();
    }

    // 排序（确保与 walk 模式输出一致）
    sort_tree(&mut tree, ctx.sort_key, ctx.reverse);

    let duration = start.elapsed();
    let directory_count = tree.count_directories();
    let file_count = tree.count_files();

    Ok(ScanStats {
        tree,
        duration,
        directory_count,
        file_count,
    })
}

// ============================================================================
// 统一扫描入口
// ============================================================================

/// 执行目录扫描
///
/// 根据配置的线程数自动选择扫描模式：
/// - 线程数为 1 时使用单线程模式
/// - 否则使用多线程模式
///
/// # Errors
///
/// 返回 `TreeppError` 如果扫描过程中发生错误。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::scan;
///
/// let config = Config::with_root(PathBuf::from(".")).validate().unwrap();
/// let stats = scan(&config).expect("扫描失败");
/// println!("{} 个目录, {} 个文件", stats.directory_count, stats.file_count);
/// ```
pub fn scan(config: &Config) -> TreeppResult<ScanStats> {
    if config.scan.thread_count.get() == 1 {
        scan_walk(config)
    } else {
        scan_parallel(config)
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    /// 创建测试目录结构
    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().expect("创建临时目录失败");
        let root = dir.path();

        // 创建目录结构
        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir(root.join("tests")).unwrap();
        fs::create_dir(root.join("empty")).unwrap();

        // 创建文件
        File::create(root.join("Cargo.toml"))
            .unwrap()
            .write_all(b"[package]")
            .unwrap();
        File::create(root.join("README.md"))
            .unwrap()
            .write_all(b"# Test")
            .unwrap();
        File::create(root.join("src/main.rs"))
            .unwrap()
            .write_all(b"fn main() {}")
            .unwrap();
        File::create(root.join("src/lib.rs"))
            .unwrap()
            .write_all(b"pub fn lib() {}")
            .unwrap();
        File::create(root.join("tests/test.rs"))
            .unwrap()
            .write_all(b"#[test]")
            .unwrap();

        dir
    }

    /// 创建带 .gitignore 的测试目录
    fn setup_gitignore_dir() -> TempDir {
        let dir = setup_test_dir();
        let root = dir.path();

        // 创建 .gitignore
        File::create(root.join(".gitignore"))
            .unwrap()
            .write_all(b"target/\n*.log\n")
            .unwrap();

        // 创建应被忽略的目录和文件
        fs::create_dir(root.join("target")).unwrap();
        File::create(root.join("target/debug"))
            .unwrap()
            .write_all(b"")
            .unwrap();
        File::create(root.join("app.log"))
            .unwrap()
            .write_all(b"log")
            .unwrap();

        dir
    }

    #[test]
    fn test_entry_kind_from_metadata() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let file_meta = fs::metadata(&file_path).unwrap();
        assert_eq!(EntryKind::from_metadata(&file_meta), EntryKind::File);

        let dir_meta = fs::metadata(dir.path()).unwrap();
        assert_eq!(EntryKind::from_metadata(&dir_meta), EntryKind::Directory);
    }

    #[test]
    fn test_tree_node_new() {
        let node = TreeNode::new(
            PathBuf::from("/test/main.rs"),
            EntryKind::File,
            EntryMetadata {
                size: 1024,
                ..Default::default()
            },
        );

        assert_eq!(node.name, "main.rs");
        assert_eq!(node.kind, EntryKind::File);
        assert_eq!(node.metadata.size, 1024);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_tree_node_count_files() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("b.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut subdir = TreeNode::new(
            PathBuf::from("sub"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        subdir.children.push(TreeNode::new(
            PathBuf::from("c.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(subdir);

        assert_eq!(root.count_files(), 3);
    }

    #[test]
    fn test_tree_node_count_directories() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut sub1 = TreeNode::new(
            PathBuf::from("sub1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        sub1.children.push(TreeNode::new(
            PathBuf::from("nested"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        root.children.push(sub1);
        root.children.push(TreeNode::new(
            PathBuf::from("sub2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        assert_eq!(root.count_directories(), 3);
    }

    #[test]
    fn test_tree_node_is_empty_dir() {
        let empty = TreeNode::new(
            PathBuf::from("empty"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        assert!(empty.is_empty_dir());

        let file = TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        );
        assert!(!file.is_empty_dir());

        let mut with_content = TreeNode::new(
            PathBuf::from("dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        with_content.children.push(TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        assert!(!with_content.is_empty_dir());
    }

    #[test]
    fn test_tree_node_compute_disk_usage() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("a.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 100,
                ..Default::default()
            },
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("b.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 200,
                ..Default::default()
            },
        ));

        root.compute_disk_usage();
        assert_eq!(root.disk_usage, Some(300));
    }

    #[test]
    fn test_tree_node_prune_empty_dirs() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("empty"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        assert_eq!(root.children.len(), 2);
        root.prune_empty_dirs();
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name, "file.txt");
    }

    #[test]
    fn test_scan_walk_basic() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan_walk(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3); // src, tests, empty
        assert_eq!(stats.file_count, 5); // Cargo.toml, README.md, main.rs, lib.rs, test.rs
    }

    #[test]
    fn test_scan_walk_dirs_only() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let stats = scan_walk(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 0); // show_files = false
    }

    #[test]
    fn test_scan_walk_max_depth() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.max_depth = Some(1);

        let stats = scan_walk(&config).expect("扫描失败");

        // 深度 1 只包含根目录下的直接子项
        assert!(stats.file_count <= 2); // Cargo.toml, README.md
    }

    #[test]
    fn test_scan_walk_with_prune() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.prune_empty = true;

        let stats = scan_walk(&config).expect("扫描失败");

        // empty 目录应被修剪
        assert_eq!(stats.directory_count, 2); // src, tests
    }

    #[test]
    fn test_scan_walk_with_include() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];

        let stats = scan_walk(&config).expect("扫描失败");

        // 只包含 .rs 文件
        assert_eq!(stats.file_count, 3); // main.rs, lib.rs, test.rs
    }

    #[test]
    fn test_scan_walk_with_exclude() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.exclude_patterns = vec!["*.md".to_string()];

        let stats = scan_walk(&config).expect("扫描失败");

        // 排除 .md 文件
        assert_eq!(stats.file_count, 4); // 不包含 README.md
    }

    #[test]
    fn test_scan_walk_with_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan_walk(&config).expect("扫描失败");

        // target 目录和 .log 文件应被忽略
        assert!(
            !stats
                .tree
                .children
                .iter()
                .any(|c| c.name == "target" || c.name == "app.log")
        );
    }

    #[test]
    fn test_scan_parallel_basic() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan_parallel(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 5);
    }

    #[test]
    fn test_scan_walk_parallel_consistency() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let walk_stats = scan_walk(&config).expect("walk 扫描失败");
        let parallel_stats = scan_parallel(&config).expect("parallel 扫描失败");

        // 结果应一致
        assert_eq!(walk_stats.directory_count, parallel_stats.directory_count);
        assert_eq!(walk_stats.file_count, parallel_stats.file_count);

        // 验证结构一致性
        fn collect_names(node: &TreeNode) -> Vec<String> {
            let mut names = vec![node.name.clone()];
            for child in &node.children {
                names.extend(collect_names(child));
            }
            names.sort();
            names
        }

        let walk_names = collect_names(&walk_stats.tree);
        let parallel_names = collect_names(&parallel_stats.tree);
        assert_eq!(walk_names, parallel_names);
    }

    #[test]
    fn test_scan_nonexistent_path() {
        let config = Config::with_root(PathBuf::from("/nonexistent/path/12345"));
        let result = scan_walk(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_file_as_root() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("file.txt");
        File::create(&file_path).unwrap();

        let config = Config::with_root(file_path);
        let result = scan_walk(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_sort_by_name() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("zebra.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("alpha.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("beta.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        sort_tree(&mut root, SortKey::Name, false);

        assert_eq!(root.children[0].name, "alpha.txt");
        assert_eq!(root.children[1].name, "beta.txt");
        assert_eq!(root.children[2].name, "zebra.txt");
    }

    #[test]
    fn test_sort_by_size() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("large.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 1000,
                ..Default::default()
            },
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("small.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 100,
                ..Default::default()
            },
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("medium.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 500,
                ..Default::default()
            },
        ));

        sort_tree(&mut root, SortKey::Size, false);

        assert_eq!(root.children[0].name, "small.txt");
        assert_eq!(root.children[1].name, "medium.txt");
        assert_eq!(root.children[2].name, "large.txt");
    }

    #[test]
    fn test_sort_reverse() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("b.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("c.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        sort_tree(&mut root, SortKey::Name, true);

        assert_eq!(root.children[0].name, "c.txt");
        assert_eq!(root.children[1].name, "b.txt");
        assert_eq!(root.children[2].name, "a.txt");
    }

    #[test]
    fn test_sort_dirs_before_files() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));

        sort_tree(&mut root, SortKey::Name, false);

        assert_eq!(root.children[0].kind, EntryKind::Directory);
        assert_eq!(root.children[1].kind, EntryKind::File);
    }

    #[test]
    fn test_compile_pattern_basic() {
        let pattern = compile_pattern("*.rs", false).expect("编译失败");
        assert!(pattern.matches("main.rs"));
        assert!(!pattern.matches("main.txt"));
    }

    #[test]
    fn test_compile_pattern_ignore_case() {
        let pattern = compile_pattern("*.RS", true).expect("编译失败");
        assert!(pattern.matches("main.rs"));
        assert!(pattern.matches("lib.rs"));
    }

    #[test]
    fn test_compile_pattern_invalid() {
        let result = compile_pattern("[invalid", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_unified_scan_selects_mode() {
        let dir = setup_test_dir();

        // 单线程模式
        let mut config1 = Config::with_root(dir.path().to_path_buf());
        config1.scan.show_files = true;
        config1.scan.thread_count = std::num::NonZeroUsize::new(1).unwrap();
        let stats1 = scan(&config1).expect("扫描失败");

        // 多线程模式
        let mut config8 = Config::with_root(dir.path().to_path_buf());
        config8.scan.show_files = true;
        config8.scan.thread_count = std::num::NonZeroUsize::new(8).unwrap();
        let stats8 = scan(&config8).expect("扫描失败");

        // 结果应一致
        assert_eq!(stats1.file_count, stats8.file_count);
        assert_eq!(stats1.directory_count, stats8.directory_count);
    }
}
