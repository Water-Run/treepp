//! Directory tree scanning engine and unified intermediate representation.
//!
//! This module provides directory tree traversal and construction with:
//!
//! - **Unified IR**: `TreeNode` and `EntryKind` represent directory tree structure
//! - **Scan statistics**: `ScanStats` records scan results and timing
//! - **Parallel scanning**: Uses rayon divide-and-conquer strategy with configurable thread count
//! - **Streaming scanning**: `scan_streaming` supports callback-based real-time output
//! - **Filtering**: Include/exclude glob patterns, depth limits, empty directory pruning
//! - **Gitignore support**: Layered `.gitignore` rules with inheritance and caching
//! - **Deterministic sorting**: Windows-style sorting with optional reverse order
//!
//! File: src/scan.rs
//! Author: WaterRun
//! Date: 2026-01-13

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use glob::Pattern;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

use crate::config::Config;
use crate::error::{MatchError, ScanError, TreeppResult};

/// Filesystem entry type distinguishing directories from files.
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
    /// A directory entry.
    Directory,
    /// A file entry.
    File,
}

impl EntryKind {
    /// Creates an `EntryKind` from filesystem metadata.
    ///
    /// # Arguments
    ///
    /// * `meta` - Filesystem metadata to examine.
    ///
    /// # Returns
    ///
    /// `EntryKind::Directory` if the metadata indicates a directory,
    /// `EntryKind::File` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs;
    /// use treepp::scan::EntryKind;
    ///
    /// let meta = fs::metadata(".").unwrap();
    /// let kind = EntryKind::from_metadata(&meta);
    /// assert_eq!(kind, EntryKind::Directory);
    /// ```
    #[must_use]
    pub fn from_metadata(meta: &Metadata) -> Self {
        if meta.is_dir() {
            Self::Directory
        } else {
            Self::File
        }
    }
}

/// Metadata for a filesystem entry.
///
/// Stores additional information about files and directories for display and sorting.
///
/// # Examples
///
/// ```
/// use treepp::scan::EntryMetadata;
///
/// let meta = EntryMetadata::default();
/// assert_eq!(meta.size, 0);
/// assert!(meta.modified.is_none());
/// assert!(meta.created.is_none());
/// ```
#[derive(Debug, Clone, Default)]
pub struct EntryMetadata {
    /// File size in bytes. Always 0 for directories.
    pub size: u64,
    /// Last modification time, if available.
    pub modified: Option<SystemTime>,
    /// Creation time, if available.
    pub created: Option<SystemTime>,
}

impl EntryMetadata {
    /// Creates `EntryMetadata` from filesystem metadata.
    ///
    /// # Arguments
    ///
    /// * `meta` - Filesystem metadata to extract information from.
    ///
    /// # Returns
    ///
    /// A new `EntryMetadata` instance with size (for files only),
    /// modification time, and creation time populated from the metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::fs;
    /// use treepp::scan::EntryMetadata;
    ///
    /// let meta = fs::metadata("Cargo.toml").unwrap();
    /// let entry_meta = EntryMetadata::from_fs_metadata(&meta);
    /// assert!(entry_meta.size > 0);
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

/// A node in the directory tree structure.
///
/// Represents a single entry in the directory tree, which can recursively
/// contain child nodes. This is the unified intermediate representation (IR)
/// produced by scanning.
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
    /// Entry name without path components.
    pub name: String,
    /// Full path to the entry.
    pub path: PathBuf,
    /// Type of the entry (directory or file).
    pub kind: EntryKind,
    /// Entry metadata (size, timestamps).
    pub metadata: EntryMetadata,
    /// Child nodes (only populated for directories).
    pub children: Vec<TreeNode>,
    /// Cumulative size for disk usage display.
    pub disk_usage: Option<u64>,
}

impl TreeNode {
    /// Creates a new leaf node without children.
    ///
    /// # Arguments
    ///
    /// * `path` - Full path to the entry.
    /// * `kind` - Type of the entry.
    /// * `metadata` - Entry metadata.
    ///
    /// # Returns
    ///
    /// A new `TreeNode` with an empty children vector.
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
    /// assert!(node.children.is_empty());
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

    /// Creates a directory node with pre-existing children.
    ///
    /// # Arguments
    ///
    /// * `path` - Full path to the directory.
    /// * `kind` - Type of the entry (typically `EntryKind::Directory`).
    /// * `metadata` - Entry metadata.
    /// * `children` - Vector of child nodes.
    ///
    /// # Returns
    ///
    /// A new `TreeNode` with the specified children.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let child = TreeNode::new(
    ///     PathBuf::from("file.txt"),
    ///     EntryKind::File,
    ///     EntryMetadata::default(),
    /// );
    /// let parent = TreeNode::with_children(
    ///     PathBuf::from("dir"),
    ///     EntryKind::Directory,
    ///     EntryMetadata::default(),
    ///     vec![child],
    /// );
    /// assert_eq!(parent.children.len(), 1);
    /// ```
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

    /// Recursively counts the number of directories (excluding root).
    ///
    /// # Returns
    ///
    /// The total number of directory nodes in the subtree, not counting the
    /// current node.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let mut root = TreeNode::new(
    ///     PathBuf::from("."),
    ///     EntryKind::Directory,
    ///     EntryMetadata::default(),
    /// );
    /// root.children.push(TreeNode::new(
    ///     PathBuf::from("src"),
    ///     EntryKind::Directory,
    ///     EntryMetadata::default(),
    /// ));
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

    /// Recursively counts the number of files.
    ///
    /// # Returns
    ///
    /// The total number of file nodes in the subtree, including the current
    /// node if it is a file.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let mut root = TreeNode::new(
    ///     PathBuf::from("."),
    ///     EntryKind::Directory,
    ///     EntryMetadata::default(),
    /// );
    /// root.children.push(TreeNode::new(
    ///     PathBuf::from("main.rs"),
    ///     EntryKind::File,
    ///     EntryMetadata::default(),
    /// ));
    /// assert_eq!(root.count_files(), 1);
    /// ```
    #[must_use]
    pub fn count_files(&self) -> usize {
        let self_count = if self.kind == EntryKind::File { 1 } else { 0 };
        self_count + self.children.iter().map(Self::count_files).sum::<usize>()
    }

    /// Recursively computes and stores cumulative directory sizes.
    ///
    /// For files, returns the file size. For directories, computes the sum
    /// of all descendant file sizes and stores it in `disk_usage`.
    ///
    /// # Returns
    ///
    /// The cumulative size of this node and all descendants.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
    ///
    /// let mut root = TreeNode::new(
    ///     PathBuf::from("."),
    ///     EntryKind::Directory,
    ///     EntryMetadata::default(),
    /// );
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
}

