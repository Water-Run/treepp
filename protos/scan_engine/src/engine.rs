//! 扫描引擎核心实现
//!
//! 提供单线程和多线程两种目录扫描模式，以及一致性验证功能。

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime};

use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

use crate::error::{ScanError, ScanResult};

// ============================================================================
// 常量定义
// ============================================================================

/// 默认线程数
const DEFAULT_THREAD_COUNT: usize = 8;

/// 性能测试预热次数
pub const WARMUP_RUNS: usize = 2;

/// 性能测试采样次数
pub const BENCHMARK_RUNS: usize = 3;

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

impl Display for EntryKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory => write!(f, "目录"),
            Self::File => write!(f, "文件"),
        }
    }
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
    /// 从文件系统元数据创建
    fn from_fs_metadata(meta: &Metadata) -> Self {
        Self {
            size: if meta.is_file() { meta.len() } else { 0 },
            modified: meta.modified().ok(),
            created: meta.created().ok(),
        }
    }

    /// 创建空元数据（用于无法读取元数据的情况）
    fn empty() -> Self {
        Self {
            size: 0,
            modified: None,
            created: None,
        }
    }
}

impl Default for EntryMetadata {
    fn default() -> Self {
        Self::empty()
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
    /// 创建新的叶子节点
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
        }
    }

    /// 创建带子节点的目录节点
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
        }
    }

    /// 递归统计目录数量
    pub fn count_directories(&self) -> usize {
        let self_count = usize::from(self.kind == EntryKind::Directory);
        self_count + self.children.iter().map(Self::count_directories).sum::<usize>()
    }

    /// 递归统计文件数量
    pub fn count_files(&self) -> usize {
        let self_count = usize::from(self.kind == EntryKind::File);
        self_count + self.children.iter().map(Self::count_files).sum::<usize>()
    }

    /// 递归统计总条目数
    pub fn count_total(&self) -> usize {
        1 + self.children.iter().map(Self::count_total).sum::<usize>()
    }

    /// 计算最大深度（仅计算目录深度）
    pub fn max_depth(&self) -> usize {
        if self.kind == EntryKind::File {
            return 0;
        }

        let child_max = self
            .children
            .iter()
            .filter(|c| c.kind == EntryKind::Directory)
            .map(Self::max_depth)
            .max()
            .unwrap_or(0);

        1 + child_max
    }

    /// 对子节点进行确定性排序（递归）
    ///
    /// 排序规则：
    /// 1. 目录在前，文件在后
    /// 2. 同类型按名称字典序（大小写不敏感）
    pub fn sort_deterministic(&mut self) {
        self.children.sort_by(|a, b| {
            match (a.kind, b.kind) {
                (EntryKind::Directory, EntryKind::File) => Ordering::Less,
                (EntryKind::File, EntryKind::Directory) => Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        for child in &mut self.children {
            child.sort_deterministic();
        }
    }

    /// 深度结构相等比较（忽略元数据）
    pub fn structural_eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.kind == other.kind
            && self.children.len() == other.children.len()
            && self
            .children
            .iter()
            .zip(other.children.iter())
            .all(|(a, b)| a.structural_eq(b))
    }

    /// 收集所有路径
    pub fn collect_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![self.path.clone()];
        for child in &self.children {
            paths.extend(child.collect_paths());
        }
        paths
    }

    /// 收集所有名称
    pub fn collect_names(&self) -> Vec<String> {
        let mut names = vec![self.name.clone()];
        for child in &self.children {
            names.extend(child.collect_names());
        }
        names
    }

    /// 格式化输出辅助方法
    fn fmt_tree(&self, f: &mut Formatter<'_>, prefix: &str, is_last: bool) -> fmt::Result {
        let connector = if prefix.is_empty() {
            ""
        } else if is_last {
            "└─"
        } else {
            "├─"
        };

        let kind_suffix = match self.kind {
            EntryKind::Directory => "/",
            EntryKind::File => "",
        };

        writeln!(f, "{prefix}{connector}{}{kind_suffix}", self.name)?;

        let child_prefix = if prefix.is_empty() {
            String::new()
        } else if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };

        for (i, child) in self.children.iter().enumerate() {
            let child_is_last = i == self.children.len() - 1;
            child.fmt_tree(f, &child_prefix, child_is_last)?;
        }

        Ok(())
    }
}

impl Display for TreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_tree(f, "", true)
    }
}

/// 扫描配置构建器
#[derive(Debug, Clone)]
pub struct ScanConfigBuilder {
    root: PathBuf,
    include_files: bool,
    thread_count: usize,
}

impl ScanConfigBuilder {
    /// 设置根路径
    pub fn root(mut self, path: PathBuf) -> Self {
        self.root = path;
        self
    }

    /// 设置是否包含文件
    pub fn include_files(mut self, include: bool) -> Self {
        self.include_files = include;
        self
    }

