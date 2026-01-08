//! 扫描模块：目录树扫描引擎与统一 IR
//!
//! 本模块负责目录树的遍历与构建，提供：
//!
//! - **统一 IR**：`TreeNode` 与 `EntryKind` 表示目录树结构
//! - **扫描统计**：`ScanStats` 记录扫描结果与耗时
//! - **并行扫描**：使用 rayon 分治策略，线程数可配置
//! - **流式扫描**：`scan_streaming` 支持边扫边回调，实现实时输出
//! - **过滤功能**：include/exclude 通配、ignore-case、level 限制、prune 空目录
//! - **gitignore 支持**：分层叠加 `.gitignore` 规则，支持规则链继承与缓存
//! - **确定性排序**：按 `SortKey` 排序，支持逆序
//!
//! 文件: src/scan.rs
//! 作者: WaterRun
//! 更新于: 2026-01-08

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use glob::Pattern;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use crate::config::Config;
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
// 流式扫描类型
// ============================================================================

/// 流式扫描条目
///
/// 在流式扫描模式下，每个发现的条目会通过回调传递此结构。
/// 包含条目的完整信息以及用于树形渲染的位置信息。
#[derive(Debug, Clone)]
pub struct StreamEntry {
    /// 条目完整路径
    pub path: PathBuf,
    /// 条目名称（不含路径）
    pub name: String,
    /// 条目类型
    pub kind: EntryKind,
    /// 条目元数据
    pub metadata: EntryMetadata,
    /// 当前深度（根目录子项为 0）
    pub depth: usize,
    /// 是否为当前层级最后一个条目（用于渲染连接符）
    pub is_last: bool,
    /// 是否为文件（用于区分渲染方式）
    pub is_file: bool,
    /// 是否还有后续目录（用于文件渲染时决定前缀）
    pub has_more_dirs: bool,
}

/// 流式扫描统计（简化版，不含树结构）
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use treepp::scan::StreamStats;
///
/// let stats = StreamStats {
///     duration: Duration::from_millis(50),
///     directory_count: 3,
///     file_count: 10,
/// };
/// assert_eq!(stats.directory_count, 3);
/// ```
#[derive(Debug, Clone)]
pub struct StreamStats {
    /// 扫描耗时
    pub duration: Duration,
    /// 目录总数（不含根）
    pub directory_count: usize,
    /// 文件总数
    pub file_count: usize,
}

/// 流式扫描事件类型
///
/// 用于通知回调当前扫描状态变化。
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 进入目录（在处理完目录条目后、处理其子项前触发）
    EnterDir {
        /// 是否为当前层级最后一个目录
        is_last: bool,
    },
    /// 离开目录（在处理完目录所有子项后触发）
    LeaveDir,
    /// 发现条目
    Entry(StreamEntry),
}

// ============================================================================
// 匹配规则
// ============================================================================

/// 编译单个通配符模式
fn compile_pattern(pattern: &str) -> Result<Pattern, MatchError> {
    Pattern::new(pattern).map_err(|e| MatchError::InvalidPattern {
        pattern: pattern.to_string(),
        reason: e.msg.to_string(),
    })
}

/// 编译后的匹配规则集
struct CompiledRules {
    /// 包含模式
    include_patterns: Vec<Pattern>,
    /// 排除模式
    exclude_patterns: Vec<Pattern>,
}

impl CompiledRules {
    /// 从配置编译匹配规则
    fn compile(config: &Config) -> Result<Self, MatchError> {
        let include_patterns = config
            .matching
            .include_patterns
            .iter()
            .map(|p| compile_pattern(p))
            .collect::<Result<Vec<_>, _>>()?;

        let exclude_patterns = config
            .matching
            .exclude_patterns
            .iter()
            .map(|p| compile_pattern(p))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            include_patterns,
            exclude_patterns,
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

        self.include_patterns.iter().any(|p| p.matches(name))
    }

    /// 检查名称是否应被排除
    fn should_exclude(&self, name: &str) -> bool {
        if self.exclude_patterns.is_empty() {
            return false;
        }

        self.exclude_patterns.iter().any(|p| p.matches(name))
    }
}

// ============================================================================
// Gitignore 支持
// ============================================================================

/// Gitignore 规则链
///
/// 支持多级目录的 gitignore 规则叠加，子目录继承父目录规则。
/// 使用 `Arc<Gitignore>` 实现跨线程安全共享。
#[derive(Clone, Default)]
struct GitignoreChain {
    /// 规则链（从根到当前目录），使用 Arc 共享以支持多线程
    rules: Vec<Arc<Gitignore>>,
}

impl GitignoreChain {
    /// 创建空的规则链
    fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 创建包含新规则的子链
    ///
    /// 返回一个新的链，包含当前链的所有规则加上新规则。
    fn with_child(&self, gitignore: Arc<Gitignore>) -> Self {
        let mut new_chain = self.clone();
        new_chain.rules.push(gitignore);
        new_chain
    }

    /// 检查路径是否被任一规则忽略
    ///
    /// 从最具体（最深层）的规则开始检查，支持白名单规则。
    fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        // 从最具体（最深层）的规则开始检查
        for gi in self.rules.iter().rev() {
            let matched = gi.matched(path, is_dir);
            if matched.is_ignore() {
                return true;
            }
            if matched.is_whitelist() {
                return false;
            }
        }
        false
    }
}

/// Gitignore 缓存
///
/// 避免重复加载同一目录的 .gitignore 文件。
/// 线程安全，支持并发访问。
struct GitignoreCache {
    /// 缓存映射：目录路径 -> 可选的 gitignore 规则
    cache: Mutex<HashMap<PathBuf, Option<Arc<Gitignore>>>>,
}

impl GitignoreCache {
    /// 创建新的缓存
    fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// 获取或加载指定目录的 gitignore 规则
    ///
    /// 如果已缓存则直接返回，否则从文件系统加载并缓存。
    fn get_or_load(&self, dir: &Path) -> Option<Arc<Gitignore>> {
        let mut cache = self.cache.lock().unwrap();

        if let Some(cached) = cache.get(dir) {
            return cached.clone();
        }

        let gitignore = load_gitignore_from_path(dir).map(Arc::new);
        cache.insert(dir.to_path_buf(), gitignore.clone());
        gitignore
    }
}