/// Statistics from a completed scan operation.
///
/// Contains the resulting directory tree, timing information, and entry counts.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use std::time::Duration;
/// use treepp::scan::{ScanStats, TreeNode, EntryKind, EntryMetadata};
///
/// let tree = TreeNode::new(
///     PathBuf::from("."),
///     EntryKind::Directory,
///     EntryMetadata::default(),
/// );
/// let stats = ScanStats {
///     tree,
///     duration: Duration::from_millis(100),
///     directory_count: 5,
///     file_count: 20,
/// };
/// assert_eq!(stats.directory_count, 5);
/// assert_eq!(stats.file_count, 20);
/// ```
#[derive(Debug)]
pub struct ScanStats {
    /// Root node of the scanned tree.
    pub tree: TreeNode,
    /// Total scan duration.
    pub duration: Duration,
    /// Number of directories (excluding root).
    pub directory_count: usize,
    /// Number of files.
    pub file_count: usize,
}

/// An entry discovered during streaming scan.
///
/// Contains complete entry information plus position data for tree rendering.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::scan::{StreamEntry, EntryKind, EntryMetadata};
///
/// let entry = StreamEntry {
///     path: PathBuf::from("src/main.rs"),
///     name: "main.rs".to_string(),
///     kind: EntryKind::File,
///     metadata: EntryMetadata { size: 1024, ..Default::default() },
///     depth: 1,
///     is_last: true,
///     is_file: true,
///     has_more_dirs: false,
/// };
/// assert_eq!(entry.name, "main.rs");
/// assert!(entry.is_last);
/// ```
#[derive(Debug, Clone)]
pub struct StreamEntry {
    /// Full path to the entry.
    pub path: PathBuf,
    /// Entry name without path components.
    pub name: String,
    /// Type of the entry.
    pub kind: EntryKind,
    /// Entry metadata.
    pub metadata: EntryMetadata,
    /// Depth from root (root children have depth 0).
    pub depth: usize,
    /// Whether this is the last entry at its level.
    pub is_last: bool,
    /// Whether this entry is a file.
    pub is_file: bool,
    /// Whether more directories follow at this level.
    pub has_more_dirs: bool,
}

/// Simplified statistics for streaming scan (without tree structure).
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
/// assert_eq!(stats.file_count, 10);
/// ```
#[derive(Debug, Clone)]
pub struct StreamStats {
    /// Total scan duration.
    pub duration: Duration,
    /// Number of directories (excluding root).
    pub directory_count: usize,
    /// Number of files.
    pub file_count: usize,
}

/// Events emitted during streaming scan.
///
/// Used to notify callbacks about scan progress and discovered entries.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::scan::{StreamEvent, StreamEntry, EntryKind, EntryMetadata};
///
/// let entry = StreamEntry {
///     path: PathBuf::from("test.txt"),
///     name: "test.txt".to_string(),
///     kind: EntryKind::File,
///     metadata: EntryMetadata::default(),
///     depth: 0,
///     is_last: true,
///     is_file: true,
///     has_more_dirs: false,
/// };
/// let event = StreamEvent::Entry(entry);
/// ```
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Entering a directory (after processing the directory entry, before its children).
    EnterDir {
        /// Whether this is the last directory at its level.
        is_last: bool,
    },
    /// Leaving a directory (after processing all children).
    LeaveDir,
    /// A discovered entry.
    Entry(StreamEntry),
}

/// Compiles a glob pattern string into a `Pattern`.
///
/// # Arguments
///
/// * `pattern` - The glob pattern string to compile.
///
/// # Returns
///
/// A compiled `Pattern` on success, or a `MatchError` if the pattern is invalid.
///
/// # Errors
///
/// Returns `MatchError::InvalidPattern` if the pattern syntax is invalid.
///
/// # Examples
///
/// ```
/// use treepp::scan::compile_pattern;
///
/// let pattern = compile_pattern("*.rs").unwrap();
/// assert!(pattern.matches("main.rs"));
/// assert!(!pattern.matches("main.txt"));
/// ```
pub fn compile_pattern(pattern: &str) -> Result<Pattern, MatchError> {
    Pattern::new(pattern).map_err(|e| MatchError::InvalidPattern {
        pattern: pattern.to_string(),
        reason: e.msg.to_string(),
    })
}

/// Compiled include and exclude pattern sets.
struct CompiledRules {
    include_patterns: Vec<Pattern>,
    exclude_patterns: Vec<Pattern>,
}

impl CompiledRules {
    /// Compiles matching rules from configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing pattern strings.
    ///
    /// # Returns
    ///
    /// Compiled rules on success, or a `MatchError` if any pattern is invalid.
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

    /// Checks if a name should be included based on include patterns.
    ///
    /// Directories are always included. Files are included if no include
    /// patterns are specified, or if they match at least one pattern.
    fn should_include(&self, name: &str, is_dir: bool) -> bool {
        if is_dir {
            return true;
        }
        if self.include_patterns.is_empty() {
            return true;
        }
        self.include_patterns.iter().any(|p| p.matches(name))
    }

    /// Checks if a name should be excluded based on exclude patterns.
    fn should_exclude(&self, name: &str) -> bool {
        if self.exclude_patterns.is_empty() {
            return false;
        }
        self.exclude_patterns.iter().any(|p| p.matches(name))
    }
}

/// A chain of gitignore rules supporting inheritance.
///
/// Allows child directories to inherit and extend parent rules.
#[derive(Clone, Default)]
struct GitignoreChain {
    rules: Vec<Arc<Gitignore>>,
}

impl GitignoreChain {
    /// Creates an empty rule chain.
    fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Creates a new chain with an additional rule appended.
    fn with_child(&self, gitignore: Arc<Gitignore>) -> Self {
        let mut new_chain = self.clone();
        new_chain.rules.push(gitignore);
        new_chain
    }

