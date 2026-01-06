//! 扫描模块：目录树扫描引擎与统一 IR
//!
//! 本模块负责目录树的遍历与构建，提供：
//!
//! - **统一 IR**：`TreeNode` 与 `EntryKind` 表示目录树结构
//! - **扫描统计**：`ScanStats` 记录扫描结果与耗时
//! - **双模式扫描**：单线程 `walk` 与多线程 `parallel` 模式，输出保证一致
//! - **过滤功能**：include/exclude 通配、ignore-case、level 限制、prune 空目录
//! - **gitignore 支持**：分层叠加 `.gitignore` 规则，支持规则链继承与缓存
//! - **确定性排序**：按 `SortKey` 排序，支持逆序
//!
//! 作者: WaterRun
//! 更新于: 2025-01-06

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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
pub fn sort_tree(node: &mut TreeNode, sort_key: SortKey, reverse: bool, dirs_first: bool) {
    // 排序子节点
    node.children.sort_by(|a, b| {
        // 目录优先排序（仅当 dirs_first 为 true 时）
        let kind_order = if dirs_first {
            match (a.kind, b.kind) {
                (EntryKind::Directory, EntryKind::File) => Ordering::Less,
                (EntryKind::File, EntryKind::Directory) => Ordering::Greater,
                _ => Ordering::Equal,
            }
        } else {
            Ordering::Equal
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
        sort_tree(child, sort_key, reverse, dirs_first);
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
    /// 是否目录优先
    dirs_first: bool,
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
            max_depth: config.scan.max_depth,
            respect_gitignore: config.scan.respect_gitignore,
            rules: CompiledRules::compile(config)?,
            sort_key: config.render.sort_key,
            reverse: config.render.reverse_sort,
            dirs_first: config.render.dirs_first,
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

        // 文件显示检查
        if !is_dir && !self.show_files {
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
// 单线程扫描
// ============================================================================

/// 单线程递归扫描
fn walk_recursive(
    path: &Path,
    depth: usize,
    ctx: &ScanContext,
    parent_chain: &GitignoreChain,
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

    // 构建当前目录的 gitignore 链
    let current_chain = if ctx.respect_gitignore && kind == EntryKind::Directory {
        if let Some(gi) = ctx.get_gitignore(path) {
            parent_chain.with_child(gi)
        } else {
            parent_chain.clone()
        }
    } else {
        parent_chain.clone()
    };

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
            if ctx.respect_gitignore && current_chain.is_ignored(&entry_path, is_dir) {
                continue;
            }

            // 过滤检查
            if ctx.should_filter(&entry_name, is_dir) {
                continue;
            }

            // 递归处理
            if let Some(child) = walk_recursive(&entry_path, depth + 1, ctx, &current_chain) {
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

    // 初始化空的 gitignore 链
    let initial_chain = GitignoreChain::new();

    // 执行扫描
    let mut tree =
        walk_recursive(&config.root_path, 0, &ctx, &initial_chain).ok_or_else(|| {
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
    sort_tree(&mut tree, ctx.sort_key, ctx.reverse, ctx.dirs_first);

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
    parent_chain: GitignoreChain,
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

    // 并行处理子目录，正确传递 gitignore 链
    let subdir_trees: Vec<TreeNode> = subdirs
        .into_par_iter()
        .filter_map(|subdir| {
            // 克隆当前链传递给子目录
            parallel_scan_dir(&subdir, depth + 1, ctx, current_chain.clone())
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

    // 初始化空的 gitignore 链
    let initial_chain = GitignoreChain::new();

    // 在线程池中执行扫描
    let root_path = config.root_path.clone();
    let mut tree = pool
        .install(|| parallel_scan_dir(&root_path, 0, &ctx, initial_chain))
        .ok_or_else(|| ScanError::ReadDirFailed {
            path: config.root_path.clone(),
            source: std::io::Error::new(std::io::ErrorKind::Other, "无法读取根目录"),
        })?;

    // 计算目录累计大小（如需要）
    if ctx.needs_size {
        tree.compute_disk_usage();
    }

    // 修剪空目录（如需要）
    if ctx.prune_empty {
        tree.prune_empty_dirs();
    }

    // 排序（确保与 walk 模式输出一致）
    sort_tree(&mut tree, ctx.sort_key, ctx.reverse, ctx.dirs_first);

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

    // ------------------------------------------------------------------------
    // 匹配规则测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_compile_pattern_basic() {
        let pattern = compile_pattern("*.rs", false).expect("编译失败");
        assert!(pattern.matches("main.rs"));
        assert!(pattern.matches("lib.rs"));
        assert!(!pattern.matches("main.txt"));
    }

    #[test]
    fn test_compile_pattern_case_sensitive() {
        let pattern = compile_pattern("*.RS", false).expect("编译失败");
        assert!(pattern.matches("main.RS"));
        assert!(!pattern.matches("main.rs"));
    }

    #[test]
    fn test_compile_pattern_ignore_case() {
        let pattern = compile_pattern("*.RS", true).expect("编译失败");
        // 忽略大小写时模式被转为小写
        assert!(pattern.matches("main.rs"));
        assert!(pattern.matches("lib.rs"));
    }

    #[test]
    fn test_compile_pattern_invalid() {
        let result = compile_pattern("[invalid", false);
        assert!(result.is_err());

        if let Err(MatchError::InvalidPattern { pattern, .. }) = result {
            assert_eq!(pattern, "[invalid");
        } else {
            panic!("期望 InvalidPattern 错误");
        }
    }

    #[test]
    fn test_compiled_rules_should_include_no_patterns() {
        let config = Config::default();
        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_include("any.rs", false));
        assert!(rules.should_include("any.txt", false));
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
    // 排序测试
    // ------------------------------------------------------------------------

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

        sort_tree(&mut root, SortKey::Name, false, false);

        assert_eq!(root.children[0].name, "alpha.txt");
        assert_eq!(root.children[1].name, "beta.txt");
        assert_eq!(root.children[2].name, "zebra.txt");
    }

    #[test]
    fn test_sort_tree_by_name_case_insensitive() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("Zebra.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("alpha.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("Beta.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        sort_tree(&mut root, SortKey::Name, false, false);

        assert_eq!(root.children[0].name, "alpha.txt");
        assert_eq!(root.children[1].name, "Beta.txt");
        assert_eq!(root.children[2].name, "Zebra.txt");
    }

    #[test]
    fn test_sort_tree_by_size() {
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

        sort_tree(&mut root, SortKey::Size, false, false);

        assert_eq!(root.children[0].name, "small.txt");
        assert_eq!(root.children[1].name, "medium.txt");
        assert_eq!(root.children[2].name, "large.txt");
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

        sort_tree(&mut root, SortKey::Name, true, false);

        assert_eq!(root.children[0].name, "c.txt");
        assert_eq!(root.children[1].name, "b.txt");
        assert_eq!(root.children[2].name, "a.txt");
    }

    #[test]
    fn test_sort_tree_dirs_first_disabled() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("z_file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("a_dir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("b_file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        // dirs_first = false，按名称排序，不区分目录和文件
        sort_tree(&mut root, SortKey::Name, false, false);

        assert_eq!(root.children[0].name, "a_dir");
        assert_eq!(root.children[1].name, "b_file.txt");
        assert_eq!(root.children[2].name, "z_file.txt");
    }

    #[test]
    fn test_sort_tree_dirs_before_files() {
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
        root.children.push(TreeNode::new(
            PathBuf::from("another.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        sort_tree(&mut root, SortKey::Name, false, true);

        assert_eq!(root.children[0].kind, EntryKind::Directory);
        assert_eq!(root.children[1].kind, EntryKind::File);
        assert_eq!(root.children[2].kind, EntryKind::File);
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

        sort_tree(&mut root, SortKey::Name, false, true);

        // 验证子目录内容也被排序
        assert_eq!(root.children[0].children[0].name, "a.txt");
        assert_eq!(root.children[0].children[1].name, "z.txt");
    }

    // ------------------------------------------------------------------------
    // 基本扫描测试
    // ------------------------------------------------------------------------

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
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn test_scan_walk_max_depth_zero() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.max_depth = Some(0);

        let stats = scan_walk(&config).expect("扫描失败");

        // 深度 0 只包含根目录本身
        assert_eq!(stats.directory_count, 0);
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn test_scan_walk_max_depth_one() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.max_depth = Some(1);

        let stats = scan_walk(&config).expect("扫描失败");

        // 深度 1 包含根目录下的直接子项
        assert_eq!(stats.directory_count, 3); // src, tests, empty
        assert_eq!(stats.file_count, 2); // Cargo.toml, README.md
    }

    #[test]
    fn test_scan_walk_with_prune() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.prune_empty = true;

        let stats = scan_walk(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 2); // src, tests (empty 被修剪)
    }

    #[test]
    fn test_scan_walk_with_include() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];

        let stats = scan_walk(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 3); // main.rs, lib.rs, test.rs
    }

    #[test]
    fn test_scan_walk_with_exclude() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.exclude_patterns = vec!["*.md".to_string()];

        let stats = scan_walk(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 4); // 不包含 README.md
        assert!(!has_node_with_name(&stats.tree, "README.md"));
    }

    #[test]
    fn test_scan_walk_nonexistent_path() {
        let config = Config::with_root(PathBuf::from("/nonexistent/path/12345"));
        let result = scan_walk(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_scan_walk_file_as_root() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("file.txt");
        File::create(&file_path).unwrap();

        let config = Config::with_root(file_path);
        let result = scan_walk(&config);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Gitignore 扫描测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_walk_with_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan_walk(&config).expect("扫描失败");

        // target 目录和 .log 文件应被忽略
        assert!(!has_node_with_name(&stats.tree, "target"));
        assert!(!has_node_with_name(&stats.tree, "app.log"));

        // 其他文件应存在
        assert!(has_node_with_name(&stats.tree, "Cargo.toml"));
    }

    #[test]
    fn test_scan_walk_without_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = false;

        let stats = scan_walk(&config).expect("扫描失败");

        // 不启用 gitignore 时，所有文件都应存在
        assert!(has_node_with_name(&stats.tree, "target"));
        assert!(has_node_with_name(&stats.tree, "app.log"));
    }

    #[test]
    fn test_scan_walk_nested_gitignore_inheritance() {
        let dir = setup_nested_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan_walk(&config).expect("扫描失败");

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
    // 多线程扫描测试
    // ------------------------------------------------------------------------

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
    fn test_scan_parallel_with_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan_parallel(&config).expect("扫描失败");

        assert!(!has_node_with_name(&stats.tree, "target"));
        assert!(!has_node_with_name(&stats.tree, "app.log"));
    }

    #[test]
    fn test_scan_parallel_nested_gitignore() {
        let dir = setup_nested_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan_parallel(&config).expect("扫描失败");

        // 验证嵌套规则生效
        assert!(!has_node_with_name(&stats.tree, "l3.tmp"));
        assert!(!has_node_with_name(&stats.tree, "l3.bak"));
        assert!(!has_node_with_name(&stats.tree, "l3.cache"));
        assert!(has_node_with_name(&stats.tree, "l3.txt"));
    }

    // ------------------------------------------------------------------------
    // 单线程与多线程一致性测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_walk_parallel_consistency_basic() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let walk_stats = scan_walk(&config).expect("walk 扫描失败");
        let parallel_stats = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk_stats.directory_count, parallel_stats.directory_count);
        assert_eq!(walk_stats.file_count, parallel_stats.file_count);

        let walk_names = collect_names(&walk_stats.tree);
        let parallel_names = collect_names(&parallel_stats.tree);
        assert_eq!(walk_names, parallel_names);
    }

    #[test]
    fn test_scan_walk_parallel_consistency_with_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let walk_stats = scan_walk(&config).expect("walk 扫描失败");
        let parallel_stats = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk_stats.directory_count, parallel_stats.directory_count);
        assert_eq!(walk_stats.file_count, parallel_stats.file_count);

        let walk_names = collect_names(&walk_stats.tree);
        let parallel_names = collect_names(&parallel_stats.tree);
        assert_eq!(walk_names, parallel_names);
    }

    #[test]
    fn test_scan_walk_parallel_consistency_nested_gitignore() {
        let dir = setup_nested_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let walk_stats = scan_walk(&config).expect("walk 扫描失败");
        let parallel_stats = scan_parallel(&config).expect("parallel 扫描失败");

        // 验证计数一致
        assert_eq!(
            walk_stats.directory_count, parallel_stats.directory_count,
            "目录数量不一致"
        );
        assert_eq!(
            walk_stats.file_count, parallel_stats.file_count,
            "文件数量不一致"
        );

        // 验证结构一致
        let walk_names = collect_names(&walk_stats.tree);
        let parallel_names = collect_names(&parallel_stats.tree);
        assert_eq!(walk_names, parallel_names, "节点名称集合不一致");
    }

    #[test]
    fn test_scan_walk_parallel_consistency_with_filters() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];
        config.matching.prune_empty = true;

        let walk_stats = scan_walk(&config).expect("walk 扫描失败");
        let parallel_stats = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk_stats.file_count, parallel_stats.file_count);

        let walk_names = collect_names(&walk_stats.tree);
        let parallel_names = collect_names(&parallel_stats.tree);
        assert_eq!(walk_names, parallel_names);
    }

    // ------------------------------------------------------------------------
    // 统一入口测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_unified_scan_selects_walk_for_single_thread() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.thread_count = std::num::NonZeroUsize::new(1).unwrap();

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 5);
    }

    #[test]
    fn test_unified_scan_selects_parallel_for_multi_thread() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.thread_count = std::num::NonZeroUsize::new(4).unwrap();

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 5);
    }

    #[test]
    fn test_unified_scan_consistency() {
        let dir = setup_test_dir();

        let mut config1 = Config::with_root(dir.path().to_path_buf());
        config1.scan.show_files = true;
        config1.scan.thread_count = std::num::NonZeroUsize::new(1).unwrap();

        let mut config8 = Config::with_root(dir.path().to_path_buf());
        config8.scan.show_files = true;
        config8.scan.thread_count = std::num::NonZeroUsize::new(8).unwrap();

        let stats1 = scan(&config1).expect("单线程扫描失败");
        let stats8 = scan(&config8).expect("多线程扫描失败");

        assert_eq!(stats1.file_count, stats8.file_count);
        assert_eq!(stats1.directory_count, stats8.directory_count);
    }

    // ------------------------------------------------------------------------
    // 边界条件测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_empty_directory() {
        let dir = TempDir::new().unwrap();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan_walk(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 0);
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn test_scan_single_file_directory() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("only.txt")).unwrap();

        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan_walk(&config).expect("扫描失败");

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

        let stats = scan_walk(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 5);
        assert_eq!(stats.file_count, 1);
        assert!(has_node_with_name(&stats.tree, "deep.txt"));
    }

    #[test]
    fn test_scan_with_ignore_case() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join("Test.RS")).unwrap();
        File::create(root.join("other.txt")).unwrap();

        let mut config = Config::with_root(root.to_path_buf());
        config.scan.show_files = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];
        config.matching.ignore_case = true;

        let stats = scan_walk(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 1);
        assert!(has_node_with_name(&stats.tree, "Test.RS"));
    }

    // ------------------------------------------------------------------------
    // 统计信息测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_scan_stats_duration_is_measured() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let stats = scan_walk(&config).expect("扫描失败");

        // 扫描应该花费一些时间（虽然可能很短）
        assert!(stats.duration.as_nanos() > 0);
    }

    #[test]
    fn test_scan_stats_counts_match_tree() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan_walk(&config).expect("扫描失败");

        // 验证统计信息与树结构一致
        assert_eq!(stats.directory_count, stats.tree.count_directories());
        assert_eq!(stats.file_count, stats.tree.count_files());
    }
}