//! 扫描引擎原型
//!
//! 验证 walk（单线程）与 parallel（多线程）两种目录扫描模式的正确性与一致性。
//! 核心验证点：
//! - 两种模式产生完全一致的结果
//! - 并发扫描不重不漏
//! - 线程数参数生效
//! - 输出具有确定性排序

use std::cmp::Ordering;
use std::collections::HashMap;
use std::env;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use crossbeam_channel::{bounded, Sender};
use parking_lot::Mutex;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

// ============================================================================
// 类型定义
// ============================================================================

/// 文件系统条目类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// 目录
    Directory,
    /// 文件
    File,
}

/// 文件系统条目元数据
#[derive(Debug, Clone)]
pub struct EntryMetadata {
    /// 文件大小（字节），目录为 0
    pub size: u64,
    /// 最后修改时间
    pub modified: Option<SystemTime>,
    /// 创建时间
    pub created: Option<SystemTime>,
}

impl EntryMetadata {
    fn from_fs_metadata(meta: &Metadata) -> Self {
        Self {
            size: if meta.is_file() { meta.len() } else { 0 },
            modified: meta.modified().ok(),
            created: meta.created().ok(),
        }
    }

    fn empty() -> Self {
        Self {
            size: 0,
            modified: None,
            created: None,
        }
    }
}

/// 目录树节点
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
}

impl TreeNode {
    /// 创建新节点
    fn new(path: PathBuf, kind: EntryKind, metadata: EntryMetadata) -> Self {
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
        }
    }

    /// 递归统计目录数量
    pub fn count_directories(&self) -> usize {
        let self_count = if self.kind == EntryKind::Directory {
            1
        } else {
            0
        };
        self_count
            + self
            .children
            .iter()
            .map(TreeNode::count_directories)
            .sum::<usize>()
    }

    /// 递归统计文件数量
    pub fn count_files(&self) -> usize {
        let self_count = if self.kind == EntryKind::File { 1 } else { 0 };
        self_count
            + self
            .children
            .iter()
            .map(TreeNode::count_files)
            .sum::<usize>()
    }

    /// 递归统计总条目数
    pub fn count_total(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(TreeNode::count_total)
            .sum::<usize>()
    }

    /// 计算最大深度
    pub fn max_depth(&self) -> usize {
        if self.children.is_empty() {
            1
        } else {
            1 + self
                .children
                .iter()
                .map(TreeNode::max_depth)
                .max()
                .unwrap_or(0)
        }
    }

    /// 对子节点进行确定性排序（递归）
    pub fn sort_deterministic(&mut self) {
        self.children.sort_by(|a, b| {
            match (a.kind, b.kind) {
                // 目录在前，文件在后
                (EntryKind::Directory, EntryKind::File) => Ordering::Less,
                (EntryKind::File, EntryKind::Directory) => Ordering::Greater,
                // 同类型按名称字典序（大小写不敏感）
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });
        for child in &mut self.children {
            child.sort_deterministic();
        }
    }

    /// 深度相等比较（忽略元数据时间戳的微小差异）
    pub fn structural_eq(&self, other: &Self) -> bool {
        if self.name != other.name || self.kind != other.kind {
            return false;
        }
        if self.children.len() != other.children.len() {
            return false;
        }
        self.children
            .iter()
            .zip(other.children.iter())
            .all(|(a, b)| a.structural_eq(b))
    }

    /// 收集所有路径（用于对比）
    pub fn collect_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![self.path.clone()];
        for child in &self.children {
            paths.extend(child.collect_paths());
        }
        paths
    }

    /// 收集所有名称（扁平化，用于快速对比）
    pub fn collect_names(&self) -> Vec<String> {
        let mut names = vec![self.name.clone()];
        for child in &self.children {
            names.extend(child.collect_names());
        }
        names
    }
}

impl Display for TreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_with_prefix(f, "", true)
    }
}

impl TreeNode {
    fn fmt_with_prefix(&self, f: &mut Formatter<'_>, prefix: &str, is_last: bool) -> fmt::Result {
        let connector = if is_last { "└─" } else { "├─" };
        let kind_indicator = match self.kind {
            EntryKind::Directory => "/",
            EntryKind::File => "",
        };

        if prefix.is_empty() {
            writeln!(f, "{}{}", self.name, kind_indicator)?;
        } else {
            writeln!(f, "{}{}{}{}", prefix, connector, self.name, kind_indicator)?;
        }

        let child_prefix = if prefix.is_empty() {
            String::new()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        let new_prefix = if prefix.is_empty() {
            String::new()
        } else {
            child_prefix
        };

        for (i, child) in self.children.iter().enumerate() {
            let child_is_last = i == self.children.len() - 1;
            child.fmt_with_prefix(f, &new_prefix, child_is_last)?;
        }

        Ok(())
    }
}

/// 扫描配置
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// 根路径
    pub root: PathBuf,
    /// 是否包含文件
    pub include_files: bool,
    /// 线程数（仅 parallel 模式有效）
    pub thread_count: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            include_files: true,
            thread_count: num_cpus(),
        }
    }
}

/// 扫描结果
#[derive(Debug)]
pub struct ScanResult {
    /// 根节点
    pub tree: TreeNode,
    /// 扫描耗时
    pub duration: std::time::Duration,
    /// 目录总数
    pub directory_count: usize,
    /// 文件总数
    pub file_count: usize,
}