    /// Checks if a path is ignored by any rule in the chain.
    ///
    /// Checks from most specific (deepest) to least specific, respecting
    /// whitelist rules.
    fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
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

/// Thread-safe cache for loaded gitignore files.
struct GitignoreCache {
    cache: Mutex<HashMap<PathBuf, Option<Arc<Gitignore>>>>,
}

impl GitignoreCache {
    /// Creates a new empty cache.
    fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Gets or loads the gitignore rules for a directory.
    ///
    /// Returns cached result if available, otherwise loads from disk and caches.
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

/// Loads gitignore rules from a directory's `.gitignore` file.
///
/// # Arguments
///
/// * `dir` - Directory to load `.gitignore` from.
///
/// # Returns
///
/// `Some(Gitignore)` if the file exists and parses successfully, `None` otherwise.
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

/// Returns the Windows-style sort priority for a character.
///
/// Priority groups (lower = earlier in sort order):
/// 1. ASCII punctuation (`.`, `-`, etc.)
/// 2. Digits (`0-9`)
/// 3. Letters (`A-Za-z`, case-insensitive)
/// 4. Underscore (`_`)
/// 5. Non-ASCII characters
#[inline]
fn windows_char_priority(c: char) -> (u8, char) {
    match c {
        '_' => (4, '_'),
        'A'..='Z' => (3, c.to_ascii_lowercase()),
        'a'..='z' => (3, c),
        '0'..='9' => (2, c),
        _ if c.is_ascii() => (1, c),
        _ => (5, c),
    }
}

/// Compares two strings using Windows tree command sort order.
///
/// # Arguments
///
/// * `a` - First string to compare.
/// * `b` - Second string to compare.
///
/// # Returns
///
/// Ordering result following Windows tree command conventions.
fn windows_compare_names(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars();
    let mut b_chars = b.chars();

    loop {
        match (a_chars.next(), b_chars.next()) {
            (Some(ca), Some(cb)) => {
                let (pri_a, char_a) = windows_char_priority(ca);
                let (pri_b, char_b) = windows_char_priority(cb);

                match pri_a.cmp(&pri_b) {
                    std::cmp::Ordering::Equal => match char_a.cmp(&char_b) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    },
                    other => return other,
                }
            }
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (None, None) => return std::cmp::Ordering::Equal,
        }
    }
}

/// Sorts tree nodes using Windows-style ordering.
///
/// Sorts recursively with files before directories, then by name using
/// Windows tree command conventions.
///
/// # Arguments
///
/// * `node` - Root node to sort (modified in place).
/// * `reverse` - Whether to reverse the sort order.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata, sort_tree};
///
/// let mut root = TreeNode::new(
///     PathBuf::from("."),
///     EntryKind::Directory,
///     EntryMetadata::default(),
/// );
/// root.children.push(TreeNode::new(
///     PathBuf::from("zebra.txt"),
///     EntryKind::File,
///     EntryMetadata::default(),
/// ));
/// root.children.push(TreeNode::new(
///     PathBuf::from("alpha.txt"),
///     EntryKind::File,
///     EntryMetadata::default(),
/// ));
///
/// sort_tree(&mut root, false);
/// assert_eq!(root.children[0].name, "alpha.txt");
/// assert_eq!(root.children[1].name, "zebra.txt");
/// ```
pub fn sort_tree(node: &mut TreeNode, reverse: bool) {
    node.children.sort_by(|a, b| {
        let kind_order = match (a.kind, b.kind) {
            (EntryKind::Directory, EntryKind::File) => std::cmp::Ordering::Greater,
            (EntryKind::File, EntryKind::Directory) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        };

        if kind_order != std::cmp::Ordering::Equal {
            return kind_order;
        }

        let cmp = windows_compare_names(&a.name, &b.name);

        if reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });

    for child in &mut node.children {
        sort_tree(child, reverse);
    }
}

/// Sorts a list of path-metadata pairs using Windows-style ordering.
fn sort_entries(entries: &mut [(PathBuf, Metadata)], reverse: bool) {
    entries.sort_by(|(path_a, meta_a), (path_b, meta_b)| {
        let is_dir_a = meta_a.is_dir();
        let is_dir_b = meta_b.is_dir();

        let kind_order = match (is_dir_a, is_dir_b) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        };

        if kind_order != std::cmp::Ordering::Equal {
            return kind_order;
        }

        let name_a = path_a
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name_b = path_b
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        let cmp = windows_compare_names(&name_a, &name_b);

        if reverse {
            cmp.reverse()
        } else {
            cmp
        }
    });
}

/// Internal scan context holding all scan configuration.
struct ScanContext {
    show_files: bool,
    collect_files_for_size: bool,
    max_depth: Option<usize>,
    respect_gitignore: bool,
    rules: CompiledRules,
    reverse: bool,
    needs_size: bool,
    gitignore_cache: Arc<GitignoreCache>,
}

impl ScanContext {
    /// Creates a scan context from configuration.
    fn from_config(config: &Config) -> Result<Self, MatchError> {
        Ok(Self {
            show_files: config.scan.show_files,
            collect_files_for_size: config.render.show_disk_usage,
            max_depth: config.scan.max_depth,
            respect_gitignore: config.scan.respect_gitignore,
            rules: CompiledRules::compile(config)?,
            reverse: config.render.reverse_sort,
            needs_size: config.needs_size_info(),
            gitignore_cache: Arc::new(GitignoreCache::new()),
        })
    }

    /// Checks if an entry should be filtered out.
    fn should_filter(&self, name: &str, is_dir: bool) -> bool {
        if self.rules.should_exclude(name) {
            return true;
        }

        if !is_dir && !self.rules.should_include(name, is_dir) {
            return true;
        }

        if !is_dir && !self.show_files && !self.collect_files_for_size {
            return true;
        }

        false
    }

    /// Gets or loads gitignore rules for a directory.
    fn get_gitignore(&self, dir: &Path) -> Option<Arc<Gitignore>> {
        if !self.respect_gitignore {
            return None;
        }
        self.gitignore_cache.get_or_load(dir)
    }
}

/// Recursively scans a directory and builds a tree node.
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

    if let Some(max) = ctx.max_depth {
        if depth >= max && !ctx.collect_files_for_size {
            return Some(TreeNode::new(path.to_path_buf(), kind, metadata));
        }
    }

    let current_chain = if ctx.respect_gitignore {
        if let Some(gi) = ctx.get_gitignore(path) {
            parent_chain.with_child(gi)
        } else {
            parent_chain
        }
    } else {
        parent_chain
    };

    let entries: Vec<_> = fs::read_dir(path).ok()?.flatten().collect();

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

        if ctx.respect_gitignore && current_chain.is_ignored(&entry_path, is_dir) {
            continue;
        }

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

    let subdir_trees: Vec<TreeNode> = subdirs
        .into_par_iter()
        .filter_map(|subdir| scan_dir(&subdir, depth + 1, ctx, current_chain.clone()))
        .collect();

    let mut children = subdir_trees;
    children.extend(files);

    Some(TreeNode::with_children(
        path.to_path_buf(),
        EntryKind::Directory,
        metadata,
        children,
    ))
}