/// 从指定目录加载 .gitignore 规则
fn load_gitignore_from_path(dir: &Path) -> Option<Gitignore> {
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

// ============================================================================
// 排序
// ============================================================================

/// 对树节点进行确定性排序（递归）
///
/// 按名称排序，文件优先（与原生 Windows tree 一致）。
pub fn sort_tree(node: &mut TreeNode, reverse: bool) {
    // 排序子节点
    node.children.sort_by(|a, b| {
        // 文件优先（与原生 Windows tree 一致）
        let kind_order = match (a.kind, b.kind) {
            (EntryKind::Directory, EntryKind::File) => std::cmp::Ordering::Greater,
            (EntryKind::File, EntryKind::Directory) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        };

        if kind_order != std::cmp::Ordering::Equal {
            return kind_order;
        }

        // 同类型内按名称排序
        let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());

        if reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });

    // 递归排序子节点的子节点
    for child in &mut node.children {
        sort_tree(child, reverse);
    }
}

/// 对条目列表进行排序（用于流式扫描）
///
/// 按名称排序，文件优先（与原生 Windows tree 一致）。
fn sort_entries(entries: &mut [(PathBuf, Metadata)], reverse: bool) {
    entries.sort_by(|(path_a, meta_a), (path_b, meta_b)| {
        let is_dir_a = meta_a.is_dir();
        let is_dir_b = meta_b.is_dir();

        // 文件优先（与原生 Windows tree 一致）
        let kind_order = match (is_dir_a, is_dir_b) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        };

        if kind_order != std::cmp::Ordering::Equal {
            return kind_order;
        }

        // 同类型内按名称排序
        let name_a = path_a
            .file_name()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let name_b = path_b
            .file_name()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let cmp = name_a.cmp(&name_b);

        if reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });
}

// ============================================================================
// 扫描上下文
// ============================================================================

/// 扫描上下文，包含所有扫描配置
struct ScanContext {
    /// 是否显示文件
    show_files: bool,
    /// 是否为大小计算收集文件（即使不显示文件）
    collect_files_for_size: bool,
    /// 最大递归深度
    max_depth: Option<usize>,
    /// 是否遵循 gitignore
    respect_gitignore: bool,
    /// 编译后的匹配规则
    rules: CompiledRules,
    /// 是否逆序
    reverse: bool,
    /// 是否修剪空目录
    prune_empty: bool,
    /// 是否需要大小信息
    needs_size: bool,
    /// Gitignore 缓存（多线程共享）
    gitignore_cache: Arc<GitignoreCache>,
}