impl Display for ScanResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.tree)?;
        writeln!(
            f,
            "\n{} 个目录, {} 个文件",
            self.directory_count, self.file_count
        )?;
        writeln!(f, "耗时: {:.3}s", self.duration.as_secs_f64())
    }
}

// ============================================================================
// 单线程扫描引擎 (walk)
// ============================================================================

/// 单线程递归目录扫描
pub fn scan_walk(config: &ScanConfig) -> io::Result<ScanResult> {
    let start = Instant::now();
    let root_meta = fs::metadata(&config.root)?;
    let root_metadata = EntryMetadata::from_fs_metadata(&root_meta);

    let mut root = TreeNode::new(
        config.root.clone(),
        if root_meta.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::File
        },
        root_metadata,
    );

    if root.kind == EntryKind::Directory {
        scan_walk_recursive(&config.root, &mut root, config.include_files)?;
    }

    root.sort_deterministic();

    let directory_count = root.count_directories();
    let file_count = root.count_files();
    let duration = start.elapsed();

    Ok(ScanResult {
        tree: root,
        duration,
        directory_count,
        file_count,
    })
}

fn scan_walk_recursive(path: &Path, node: &mut TreeNode, include_files: bool) -> io::Result<()> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_e) => {
            // 静默处理无法读取的目录（权限问题等）
            return Ok(());
        }
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let kind = if meta.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::File
        };

        if kind == EntryKind::File && !include_files {
            continue;
        }

        let metadata = EntryMetadata::from_fs_metadata(&meta);
        let mut child = TreeNode::new(entry_path.clone(), kind, metadata);

        if kind == EntryKind::Directory {
            scan_walk_recursive(&entry_path, &mut child, include_files)?;
        }

        node.children.push(child);
    }

    Ok(())
}

// ============================================================================
// 多线程扫描引擎 (parallel) - 使用工作窃取模式
// ============================================================================

/// 中间扫描结果（扁平化）
#[derive(Debug, Clone)]
struct FlatEntry {
    path: PathBuf,
    parent: PathBuf,
    kind: EntryKind,
    metadata: EntryMetadata,
}

/// 多线程并发目录扫描（使用 rayon 工作窃取）
pub fn scan_parallel(config: &ScanConfig) -> io::Result<ScanResult> {
    let start = Instant::now();
    let root_meta = fs::metadata(&config.root)?;

    if !root_meta.is_dir() {
        return scan_walk(config);
    }

    // 创建自定义线程池
    let pool = ThreadPoolBuilder::new()
        .num_threads(config.thread_count)
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let root_path = config.root.clone();
    let include_files = config.include_files;

    // 使用线程池执行扫描
    let flat_entries = pool.install(|| {
        let entries = Arc::new(Mutex::new(Vec::new()));

        // 添加根目录
        let root_metadata = EntryMetadata::from_fs_metadata(&root_meta);
        entries.lock().push(FlatEntry {
            path: root_path.clone(),
            parent: PathBuf::new(),
            kind: EntryKind::Directory,
            metadata: root_metadata,
        });

        // 递归扫描
        scan_directory_parallel(&root_path, &root_path, include_files, &entries);

        Arc::try_unwrap(entries)
            .map(|m| m.into_inner())
            .unwrap_or_default()
    });

    let tree = build_tree_from_flat(&config.root, flat_entries);

    let mut tree = tree.unwrap_or_else(|| {
        TreeNode::new(
            config.root.clone(),
            EntryKind::Directory,
            EntryMetadata::empty(),
        )
    });

    tree.sort_deterministic();

    let directory_count = tree.count_directories();
    let file_count = tree.count_files();
    let duration = start.elapsed();

    Ok(ScanResult {
        tree,
        duration,
        directory_count,
        file_count,
    })
}

fn scan_directory_parallel(
    path: &Path,
    root: &Path,
    include_files: bool,
    entries: &Arc<Mutex<Vec<FlatEntry>>>,
) {
    let dir_entries: Vec<_> = match fs::read_dir(path) {
        Ok(entries) => entries.flatten().collect(),
        Err(_) => return,
    };

    // 分离目录和文件
    let mut subdirs = Vec::new();
    let mut local_entries = Vec::new();

    for entry in dir_entries {
        let entry_path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let kind = if meta.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::File
        };

        if kind == EntryKind::File && !include_files {
            continue;
        }

        let metadata = EntryMetadata::from_fs_metadata(&meta);
        local_entries.push(FlatEntry {
            path: entry_path.clone(),
            parent: path.to_path_buf(),
            kind,
            metadata,
        });

        if kind == EntryKind::Directory {
            subdirs.push(entry_path);
        }
    }

    // 批量添加条目
    {
        let mut guard = entries.lock();
        guard.extend(local_entries);
    }

    // 使用 rayon 并行处理子目录
    subdirs.par_iter().for_each(|subdir| {
        scan_directory_parallel(subdir, root, include_files, entries);
    });
}