/// Scans a directory tree and returns the result with statistics.
///
/// Uses rayon for parallel scanning with configurable thread count.
///
/// # Arguments
///
/// * `config` - Scan configuration specifying root path, filters, and options.
///
/// # Returns
///
/// `ScanStats` containing the tree, timing, and counts on success.
///
/// # Errors
///
/// Returns `ScanError::PathNotFound` if the root path doesn't exist.
/// Returns `ScanError::NotADirectory` if the root path is not a directory.
/// Returns `MatchError` if pattern compilation fails.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::scan;
///
/// let config = Config::with_root(PathBuf::from(".")).validate().unwrap();
/// let stats = scan(&config).expect("scan failed");
/// println!("{} directories, {} files", stats.directory_count, stats.file_count);
/// ```
pub fn scan(config: &Config) -> TreeppResult<ScanStats> {
    let start = Instant::now();

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

    let ctx = ScanContext::from_config(config)?;

    let thread_count = config.scan.thread_count.get();
    let pool = ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .map_err(|e| ScanError::WalkError {
            message: format!("thread pool creation failed: {}", e),
            path: Some(config.root_path.clone()),
        })?;

    let initial_chain = GitignoreChain::new();

    let root_path = config.root_path.clone();
    let mut tree = pool
        .install(|| scan_dir(&root_path, 0, &ctx, initial_chain))
        .ok_or_else(|| ScanError::ReadDirFailed {
            path: config.root_path.clone(),
            source: std::io::Error::other("cannot read root directory"),
        })?;

    if ctx.needs_size {
        tree.compute_disk_usage();
    }

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

/// Performs streaming scan with callback-based output.
///
/// Traverses depth-first, calling the callback for each discovered entry.
/// Suitable for real-time output without building the full tree in memory.
///
/// # Arguments
///
/// * `config` - Scan configuration.
/// * `callback` - Function called for each `StreamEvent`.
///
/// # Returns
///
/// `StreamStats` with timing and counts on success.
///
/// # Errors
///
/// Returns `ScanError::PathNotFound` if the root path doesn't exist.
/// Returns `ScanError::NotADirectory` if the root path is not a directory.
/// Propagates any error returned by the callback.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::{scan_streaming, StreamEvent, EntryKind};
///
/// let config = Config::with_root(PathBuf::from(".")).validate().unwrap();
/// let stats = scan_streaming(&config, |event| {
///     if let StreamEvent::Entry(entry) = event {
///         println!("{}: {}", if entry.kind == EntryKind::Directory { "dir" } else { "file" }, entry.name);
///     }
///     Ok(())
/// }).expect("scan failed");
/// println!("{} directories, {} files", stats.directory_count, stats.file_count);
/// ```
pub fn scan_streaming<F>(config: &Config, mut callback: F) -> TreeppResult<StreamStats>
where
    F: FnMut(StreamEvent) -> Result<(), ScanError>,
{
    let start = Instant::now();

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

    let ctx = ScanContext::from_config(config)?;
    let initial_chain = GitignoreChain::new();

    let (dir_count, file_count) =
        streaming_scan_dir(&config.root_path, 0, &ctx, &initial_chain, &mut callback)?;

    let duration = start.elapsed();

    Ok(StreamStats {
        duration,
        directory_count: dir_count,
        file_count,
    })
}

/// Recursively performs streaming scan of a directory.
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
    if let Some(max) = ctx.max_depth {
        if depth >= max {
            return Ok((0, 0));
        }
    }

    let current_chain = if ctx.respect_gitignore {
        if let Some(gi) = ctx.get_gitignore(path) {
            parent_chain.with_child(gi)
        } else {
            parent_chain.clone()
        }
    } else {
        parent_chain.clone()
    };

    let raw_entries: Vec<_> = match fs::read_dir(path) {
        Ok(entries) => entries.flatten().collect(),
        Err(_) => return Ok((0, 0)),
    };

    let entries_with_meta: Vec<(PathBuf, Metadata)> = raw_entries
        .into_iter()
        .filter_map(|entry| {
            let entry_path = entry.path();
            let meta = entry.metadata().ok()?;
            Some((entry_path, meta))
        })
        .collect();

    let mut filtered: Vec<(PathBuf, Metadata)> = entries_with_meta
        .into_iter()
        .filter(|(entry_path, meta)| {
            let entry_name = entry_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            let is_dir = meta.is_dir();

            if ctx.respect_gitignore && current_chain.is_ignored(entry_path, is_dir) {
                return false;
            }

            !ctx.should_filter(&entry_name, is_dir)
        })
        .collect();

    sort_entries(&mut filtered, ctx.reverse);

    let mut files: Vec<(PathBuf, Metadata)> = Vec::new();
    let mut dirs: Vec<(PathBuf, Metadata)> = Vec::new();

    for (entry_path, meta) in filtered {
        if meta.is_dir() {
            dirs.push((entry_path, meta));
        } else {
            files.push((entry_path, meta));
        }
    }

    let mut dir_count = 0;
    let mut file_count = 0;

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

        callback(StreamEvent::EnterDir { is_last })?;

        let (sub_dirs, sub_files) =
            streaming_scan_dir(&entry_path, depth + 1, ctx, &current_chain, callback)?;
        dir_count += sub_dirs;
        file_count += sub_files;

        callback(StreamEvent::LeaveDir)?;
    }

    Ok((dir_count, file_count))
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().expect("创建临时目录失败");
        let root = dir.path();

        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir(root.join("tests")).unwrap();
        fs::create_dir(root.join("empty")).unwrap();

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

    fn setup_gitignore_dir() -> TempDir {
        let dir = setup_test_dir();
        let root = dir.path();

        File::create(root.join(".gitignore"))
            .unwrap()
            .write_all(b"target/\n*.log\n")
            .unwrap();

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

    fn setup_nested_gitignore_dir() -> TempDir {
        let dir = TempDir::new().expect("创建临时目录失败");
        let root = dir.path();

        fs::create_dir(root.join("level1")).unwrap();
        fs::create_dir(root.join("level1/level2")).unwrap();
        fs::create_dir(root.join("level1/level2/level3")).unwrap();

        File::create(root.join(".gitignore"))
            .unwrap()
            .write_all(b"*.tmp\n")
            .unwrap();

        File::create(root.join("level1/.gitignore"))
            .unwrap()
            .write_all(b"*.bak\n")
            .unwrap();

        File::create(root.join("level1/level2/.gitignore"))
            .unwrap()
            .write_all(b"*.cache\n")
            .unwrap();

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

    fn collect_names(node: &TreeNode) -> Vec<String> {
        let mut names = vec![node.name.clone()];
        for child in &node.children {
            names.extend(collect_names(child));
        }
        names.sort();
        names
    }

    fn has_node_with_name(node: &TreeNode, name: &str) -> bool {
        if node.name == name {
            return true;
        }
        node.children.iter().any(|c| has_node_with_name(c, name))
    }

    fn count_files_in_tree(node: &TreeNode) -> usize {
        let self_count = if node.kind == EntryKind::File { 1 } else { 0 };
        self_count
            + node
            .children
            .iter()
            .map(count_files_in_tree)
            .sum::<usize>()
    }

    #[test]
    fn entry_kind_from_file_metadata() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let file_meta = fs::metadata(&file_path).unwrap();
        assert_eq!(EntryKind::from_metadata(&file_meta), EntryKind::File);
    }

    #[test]
    fn entry_kind_from_directory_metadata() {
        let dir = TempDir::new().unwrap();
        let dir_meta = fs::metadata(dir.path()).unwrap();
        assert_eq!(EntryKind::from_metadata(&dir_meta), EntryKind::Directory);
    }

    #[test]
    fn entry_kind_equality_comparison() {
        assert_eq!(EntryKind::Directory, EntryKind::Directory);
        assert_eq!(EntryKind::File, EntryKind::File);
        assert_ne!(EntryKind::Directory, EntryKind::File);
    }

    #[test]
    fn entry_kind_hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(EntryKind::File);
        set.insert(EntryKind::Directory);
        assert_eq!(set.len(), 2);
        set.insert(EntryKind::File);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn entry_metadata_default_values() {
        let meta = EntryMetadata::default();
        assert_eq!(meta.size, 0);
        assert!(meta.modified.is_none());
        assert!(meta.created.is_none());
    }

    #[test]
    fn entry_metadata_from_file_fs_metadata() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path)
            .unwrap()
            .write_all(b"hello world")
            .unwrap();

        let fs_meta = fs::metadata(&file_path).unwrap();
        let entry_meta = EntryMetadata::from_fs_metadata(&fs_meta);

        assert_eq!(entry_meta.size, 11);
        assert!(entry_meta.modified.is_some());
    }

    #[test]
    fn entry_metadata_directory_has_zero_size() {
        let dir = TempDir::new().unwrap();
        let fs_meta = fs::metadata(dir.path()).unwrap();
        let entry_meta = EntryMetadata::from_fs_metadata(&fs_meta);

        assert_eq!(entry_meta.size, 0);
    }

    #[test]
    fn entry_metadata_clone_preserves_values() {
        let meta = EntryMetadata {
            size: 42,
            modified: Some(SystemTime::UNIX_EPOCH),
            created: None,
        };
        let cloned = meta.clone();
        assert_eq!(cloned.size, 42);
        assert_eq!(cloned.modified, Some(SystemTime::UNIX_EPOCH));
    }

    #[test]
    fn tree_node_new_extracts_name_from_path() {
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
    fn tree_node_new_handles_root_path() {
        let node = TreeNode::new(
            PathBuf::from("/"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        assert_eq!(node.name, "/");
    }

    #[test]
    fn tree_node_with_children_sets_children() {
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
    fn tree_node_count_files_single_file() {
        let node = TreeNode::new(
            PathBuf::from("file.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        );
        assert_eq!(node.count_files(), 1);
    }

    #[test]
    fn tree_node_count_files_nested_structure() {
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
    fn tree_node_count_files_directory_only() {
        let root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        assert_eq!(root.count_files(), 0);
    }

    #[test]
    fn tree_node_count_directories_empty() {
        let root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        assert_eq!(root.count_directories(), 0);
    }

    #[test]
    fn tree_node_count_directories_nested() {
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
    fn tree_node_compute_disk_usage_single_file() {
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
    fn tree_node_compute_disk_usage_directory() {
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
    fn tree_node_compute_disk_usage_nested() {
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
    fn tree_node_compute_disk_usage_empty_directory() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.compute_disk_usage();
        assert_eq!(root.disk_usage, Some(0));
    }

    #[test]
    fn stream_entry_creation() {
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
    fn stream_entry_clone() {
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
        let cloned = entry.clone();
        assert_eq!(cloned.name, "test.txt");
    }

    #[test]
    fn stream_stats_creation() {
        let stats = StreamStats {
            duration: Duration::from_millis(100),
            directory_count: 5,
            file_count: 20,
        };

        assert_eq!(stats.directory_count, 5);
        assert_eq!(stats.file_count, 20);
        assert_eq!(stats.duration.as_millis(), 100);
    }

    #[test]
    fn stream_event_entry_variant() {
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
        let event = StreamEvent::Entry(entry);
        if let StreamEvent::Entry(e) = event {
            assert_eq!(e.name, "test.txt");
        } else {
            panic!("Expected Entry variant");
        }
    }

    #[test]
    fn stream_event_enter_dir_variant() {
        let event = StreamEvent::EnterDir { is_last: true };
        if let StreamEvent::EnterDir { is_last } = event {
            assert!(is_last);
        } else {
            panic!("Expected EnterDir variant");
        }
    }

    #[test]
    fn stream_event_leave_dir_variant() {
        let event = StreamEvent::LeaveDir;
        assert!(matches!(event, StreamEvent::LeaveDir));
    }

    #[test]
    fn gitignore_chain_new_empty() {
        let chain = GitignoreChain::new();
        assert!(!chain.is_ignored(Path::new("test.txt"), false));
        assert!(!chain.is_ignored(Path::new("anything"), true));
    }

    #[test]
    fn gitignore_chain_with_child_creates_independent_chain() {
        let chain1 = GitignoreChain::new();
        let chain2 = chain1.clone();
        assert_eq!(chain1.rules.len(), chain2.rules.len());
    }

    #[test]
    fn gitignore_chain_is_ignored_basic() {
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

    #[test]
    fn gitignore_cache_returns_none_for_missing() {
        let dir = TempDir::new().unwrap();
        let cache = GitignoreCache::new();

        let result = cache.get_or_load(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn gitignore_cache_loads_existing() {
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
    fn gitignore_cache_caches_result() {
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
        assert!(Arc::ptr_eq(&result1.unwrap(), &result2.unwrap()));
    }

    #[test]
    fn gitignore_cache_caches_none() {
        let dir = TempDir::new().unwrap();
        let cache = GitignoreCache::new();

        let _result1 = cache.get_or_load(dir.path());
        let _result2 = cache.get_or_load(dir.path());

        let inner = cache.cache.lock().unwrap();
        assert!(inner.contains_key(dir.path()));
    }

    #[test]
    fn load_gitignore_from_path_returns_none_when_missing() {
        let dir = TempDir::new().unwrap();
        assert!(load_gitignore_from_path(dir.path()).is_none());
    }

    #[test]
    fn load_gitignore_from_path_loads_valid_file() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join(".gitignore"))
            .unwrap()
            .write_all(b"*.txt\n")
            .unwrap();
        assert!(load_gitignore_from_path(dir.path()).is_some());
    }

    #[test]
    fn compile_pattern_basic() {
        let pattern = compile_pattern("*.rs").expect("编译失败");
        assert!(pattern.matches("main.rs"));
        assert!(pattern.matches("lib.rs"));
        assert!(!pattern.matches("main.txt"));
    }

    #[test]
    fn compile_pattern_invalid() {
        let result = compile_pattern("[invalid");
        assert!(result.is_err());

        if let Err(MatchError::InvalidPattern { pattern, .. }) = result {
            assert_eq!(pattern, "[invalid");
        } else {
            panic!("Expected InvalidPattern error");
        }
    }

    #[test]
    fn compile_pattern_complex_glob() {
        let pattern = compile_pattern("test_*.rs").unwrap();
        assert!(pattern.matches("test_foo.rs"));
        assert!(!pattern.matches("foo_test.rs"));
    }

    #[test]
    fn compiled_rules_should_include_with_pattern() {
        let mut config = Config::default();
        config.matching.include_patterns = vec!["*.rs".to_string()];

        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_include("main.rs", false));
        assert!(!rules.should_include("main.txt", false));
    }

    #[test]
    fn compiled_rules_should_include_no_patterns() {
        let config = Config::default();
        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_include("any.rs", false));
        assert!(rules.should_include("any.txt", false));
    }

    #[test]
    fn compiled_rules_should_include_directory_always() {
        let mut config = Config::default();
        config.matching.include_patterns = vec!["*.rs".to_string()];

        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_include("src", true));
        assert!(rules.should_include("tests", true));
    }

    #[test]
    fn compiled_rules_should_exclude_no_patterns() {
        let config = Config::default();
        let rules = CompiledRules::compile(&config).unwrap();

        assert!(!rules.should_exclude("any.rs"));
        assert!(!rules.should_exclude("any.txt"));
    }

    #[test]
    fn compiled_rules_should_exclude_with_pattern() {
        let mut config = Config::default();
        config.matching.exclude_patterns = vec!["*.log".to_string()];

        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_exclude("app.log"));
        assert!(!rules.should_exclude("app.txt"));
    }

    #[test]
    fn compiled_rules_multiple_patterns() {
        let mut config = Config::default();
        config.matching.include_patterns = vec!["*.rs".to_string(), "*.toml".to_string()];
        config.matching.exclude_patterns = vec!["test_*".to_string()];

        let rules = CompiledRules::compile(&config).unwrap();

        assert!(rules.should_include("main.rs", false));
        assert!(rules.should_include("Cargo.toml", false));
        assert!(!rules.should_include("README.md", false));
        assert!(rules.should_exclude("test_foo.rs"));
    }

    #[test]
    fn windows_char_priority_ordering() {
        let (pri_dot, _) = windows_char_priority('.');
        let (pri_dash, _) = windows_char_priority('-');
        let (pri_digit, _) = windows_char_priority('5');
        let (pri_letter, _) = windows_char_priority('a');
        let (pri_underscore, _) = windows_char_priority('_');

        assert!(pri_dot < pri_digit, "dot should come before digit");
        assert!(pri_dash < pri_digit, "dash should come before digit");
        assert!(pri_digit < pri_letter, "digit should come before letter");
        assert!(
            pri_letter < pri_underscore,
            "letter should come before underscore"
        );
    }

    #[test]
    fn windows_char_priority_case_insensitive() {
        let (pri_upper, char_upper) = windows_char_priority('A');
        let (pri_lower, char_lower) = windows_char_priority('a');
        assert_eq!(pri_upper, pri_lower);
        assert_eq!(char_upper, char_lower);
    }

    #[test]
    fn windows_compare_names_underscore_after_letter() {
        assert_eq!(
            windows_compare_names("_test", "apple"),
            std::cmp::Ordering::Greater,
            "_test should come after apple"
        );
        assert_eq!(
            windows_compare_names("_zebra", "apple"),
            std::cmp::Ordering::Greater,
            "_zebra should come after apple"
        );
    }

    #[test]
    fn windows_compare_names_utf8_case() {
        assert_eq!(
            windows_compare_names("utf8parse", "utf8_iter"),
            std::cmp::Ordering::Less,
            "utf8parse should come before utf8_iter"
        );
    }

    #[test]
    fn windows_compare_names_special_chars_first() {
        assert_eq!(
            windows_compare_names(".hidden", "apple"),
            std::cmp::Ordering::Less,
            ".hidden should come before apple"
        );
        assert_eq!(
            windows_compare_names("-dash", "apple"),
            std::cmp::Ordering::Less,
            "-dash should come before apple"
        );
    }

    #[test]
    fn windows_compare_names_numbers_before_letters() {
        assert_eq!(
            windows_compare_names("123", "abc"),
            std::cmp::Ordering::Less,
            "123 should come before abc"
        );
        assert_eq!(
            windows_compare_names("1file", "afile"),
            std::cmp::Ordering::Less,
            "1file should come before afile"
        );
    }

    #[test]
    fn windows_compare_names_case_insensitive() {
        assert_eq!(
            windows_compare_names("Apple", "apple"),
            std::cmp::Ordering::Equal,
            "Apple and apple should be equal"
        );
        assert_eq!(
            windows_compare_names("ZEBRA", "apple"),
            std::cmp::Ordering::Greater,
            "ZEBRA should come after apple"
        );
    }

    #[test]
    fn windows_compare_names_length_matters() {
        assert_eq!(
            windows_compare_names("ab", "abc"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            windows_compare_names("abc", "ab"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn sort_tree_by_name() {
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
    fn sort_tree_reverse() {
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
    fn sort_tree_files_before_dirs() {
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

        assert_eq!(root.children[0].kind, EntryKind::File);
        assert_eq!(root.children[1].kind, EntryKind::Directory);
    }

    #[test]
    fn sort_tree_recursive() {
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

        assert_eq!(root.children[0].children[0].name, "a.txt");
        assert_eq!(root.children[0].children[1].name, "z.txt");
    }

    #[test]
    fn sort_tree_windows_style() {
        let mut root = TreeNode::new(
            PathBuf::from("."),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let names = vec!["_underscore", "apple", ".dotfile", "123", "Banana"];
        for name in names {
            root.children.push(TreeNode::new(
                PathBuf::from(name),
                EntryKind::File,
                EntryMetadata::default(),
            ));
        }

        sort_tree(&mut root, false);

        let sorted_names: Vec<_> = root.children.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            sorted_names,
            vec![".dotfile", "123", "apple", "Banana", "_underscore"],
            "sort should follow Windows tree style"
        );
    }

    #[test]
    fn sort_entries_by_name() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

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

    #[test]
    fn sort_entries_reverse() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join("a.txt")).unwrap();
        File::create(root.join("b.txt")).unwrap();

        let mut entries: Vec<(PathBuf, Metadata)> = fs::read_dir(root)
            .unwrap()
            .flatten()
            .filter_map(|e| {
                let path = e.path();
                let meta = e.metadata().ok()?;
                Some((path, meta))
            })
            .collect();

        sort_entries(&mut entries, true);

        let names: Vec<_> = entries
            .iter()
            .map(|(p, _)| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["b.txt", "a.txt"]);
    }

    #[test]
    fn scan_basic() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 5);
    }

    #[test]
    fn scan_dirs_only() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn scan_max_depth_zero() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.max_depth = Some(0);

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 0);
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn scan_max_depth_one() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.max_depth = Some(1);

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 2);
    }

    #[test]
    fn scan_with_include() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 3);
    }

    #[test]
    fn scan_with_exclude() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.exclude_patterns = vec!["*.md".to_string()];

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 4);
        assert!(!has_node_with_name(&stats.tree, "README.md"));
    }

    #[test]
    fn scan_nonexistent_path() {
        let config = Config::with_root(PathBuf::from("/nonexistent/path/12345"));
        let result = scan(&config);
        assert!(result.is_err());
    }

    #[test]
    fn scan_file_as_root() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("file.txt");
        File::create(&file_path).unwrap();

        let config = Config::with_root(file_path);
        let result = scan(&config);
        assert!(result.is_err());
    }

    #[test]
    fn scan_with_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan(&config).expect("扫描失败");

        assert!(!has_node_with_name(&stats.tree, "target"));
        assert!(!has_node_with_name(&stats.tree, "app.log"));
        assert!(has_node_with_name(&stats.tree, "Cargo.toml"));
    }

    #[test]
    fn scan_without_gitignore() {
        let dir = setup_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = false;

        let stats = scan(&config).expect("扫描失败");

        assert!(has_node_with_name(&stats.tree, "target"));
        assert!(has_node_with_name(&stats.tree, "app.log"));
    }

    #[test]
    fn scan_nested_gitignore_inheritance() {
        let dir = setup_nested_gitignore_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.respect_gitignore = true;

        let stats = scan(&config).expect("扫描失败");

        assert!(!has_node_with_name(&stats.tree, "root.tmp"));
        assert!(!has_node_with_name(&stats.tree, "l1.tmp"));
        assert!(!has_node_with_name(&stats.tree, "l2.tmp"));
        assert!(!has_node_with_name(&stats.tree, "l3.tmp"));

        assert!(!has_node_with_name(&stats.tree, "l1.bak"));
        assert!(!has_node_with_name(&stats.tree, "l2.bak"));
        assert!(!has_node_with_name(&stats.tree, "l3.bak"));

        assert!(!has_node_with_name(&stats.tree, "l2.cache"));
        assert!(!has_node_with_name(&stats.tree, "l3.cache"));

        assert!(has_node_with_name(&stats.tree, "root.txt"));
        assert!(has_node_with_name(&stats.tree, "l1.txt"));
        assert!(has_node_with_name(&stats.tree, "l2.txt"));
        assert!(has_node_with_name(&stats.tree, "l3.txt"));
    }

    #[test]
    fn scan_single_thread() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.thread_count = std::num::NonZeroUsize::new(1).unwrap();

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 5);
    }

    #[test]
    fn scan_multi_thread() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.scan.thread_count = std::num::NonZeroUsize::new(4).unwrap();

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 5);
    }

    #[test]
    fn scan_thread_count_consistency() {
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

        assert_eq!(stats1.file_count, stats4.file_count);
        assert_eq!(stats4.file_count, stats8.file_count);
        assert_eq!(stats1.directory_count, stats4.directory_count);
        assert_eq!(stats4.directory_count, stats8.directory_count);

        let names1 = collect_names(&stats1.tree);
        let names4 = collect_names(&stats4.tree);
        let names8 = collect_names(&stats8.tree);
        assert_eq!(names1, names4);
        assert_eq!(names4, names8);
    }

    #[test]
    fn scan_with_gitignore_thread_consistency() {
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

    #[test]
    fn scan_streaming_basic() {
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
        assert_eq!(entries.len(), 8);
        assert_eq!(enter_count, 3);
        assert_eq!(leave_count, 3);
    }

    #[test]
    fn scan_streaming_dirs_only() {
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
    fn scan_streaming_with_gitignore() {
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

        assert!(!names.contains(&"target".to_string()));
        assert!(!names.contains(&"app.log".to_string()));
    }

    #[test]
    fn scan_streaming_depth_info() {
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

        let root_entries: Vec<_> = entries.iter().filter(|(_, d)| *d == 0).collect();
        assert!(!root_entries.is_empty());

        let nested_entries: Vec<_> = entries.iter().filter(|(_, d)| *d == 1).collect();
        assert!(!nested_entries.is_empty());
    }

    #[test]
    fn scan_streaming_is_last_flag() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

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

        assert_eq!(entries.len(), 3);

        let last_count = entries.iter().filter(|(_, is_last)| *is_last).count();
        assert_eq!(last_count, 1);
    }

    #[test]
    fn scan_streaming_nonexistent_path() {
        let config = Config::with_root(PathBuf::from("/nonexistent/path/12345"));

        let result = scan_streaming(&config, |_| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn scan_streaming_max_depth() {
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

        assert_eq!(stats.directory_count, 3);
        assert_eq!(stats.file_count, 2);
    }

    #[test]
    fn scan_streaming_callback_error_propagation() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let mut count = 0;
        let result = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(_) = event {
                count += 1;
                if count >= 2 {
                    return Err(ScanError::WalkError {
                        message: "test error".to_string(),
                        path: None,
                    });
                }
            }
            Ok(())
        });

        assert!(result.is_err());
    }

    #[test]
    fn streaming_vs_batch_entry_names() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let batch_stats = scan(&config).expect("批处理扫描失败");
        let batch_names = collect_names(&batch_stats.tree);

        let mut stream_names = Vec::new();
        let _stream_stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                stream_names.push(entry.name.clone());
            }
            Ok(())
        })
            .expect("流式扫描失败");
        stream_names.sort();

        let batch_without_root: Vec<_> = batch_names
            .into_iter()
            .filter(|n| n != &batch_stats.tree.name)
            .collect();

        assert_eq!(stream_names, batch_without_root);
    }

    #[test]
    fn streaming_vs_batch_counts() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let batch_stats = scan(&config).expect("批处理扫描失败");
        let stream_stats = scan_streaming(&config, |_| Ok(())).expect("流式扫描失败");

        assert_eq!(batch_stats.directory_count, stream_stats.directory_count);
        assert_eq!(batch_stats.file_count, stream_stats.file_count);
    }

    #[test]
    fn scan_empty_directory() {
        let dir = TempDir::new().unwrap();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, 0);
        assert_eq!(stats.file_count, 0);
    }

    #[test]
    fn scan_single_file_directory() {
        let dir = TempDir::new().unwrap();
        File::create(dir.path().join("only.txt")).unwrap();

        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 1);
    }

    #[test]
    fn scan_deeply_nested() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

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

    #[test]
    fn scan_stats_duration_is_measured() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let stats = scan(&config).expect("扫描失败");

        assert!(stats.duration.as_nanos() > 0);
    }

    #[test]
    fn scan_stats_counts_match_tree() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.directory_count, stats.tree.count_directories());
        assert_eq!(stats.file_count, stats.tree.count_files());
    }

    #[test]
    fn stream_stats_duration_is_measured() {
        let dir = setup_test_dir();
        let config = Config::with_root(dir.path().to_path_buf());

        let stats = scan_streaming(&config, |_| Ok(())).expect("扫描失败");

        assert!(stats.duration.as_nanos() > 0);
    }

    #[test]
    fn stream_entry_new_fields() {
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
    fn scan_collects_files_for_disk_usage_even_without_show_files() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.batch_mode = true;
        config.scan.show_files = false;
        config.render.show_disk_usage = true;

        let stats = scan(&config).expect("扫描失败");

        assert!(stats.tree.disk_usage.is_some());
        assert!(stats.tree.disk_usage.unwrap() > 0);
    }

    #[test]
    fn scan_disk_usage_correct_without_show_files() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir(root.join("subdir")).unwrap();
        File::create(root.join("file1.txt"))
            .unwrap()
            .write_all(b"12345")
            .unwrap();
        File::create(root.join("subdir/file2.txt"))
            .unwrap()
            .write_all(b"1234567890")
            .unwrap();

        let mut config = Config::with_root(root.to_path_buf());
        config.batch_mode = true;
        config.scan.show_files = false;
        config.render.show_disk_usage = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.tree.disk_usage, Some(15));
    }

    #[test]
    fn scan_no_files_collected_when_neither_show_files_nor_disk_usage() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = false;
        config.render.show_disk_usage = false;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 0);
        assert_eq!(count_files_in_tree(&stats.tree), 0);
    }

    #[test]
    fn scan_context_collect_files_for_size_enabled_when_disk_usage() {
        let mut config = Config::default();
        config.batch_mode = true;
        config.render.show_disk_usage = true;
        config.scan.show_files = false;

        let ctx = ScanContext::from_config(&config).unwrap();

        assert!(!ctx.show_files);
        assert!(ctx.collect_files_for_size);
    }

    #[test]
    fn scan_context_collect_files_for_size_disabled_by_default() {
        let config = Config::default();

        let ctx = ScanContext::from_config(&config).unwrap();

        assert!(!ctx.show_files);
        assert!(!ctx.collect_files_for_size);
    }

    #[test]
    fn should_filter_includes_files_when_collect_for_size() {
        let mut config = Config::default();
        config.batch_mode = true;
        config.render.show_disk_usage = true;
        config.scan.show_files = false;

        let ctx = ScanContext::from_config(&config).unwrap();

        assert!(!ctx.should_filter("test.txt", false));
    }

    #[test]
    fn should_filter_excludes_files_when_no_show_no_collect() {
        let config = Config::default();

        let ctx = ScanContext::from_config(&config).unwrap();

        assert!(ctx.should_filter("test.txt", false));
    }

    #[test]
    fn scan_disk_usage_with_max_depth_collects_full_size() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("a/b")).unwrap();
        File::create(root.join("a/b/deep.txt"))
            .unwrap()
            .write_all(b"1234567")
            .unwrap();

        let mut config = Config::with_root(root.to_path_buf());
        config.batch_mode = true;
        config.render.show_disk_usage = true;
        config.scan.show_files = false;
        config.scan.max_depth = Some(1);

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.tree.disk_usage, Some(7));
        let dir_a = stats.tree.children.iter().find(|c| c.name == "a").unwrap();
        assert_eq!(dir_a.disk_usage, Some(7));
    }

    #[test]
    fn scan_context_from_config_with_all_options() {
        let mut config = Config::default();
        config.scan.show_files = true;
        config.scan.max_depth = Some(5);
        config.scan.respect_gitignore = true;
        config.render.reverse_sort = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];
        config.matching.exclude_patterns = vec!["test_*".to_string()];

        let ctx = ScanContext::from_config(&config).unwrap();

        assert!(ctx.show_files);
        assert_eq!(ctx.max_depth, Some(5));
        assert!(ctx.respect_gitignore);
        assert!(ctx.reverse);
    }

    #[test]
    fn scan_context_should_filter_respects_exclude_over_include() {
        let mut config = Config::default();
        config.scan.show_files = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];
        config.matching.exclude_patterns = vec!["test_*.rs".to_string()];

        let ctx = ScanContext::from_config(&config).unwrap();

        assert!(!ctx.should_filter("main.rs", false));
        assert!(ctx.should_filter("test_main.rs", false));
    }

    #[test]
    fn scan_with_multiple_exclude_patterns() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.exclude_patterns = vec!["*.md".to_string(), "*.toml".to_string()];

        let stats = scan(&config).expect("扫描失败");

        assert!(!has_node_with_name(&stats.tree, "README.md"));
        assert!(!has_node_with_name(&stats.tree, "Cargo.toml"));
        assert!(has_node_with_name(&stats.tree, "main.rs"));
    }

    #[test]
    fn scan_streaming_with_exclude() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;
        config.matching.exclude_patterns = vec!["*.md".to_string()];

        let mut names = Vec::new();
        let _stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                names.push(entry.name.clone());
            }
            Ok(())
        })
            .expect("流式扫描失败");

        assert!(!names.contains(&"README.md".to_string()));
    }

    #[test]
    fn tree_node_clone() {
        let mut original = TreeNode::new(
            PathBuf::from("test"),
            EntryKind::Directory,
            EntryMetadata {
                size: 0,
                ..Default::default()
            },
        );
        original.children.push(TreeNode::new(
            PathBuf::from("child.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        original.disk_usage = Some(100);

        let cloned = original.clone();

        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.children.len(), 1);
        assert_eq!(cloned.disk_usage, Some(100));
    }

    #[test]
    fn scan_handles_special_characters_in_filenames() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join("file with spaces.txt")).unwrap();
        File::create(root.join("file-with-dashes.txt")).unwrap();
        File::create(root.join("file_with_underscores.txt")).unwrap();

        let mut config = Config::with_root(root.to_path_buf());
        config.scan.show_files = true;

        let stats = scan(&config).expect("扫描失败");

        assert_eq!(stats.file_count, 3);
        assert!(has_node_with_name(&stats.tree, "file with spaces.txt"));
        assert!(has_node_with_name(&stats.tree, "file-with-dashes.txt"));
        assert!(has_node_with_name(&stats.tree, "file_with_underscores.txt"));
    }

    #[test]
    fn scan_streaming_has_more_dirs_flag() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join("file.txt")).unwrap();
        fs::create_dir(root.join("dir1")).unwrap();
        fs::create_dir(root.join("dir2")).unwrap();

        let mut config = Config::with_root(root.to_path_buf());
        config.scan.show_files = true;

        let mut file_entries = Vec::new();
        let _stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                if entry.is_file {
                    file_entries.push(entry);
                }
            }
            Ok(())
        })
            .expect("流式扫描失败");

        assert!(!file_entries.is_empty());
        assert!(file_entries[0].has_more_dirs);
    }

    #[test]
    fn streaming_vs_batch_order_consistency() {
        let dir = setup_test_dir();
        let mut config = Config::with_root(dir.path().to_path_buf());
        config.scan.show_files = true;

        let batch_stats = scan(&config).expect("批处理扫描失败");

        let mut batch_order = Vec::new();
        fn collect_order(node: &TreeNode, order: &mut Vec<String>) {
            for child in &node.children {
                order.push(child.name.clone());
                if child.kind == EntryKind::Directory {
                    collect_order(child, order);
                }
            }
        }
        collect_order(&batch_stats.tree, &mut batch_order);

        let mut stream_order = Vec::new();
        let _stream_stats = scan_streaming(&config, |event| {
            if let StreamEvent::Entry(entry) = event {
                stream_order.push(entry.name.clone());
            }
            Ok(())
        })
            .expect("流式扫描失败");

        assert_eq!(batch_order, stream_order);
    }
}