    /// 设置线程数
    pub fn thread_count(mut self, count: usize) -> Self {
        self.thread_count = count.max(1);
        self
    }

    /// 构建配置
    pub fn build(self) -> ScanConfig {
        ScanConfig {
            root: self.root,
            include_files: self.include_files,
            thread_count: self.thread_count,
        }
    }
}

impl Default for ScanConfigBuilder {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            include_files: true,
            thread_count: DEFAULT_THREAD_COUNT,
        }
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

impl ScanConfig {
    /// 创建配置构建器
    pub fn builder() -> ScanConfigBuilder {
        ScanConfigBuilder::default()
    }

    /// 验证配置有效性
    pub fn validate(&self) -> ScanResult<()> {
        if !self.root.exists() {
            return Err(ScanError::PathNotFound {
                path: self.root.clone(),
            });
        }
        Ok(())
    }
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// 扫描结果
#[derive(Debug)]
pub struct ScanStats {
    /// 根节点
    pub tree: TreeNode,
    /// 扫描耗时
    pub duration: std::time::Duration,
    /// 目录总数
    pub directory_count: usize,
    /// 文件总数
    pub file_count: usize,
}

impl Display for ScanStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.tree)?;
        writeln!(
            f,
            "\n{} 个目录, {} 个文件",
            self.directory_count, self.file_count
        )?;
        write!(f, "耗时: {:.3}s", self.duration.as_secs_f64())
    }
}

/// 原生 tree 命令结果
#[derive(Debug)]
pub struct NativeTreeResult {
    /// 输出行
    pub lines: Vec<String>,
    /// 执行耗时
    pub duration: std::time::Duration,
    /// 目录数量
    pub directory_count: usize,
    /// 文件数量
    pub file_count: usize,
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
    /// 检查是否完全一致
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
            if self.directory_count_match { "✓" } else { "✗" }
        )?;
        writeln!(
            f,
            "文件数量匹配: {}",
            if self.file_count_match { "✓" } else { "✗" }
        )?;

        if !self.only_in_walk.is_empty() {
            writeln!(f, "\n仅在 walk 结果中:")?;
            for p in &self.only_in_walk {
                writeln!(f, "  - {}", p.display())?;
            }
        }

        if !self.only_in_parallel.is_empty() {
            writeln!(f, "\n仅在 parallel 结果中:")?;
            for p in &self.only_in_parallel {
                writeln!(f, "  - {}", p.display())?;
            }
        }

        write!(
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
// 公共函数
// ============================================================================

/// 获取可用并行度
pub fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(DEFAULT_THREAD_COUNT)
}

/// 单线程递归目录扫描
///
/// # 错误
///
/// 当根路径不存在或无法读取时返回错误。
pub fn scan_walk(config: &ScanConfig) -> crate::error::ScanResult<ScanStats> {
    config.validate()?;

    let start = Instant::now();

    let root_meta = fs::metadata(&config.root).map_err(|e| ScanError::MetadataError {
        path: config.root.clone(),
        source: e,
    })?;

    let root_metadata = EntryMetadata::from_fs_metadata(&root_meta);
    let kind = if root_meta.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::File
    };

    let mut root = TreeNode::new(config.root.clone(), kind, root_metadata);

    if root.kind == EntryKind::Directory {
        walk_recursive(&config.root, &mut root, config.include_files);
    }

    root.sort_deterministic();

    let directory_count = root.count_directories();
    let file_count = root.count_files();
    let duration = start.elapsed();

    Ok(ScanStats {
        tree: root,
        duration,
        directory_count,
        file_count,
    })
}

/// 多线程并发目录扫描
///
/// 使用 rayon 分治策略并行扫描目录树。
///
/// # 错误
///
/// 当根路径不存在、无法读取或线程池创建失败时返回错误。
pub fn scan_parallel(config: &ScanConfig) -> crate::error::ScanResult<ScanStats> {
    config.validate()?;

    let start = Instant::now();

    let root_meta = fs::metadata(&config.root).map_err(|e| ScanError::MetadataError {
        path: config.root.clone(),
        source: e,
    })?;

    if !root_meta.is_dir() {
        return scan_walk(config);
    }

    let pool = ThreadPoolBuilder::new()
        .num_threads(config.thread_count)
        .build()
        .map_err(|e| ScanError::ThreadPoolError(e.to_string()))?;

    let root_path = config.root.clone();
    let include_files = config.include_files;
    let root_metadata = EntryMetadata::from_fs_metadata(&root_meta);

    let tree = pool.install(|| parallel_divide_conquer(&root_path, include_files));

    let mut tree = tree.unwrap_or_else(|| {
        TreeNode::new(root_path, EntryKind::Directory, root_metadata)
    });

    tree.sort_deterministic();

    let directory_count = tree.count_directories();
    let file_count = tree.count_files();
    let duration = start.elapsed();

    Ok(ScanStats {
        tree,
        duration,
        directory_count,
        file_count,
    })
}