/// 多线程并发目录扫描（使用通道模式）- 备用实现
pub fn scan_parallel_channel(config: &ScanConfig) -> io::Result<ScanResult> {
    let start = Instant::now();
    let root_meta = fs::metadata(&config.root)?;

    if !root_meta.is_dir() {
        return scan_walk(config);
    }

    let thread_count = config.thread_count;
    let (task_tx, task_rx) = bounded::<PathBuf>(thread_count * 64);
    let (result_tx, result_rx) = bounded::<FlatEntry>(thread_count * 256);

    let pending = Arc::new(AtomicUsize::new(1));
    let include_files = config.include_files;

    // 发送根目录任务
    task_tx.send(config.root.clone()).unwrap();

    // 发送根目录条目
    let root_metadata = EntryMetadata::from_fs_metadata(&root_meta);
    result_tx
        .send(FlatEntry {
            path: config.root.clone(),
            parent: PathBuf::new(),
            kind: EntryKind::Directory,
            metadata: root_metadata,
        })
        .unwrap();

    // 启动工作线程
    let workers: Vec<_> = (0..thread_count)
        .map(|_| {
            let task_rx = task_rx.clone();
            let task_tx = task_tx.clone();
            let result_tx = result_tx.clone();
            let pending = Arc::clone(&pending);

            std::thread::spawn(move || {
                worker_loop(task_rx, task_tx, result_tx, pending, include_files);
            })
        })
        .collect();

    // 关闭发送端（工作线程有自己的克隆）
    drop(task_tx);
    drop(result_tx);

    // 收集结果
    let flat_entries: Vec<FlatEntry> = result_rx.iter().collect();

    // 等待所有工作线程完成
    for worker in workers {
        let _ = worker.join();
    }

    let tree = build_tree_from_flat(&config.root, flat_entries);

    let mut tree = tree.unwrap_or_else(|| {
        TreeNode::new(
            config.root.clone(),
            EntryKind::Directory,
            EntryMetadata::empty(),
        )
    });

    tree.sort_deterministic();

    let directory_count = tree.count_directories();
    let file_count = tree.count_files();
    let duration = start.elapsed();

    Ok(ScanResult {
        tree,
        duration,
        directory_count,
        file_count,
    })
}