impl ScanContext {
    /// 从配置创建扫描上下文
    fn from_config(config: &Config) -> Result<Self, MatchError> {
        Ok(Self {
            show_files: config.scan.show_files,
            // 当启用 disk_usage 时，即使不显示文件也需要收集文件信息用于大小计算
            collect_files_for_size: config.render.show_disk_usage,
            max_depth: config.scan.max_depth,
            respect_gitignore: config.scan.respect_gitignore,
            rules: CompiledRules::compile(config)?,
            reverse: config.render.reverse_sort,
            prune_empty: config.matching.prune_empty,
            needs_size: config.needs_size_info(),
            gitignore_cache: Arc::new(GitignoreCache::new()),
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

        // 文件显示检查：
        // - 如果需要收集文件用于大小计算（collect_files_for_size），则不过滤文件
        // - 否则根据 show_files 决定
        if !is_dir && !self.show_files && !self.collect_files_for_size {
            return true;
        }

        false
    }

    /// 获取或加载指定目录的 gitignore 规则
    fn get_gitignore(&self, dir: &Path) -> Option<Arc<Gitignore>> {
        if !self.respect_gitignore {
            return None;
        }
        self.gitignore_cache.get_or_load(dir)
    }
}

// ============================================================================
// 扫描实现
// ============================================================================

/// 扫描目录（内部递归函数）
///
/// 使用批量收集模式：先读取当前目录所有条目，再并行处理子目录。
fn scan_dir(
    path: &Path,
    depth: usize,
    ctx: &ScanContext,
    parent_chain: GitignoreChain,
) -> Option<TreeNode> {
    let meta = fs::metadata(path).ok()?;
    let kind = EntryKind::from_metadata(&meta);
    let metadata = EntryMetadata::from_fs_metadata(&meta);

    if kind != EntryKind::Directory {
        return Some(TreeNode::new(path.to_path_buf(), kind, metadata));
    }

    // 深度限制检查：如果已达到最大深度，返回空目录节点（不处理子项）
    if let Some(max) = ctx.max_depth
        && depth >= max {
        return Some(TreeNode::new(path.to_path_buf(), kind, metadata));
    }

    // 构建当前目录的 gitignore 链
    let current_chain = if ctx.respect_gitignore {
        if let Some(gi) = ctx.get_gitignore(path) {
            parent_chain.with_child(gi)
        } else {
            parent_chain
        }
    } else {
        parent_chain
    };

    // 批量读取目录条目
    let entries: Vec<_> = fs::read_dir(path).ok()?.flatten().collect();

    // 批量获取元数据并分类
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
        if ctx.respect_gitignore && current_chain.is_ignored(&entry_path, is_dir) {
            continue;
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
    let subdir_trees: Vec<TreeNode> = subdirs
        .into_par_iter()
        .filter_map(|subdir| scan_dir(&subdir, depth + 1, ctx, current_chain.clone()))
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

/// 执行目录扫描
///
/// 使用 rayon 线程池进行并行扫描。线程数由配置控制，
/// 即使设为 1 也使用相同的批量收集模式以保证性能一致性。
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

    // 初始化空的 gitignore 链
    let initial_chain = GitignoreChain::new();

    // 在线程池中执行扫描
    let root_path = config.root_path.clone();
    let mut tree = pool
        .install(|| scan_dir(&root_path, 0, &ctx, initial_chain))
        .ok_or_else(|| ScanError::ReadDirFailed {
            path: config.root_path.clone(),
            source: std::io::Error::other("无法读取根目录"),
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
    sort_tree(&mut tree, ctx.reverse);

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
// 流式扫描
// ============================================================================

/// 流式扫描（边扫边回调，支持实时输出）
///
/// 使用深度优先遍历顺序，在每个目录内批量获取元数据以保持性能。
/// 回调函数在发现每个条目时立即被调用，支持实时滚动输出。
///
/// # 参数
///
/// * `config` - 扫描配置
/// * `callback` - 事件回调函数，接收 `StreamEvent` 事件
///
/// # 返回值
///
/// 成功返回扫描统计信息（不含树结构）。
///
/// # Errors
///
/// 返回 `ScanError` 如果根路径不存在或回调返回错误。
///
/// # Examples
///
/// ```no_run
/// use treepp::config::Config;
/// use treepp::scan::{scan_streaming, StreamEvent, EntryKind};
/// use std::path::PathBuf;
///
/// let config = Config::with_root(PathBuf::from(".")).validate().unwrap();
/// let stats = scan_streaming(&config, |event| {
///     match event {
///         StreamEvent::Entry(entry) => {
///             println!("{}: {}", if entry.kind == EntryKind::Directory { "目录" } else { "文件" }, entry.name);
///         }
///         StreamEvent::EnterDir { .. } => {}
///         StreamEvent::LeaveDir => {}
///     }
///     Ok(())
/// }).expect("扫描失败");
/// println!("共 {} 个目录, {} 个文件", stats.directory_count, stats.file_count);
/// ```
pub fn scan_streaming<F>(config: &Config, mut callback: F) -> TreeppResult<StreamStats>
where
    F: FnMut(StreamEvent) -> Result<(), ScanError>,
{
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

    // 初始化 gitignore 链
    let initial_chain = GitignoreChain::new();

    // 执行流式扫描（不需要线程池，因为流式输出必须顺序执行）
    let (dir_count, file_count) =
        streaming_scan_dir(&config.root_path, 0, &ctx, &initial_chain, &mut callback)?;

    let duration = start.elapsed();

    Ok(StreamStats {
        duration,
        directory_count: dir_count,
        file_count,
    })
}

/// 流式扫描目录（内部递归函数）
fn streaming_scan_dir<F>(
    path: &Path,
    depth: usize,
    ctx: &ScanContext,
    parent_chain: &GitignoreChain,
    callback: &mut F,
) -> Result<(usize, usize), ScanError>
where
    F: FnMut(StreamEvent) -> Result<(), ScanError>,
{
    // 深度限制检查
    if let Some(max) = ctx.max_depth
        && depth >= max
    {
        return Ok((0, 0));
    }

    // 构建当前目录的 gitignore 链
    let current_chain = if ctx.respect_gitignore {
        if let Some(gi) = ctx.get_gitignore(path) {
            parent_chain.with_child(gi)
        } else {
            parent_chain.clone()
        }
    } else {
        parent_chain.clone()
    };

    // 批量读取目录条目（权限错误时跳过而非崩溃）
    let raw_entries: Vec<_> = match fs::read_dir(path) {
        Ok(entries) => entries.flatten().collect(),
        Err(_) => {
            // 无法读取目录（权限不足等），静默跳过
            return Ok((0, 0));
        }
    };

    // 批量获取元数据
    let entries_with_meta: Vec<(PathBuf, Metadata)> = raw_entries
        .into_iter()
        .filter_map(|entry| {
            let entry_path = entry.path();
            let meta = entry.metadata().ok()?;
            Some((entry_path, meta))
        })
        .collect();

    // 过滤
    let mut filtered: Vec<(PathBuf, Metadata)> = entries_with_meta
        .into_iter()
        .filter(|(entry_path, meta)| {
            let entry_name = entry_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            let is_dir = meta.is_dir();

            // gitignore 检查
            if ctx.respect_gitignore && current_chain.is_ignored(entry_path, is_dir) {
                return false;
            }

            // 过滤检查
            !ctx.should_filter(&entry_name, is_dir)
        })
        .collect();

    // 排序：文件优先（与原生 tree 一致）
    sort_entries(&mut filtered, ctx.reverse);

    // 分离文件和目录，保持各自内部顺序
    let mut files: Vec<(PathBuf, Metadata)> = Vec::new();
    let mut dirs: Vec<(PathBuf, Metadata)> = Vec::new();

    for (entry_path, meta) in filtered {
        if meta.is_dir() {
            dirs.push((entry_path, meta));
        } else {
            files.push((entry_path, meta));
        }
    }

    // 统计计数
    let mut dir_count = 0;
    let mut file_count = 0;

    // 先处理文件
    let file_total = files.len();
    for (i, (entry_path, meta)) in files.into_iter().enumerate() {
        let is_last_file = i == file_total - 1;
        let is_last_overall = is_last_file && dirs.is_empty();
        let entry_meta = EntryMetadata::from_fs_metadata(&meta);
        let name = entry_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        let entry = StreamEntry {
            path: entry_path,
            name,
            kind: EntryKind::File,
            metadata: entry_meta,
            depth,
            is_last: is_last_overall,
            is_file: true,
            has_more_dirs: !dirs.is_empty(),
        };
        callback(StreamEvent::Entry(entry))?;
        file_count += 1;
    }

    // 再处理目录
    let dir_total = dirs.len();
    for (i, (entry_path, meta)) in dirs.into_iter().enumerate() {
        let is_last = i == dir_total - 1;
        let entry_meta = EntryMetadata::from_fs_metadata(&meta);
        let name = entry_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        let entry = StreamEntry {
            path: entry_path.clone(),
            name,
            kind: EntryKind::Directory,
            metadata: entry_meta,
            depth,
            is_last,
            is_file: false,
            has_more_dirs: !is_last,
        };
        callback(StreamEvent::Entry(entry))?;
        dir_count += 1;

        // 发送进入目录事件
        callback(StreamEvent::EnterDir { is_last })?;

        // 递归处理子目录
        let (sub_dirs, sub_files) =
            streaming_scan_dir(&entry_path, depth + 1, ctx, &current_chain, callback)?;
        dir_count += sub_dirs;
        file_count += sub_files;

        // 发送离开目录事件
        callback(StreamEvent::LeaveDir)?;
    }

    Ok((dir_count, file_count))
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

    // ------------------------------------------------------------------------
    // 测试辅助函数
    // ------------------------------------------------------------------------

    /// 创建基本测试目录结构
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

    /// 创建嵌套 .gitignore 的测试目录
    fn setup_nested_gitignore_dir() -> TempDir {
        let dir = TempDir::new().expect("创建临时目录失败");
        let root = dir.path();

        // 创建目录结构
        fs::create_dir(root.join("level1")).unwrap();
        fs::create_dir(root.join("level1/level2")).unwrap();
        fs::create_dir(root.join("level1/level2/level3")).unwrap();

        // 根目录 .gitignore：忽略 *.tmp
        File::create(root.join(".gitignore"))
            .unwrap()
            .write_all(b"*.tmp\n")
            .unwrap();

        // level1 .gitignore：忽略 *.bak
        File::create(root.join("level1/.gitignore"))
            .unwrap()
            .write_all(b"*.bak\n")
            .unwrap();

        // level2 .gitignore：忽略 *.cache
        File::create(root.join("level1/level2/.gitignore"))
            .unwrap()
            .write_all(b"*.cache\n")
            .unwrap();

        // 创建各种应被忽略的文件
        File::create(root.join("root.tmp"))
            .unwrap()
            .write_all(b"")
            .unwrap();
        File::create(root.join("root.txt"))
            .unwrap()
            .write_all(b"")
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

        File::create(root.join("level1/level2/level3/l3.tmp"))
            .unwrap()
            .write_all(b"")
            .unwrap();
        File::create(root.join("level1/level2/level3/l3.bak"))
            .unwrap()
            .write_all(b"")
            .unwrap();
        File::create(root.join("level1/level2/level3/l3.cache"))
            .unwrap()
            .write_all(b"")
            .unwrap();
        File::create(root.join("level1/level2/level3/l3.txt"))
            .unwrap()
            .write_all(b"")
            .unwrap();

        dir
    }

    /// 收集树中所有节点名称（递归）
    fn collect_names(node: &TreeNode) -> Vec<String> {
        let mut names = vec![node.name.clone()];
        for child in &node.children {
            names.extend(collect_names(child));
        }
        names.sort();
        names
    }

    /// 检查树中是否存在指定名称的节点
    fn has_node_with_name(node: &TreeNode, name: &str) -> bool {
        if node.name == name {
            return true;
        }
        node.children.iter().any(|c| has_node_with_name(c, name))
    }

    // ------------------------------------------------------------------------
    // EntryKind 测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_entry_kind_from_metadata_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let file_meta = fs::metadata(&file_path).unwrap();
        assert_eq!(EntryKind::from_metadata(&file_meta), EntryKind::File);
    }

    #[test]
    fn test_entry_kind_from_metadata_directory() {
        let dir = TempDir::new().unwrap();
        let dir_meta = fs::metadata(dir.path()).unwrap();
        assert_eq!(EntryKind::from_metadata(&dir_meta), EntryKind::Directory);
    }

    #[test]
    fn test_entry_kind_equality() {
        assert_eq!(EntryKind::Directory, EntryKind::Directory);
        assert_eq!(EntryKind::File, EntryKind::File);
        assert_ne!(EntryKind::Directory, EntryKind::File);
    }

    // ------------------------------------------------------------------------
    // EntryMetadata 测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_entry_metadata_default() {
        let meta = EntryMetadata::default();
        assert_eq!(meta.size, 0);
        assert!(meta.modified.is_none());
        assert!(meta.created.is_none());
    }

    #[test]
    fn test_entry_metadata_from_fs_metadata() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path)
            .unwrap()
            .write_all(b"hello world")
            .unwrap();

        let fs_meta = fs::metadata(&file_path).unwrap();
        let entry_meta = EntryMetadata::from_fs_metadata(&fs_meta);

        assert_eq!(entry_meta.size, 11); // "hello world" = 11 bytes
        assert!(entry_meta.modified.is_some());
    }

    #[test]
    fn test_entry_metadata_directory_size_is_zero() {
        let dir = TempDir::new().unwrap();
        let fs_meta = fs::metadata(dir.path()).unwrap();
        let entry_meta = EntryMetadata::from_fs_metadata(&fs_meta);

        assert_eq!(entry_meta.size, 0);
    }

    // ------------------------------------------------------------------------
    // TreeNode 测试
    // ------------------------------------------------------------------------

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
        assert_eq!(node.path, PathBuf::from("/test/main.rs"));
        assert_eq!(node.kind, EntryKind::File);
        assert_eq!(node.metadata.size, 1024);
        assert!(node.children.is_empty());
        assert!(node.disk_usage.is_none());
    }

    #[test]
    fn test_tree_node_with_children() {
        let child = TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        );

        let node = TreeNode::with_children(
            PathBuf::from("dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
            vec![child],
        );

        assert_eq!(node.name, "dir");
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].name, "file.txt");
    }

    #[test]
    fn test_tree_node_count_files_single() {
        let node = TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        );
        assert_eq!(node.count_files(), 1);
    }

    #[test]
    fn test_tree_node_count_files_nested() {
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
    fn test_tree_node_count_directories_empty() {
        let root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        assert_eq!(root.count_directories(), 0);
    }

    #[test]
    fn test_tree_node_count_directories_nested() {
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
    fn test_tree_node_is_empty_dir_empty() {
        let empty = TreeNode::new(
            PathBuf::from("empty"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        assert!(empty.is_empty_dir());
    }

    #[test]
    fn test_tree_node_is_empty_dir_file() {
        let file = TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        );
        assert!(!file.is_empty_dir());
    }

    #[test]
    fn test_tree_node_is_empty_dir_with_content() {
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
    fn test_tree_node_is_empty_dir_nested_empty() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("empty_child"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));
        // 只含空子目录也算空
        assert!(root.is_empty_dir());
    }

    #[test]
    fn test_tree_node_compute_disk_usage_single_file() {
        let mut file = TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 100,
                ..Default::default()
            },
        );
        let usage = file.compute_disk_usage();
        assert_eq!(usage, 100);
    }

    #[test]
    fn test_tree_node_compute_disk_usage_directory() {
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
    fn test_tree_node_compute_disk_usage_nested() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut subdir = TreeNode::new(
            PathBuf::from("sub"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        subdir.children.push(TreeNode::new(
            PathBuf::from("nested.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 50,
                ..Default::default()
            },
        ));

        root.children.push(subdir);
        root.children.push(TreeNode::new(
            PathBuf::from("root.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 100,
                ..Default::default()
            },
        ));

        root.compute_disk_usage();
        assert_eq!(root.disk_usage, Some(150));
    }

    #[test]
    fn test_tree_node_prune_empty_dirs_removes_empty() {
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
    fn test_tree_node_prune_empty_dirs_keeps_non_empty() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut non_empty = TreeNode::new(
            PathBuf::from("non_empty"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        non_empty.children.push(TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(non_empty);

        root.prune_empty_dirs();
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].name, "non_empty");
    }

    #[test]
    fn test_tree_node_prune_empty_dirs_recursive() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut level1 = TreeNode::new(
            PathBuf::from("level1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        level1.children.push(TreeNode::new(
            PathBuf::from("empty_nested"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));
        root.children.push(level1);

        root.prune_empty_dirs();
        assert!(root.children.is_empty());
    }

    // ------------------------------------------------------------------------
    // StreamEntry 测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_stream_entry_creation() {
        let entry = StreamEntry {
            path: PathBuf::from("src/main.rs"),
            name: "main.rs".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata {
                size: 1024,
                ..Default::default()
            },
            depth: 1,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        assert_eq!(entry.name, "main.rs");
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.depth, 1);
        assert!(entry.is_last);
        assert!(entry.is_file);
        assert!(!entry.has_more_dirs);
        assert_eq!(entry.metadata.size, 1024);
    }

    #[test]
    fn test_stream_stats_creation() {
        let stats = StreamStats {
            duration: Duration::from_millis(100),
            directory_count: 5,
            file_count: 20,
        };

        assert_eq!(stats.directory_count, 5);
        assert_eq!(stats.file_count, 20);
        assert_eq!(stats.duration.as_millis(), 100);
    }

    // ------------------------------------------------------------------------
    // GitignoreChain 测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_gitignore_chain_new_empty() {
        let chain = GitignoreChain::new();
        assert!(!chain.is_ignored(Path::new("test.txt"), false));
        assert!(!chain.is_ignored(Path::new("anything"), true));
    }

    #[test]
    fn test_gitignore_chain_with_child() {
        let chain1 = GitignoreChain::new();
        let chain2 = chain1.clone();

        // 验证克隆独立
        assert_eq!(chain1.rules.len(), chain2.rules.len());
    }

    #[test]
    fn test_gitignore_chain_is_ignored_basic() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join(".gitignore"))
            .unwrap()
            .write_all(b"*.log\n")
            .unwrap();

        let gi = load_gitignore_from_path(root).unwrap();
        let chain = GitignoreChain::new().with_child(Arc::new(gi));

        assert!(chain.is_ignored(&root.join("test.log"), false));
        assert!(!chain.is_ignored(&root.join("test.txt"), false));
    }

    // ------------------------------------------------------------------------
    // GitignoreCache 测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_gitignore_cache_returns_none_for_missing() {
        let dir = TempDir::new().unwrap();
        let cache = GitignoreCache::new();

        let result = cache.get_or_load(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_gitignore_cache_loads_existing() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join(".gitignore"))
            .unwrap()
            .write_all(b"*.log\n")
            .unwrap();

        let cache = GitignoreCache::new();
        let result = cache.get_or_load(root);

        assert!(result.is_some());
    }

    #[test]
    fn test_gitignore_cache_caches_result() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join(".gitignore"))
            .unwrap()
            .write_all(b"*.log\n")
            .unwrap();

        let cache = GitignoreCache::new();

        let result1 = cache.get_or_load(root);
        let result2 = cache.get_or_load(root);

        assert!(result1.is_some());
        assert!(result2.is_some());

        // 验证返回的是同一个 Arc
        assert!(Arc::ptr_eq(&result1.unwrap(), &result2.unwrap()));
    }

    #[test]
    fn test_gitignore_cache_caches_none() {
        let dir = TempDir::new().unwrap();
        let cache = GitignoreCache::new();

        let _result1 = cache.get_or_load(dir.path());
        let _result2 = cache.get_or_load(dir.path());

        // 验证缓存了 None 结果
        let inner = cache.cache.lock().unwrap();
        assert!(inner.contains_key(dir.path()));
    }

    #[test]
    fn test_sort_tree_by_name() {
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

        sort_tree(&mut root, false);

        assert_eq!(root.children[0].name, "alpha.txt");
        assert_eq!(root.children[1].name, "beta.txt");
        assert_eq!(root.children[2].name, "zebra.txt");
    }

    #[test]
    fn test_sort_tree_reverse() {
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

        sort_tree(&mut root, true);

        assert_eq!(root.children[0].name, "c.txt");
        assert_eq!(root.children[1].name, "b.txt");
        assert_eq!(root.children[2].name, "a.txt");
    }

    #[test]
    fn test_sort_tree_files_before_dirs() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        sort_tree(&mut root, false);

        // 文件应该在目录之前
        assert_eq!(root.children[0].kind, EntryKind::File);
        assert_eq!(root.children[1].kind, EntryKind::Directory);
    }

    #[test]
    fn test_sort_tree_recursive() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut subdir = TreeNode::new(
            PathBuf::from("sub"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        subdir.children.push(TreeNode::new(
            PathBuf::from("z.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        subdir.children.push(TreeNode::new(
            PathBuf::from("a.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        root.children.push(subdir);

        sort_tree(&mut root, false);

        // 验证子目录内容也被排序
        assert_eq!(root.children[0].children[0].name, "a.txt");
        assert_eq!(root.children[0].children[1].name, "z.txt");
    }

    #[test]
    fn test_sort_entries_by_name() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // 创建文件
        File::create(root.join("zebra.txt")).unwrap();
        File::create(root.join("alpha.txt")).unwrap();
        File::create(root.join("beta.txt")).unwrap();

        let mut entries: Vec<(PathBuf, Metadata)> = fs::read_dir(root)
            .unwrap()
            .flatten()
            .filter_map(|e| {
                let path = e.path();
                let meta = e.metadata().ok()?;
                Some((path, meta))
            })
            .collect();

        sort_entries(&mut entries, false);

        let names: Vec<_> = entries
            .iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["alpha.txt", "beta.txt", "zebra.txt"]);
    }

    // ------------------------------------------------------------------------
    // 匹配规则测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_compile_pattern_basic() {
        let pattern = compile_pattern("*.rs").expect("编译失败");
        assert!(pattern.matches("main.rs"));
        assert!(pattern.matches("lib.rs"));
        assert!(!pattern.matches("main.txt"));
    }

    #[test]
    fn test_compile_pattern_invalid() {
        let result = compile_pattern("[invalid");
        assert!(result.is_err());

        if let Err(MatchError::InvalidPattern { pattern, .. }) = result {
            assert_eq!(pattern, "[invalid");
        } else {
            panic!("期望 InvalidPattern 错误");
        }
    }

    #[test]
    fn test_compiled_rules_should_include_with_pattern() {
        let mut config = Config::default();
        config.matching.include_patterns = vec!["*.rs".to_string()];

        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_include("main.rs", false));
        assert!(!rules.should_include("main.txt", false));
    }

    #[test]
    fn test_compiled_rules_should_include_no_patterns() {
        let config = Config::default();
        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_include("any.rs", false));
        assert!(rules.should_include("any.txt", false));
    }

    #[test]
    fn test_compiled_rules_should_include_directory_always() {
        let mut config = Config::default();
        config.matching.include_patterns = vec!["*.rs".to_string()];

        let rules = CompiledRules::compile(&config).unwrap();

        // 目录始终包含
        assert!(rules.should_include("src", true));
        assert!(rules.should_include("tests", true));
    }

    #[test]
    fn test_compiled_rules_should_exclude_no_patterns() {
        let config = Config::default();
        let rules = CompiledRules::compile(&config).unwrap();

        assert!(!rules.should_exclude("any.rs"));
        assert!(!rules.should_exclude("any.txt"));
    }

    #[test]
    fn test_compiled_rules_should_exclude_with_pattern() {
        let mut config = Config::default();
        config.matching.exclude_patterns = vec!["*.log".to_string()];

        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_exclude("app.log"));
        assert!(!rules.should_exclude("app.txt"));
    }

    // ------------------------------------------------------------------------
    // 基本扫描测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_basic() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3); // src, tests, empty
        assert_eq!(stats.file_count, 5); // Cargo.toml, README.md, main.rs, lib.rs, test.rs
    }

    #[test]
    fn test_scan_dirs_only() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn test_scan_max_depth_zero() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.max_depth = Some(0);

        let stats = scan(&config).expect("扫描失败");

        // 深度 0 只包含根目录本身
        assert_eq!(stats.directory_count, 0);
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn test_scan_max_depth_one() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.max_depth = Some(1);

        let stats = scan(&config).expect("扫描失败");

        // 深度 1 包含根目录下的直接子项
        assert_eq!(stats.directory_count, 3); // src, tests, empty
        assert_eq!(stats.file_count, 2); // Cargo.toml, README.md
    }

    #[test]
    fn test_scan_with_prune() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.prune_empty = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 2); // src, tests (empty 被修剪)
    }

    #[test]
    fn test_scan_with_include() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 3); // main.rs, lib.rs, test.rs
    }

    #[test]
    fn test_scan_with_exclude() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.exclude_patterns = vec!["*.md".to_string()];

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 4); // 不包含 README.md
        assert!(!has_node_with_name(&stats.tree, "README.md"));
    }

    #[test]
    fn test_scan_nonexistent_path() {
        let config = Config::with_root(PathBuf::from("/nonexistent/path/12345"));
        let result = scan(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_file_as_root() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("file.txt");
        File::create(&file_path).unwrap();

        let config = Config::with_root(file_path);
        let result = scan(&config);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Gitignore 扫描测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_with_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan(&config).expect("扫描失败");

        // target 目录和 .log 文件应被忽略
        assert!(!has_node_with_name(&stats.tree, "target"));
        assert!(!has_node_with_name(&stats.tree, "app.log"));

        // 其他文件应存在
        assert!(has_node_with_name(&stats.tree, "Cargo.toml"));
    }

    #[test]
    fn test_scan_without_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = false;

        let stats = scan(&config).expect("扫描失败");

        // 不启用 gitignore 时，所有文件都应存在
        assert!(has_node_with_name(&stats.tree, "target"));
        assert!(has_node_with_name(&stats.tree, "app.log"));
    }

    #[test]
    fn test_scan_nested_gitignore_inheritance() {
        let dir = setup_nested_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan(&config).expect("扫描失败");

        // 验证根目录规则生效：所有 *.tmp 被忽略
        assert!(!has_node_with_name(&stats.tree, "root.tmp"));
        assert!(!has_node_with_name(&stats.tree, "l1.tmp"));
        assert!(!has_node_with_name(&stats.tree, "l2.tmp"));
        assert!(!has_node_with_name(&stats.tree, "l3.tmp"));

        // 验证 level1 规则生效：level1 及以下 *.bak 被忽略
        assert!(!has_node_with_name(&stats.tree, "l1.bak"));
        assert!(!has_node_with_name(&stats.tree, "l2.bak"));
        assert!(!has_node_with_name(&stats.tree, "l3.bak"));

        // 验证 level2 规则生效：level2 及以下 *.cache 被忽略
        assert!(!has_node_with_name(&stats.tree, "l2.cache"));
        assert!(!has_node_with_name(&stats.tree, "l3.cache"));

        // 验证正常文件存在
        assert!(has_node_with_name(&stats.tree, "root.txt"));
        assert!(has_node_with_name(&stats.tree, "l1.txt"));
        assert!(has_node_with_name(&stats.tree, "l2.txt"));
        assert!(has_node_with_name(&stats.tree, "l3.txt"));
    }

    // ------------------------------------------------------------------------
    // 不同线程数测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_single_thread() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.thread_count = std::num::NonZeroUsize::new(1).unwrap();

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 5);
    }

    #[test]
    fn test_scan_multi_thread() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.thread_count = std::num::NonZeroUsize::new(4).unwrap();

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 5);
    }

    #[test]
    fn test_scan_thread_count_consistency() {
        let dir = setup_test_dir();

        let mut config1 = Config::with_root(dir.path().to_path_buf());
        config1.scan.show_files = true;
        config1.scan.thread_count = std::num::NonZeroUsize::new(1).unwrap();

        let mut config4 = Config::with_root(dir.path().to_path_buf());
        config4.scan.show_files = true;
        config4.scan.thread_count = std::num::NonZeroUsize::new(4).unwrap();

        let mut config8 = Config::with_root(dir.path().to_path_buf());
        config8.scan.show_files = true;
        config8.scan.thread_count = std::num::NonZeroUsize::new(8).unwrap();

        let stats1 = scan(&config1).expect("单线程扫描失败");
        let stats4 = scan(&config4).expect("4线程扫描失败");
        let stats8 = scan(&config8).expect("8线程扫描失败");

        // 所有线程数配置应产生相同结果
        assert_eq!(stats1.file_count, stats4.file_count);
        assert_eq!(stats4.file_count, stats8.file_count);
        assert_eq!(stats1.directory_count, stats4.directory_count);
        assert_eq!(stats4.directory_count, stats8.directory_count);

        // 验证名称集合一致
        let names1 = collect_names(&stats1.tree);
        let names4 = collect_names(&stats4.tree);
        let names8 = collect_names(&stats8.tree);
        assert_eq!(names1, names4);
        assert_eq!(names4, names8);
    }

    #[test]
    fn test_scan_with_gitignore_thread_consistency() {
        let dir = setup_nested_gitignore_dir();

        let mut config1 = Config::with_root(dir.path().to_path_buf());
        config1.scan.show_files = true;
        config1.scan.respect_gitignore = true;
        config1.scan.thread_count = std::num::NonZeroUsize::new(1).unwrap();

        let mut config8 = Config::with_root(dir.path().to_path_buf());
        config8.scan.show_files = true;
        config8.scan.respect_gitignore = true;
        config8.scan.thread_count = std::num::NonZeroUsize::new(8).unwrap();

        let stats1 = scan(&config1).expect("单线程扫描失败");
        let stats8 = scan(&config8).expect("8线程扫描失败");

        assert_eq!(stats1.file_count, stats8.file_count);
        assert_eq!(stats1.directory_count, stats8.directory_count);

        let names1 = collect_names(&stats1.tree);
        let names8 = collect_names(&stats8.tree);
        assert_eq!(names1, names8);
    }

    // ------------------------------------------------------------------------
    // 流式扫描测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_streaming_basic() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let mut entries = Vec::new();
        let mut enter_count = 0;
        let mut leave_count = 0;

        let stats = scan_streaming(&config, |event| {
            match event {
                StreamEvent::Entry(entry) => {
                    entries.push(entry);
                }
                StreamEvent::EnterDir { .. } => {
                    enter_count += 1;
                }
                StreamEvent::LeaveDir => {
                    leave_count += 1;
                }
            }
            Ok(())
        })
            .expect("流式扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 5);
        assert_eq!(entries.len(), 8); // 3 目录 + 5 文件
        assert_eq!(enter_count, 3); // 进入 3 个目录
        assert_eq!(leave_count, 3); // 离开 3 个目录
    }

    #[test]
    fn test_scan_streaming_dirs_only() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let mut entries = Vec::new();

        let stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                entries.push(entry);
            }
            Ok(())
        })
            .expect("流式扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 0);
        assert_eq!(entries.len(), 3);

        for entry in &entries {
            assert_eq!(entry.kind, EntryKind::Directory);
        }
    }

    #[test]
    fn test_scan_streaming_with_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let mut names = Vec::new();

        let _stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                names.push(entry.name.clone());
            }
            Ok(())
        })
            .expect("流式扫描失败");

        // 验证 target 和 app.log 被忽略
        assert!(!names.contains(&"target".to_string()));
        assert!(!names.contains(&"app.log".to_string()));
    }

    #[test]
    fn test_scan_streaming_depth_info() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let mut entries = Vec::new();

        let _stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                entries.push((entry.name.clone(), entry.depth));
            }
            Ok(())
        })
            .expect("流式扫描失败");

        // 根目录下的条目应该是 depth=0
        let root_entries: Vec<_> = entries.iter().filter(|(_, d)| *d == 0).collect();
        assert!(!root_entries.is_empty());

        // src 目录下的文件应该是 depth=1
        let nested_entries: Vec<_> = entries.iter().filter(|(_, d)| *d == 1).collect();
        assert!(!nested_entries.is_empty());
    }

    #[test]
    fn test_scan_streaming_is_last_flag() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // 创建简单结构
        File::create(root.join("a.txt")).unwrap();
        File::create(root.join("b.txt")).unwrap();
        File::create(root.join("c.txt")).unwrap();

        let mut config = Config::with_root(root.to_path_buf());
        config.scan.show_files = true;

        let mut entries = Vec::new();

        let _stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                entries.push((entry.name.clone(), entry.is_last));
            }
            Ok(())
        })
            .expect("流式扫描失败");

        // 应该有 3 个条目
        assert_eq!(entries.len(), 3);

        // 只有最后一个应该标记为 is_last
        let last_count = entries.iter().filter(|(_, is_last)| *is_last).count();
        assert_eq!(last_count, 1);
    }

    #[test]
    fn test_scan_streaming_nonexistent_path() {
        let config = Config::with_root(PathBuf::from("/nonexistent/path/12345"));

        let result = scan_streaming(&config, |_| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_streaming_max_depth() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.max_depth = Some(1);

        let mut entries = Vec::new();

        let stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                entries.push(entry);
            }
            Ok(())
        })
            .expect("流式扫描失败");

        // 深度 1 应该包含根目录下的直接子项
        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 2);
    }

    #[test]
    fn test_scan_streaming_callback_error_propagation() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let mut count = 0;
        let result = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(_) = event {
                count += 1;
                if count >= 2 {
                    return Err(ScanError::WalkError {
                        message: "测试错误".to_string(),
                        path: None,
                    });
                }
            }
            Ok(())
        });

        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // 流式扫描与批处理一致性测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_streaming_vs_batch_entry_names() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        // 批处理扫描
        let batch_stats = scan(&config).expect("批处理扫描失败");
        let batch_names = collect_names(&batch_stats.tree);

        // 流式扫描
        let mut stream_names = Vec::new();
        let _stream_stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                stream_names.push(entry.name.clone());
            }
            Ok(())
        })
            .expect("流式扫描失败");
        stream_names.sort();

        // 流式扫描不包含根节点名称，所以需要去掉批处理的根节点
        let batch_without_root: Vec<_> = batch_names
            .into_iter()
            .filter(|n| n != &batch_stats.tree.name)
            .collect();

        assert_eq!(stream_names, batch_without_root);
    }

    #[test]
    fn test_streaming_vs_batch_counts() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let batch_stats = scan(&config).expect("批处理扫描失败");
        let stream_stats = scan_streaming(&config, |_| Ok(())).expect("流式扫描失败");

        assert_eq!(batch_stats.directory_count, stream_stats.directory_count);
        assert_eq!(batch_stats.file_count, stream_stats.file_count);
    }

    // ------------------------------------------------------------------------
    // 边界条件测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_empty_directory() {
        let dir = TempDir::new().unwrap();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 0);
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn test_scan_single_file_directory() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("only.txt")).unwrap();

        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 1);
    }

    #[test]
    fn test_scan_deeply_nested() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // 创建 5 层嵌套
        let mut current = root.to_path_buf();
        for i in 0..5 {
            current = current.join(format!("level{}", i));
            fs::create_dir(&current).unwrap();
        }
        File::create(current.join("deep.txt")).unwrap();

        let mut config = Config::with_root(root.to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 5);
        assert_eq!(stats.file_count, 1);
        assert!(has_node_with_name(&stats.tree, "deep.txt"));
    }

    // ------------------------------------------------------------------------
    // 统计信息测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_stats_duration_is_measured() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let stats = scan(&config).expect("扫描失败");

        // 扫描应该花费一些时间（虽然可能很短）
        assert!(stats.duration.as_nanos() > 0);
    }

    #[test]
    fn test_scan_stats_counts_match_tree() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        // 验证统计信息与树结构一致
        assert_eq!(stats.directory_count, stats.tree.count_directories());
        assert_eq!(stats.file_count, stats.tree.count_files());
    }

    #[test]
    fn test_stream_stats_duration_is_measured() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let stats = scan_streaming(&config, |_| Ok(())).expect("扫描失败");

        assert!(stats.duration.as_nanos() > 0);
    }

    #[test]
    fn test_stream_entry_new_fields() {
        let entry = StreamEntry {
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            kind: EntryKind::File,
            metadata: EntryMetadata::default(),
            depth: 0,
            is_last: true,
            is_file: true,
            has_more_dirs: false,
        };

        assert!(entry.is_file);
        assert!(!entry.has_more_dirs);
    }

    #[test]
    fn test_streaming_scan_skips_permission_denied() {
        // 这个测试验证权限错误不会导致崩溃
        // 使用系统目录测试（如果存在）
        let system_dir = PathBuf::from("C:\\System Volume Information");
        if !system_dir.exists() {
            return; // 跳过测试
        }

        let mut config = Config::with_root(PathBuf::from("C:\\"));
        config.scan.show_files = true;
        config.scan.max_depth = Some(1);

        // 这应该不会崩溃
        let result = scan_streaming(&config, |_| Ok(()));
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------------
    // disk_usage 与 show_files 独立性测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_collects_files_for_disk_usage_even_without_show_files() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.batch_mode = true;
        config.scan.show_files = false; // 不显示文件
        config.render.show_disk_usage = true; // 但需要计算累计大小

        let stats = scan(&config).expect("扫描失败");

        // 验证树中包含文件（用于 disk_usage 计算）
        // 即使 show_files=false，扫描时也应该收集文件
        // 注意：渲染时会过滤掉文件，但扫描阶段应该包含
        assert!(stats.tree.disk_usage.is_some());
        assert!(stats.tree.disk_usage.unwrap() > 0);
    }

    #[test]
    fn test_scan_disk_usage_correct_without_show_files() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // 创建文件结构
        fs::create_dir(root.join("subdir")).unwrap();
        File::create(root.join("file1.txt"))
            .unwrap()
            .write_all(b"12345")
            .unwrap(); // 5 bytes
        File::create(root.join("subdir/file2.txt"))
            .unwrap()
            .write_all(b"1234567890")
            .unwrap(); // 10 bytes

        let mut config = Config::with_root(root.to_path_buf());
        config.batch_mode = true;
        config.scan.show_files = false;
        config.render.show_disk_usage = true;

        let stats = scan(&config).expect("扫描失败");

        // 验证根目录累计大小 = 5 + 10 = 15
        assert_eq!(stats.tree.disk_usage, Some(15));
    }

    #[test]
    fn test_scan_no_files_collected_when_neither_show_files_nor_disk_usage() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = false;
        config.render.show_disk_usage = false;
        // batch_mode 默认为 false

        let stats = scan(&config).expect("扫描失败");

        // 验证文件计数为 0
        assert_eq!(stats.file_count, 0);

        // 验证树中没有文件节点
        fn count_files_in_tree(node: &TreeNode) -> usize {
            let self_count = if node.kind == EntryKind::File { 1 } else { 0 };
            self_count + node.children.iter().map(count_files_in_tree).sum::<usize>()
        }
        assert_eq!(count_files_in_tree(&stats.tree), 0);
    }

    #[test]
    fn test_scan_context_collect_files_for_size_enabled_when_disk_usage() {
        let mut config = Config::default();
        config.batch_mode = true;
        config.render.show_disk_usage = true;
        config.scan.show_files = false;

        let ctx = ScanContext::from_config(&config).unwrap();

        assert!(!ctx.show_files);
        assert!(ctx.collect_files_for_size);
    }

    #[test]
    fn test_scan_context_collect_files_for_size_disabled_by_default() {
        let config = Config::default();

        let ctx = ScanContext::from_config(&config).unwrap();

        assert!(!ctx.show_files);
        assert!(!ctx.collect_files_for_size);
    }

    #[test]
    fn test_should_filter_includes_files_when_collect_for_size() {
        let mut config = Config::default();
        config.batch_mode = true;
        config.render.show_disk_usage = true;
        config.scan.show_files = false;

        let ctx = ScanContext::from_config(&config).unwrap();

        // 文件不应被过滤（因为需要收集用于大小计算）
        assert!(!ctx.should_filter("test.txt", false));
    }

    #[test]
    fn test_should_filter_excludes_files_when_no_show_no_collect() {
        let config = Config::default();
        // show_files = false, show_disk_usage = false

        let ctx = ScanContext::from_config(&config).unwrap();

        // 文件应被过滤
        assert!(ctx.should_filter("test.txt", false));
    }
}