/// 调用原生 tree 命令
pub fn scan_native_tree(path: &Path, include_files: bool) -> crate::error::ScanResult<NativeTreeResult> {
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

    let (directory_count, file_count) = parse_native_output(&lines, include_files);

    Ok(NativeTreeResult {
        lines,
        duration,
        directory_count,
        file_count,
    })
}

/// 验证两个扫描结果的一致性
pub fn verify_consistency(walk: &ScanStats, parallel: &ScanStats) -> ConsistencyReport {
    let walk_paths = walk.tree.collect_paths();
    let parallel_paths = parallel.tree.collect_paths();

    let walk_set: HashSet<_> = walk_paths.iter().collect();
    let parallel_set: HashSet<_> = parallel_paths.iter().collect();

    let only_in_walk: Vec<PathBuf> = walk_set
        .difference(&parallel_set)
        .map(|p| (*p).clone())
        .collect();

    let only_in_parallel: Vec<PathBuf> = parallel_set
        .difference(&walk_set)
        .map(|p| (*p).clone())
        .collect();

    ConsistencyReport {
        structural_match: walk.tree.structural_eq(&parallel.tree),
        walk_count: walk_paths.len(),
        parallel_count: parallel_paths.len(),
        only_in_walk,
        only_in_parallel,
        directory_count_match: walk.directory_count == parallel.directory_count,
        file_count_match: walk.file_count == parallel.file_count,
    }
}

// ============================================================================
// 私有函数
// ============================================================================

/// 单线程递归扫描实现
fn walk_recursive(path: &Path, node: &mut TreeNode, include_files: bool) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
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
            walk_recursive(&entry_path, &mut child, include_files);
        }

        node.children.push(child);
    }
}