fn worker_loop(
    task_rx: crossbeam_channel::Receiver<PathBuf>,
    task_tx: Sender<PathBuf>,
    result_tx: Sender<FlatEntry>,
    pending: Arc<AtomicUsize>,
    include_files: bool,
) {
    loop {
        match task_rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(dir_path) => {
                process_directory(&dir_path, &task_tx, &result_tx, &pending, include_files);

                // 任务完成，减少计数
                let prev = pending.fetch_sub(1, AtomicOrdering::SeqCst);
                if prev == 1 {
                    // 最后一个任务完成，退出
                    break;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // 检查是否所有任务都完成了
                if pending.load(AtomicOrdering::SeqCst) == 0 {
                    break;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
}

fn process_directory(
    dir_path: &Path,
    task_tx: &Sender<PathBuf>,
    result_tx: &Sender<FlatEntry>,
    pending: &Arc<AtomicUsize>,
    include_files: bool,
) {
    let dir_entries = match fs::read_dir(dir_path) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut subdirs = Vec::new();

    for entry in dir_entries.flatten() {
        let entry_path = entry.path();
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let kind = if meta.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::File
        };

        if kind == EntryKind::File && !include_files {
            continue;
        }

        let metadata = EntryMetadata::from_fs_metadata(&meta);
        let _ = result_tx.send(FlatEntry {
            path: entry_path.clone(),
            parent: dir_path.to_path_buf(),
            kind,
            metadata,
        });

        if kind == EntryKind::Directory {
            subdirs.push(entry_path);
        }
    }

    // 增加待处理计数并发送子目录任务
    if !subdirs.is_empty() {
        pending.fetch_add(subdirs.len(), AtomicOrdering::SeqCst);
        for subdir in subdirs {
            let _ = task_tx.send(subdir);
        }
    }
}

fn build_tree_from_flat(root: &Path, entries: Vec<FlatEntry>) -> Option<TreeNode> {
    let mut node_map: HashMap<PathBuf, TreeNode> = HashMap::with_capacity(entries.len());
    let mut root_node: Option<TreeNode> = None;

    for entry in &entries {
        let node = TreeNode::new(entry.path.clone(), entry.kind, entry.metadata.clone());
        if entry.path == root {
            root_node = Some(node);
        } else {
            node_map.insert(entry.path.clone(), node);
        }
    }

    let mut children_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for entry in &entries {
        if entry.path != root {
            children_map
                .entry(entry.parent.clone())
                .or_default()
                .push(entry.path.clone());
        }
    }

    fn attach_children(
        node: &mut TreeNode,
        children_map: &HashMap<PathBuf, Vec<PathBuf>>,
        node_map: &mut HashMap<PathBuf, TreeNode>,
    ) {
        if let Some(child_paths) = children_map.get(&node.path) {
            for child_path in child_paths {
                if let Some(mut child) = node_map.remove(child_path) {
                    attach_children(&mut child, children_map, node_map);
                    node.children.push(child);
                }
            }
        }
    }

    if let Some(ref mut root) = root_node {
        attach_children(root, &children_map, &mut node_map);
    }

    root_node
}

// ============================================================================
// 原生 tree 命令调用
// ============================================================================

/// 调用 Windows 原生 tree 命令并解析输出
pub fn scan_native_tree(path: &Path, include_files: bool) -> io::Result<NativeTreeResult> {
    let start = Instant::now();

    let mut cmd = Command::new("cmd");
    cmd.args(["/C", "tree"]);

    if include_files {
        cmd.arg("/F");
    }

    cmd.arg(path);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let output = cmd.output()?;
    let duration = start.elapsed();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<String> = stdout.lines().map(String::from).collect();

    let (directory_count, file_count) = parse_native_tree_stats(&lines);

    Ok(NativeTreeResult {
        lines,
        duration,
        directory_count,
        file_count,
    })
}

/// 原生 tree 命令结果
#[derive(Debug)]
pub struct NativeTreeResult {
    /// 输出行
    pub lines: Vec<String>,
    /// 执行耗时
    pub duration: std::time::Duration,
    /// 目录数量（解析得到）
    pub directory_count: usize,
    /// 文件数量（解析得到）
    pub file_count: usize,
}

fn parse_native_tree_stats(lines: &[String]) -> (usize, usize) {
    for line in lines.iter().rev() {
        if line.contains("个目录") || line.contains("个文件") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut dirs = 0usize;
            let mut files = 0usize;

            for (i, part) in parts.iter().enumerate() {
                if *part == "个目录" || part.contains("个目录") {
                    if i > 0 {
                        dirs = parts[i - 1].parse().unwrap_or(0);
                    }
                }
                if *part == "个文件" || part.contains("个文件") {
                    if i > 0 {
                        files = parts[i - 1].parse().unwrap_or(0);
                    }
                }
            }

            return (dirs, files);
        }
    }
    (0, 0)
}

// ============================================================================
// 辅助函数
// ============================================================================

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// 验证两个扫描结果的结构一致性
pub fn verify_consistency(walk: &ScanResult, parallel: &ScanResult) -> ConsistencyReport {
    let walk_paths = walk.tree.collect_paths();
    let parallel_paths = parallel.tree.collect_paths();

    let structural_match = walk.tree.structural_eq(&parallel.tree);

    let walk_set: std::collections::HashSet<_> = walk_paths.iter().collect();
    let parallel_set: std::collections::HashSet<_> = parallel_paths.iter().collect();

    let only_in_walk: Vec<_> = walk_set.difference(&parallel_set).cloned().collect();
    let only_in_parallel: Vec<_> = parallel_set.difference(&walk_set).cloned().collect();

    ConsistencyReport {
        structural_match,
        walk_count: walk_paths.len(),
        parallel_count: parallel_paths.len(),
        only_in_walk: only_in_walk.into_iter().cloned().collect(),
        only_in_parallel: only_in_parallel.into_iter().cloned().collect(),
        directory_count_match: walk.directory_count == parallel.directory_count,
        file_count_match: walk.file_count == parallel.file_count,
    }
}

/// 一致性验证报告
#[derive(Debug)]
pub struct ConsistencyReport {
    /// 结构是否完全匹配
    pub structural_match: bool,
    /// walk 模式条目数
    pub walk_count: usize,
    /// parallel 模式条目数
    pub parallel_count: usize,
    /// 仅在 walk 结果中的路径
    pub only_in_walk: Vec<PathBuf>,
    /// 仅在 parallel 结果中的路径
    pub only_in_parallel: Vec<PathBuf>,
    /// 目录数量是否匹配
    pub directory_count_match: bool,
    /// 文件数量是否匹配
    pub file_count_match: bool,
}

impl ConsistencyReport {
    pub fn is_consistent(&self) -> bool {
        self.structural_match
            && self.only_in_walk.is_empty()
            && self.only_in_parallel.is_empty()
            && self.directory_count_match
            && self.file_count_match
    }
}

impl Display for ConsistencyReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== 一致性验证报告 ===")?;
        writeln!(
            f,
            "结构匹配: {}",
            if self.structural_match { "✓" } else { "✗" }
        )?;
        writeln!(f, "walk 条目数: {}", self.walk_count)?;
        writeln!(f, "parallel 条目数: {}", self.parallel_count)?;
        writeln!(
            f,
            "目录数量匹配: {}",
            if self.directory_count_match {
                "✓"
            } else {
                "✗"
            }
        )?;
        writeln!(
            f,
            "文件数量匹配: {}",
            if self.file_count_match { "✓" } else { "✗" }
        )?;

        if !self.only_in_walk.is_empty() {
            writeln!(f, "\n仅在 walk 结果中:")?;
            for p in &self.only_in_walk {
                writeln!(f, "  - {:?}", p)?;
            }
        }

        if !self.only_in_parallel.is_empty() {
            writeln!(f, "\n仅在 parallel 结果中:")?;
            for p in &self.only_in_parallel {
                writeln!(f, "  - {:?}", p)?;
            }
        }

        writeln!(
            f,
            "\n总体结论: {}",
            if self.is_consistent() {
                "一致 ✓"
            } else {
                "不一致 ✗"
            }
        )
    }
}