/// 多线程分治扫描实现
fn parallel_divide_conquer(path: &Path, include_files: bool) -> Option<TreeNode> {
    let meta = fs::metadata(path).ok()?;
    let metadata = EntryMetadata::from_fs_metadata(&meta);

    let dir_entries: Vec<_> = fs::read_dir(path).ok()?.flatten().collect();

    let mut subdirs = Vec::new();
    let mut files = Vec::new();

    for entry in dir_entries {
        let entry_path = entry.path();
        let Ok(entry_meta) = entry.metadata() else {
            continue;
        };

        if entry_meta.is_dir() {
            subdirs.push(entry_path);
        } else if include_files {
            let file_metadata = EntryMetadata::from_fs_metadata(&entry_meta);
            files.push(TreeNode::new(entry_path, EntryKind::File, file_metadata));
        }
    }

    let subdir_trees: Vec<TreeNode> = subdirs
        .into_par_iter()
        .filter_map(|subdir| parallel_divide_conquer(&subdir, include_files))
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

/// 解析原生 tree 命令输出
fn parse_native_output(lines: &[String], include_files: bool) -> (usize, usize) {
    let mut dir_count = 0usize;
    let mut file_count = 0usize;

    let content_start = lines
        .iter()
        .position(|l| l.contains('├') || l.contains('└') || l.contains('│'))
        .unwrap_or(0);

    for line in lines.iter().skip(content_start) {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.contains("没有子文件夹") {
            continue;
        }

        let name: String = line
            .replace('├', "")
            .replace('└', "")
            .replace('│', "")
            .replace('─', "")
            .trim()
            .to_string();

        if name.is_empty() {
            continue;
        }

        if !include_files {
            dir_count += 1;
        } else {
            let has_branch = line.contains("├─") || line.contains("└─");

            if has_branch {
                if name.contains('.') && !name.starts_with('.') {
                    file_count += 1;
                } else {
                    dir_count += 1;
                }
            } else if line.contains('│') || line.starts_with("   ") {
                file_count += 1;
            }
        }
    }

    (dir_count, file_count)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    // ========================================================================
    // 测试辅助
    // ========================================================================

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

    fn create_deep_directory(depth: usize) -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let mut current = temp.path().to_path_buf();

        for i in 0..depth {
            current = current.join(format!("level_{i}"));
            fs::create_dir(&current).unwrap();
        }

        temp
    }

    fn create_deep_directory_with_files(depth: usize) -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let mut current = temp.path().to_path_buf();

        for i in 0..depth {
            current = current.join(format!("level_{i}"));
            fs::create_dir(&current).unwrap();
            File::create(current.join(format!("file_{i}.txt"))).unwrap();
        }

        temp
    }

    fn create_wide_directory(width: usize) -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let root = temp.path();

        for i in 0..width {
            File::create(root.join(format!("file_{i:04}.txt"))).unwrap();
        }

        for i in 0..width / 10 {
            fs::create_dir(root.join(format!("dir_{i:04}"))).unwrap();
        }

        temp
    }

    fn create_mixed_directory() -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let root = temp.path();

        for i in 0..5 {
            let dir = root.join(format!("dir_{i}"));
            fs::create_dir_all(&dir).unwrap();

            for j in 0..3 {
                File::create(dir.join(format!("file_{j}.txt"))).unwrap();
                let subdir = dir.join(format!("subdir_{j}"));
                fs::create_dir(&subdir).unwrap();
                File::create(subdir.join("nested.txt")).unwrap();
            }
        }

        temp
    }

    fn create_complex_nested() -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let root = temp.path();

        for i in 0..3 {
            let branch = root.join(format!("branch_{i}"));
            fs::create_dir(&branch).unwrap();

            for j in 0..3 {
                let sub = branch.join(format!("sub_{j}"));
                fs::create_dir(&sub).unwrap();

                for k in 0..2 {
                    let deep = sub.join(format!("deep_{k}"));
                    fs::create_dir(&deep).unwrap();
                    File::create(deep.join("leaf.txt")).unwrap();
                }
            }
        }

        temp
    }

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

    fn get_rustup_path() -> Option<PathBuf> {
        env::var("USERPROFILE")
            .ok()
            .map(|home| PathBuf::from(home).join(".rustup"))
            .filter(|p| p.exists())
            .or_else(|| {
                env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".rustup"))
                    .filter(|p| p.exists())
            })
    }

    fn get_windows_path() -> Option<PathBuf> {
        let path = PathBuf::from("C:\\Windows");
        path.exists().then_some(path)
    }

    fn assert_consistency(walk: &ScanStats, parallel: &ScanStats, context: &str) {
        let report = verify_consistency(walk, parallel);
        assert!(report.is_consistent(), "{context}:\n{report}");
    }

    // ========================================================================
    // 基础功能测试
    // ========================================================================

    #[test]
    fn test_walk_basic_structure() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .build();

        let result = scan_walk(&config).expect("walk 扫描失败");

        assert_eq!(result.tree.kind, EntryKind::Directory);
        assert!(result.directory_count > 0);
        assert!(result.file_count > 0);
    }

    #[test]
    fn test_parallel_basic_structure() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let result = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(result.tree.kind, EntryKind::Directory);
        assert!(result.directory_count > 0);
        assert!(result.file_count > 0);
    }

    #[test]
    fn test_walk_directories_only() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(false)
            .build();

        let result = scan_walk(&config).expect("walk 扫描失败");

        assert_eq!(result.file_count, 0);
        assert!(result.directory_count > 0);
    }

    #[test]
    fn test_parallel_directories_only() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(false)
            .thread_count(4)
            .build();

        let result = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(result.file_count, 0);
        assert!(result.directory_count > 0);
    }

    #[test]
    fn test_scan_single_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("single.txt");
        File::create(&file_path).unwrap();

        let config = ScanConfig::builder()
            .root(file_path)
            .include_files(true)
            .build();

        let result = scan_walk(&config).expect("扫描失败");

        assert_eq!(result.tree.kind, EntryKind::File);
        assert_eq!(result.file_count, 1);
        assert_eq!(result.directory_count, 0);
    }

    #[test]
    fn test_parallel_single_file_fallback() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("single.txt");
        File::create(&file_path).unwrap();

        let config = ScanConfig::builder()
            .root(file_path)
            .include_files(true)
            .thread_count(4)
            .build();

        let result = scan_parallel(&config).expect("扫描失败");
        assert_eq!(result.tree.kind, EntryKind::File);
    }

    // ========================================================================
    // 一致性测试
    // ========================================================================

    #[test]
    fn test_consistency_with_files() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "基础一致性测试");
    }

    #[test]
    fn test_consistency_without_files() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(false)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "仅目录一致性测试");
    }

    #[test]
    fn test_consistency_deep_directory() {
        let temp = create_deep_directory(20);
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(8)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "深层目录一致性测试");
    }

    #[test]
    fn test_consistency_wide_directory() {
        let temp = create_wide_directory(100);
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(8)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "宽目录一致性测试");
    }

    #[test]
    fn test_consistency_mixed_directory() {
        let temp = create_mixed_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "混合目录一致性测试");
    }

    #[test]
    fn test_consistency_complex_nested() {
        let temp = create_complex_nested();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(8)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "复杂嵌套一致性测试");
    }

    #[test]
    fn test_consistency_multiple_runs() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let baseline = scan_walk(&config).expect("walk 扫描失败");

        for i in 0..5 {
            let result = scan_parallel(&config).expect("parallel 扫描失败");
            assert_consistency(&baseline, &result, &format!("多次运行一致性测试 #{i}"));
        }
    }

    #[test]
    fn test_consistency_very_wide() {
        let temp = create_wide_directory(500);
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(8)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "超宽目录一致性测试");
    }

    // ========================================================================
    // 线程数验证测试
    // ========================================================================

    #[test]
    fn test_thread_count_variations() {
        let temp = create_wide_directory(50);

        for thread_count in [1, 2, 4, 8, 16] {
            let config = ScanConfig::builder()
                .root(temp.path().to_path_buf())
                .include_files(true)
                .thread_count(thread_count)
                .build();

            let walk = scan_walk(&config).expect("walk 扫描失败");
            let parallel = scan_parallel(&config).expect("parallel 扫描失败");

            assert_consistency(
                &walk,
                &parallel,
                &format!("{thread_count} 线程一致性测试"),
            );
        }
    }

    #[test]
    fn test_single_thread_parallel() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(1)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "单线程 parallel 一致性测试");
    }

    #[test]
    fn test_extreme_thread_count() {
        let temp = create_wide_directory(200);

        for thread_count in [32, 64, 128] {
            let config = ScanConfig::builder()
                .root(temp.path().to_path_buf())
                .include_files(true)
                .thread_count(thread_count)
                .build();

            let walk = scan_walk(&config).expect("walk 扫描失败");
            let parallel = scan_parallel(&config).expect("parallel 扫描失败");

            assert_consistency(
                &walk,
                &parallel,
                &format!("{thread_count} 线程极端测试"),
            );
        }
    }

    // ========================================================================
    // 边界情况测试
    // ========================================================================

    #[test]
    fn test_empty_directory() {
        let temp = TempDir::new().unwrap();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 0);
        assert_eq!(parallel.file_count, 0);
        assert_eq!(walk.directory_count, 1);
        assert_eq!(parallel.directory_count, 1);
    }

    #[test]
    fn test_single_file_in_directory() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("single.txt")).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 1);
        assert_eq!(parallel.file_count, 1);
        assert_consistency(&walk, &parallel, "单文件目录一致性测试");
    }

    #[test]
    fn test_deeply_nested_single_file() {
        let temp = TempDir::new().unwrap();
        let deep_path = temp.path().join("a/b/c/d/e/f/g/h/i/j");
        fs::create_dir_all(&deep_path).unwrap();
        File::create(deep_path.join("deep.txt")).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "深层嵌套单文件一致性测试");
    }

    #[test]
    fn test_special_characters_in_names() {
        let temp = TempDir::new().unwrap();
        let special_names = ["文件夹", "folder with spaces", "folder-with-dashes", "test_underscore"];

        for name in &special_names {
            fs::create_dir(temp.path().join(name)).unwrap();
            File::create(temp.path().join(format!("{name}.txt"))).unwrap();
        }

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "特殊字符名称一致性测试");
    }

    #[test]
    fn test_unicode_names() {
        let temp = TempDir::new().unwrap();
        let unicode_names = ["日本語", "한국어", "中文测试", "Ελληνικά"];

        for name in &unicode_names {
            if fs::create_dir(temp.path().join(name)).is_ok() {
                let _ = File::create(temp.path().join(format!("{name}.txt")));
            }
        }

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "Unicode 名称一致性测试");
    }

    #[test]
    fn test_many_empty_subdirs() {
        let temp = TempDir::new().unwrap();

        for i in 0..50 {
            fs::create_dir(temp.path().join(format!("empty_{i}"))).unwrap();
        }

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.directory_count, 51);
        assert_eq!(parallel.directory_count, 51);
        assert_consistency(&walk, &parallel, "多空目录一致性测试");
    }

    #[test]
    fn test_file_sizes_metadata() {
        let temp = create_directory_with_sizes();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .build();

        let result = scan_walk(&config).expect("扫描失败");

        let sizes: Vec<_> = result
            .tree
            .children
            .iter()
            .map(|c| (&c.name, c.metadata.size))
            .collect();

        assert!(sizes.iter().any(|(n, s)| n.contains("small") && *s == 5));
        assert!(sizes.iter().any(|(n, s)| n.contains("medium") && *s == 1024));
        assert!(sizes.iter().any(|(n, s)| n.contains("large") && *s == 10240));
    }

    #[test]
    fn test_hidden_files() {
        let temp = TempDir::new().unwrap();

        File::create(temp.path().join(".hidden")).unwrap();
        File::create(temp.path().join(".gitignore")).unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert!(walk.file_count >= 2);
        assert!(walk.directory_count >= 2);
        assert_consistency(&walk, &parallel, "隐藏文件一致性测试");
    }

    #[test]
    fn test_dot_directories() {
        let temp = TempDir::new().unwrap();

        fs::create_dir(temp.path().join(".config")).unwrap();
        File::create(temp.path().join(".config/settings.json")).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "点目录一致性测试");
    }

    #[test]
    fn test_very_long_filename() {
        let temp = TempDir::new().unwrap();
        let long_name = "a".repeat(200);
        File::create(temp.path().join(format!("{long_name}.txt"))).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "超长文件名一致性测试");
    }

    #[test]
    fn test_nested_empty_dirs() {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("a/b/c/d/e")).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.directory_count, 6);
        assert_eq!(walk.file_count, 0);
        assert_consistency(&walk, &parallel, "嵌套空目录一致性测试");
    }

    #[test]
    fn test_only_directories_no_files() {
        let temp = TempDir::new().unwrap();

        for i in 0..10 {
            fs::create_dir(temp.path().join(format!("dir_{i}"))).unwrap();
        }

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 0);
        assert_eq!(walk.directory_count, 11);
        assert_consistency(&walk, &parallel, "纯目录一致性测试");
    }

    #[test]
    fn test_only_files_no_subdirs() {
        let temp = TempDir::new().unwrap();

        for i in 0..20 {
            File::create(temp.path().join(format!("file_{i}.txt"))).unwrap();
        }

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 20);
        assert_eq!(walk.directory_count, 1);
        assert_consistency(&walk, &parallel, "纯文件一致性测试");
    }

    #[test]
    fn test_alternating_structure() {
        let temp = TempDir::new().unwrap();

        for i in 0..10 {
            if i % 2 == 0 {
                File::create(temp.path().join(format!("item_{i}.txt"))).unwrap();
            } else {
                fs::create_dir(temp.path().join(format!("item_{i}"))).unwrap();
            }
        }

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 5);
        assert_eq!(walk.directory_count, 6);
        assert_consistency(&walk, &parallel, "交替结构一致性测试");
    }

    #[test]
    fn test_binary_tree_structure() {
        let temp = TempDir::new().unwrap();

        fn create_binary_tree(path: &Path, depth: usize) {
            if depth == 0 {
                return;
            }
            let left = path.join("left");
            let right = path.join("right");
            fs::create_dir(&left).unwrap();
            fs::create_dir(&right).unwrap();
            File::create(path.join("data.txt")).unwrap();
            create_binary_tree(&left, depth - 1);
            create_binary_tree(&right, depth - 1);
        }

        create_binary_tree(temp.path(), 4);

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(8)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "二叉树结构一致性测试");
    }

    #[test]
    fn test_sparse_deep_structure() {
        let temp = TempDir::new().unwrap();

        for branch in 0..5 {
            let mut current = temp.path().join(format!("branch_{branch}"));
            fs::create_dir(&current).unwrap();
            for level in 0..10 {
                current = current.join(format!("level_{level}"));
                fs::create_dir(&current).unwrap();
            }
            File::create(current.join("leaf.txt")).unwrap();
        }

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(8)
            .build();

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_consistency(&walk, &parallel, "稀疏深层结构一致性测试");
    }

    #[test]
    fn test_nonexistent_path() {
        let config = ScanConfig::builder()
            .root(PathBuf::from("/nonexistent/path/that/does/not/exist"))
            .build();

        let result = scan_walk(&config);
        assert!(result.is_err());

        let result = scan_parallel(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_thread_count_normalized() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .thread_count(0)
            .build();

        assert!(config.thread_count >= 1);
    }

    // ========================================================================
    // 排序确定性测试
    // ========================================================================

    #[test]
    fn test_deterministic_ordering() {
        let temp = create_wide_directory(30);
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(8)
            .build();

        let results: Vec<Vec<PathBuf>> = (0..10)
            .map(|_| {
                scan_parallel(&config)
                    .expect("扫描失败")
                    .tree
                    .collect_paths()
            })
            .collect();

        for (i, result) in results.iter().enumerate().skip(1) {
            assert_eq!(results[0], *result, "第 {i} 次运行路径顺序不一致");
        }
    }

    #[test]
    fn test_directories_before_files() {
        let temp = TempDir::new().unwrap();

        File::create(temp.path().join("aaa.txt")).unwrap();
        fs::create_dir(temp.path().join("zzz")).unwrap();
        File::create(temp.path().join("bbb.txt")).unwrap();
        fs::create_dir(temp.path().join("aaa_dir")).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

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
            "目录应排在文件之前"
        );
    }

    #[test]
    fn test_case_insensitive_sort() {
        let temp = TempDir::new().unwrap();

        File::create(temp.path().join("Apple.txt")).unwrap();
        File::create(temp.path().join("banana.txt")).unwrap();
        File::create(temp.path().join("CHERRY.txt")).unwrap();
        File::create(temp.path().join("date.txt")).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .thread_count(4)
            .build();

        let result = scan_parallel(&config).expect("扫描失败");
        let names: Vec<_> = result
            .tree
            .children
            .iter()
            .map(|c| c.name.to_lowercase())
            .collect();

        assert_eq!(names[0], "apple.txt");
        assert_eq!(names[1], "banana.txt");
        assert_eq!(names[2], "cherry.txt");
        assert_eq!(names[3], "date.txt");
    }

    // ========================================================================
    // TreeNode 方法测试
    // ========================================================================

    #[test]
    fn test_tree_node_count_total() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .build();

        let result = scan_walk(&config).expect("扫描失败");
        let total = result.tree.count_total();

        assert_eq!(total, result.directory_count + result.file_count);
    }

    #[test]
    fn test_tree_node_max_depth() {
        let temp = create_deep_directory(10);
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .build();

        let result = scan_walk(&config).expect("扫描失败");
        let depth = result.tree.max_depth();

        assert_eq!(depth, 11);
    }

    #[test]
    fn test_tree_node_max_depth_with_files() {
        let temp = create_deep_directory_with_files(10);
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .build();

        let result = scan_walk(&config).expect("扫描失败");
        let depth = result.tree.max_depth();

        assert_eq!(depth, 11);
    }

    #[test]
    fn test_tree_node_collect_names() {
        let temp = TempDir::new().unwrap();
        File::create(temp.path().join("a.txt")).unwrap();
        File::create(temp.path().join("b.txt")).unwrap();
        fs::create_dir(temp.path().join("c")).unwrap();

        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .build();

        let result = scan_walk(&config).expect("扫描失败");
        let names = result.tree.collect_names();

        assert_eq!(names.len(), 4);
        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
        assert!(names.contains(&"c".to_string()));
    }

    #[test]
    fn test_tree_node_structural_eq() {
        let temp = create_test_directory();
        let config = ScanConfig::builder()
            .root(temp.path().to_path_buf())
            .include_files(true)
            .build();

        let result1 = scan_walk(&config).expect("扫描失败");
        let result2 = scan_walk(&config).expect("扫描失败");

        assert!(result1.tree.structural_eq(&result2.tree));
    }

    // ========================================================================
    // 原生 tree 命令测试
    // ========================================================================

    #[test]
    fn test_native_tree_execution() {
        let temp = create_test_directory();

        let result = scan_native_tree(temp.path(), true);

        if result.is_ok() {
            let native = result.unwrap();
            assert!(!native.lines.is_empty());
        }
    }

    // ========================================================================
    // 大目录测试
    // ========================================================================

    #[test]
    fn test_large_directory_rustup() {
        let Some(rustup_path) = get_rustup_path() else {
            println!("跳过: rustup 路径不存在");
            return;
        };

        let config = ScanConfig::builder()
            .root(rustup_path.clone())
            .include_files(true)
            .thread_count(available_parallelism())
            .build();

        println!("\n测试大目录: {}", rustup_path.display());

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
        println!("{report}");

        let speedup = walk.duration.as_secs_f64() / parallel.duration.as_secs_f64();
        println!("加速比: {speedup:.2}x");

        assert!(report.is_consistent());
    }

    // ========================================================================
    // 性能基准测试
    // ========================================================================

    #[test]
    fn test_performance_comparison() {
        let Some(windows_path) = get_windows_path() else {
            println!("跳过: Windows 路径不存在");
            return;
        };

        println!("\n=== 性能对比测试 (C:\\Windows) ===");
        println!("预热: {WARMUP_RUNS} 次, 测量: {BENCHMARK_RUNS} 次\n");

        println!("预热中...");
        for i in 0..WARMUP_RUNS {
            print!("  预热 #{} ", i + 1);
            let config = ScanConfig::builder()
                .root(windows_path.clone())
                .include_files(true)
                .thread_count(available_parallelism())
                .build();
            let _ = scan_parallel(&config);
            println!("✓");
        }
        println!("预热完成\n");

        println!("测量原生 tree 命令...");
        let mut native_times = Vec::with_capacity(BENCHMARK_RUNS);
        for i in 0..BENCHMARK_RUNS {
            print!("  运行 #{} ", i + 1);
            if let Ok(r) = scan_native_tree(&windows_path, true) {
                native_times.push(r.duration.as_secs_f64() * 1000.0);
                println!("{:.2}ms", native_times.last().unwrap());
            } else {
                println!("失败");
            }
        }
        let native_avg = if native_times.is_empty() {
            0.0
        } else {
            native_times.iter().sum::<f64>() / native_times.len() as f64
        };

        println!("\n测量单线程扫描...");
        let mut walk_times = Vec::with_capacity(BENCHMARK_RUNS);
        let config_walk = ScanConfig::builder()
            .root(windows_path.clone())
            .include_files(true)
            .thread_count(1)
            .build();
        for i in 0..BENCHMARK_RUNS {
            print!("  运行 #{} ", i + 1);
            let result = scan_walk(&config_walk).expect("walk 扫描失败");
            let ms = result.duration.as_secs_f64() * 1000.0;
            walk_times.push(ms);
            println!("{ms:.2}ms");
        }
        let walk_avg = walk_times.iter().sum::<f64>() / walk_times.len() as f64;

        let thread_count = DEFAULT_THREAD_COUNT;
        println!("\n测量多线程扫描 (默认值: {thread_count} 线程)...");
        let mut parallel_times = Vec::with_capacity(BENCHMARK_RUNS);
        let config_parallel = ScanConfig::builder()
            .root(windows_path.clone())
            .include_files(true)
            .thread_count(thread_count)
            .build();
        for i in 0..BENCHMARK_RUNS {
            print!("  运行 #{} ", i + 1);
            let result = scan_parallel(&config_parallel).expect("parallel 扫描失败");
            let ms = result.duration.as_secs_f64() * 1000.0;
            parallel_times.push(ms);
            println!("{ms:.2}ms");
        }
        let parallel_avg = parallel_times.iter().sum::<f64>() / parallel_times.len() as f64;

        println!("\n=== 性能对比结果 ===\n");
        println!("| {:<25} | {:>12} | {:>8} |", "类型", "耗时(ms)", "倍率");
        println!("|{:-<27}|{:-<14}|{:-<10}|", "", "", "");

        if native_avg > 0.0 {
            println!(
                "| {:<25} | {:>12.2} | {:>7.2}x |",
                "原生`tree`", native_avg, 1.0
            );
            println!(
                "| {:<25} | {:>12.2} | {:>7.2}x |",
                format!("`treepp`(默认, {thread_count}线程)"),
                parallel_avg,
                native_avg / parallel_avg
            );
            println!(
                "| {:<25} | {:>12.2} | {:>7.2}x |",
                "`treepp`(1线程)",
                walk_avg,
                native_avg / walk_avg
            );
        } else {
            println!(
                "| {:<25} | {:>12.2} | {:>7.2}x |",
                "`treepp`(1线程)", walk_avg, 1.0
            );
            println!(
                "| {:<25} | {:>12.2} | {:>7.2}x |",
                format!("`treepp`(默认, {thread_count}线程)"),
                parallel_avg,
                walk_avg / parallel_avg
            );
        }

        println!("\n=== 一致性验证 ===");
        let walk_result = scan_walk(&config_walk).expect("walk 扫描失败");
        let parallel_result = scan_parallel(&config_parallel).expect("parallel 扫描失败");
        let report = verify_consistency(&walk_result, &parallel_result);
        println!(
            "结果: {}",
            if report.is_consistent() {
                "一致 ✓"
            } else {
                "不一致 ✗ (系统目录可能因权限问题导致差异)"
            }
        );
    }

    #[test]
    fn test_thread_scaling_performance() {
        let Some(rustup_path) = get_rustup_path() else {
            println!("跳过: rustup 路径不存在");
            return;
        };

        println!("\n=== 线程扩展性测试 ===");
        println!("路径: {}\n", rustup_path.display());

        println!("预热中...");
        for _ in 0..WARMUP_RUNS {
            let config = ScanConfig::builder()
                .root(rustup_path.clone())
                .include_files(true)
                .thread_count(available_parallelism())
                .build();
            let _ = scan_parallel(&config);
        }
        println!("预热完成\n");

        let config_walk = ScanConfig::builder()
            .root(rustup_path.clone())
            .include_files(true)
            .build();

        let mut walk_times = Vec::with_capacity(BENCHMARK_RUNS);
        for _ in 0..BENCHMARK_RUNS {
            let result = scan_walk(&config_walk).expect("walk 扫描失败");
            walk_times.push(result.duration.as_secs_f64() * 1000.0);
        }
        let baseline = walk_times.iter().sum::<f64>() / walk_times.len() as f64;

        println!("| {:>6} | {:>12} | {:>8} |", "线程数", "耗时(ms)", "加速比");
        println!("|{:-<8}|{:-<14}|{:-<10}|", "", "", "");
        println!("| {:>6} | {:>12.2} | {:>7.2}x |", "walk", baseline, 1.0);

        let thread_counts = [1, 2, 4, 8, 16, 32];

        for &thread_count in &thread_counts {
            let config = ScanConfig::builder()
                .root(rustup_path.clone())
                .include_files(true)
                .thread_count(thread_count)
                .build();

            let mut times = Vec::with_capacity(BENCHMARK_RUNS);
            for _ in 0..BENCHMARK_RUNS {
                let result = scan_parallel(&config).expect("parallel 扫描失败");
                times.push(result.duration.as_secs_f64() * 1000.0);
            }
            let avg = times.iter().sum::<f64>() / times.len() as f64;
            let speedup = baseline / avg;

            println!("| {:>6} | {:>12.2} | {:>7.2}x |", thread_count, avg, speedup);
        }
    }
}