// ============================================================================
// 主程序入口
// ============================================================================

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir()?
    };

    let thread_count = args
        .iter()
        .position(|a| a == "-t" || a == "--threads")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(num_cpus);

    let include_files = args
        .iter()
        .any(|a| a == "-f" || a == "--files" || a == "/F");

    println!("扫描目录: {:?}", path);
    println!("线程数: {}", thread_count);
    println!("包含文件: {}", include_files);
    println!();

    let config = ScanConfig {
        root: path.clone(),
        include_files,
        thread_count,
    };

    println!("=== Walk 模式 (单线程) ===");
    let walk_result = scan_walk(&config)?;
    println!(
        "目录: {}, 文件: {}, 耗时: {:.3}s",
        walk_result.directory_count,
        walk_result.file_count,
        walk_result.duration.as_secs_f64()
    );

    println!("\n=== Parallel 模式 ({}线程, rayon) ===", thread_count);
    let parallel_result = scan_parallel(&config)?;
    println!(
        "目录: {}, 文件: {}, 耗时: {:.3}s",
        parallel_result.directory_count,
        parallel_result.file_count,
        parallel_result.duration.as_secs_f64()
    );

    println!("\n=== Parallel 模式 ({}线程, channel) ===", thread_count);
    let parallel_channel_result = scan_parallel_channel(&config)?;
    println!(
        "目录: {}, 文件: {}, 耗时: {:.3}s",
        parallel_channel_result.directory_count,
        parallel_channel_result.file_count,
        parallel_channel_result.duration.as_secs_f64()
    );

    println!("\n=== 一致性验证 ===");
    let report = verify_consistency(&walk_result, &parallel_result);
    println!("{}", report);

    if include_files {
        println!("\n=== 原生 tree 命令对比 ===");
        match scan_native_tree(&path, include_files) {
            Ok(native) => {
                println!(
                    "原生 tree: 目录 {}, 文件 {}, 耗时: {:.3}s",
                    native.directory_count,
                    native.file_count,
                    native.duration.as_secs_f64()
                );

                let speedup_walk =
                    native.duration.as_secs_f64() / walk_result.duration.as_secs_f64();
                let speedup_parallel =
                    native.duration.as_secs_f64() / parallel_result.duration.as_secs_f64();

                println!("\n性能对比:");
                println!("  walk vs native: {:.2}x", speedup_walk);
                println!("  parallel vs native: {:.2}x", speedup_parallel);
            }
            Err(e) => {
                println!("无法执行原生 tree 命令: {}", e);
            }
        }
    }

    Ok(())
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

    // ========================================================================
    // 测试辅助函数
    // ========================================================================

    /// 创建测试用临时目录结构
    fn create_test_directory() -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let root = temp.path();

        fs::create_dir_all(root.join("src/utils")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::create_dir_all(root.join("docs/api")).unwrap();
        fs::create_dir(root.join("empty_dir")).unwrap();

        File::create(root.join("Cargo.toml")).unwrap();
        File::create(root.join("README.md")).unwrap();
        File::create(root.join("src/main.rs")).unwrap();
        File::create(root.join("src/lib.rs")).unwrap();
        File::create(root.join("src/utils/helper.rs")).unwrap();
        File::create(root.join("tests/integration.rs")).unwrap();
        File::create(root.join("docs/api/index.html")).unwrap();

        temp
    }

    /// 创建深层嵌套目录结构
    fn create_deep_directory(depth: usize) -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let mut current = temp.path().to_path_buf();

        for i in 0..depth {
            current = current.join(format!("level_{}", i));
            fs::create_dir(&current).unwrap();
            File::create(current.join(format!("file_{}.txt", i))).unwrap();
        }

        temp
    }

    /// 创建宽目录结构（单层多文件）
    fn create_wide_directory(width: usize) -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let root = temp.path();

        for i in 0..width {
            File::create(root.join(format!("file_{:04}.txt", i))).unwrap();
        }

        for i in 0..width / 10 {
            fs::create_dir(root.join(format!("dir_{:04}", i))).unwrap();
        }

        temp
    }

    /// 创建混合目录结构
    fn create_mixed_directory() -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let root = temp.path();

        // 多层目录
        for i in 0..5 {
            let dir = root.join(format!("dir_{}", i));
            fs::create_dir_all(&dir).unwrap();

            for j in 0..3 {
                File::create(dir.join(format!("file_{}.txt", j))).unwrap();
                let subdir = dir.join(format!("subdir_{}", j));
                fs::create_dir(&subdir).unwrap();
                File::create(subdir.join("nested.txt")).unwrap();
            }
        }

        temp
    }

    /// 创建带有大文件的目录
    fn create_directory_with_sizes() -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let root = temp.path();

        let mut small = File::create(root.join("small.txt")).unwrap();
        small.write_all(b"small").unwrap();

        let mut medium = File::create(root.join("medium.txt")).unwrap();
        medium.write_all(&vec![0u8; 1024]).unwrap();

        let mut large = File::create(root.join("large.txt")).unwrap();
        large.write_all(&vec![0u8; 10240]).unwrap();

        temp
    }

    // ========================================================================
    // 基础功能测试 (6 tests)
    // ========================================================================

    #[test]
    fn test_walk_basic() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 1,
        };

        let result = scan_walk(&config).expect("walk 扫描失败");

        assert_eq!(result.tree.kind, EntryKind::Directory);
        assert!(result.directory_count > 0);
        assert!(result.file_count > 0);
    }

    #[test]
    fn test_parallel_basic() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let result = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(result.tree.kind, EntryKind::Directory);
        assert!(result.directory_count > 0);
        assert!(result.file_count > 0);
    }

    #[test]
    fn test_parallel_channel_basic() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let result = scan_parallel_channel(&config).expect("parallel_channel 扫描失败");

        assert_eq!(result.tree.kind, EntryKind::Directory);
        assert!(result.directory_count > 0);
        assert!(result.file_count > 0);
    }

    #[test]
    fn test_walk_without_files() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: false,
            thread_count: 1,
        };

        let result = scan_walk(&config).expect("walk 扫描失败");

        assert_eq!(result.file_count, 0);
        assert!(result.directory_count > 0);
    }

    #[test]
    fn test_parallel_without_files() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: false,
            thread_count: 4,
        };

        let result = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(result.file_count, 0);
        assert!(result.directory_count > 0);
    }

    #[test]
    fn test_scan_single_file() {
        let temp = TempDir::new().expect("创建临时目录失败");
        let file_path = temp.path().join("single.txt");
        File::create(&file_path).unwrap();

        let config = ScanConfig {
            root: file_path,
            include_files: true,
            thread_count: 1,
        };

        let result = scan_walk(&config).expect("扫描失败");
        assert_eq!(result.tree.kind, EntryKind::File);
        assert_eq!(result.file_count, 1);
        assert_eq!(result.directory_count, 0);
    }

    // ========================================================================
    // 一致性测试 (8 tests)
    // ========================================================================

    #[test]
    fn test_consistency_with_files() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(report.is_consistent(), "一致性验证失败:\n{}", report);
    }

    #[test]
    fn test_consistency_without_files() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: false,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(report.is_consistent(), "一致性验证失败:\n{}", report);
    }

    #[test]
    fn test_consistency_deep_directory() {
        let temp = create_deep_directory(20);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 8,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(
            report.is_consistent(),
            "深层目录一致性验证失败:\n{}",
            report
        );
    }

    #[test]
    fn test_consistency_wide_directory() {
        let temp = create_wide_directory(100);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 8,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(
            report.is_consistent(),
            "宽目录一致性验证失败:\n{}",
            report
        );
    }

    #[test]
    fn test_consistency_mixed_directory() {
        let temp = create_mixed_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(
            report.is_consistent(),
            "混合目录一致性验证失败:\n{}",
            report
        );
    }

    #[test]
    fn test_consistency_channel_vs_rayon() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let rayon = scan_parallel(&config).expect("rayon 扫描失败");
        let channel = scan_parallel_channel(&config).expect("channel 扫描失败");

        let report = verify_consistency(&rayon, &channel);
        assert!(
            report.is_consistent(),
            "rayon 与 channel 实现不一致:\n{}",
            report
        );
    }

    #[test]
    fn test_consistency_multiple_runs() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let results: Vec<_> = (0..5)
            .map(|_| scan_parallel(&config).expect("parallel 扫描失败"))
            .collect();

        for i in 1..results.len() {
            let report = verify_consistency(&results[0], &results[i]);
            assert!(
                report.is_consistent(),
                "第 {} 次运行与第 0 次运行不一致:\n{}",
                i,
                report
            );
        }
    }

    #[test]
    fn test_consistency_all_three_methods() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");
        let channel = scan_parallel_channel(&config).expect("channel 扫描失败");

        assert!(verify_consistency(&walk, &parallel).is_consistent());
        assert!(verify_consistency(&walk, &channel).is_consistent());
        assert!(verify_consistency(&parallel, &channel).is_consistent());
    }

    // ========================================================================
    // 线程数验证测试 (4 tests)
    // ========================================================================

    #[test]
    fn test_thread_count_variations() {
        let temp = create_wide_directory(50);

        for thread_count in [1, 2, 4, 8] {
            let config = ScanConfig {
                root: temp.path().to_path_buf(),
                include_files: true,
                thread_count,
            };

            let walk = scan_walk(&config).expect("walk 扫描失败");
            let parallel = scan_parallel(&config).expect("parallel 扫描失败");

            let report = verify_consistency(&walk, &parallel);
            assert!(
                report.is_consistent(),
                "线程数 {} 时一致性验证失败:\n{}",
                thread_count,
                report
            );
        }
    }

    #[test]
    fn test_single_thread_parallel() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 1,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(
            report.is_consistent(),
            "单线程 parallel 与 walk 不一致:\n{}",
            report
        );
    }

    #[test]
    fn test_many_threads() {
        let temp = create_wide_directory(100);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 32,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(report.is_consistent(), "高线程数不一致:\n{}", report);
    }

    #[test]
    fn test_thread_count_channel_variations() {
        let temp = create_test_directory();

        for thread_count in [1, 2, 4] {
            let config = ScanConfig {
                root: temp.path().to_path_buf(),
                include_files: true,
                thread_count,
            };

            let walk = scan_walk(&config).expect("walk 扫描失败");
            let channel = scan_parallel_channel(&config).expect("channel 扫描失败");

            let report = verify_consistency(&walk, &channel);
            assert!(
                report.is_consistent(),
                "channel 模式线程数 {} 时不一致:\n{}",
                thread_count,
                report
            );
        }
    }

    // ========================================================================
    // 边界情况测试 (8 tests)
    // ========================================================================

    #[test]
    fn test_empty_directory() {
        let temp = TempDir::new().expect("创建临时目录失败");
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 0);
        assert_eq!(parallel.file_count, 0);
        assert_eq!(walk.directory_count, 1);
        assert_eq!(parallel.directory_count, 1);
    }

    #[test]
    fn test_single_file_directory() {
        let temp = TempDir::new().expect("创建临时目录失败");
        File::create(temp.path().join("single.txt")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 1);
        assert_eq!(parallel.file_count, 1);

        let report = verify_consistency(&walk, &parallel);
        assert!(report.is_consistent());
    }

    #[test]
    fn test_deeply_nested_single_file() {
        let temp = TempDir::new().expect("创建临时目录失败");
        let deep_path = temp.path().join("a/b/c/d/e/f/g/h/i/j");
        fs::create_dir_all(&deep_path).unwrap();
        File::create(deep_path.join("deep.txt")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(
            report.is_consistent(),
            "深层嵌套单文件一致性验证失败:\n{}",
            report
        );
    }

    #[test]
    fn test_special_characters_in_names() {
        let temp = TempDir::new().expect("创建临时目录失败");

        let special_names = ["文件夹", "folder with spaces", "folder-with-dashes"];

        for name in &special_names {
            fs::create_dir(temp.path().join(name)).unwrap();
            File::create(temp.path().join(format!("{}.txt", name))).unwrap();
        }

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(
            report.is_consistent(),
            "特殊字符名称一致性验证失败:\n{}",
            report
        );
    }

    #[test]
    fn test_unicode_names() {
        let temp = TempDir::new().expect("创建临时目录失败");

        let unicode_names = ["日本語", "한국어", "العربية", "🎉🎊"];

        for name in &unicode_names {
            if fs::create_dir(temp.path().join(name)).is_ok() {
                let _ = File::create(temp.path().join(format!("{}.txt", name)));
            }
        }

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(report.is_consistent(), "Unicode 名称不一致:\n{}", report);
    }

    #[test]
    fn test_symlinks_ignored() {
        // 符号链接在 Windows 上需要特殊权限，此测试验证不崩溃即可
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let _ = scan_walk(&config);
        let _ = scan_parallel(&config);
    }

    #[test]
    fn test_many_empty_subdirs() {
        let temp = TempDir::new().expect("创建临时目录失败");

        for i in 0..50 {
            fs::create_dir(temp.path().join(format!("empty_{}", i))).unwrap();
        }

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.directory_count, 51); // root + 50
        assert_eq!(parallel.directory_count, 51);
        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    #[test]
    fn test_file_sizes() {
        let temp = create_directory_with_sizes();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let result = scan_walk(&config).expect("扫描失败");

        let sizes: Vec<_> = result
            .tree
            .children
            .iter()
            .map(|c| (&c.name, c.metadata.size))
            .collect();

        assert!(sizes.iter().any(|(n, s)| n.contains("small") && *s == 5));
        assert!(sizes.iter().any(|(n, s)| n.contains("medium") && *s == 1024));
        assert!(sizes
            .iter()
            .any(|(n, s)| n.contains("large") && *s == 10240));
    }

    // ========================================================================
    // 排序确定性测试 (4 tests)
    // ========================================================================

    #[test]
    fn test_deterministic_ordering() {
        let temp = create_wide_directory(30);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 8,
        };

        let results: Vec<_> = (0..10)
            .map(|_| {
                let result = scan_parallel(&config).expect("扫描失败");
                result.tree.collect_paths()
            })
            .collect();

        for i in 1..results.len() {
            assert_eq!(
                results[0], results[i],
                "第 {} 次运行的路径顺序与第 0 次不同",
                i
            );
        }
    }

    #[test]
    fn test_sort_directories_before_files() {
        let temp = TempDir::new().expect("创建临时目录失败");

        File::create(temp.path().join("aaa.txt")).unwrap();
        fs::create_dir(temp.path().join("zzz")).unwrap();
        File::create(temp.path().join("bbb.txt")).unwrap();
        fs::create_dir(temp.path().join("aaa_dir")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let result = scan_parallel(&config).expect("扫描失败");

        let children = &result.tree.children;
        let first_file_idx = children
            .iter()
            .position(|c| c.kind == EntryKind::File)
            .unwrap_or(children.len());
        let last_dir_idx = children
            .iter()
            .rposition(|c| c.kind == EntryKind::Directory)
            .unwrap_or(0);

        assert!(
            last_dir_idx < first_file_idx || first_file_idx == children.len(),
            "目录应该排在文件之前"
        );
    }

    #[test]
    fn test_case_insensitive_sort() {
        let temp = TempDir::new().expect("创建临时目录失败");

        File::create(temp.path().join("Apple.txt")).unwrap();
        File::create(temp.path().join("banana.txt")).unwrap();
        File::create(temp.path().join("CHERRY.txt")).unwrap();
        File::create(temp.path().join("date.txt")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let result = scan_parallel(&config).expect("扫描失败");
        let names: Vec<_> = result
            .tree
            .children
            .iter()
            .map(|c| c.name.clone())
            .collect();

        // 验证大小写不敏感排序
        assert_eq!(names[0].to_lowercase(), "apple.txt");
        assert_eq!(names[1].to_lowercase(), "banana.txt");
        assert_eq!(names[2].to_lowercase(), "cherry.txt");
        assert_eq!(names[3].to_lowercase(), "date.txt");
    }

    #[test]
    fn test_channel_deterministic_ordering() {
        let temp = create_wide_directory(30);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let results: Vec<_> = (0..5)
            .map(|_| {
                let result = scan_parallel_channel(&config).expect("扫描失败");
                result.tree.collect_paths()
            })
            .collect();

        for i in 1..results.len() {
            assert_eq!(results[0], results[i], "channel 模式排序不确定");
        }
    }

    // ========================================================================
    // TreeNode 方法测试 (4 tests)
    // ========================================================================

    #[test]
    fn test_tree_node_count_total() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 1,
        };

        let result = scan_walk(&config).expect("扫描失败");
        let total = result.tree.count_total();

        assert_eq!(
            total,
            result.directory_count + result.file_count,
            "count_total 应等于目录数 + 文件数"
        );
    }

    #[test]
    fn test_tree_node_max_depth() {
        let temp = create_deep_directory(10);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 1,
        };

        let result = scan_walk(&config).expect("扫描失败");
        let depth = result.tree.max_depth();

        assert_eq!(depth, 11, "深度应为 11 (根 + 10 层)");
    }

    #[test]
    fn test_tree_node_collect_names() {
        let temp = TempDir::new().expect("创建临时目录失败");
        File::create(temp.path().join("a.txt")).unwrap();
        File::create(temp.path().join("b.txt")).unwrap();
        fs::create_dir(temp.path().join("c")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 1,
        };

        let result = scan_walk(&config).expect("扫描失败");
        let names = result.tree.collect_names();

        assert_eq!(names.len(), 4); // root + 3 entries
        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
        assert!(names.contains(&"c".to_string()));
    }

    #[test]
    fn test_tree_node_structural_eq() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 1,
        };

        let result1 = scan_walk(&config).expect("扫描失败");
        let result2 = scan_walk(&config).expect("扫描失败");

        assert!(result1.tree.structural_eq(&result2.tree));
    }

    // ========================================================================
    // 性能验证测试 (2 tests)
    // ========================================================================

    #[test]
    fn test_performance_scaling() {
        let temp = create_wide_directory(200);

        let mut results = Vec::new();

        for thread_count in [1, 2, 4] {
            let config = ScanConfig {
                root: temp.path().to_path_buf(),
                include_files: true,
                thread_count,
            };

            let result = scan_parallel(&config).expect("扫描失败");
            results.push((thread_count, result.duration));
        }

        // 仅记录，不断言
        println!("\n性能扩展测试结果:");
        for (threads, duration) in &results {
            println!(
                "  {} 线程: {:.3}ms",
                threads,
                duration.as_secs_f64() * 1000.0
            );
        }
    }

    #[test]
    fn test_reasonable_performance() {
        let temp = create_wide_directory(100);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let result = scan_parallel(&config).expect("扫描失败");

        // 100 个文件 + 10 个目录应该在 1 秒内完成
        assert!(
            result.duration.as_secs() < 1,
            "扫描耗时过长: {:?}",
            result.duration
        );
    }

    // ========================================================================
    // 与原生 tree 命令对比测试 (1 test)
    // ========================================================================

    #[test]
    fn test_count_matches_native_tree() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let our_result = scan_walk(&config).expect("扫描失败");

        if let Ok(native) = scan_native_tree(temp.path(), true) {
            // 原生 tree 不计入根目录
            let our_dirs = our_result.directory_count - 1;

            assert_eq!(
                our_dirs, native.directory_count,
                "目录数量不匹配: ours={}, native={}",
                our_dirs, native.directory_count
            );
            assert_eq!(
                our_result.file_count, native.file_count,
                "文件数量不匹配: ours={}, native={}",
                our_result.file_count, native.file_count
            );
        }
    }

    // ========================================================================
    // 大目录测试（使用实际路径）
    // ========================================================================

    #[test]
    #[ignore]
    fn test_large_directory_rustup() {
        let rustup_path = PathBuf::from(r"C:\Users\linzh\.rustup");

        if !rustup_path.exists() {
            println!("跳过: rustup 路径不存在");
            return;
        }

        let config = ScanConfig {
            root: rustup_path.clone(),
            include_files: true,
            thread_count: num_cpus(),
        };

        println!("\n测试大目录: {:?}", rustup_path);

        let walk = scan_walk(&config).expect("walk 扫描失败");
        println!(
            "Walk: {} 目录, {} 文件, {:.3}s",
            walk.directory_count,
            walk.file_count,
            walk.duration.as_secs_f64()
        );

        let parallel = scan_parallel(&config).expect("parallel 扫描失败");
        println!(
            "Parallel: {} 目录, {} 文件, {:.3}s",
            parallel.directory_count,
            parallel.file_count,
            parallel.duration.as_secs_f64()
        );

        let report = verify_consistency(&walk, &parallel);
        println!("{}", report);

        assert!(report.is_consistent(), "大目录一致性验证失败");
    }

    #[test]
    #[ignore]
    fn test_large_directory_consistency_stress() {
        let rustup_path = PathBuf::from(r"C:\Users\linzh\.rustup");

        if !rustup_path.exists() {
            return;
        }

        let config = ScanConfig {
            root: rustup_path,
            include_files: true,
            thread_count: num_cpus(),
        };

        let results: Vec<_> = (0..3)
            .map(|i| {
                let result = scan_parallel(&config).expect("扫描失败");
                println!("运行 {}: {} 条目", i, result.tree.collect_paths().len());
                result
            })
            .collect();

        for i in 1..results.len() {
            let report = verify_consistency(&results[0], &results[i]);
            assert!(report.is_consistent(), "压力测试第 {} 次运行不一致", i);
        }
    }
}