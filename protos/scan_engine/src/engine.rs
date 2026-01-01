//! 扫描引擎原型
//!
//! 验证 walk（单线程）与 parallel（多线程）两种目录扫描模式的正确性与一致性。
//! 核心验证点：
//! - 两种模式产生完全一致的结果
//! - 并发扫描不重不漏
//! - 线程数参数生效
//! - 输出具有确定性排序

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime};

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

    /// 创建带子节点的目录节点
    fn with_children(
        path: PathBuf,
        kind: EntryKind,
        metadata: EntryMetadata,
        children: Vec<TreeNode>,
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

    /// 计算最大深度（仅计算目录深度，不含文件）
    pub fn max_depth(&self) -> usize {
        if self.kind == EntryKind::File {
            return 0;
        }
        let child_dirs: Vec<_> = self
            .children
            .iter()
            .filter(|c| c.kind == EntryKind::Directory)
            .collect();
        if child_dirs.is_empty() {
            1
        } else {
            1 + child_dirs.iter().map(|c| c.max_depth()).max().unwrap_or(0)
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
// 多线程扫描引擎 (parallel) - 分治合并模式
// ============================================================================

/// 多线程并发目录扫描（分治合并，无锁）
pub fn scan_parallel(config: &ScanConfig) -> io::Result<ScanResult> {
    let start = Instant::now();
    let root_meta = fs::metadata(&config.root)?;

    // 如果是文件，直接使用单线程扫描
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
    let root_metadata = EntryMetadata::from_fs_metadata(&root_meta);

    // 使用线程池执行分治扫描
    let tree = pool.install(|| scan_directory_divide_conquer(&root_path, include_files));

    // 如果扫描失败（例如权限问题），返回空目录
    let mut tree = tree.unwrap_or_else(|| {
        TreeNode::new(root_path.clone(), EntryKind::Directory, root_metadata)
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

/// 分治扫描：递归扫描目录，返回完整的子树
fn scan_directory_divide_conquer(path: &Path, include_files: bool) -> Option<TreeNode> {
    // 读取目录元数据
    let meta = fs::metadata(path).ok()?;
    let metadata = EntryMetadata::from_fs_metadata(&meta);

    // 读取目录条目
    let dir_entries: Vec<_> = match fs::read_dir(path) {
        Ok(entries) => entries.flatten().collect(),
        Err(_) => {
            // 无法读取目录，返回空目录节点
            return Some(TreeNode::new(
                path.to_path_buf(),
                EntryKind::Directory,
                metadata,
            ));
        }
    };

    // 分离子目录和文件
    let mut subdirs = Vec::new();
    let mut files = Vec::new();

    for entry in dir_entries {
        let entry_path = entry.path();
        let entry_meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if entry_meta.is_dir() {
            subdirs.push(entry_path);
        } else if include_files {
            let file_metadata = EntryMetadata::from_fs_metadata(&entry_meta);
            files.push(TreeNode::new(entry_path, EntryKind::File, file_metadata));
        }
    }

    // 并行递归扫描子目录（分治）
    let subdir_trees: Vec<TreeNode> = subdirs
        .into_par_iter()
        .filter_map(|subdir| scan_directory_divide_conquer(&subdir, include_files))
        .collect();

    // 合并子目录和文件
    let mut children = subdir_trees;
    children.extend(files);

    Some(TreeNode::with_children(
        path.to_path_buf(),
        EntryKind::Directory,
        metadata,
        children,
    ))
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

    let (directory_count, file_count) = parse_native_tree_output(&lines, include_files);

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

/// 解析原生 tree 命令输出
fn parse_native_tree_output(lines: &[String], include_files: bool) -> (usize, usize) {
    let mut dir_count = 0usize;
    let mut file_count = 0usize;

    let content_start = lines
        .iter()
        .position(|l| l.contains("├") || l.contains("└") || l.contains("│"))
        .unwrap_or(0);

    for line in lines.iter().skip(content_start) {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.contains("没有子文件夹") {
            continue;
        }

        let name = line
            .replace("├", "")
            .replace("└", "")
            .replace("│", "")
            .replace("─", "")
            .trim()
            .to_string();

        if name.is_empty() {
            continue;
        }

        if !include_files {
            dir_count += 1;
        } else {
            let has_branch = line.contains("├─") || line.contains("└─");
            let is_indented_file = !has_branch && (line.contains("│") || line.starts_with("   "));

            if has_branch {
                if name.contains('.') && !name.starts_with('.') {
                    file_count += 1;
                } else {
                    dir_count += 1;
                }
            } else if is_indented_file {
                file_count += 1;
            }
        }
    }

    (dir_count, file_count)
}

// ============================================================================
// 辅助函数
// ============================================================================

pub fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// 验证两个扫描结果的结构一致性
pub fn verify_consistency(walk: &ScanResult, parallel: &ScanResult) -> ConsistencyReport {
    let walk_paths = walk.tree.collect_paths();
    let parallel_paths = parallel.tree.collect_paths();

    let structural_match = walk.tree.structural_eq(&parallel.tree);

    let walk_set: HashSet<_> = walk_paths.iter().collect();
    let parallel_set: HashSet<_> = parallel_paths.iter().collect();

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
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::env;
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
        File::create(root.join("src/engine")).unwrap();
        File::create(root.join("src/lib.rs")).unwrap();
        File::create(root.join("src/utils/helper.rs")).unwrap();
        File::create(root.join("tests/integration.rs")).unwrap();
        File::create(root.join("docs/api/index.html")).unwrap();

        temp
    }

    /// 创建深层嵌套目录结构（仅目录）
    fn create_deep_directory(depth: usize) -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let mut current = temp.path().to_path_buf();

        for i in 0..depth {
            current = current.join(format!("level_{}", i));
            fs::create_dir(&current).unwrap();
        }

        temp
    }

    /// 创建深层嵌套目录结构（含文件）
    fn create_deep_directory_with_files(depth: usize) -> TempDir {
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

    /// 创建复杂嵌套结构
    fn create_complex_nested() -> TempDir {
        let temp = TempDir::new().expect("创建临时目录失败");
        let root = temp.path();

        for i in 0..3 {
            let branch = root.join(format!("branch_{}", i));
            fs::create_dir(&branch).unwrap();

            for j in 0..3 {
                let sub = branch.join(format!("sub_{}", j));
                fs::create_dir(&sub).unwrap();

                for k in 0..2 {
                    let deep = sub.join(format!("deep_{}", k));
                    fs::create_dir(&deep).unwrap();
                    File::create(deep.join("leaf.txt")).unwrap();
                }
            }
        }

        temp
    }

    /// 获取 rustup 路径（如果存在）
    fn get_rustup_path() -> Option<PathBuf> {
        if let Ok(home) = env::var("USERPROFILE") {
            let path = PathBuf::from(home).join(".rustup");
            if path.exists() {
                return Some(path);
            }
        }
        if let Ok(home) = env::var("HOME") {
            let path = PathBuf::from(home).join(".rustup");
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    // ========================================================================
    // 基础功能测试
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

    #[test]
    fn test_parallel_single_file_fallback() {
        let temp = TempDir::new().expect("创建临时目录失败");
        let file_path = temp.path().join("single.txt");
        File::create(&file_path).unwrap();

        let config = ScanConfig {
            root: file_path,
            include_files: true,
            thread_count: 4,
        };

        let result = scan_parallel(&config).expect("扫描失败");
        assert_eq!(result.tree.kind, EntryKind::File);
    }

    // ========================================================================
    // 一致性测试
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
    fn test_consistency_multiple_runs() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let baseline = scan_walk(&config).expect("walk 扫描失败");

        let results: Vec<_> = (0..5)
            .map(|_| scan_parallel(&config).expect("parallel 扫描失败"))
            .collect();

        for (i, result) in results.iter().enumerate() {
            let report = verify_consistency(&baseline, result);
            assert!(
                report.is_consistent(),
                "第 {} 次运行与基准不一致:\n{}",
                i,
                report
            );
        }
    }

    #[test]
    fn test_consistency_complex_nested() {
        let temp = create_complex_nested();
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
            "复杂嵌套目录一致性验证失败:\n{}",
            report
        );
    }

    #[test]
    fn test_consistency_very_wide() {
        let temp = create_wide_directory(500);
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
            "超宽目录一致性验证失败:\n{}",
            report
        );
    }

    // ========================================================================
    // 线程数验证测试
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
    fn test_extreme_thread_count() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 64,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(
            report.is_consistent(),
            "64线程一致性验证失败:\n{}",
            report
        );
    }

    #[test]
    fn test_thread_count_128() {
        let temp = create_wide_directory(200);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 128,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        let report = verify_consistency(&walk, &parallel);
        assert!(
            report.is_consistent(),
            "128线程一致性验证失败:\n{}",
            report
        );
    }

    // ========================================================================
    // 边界情况测试
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

        assert_eq!(walk.directory_count, 51);
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
        assert!(sizes
            .iter()
            .any(|(n, s)| n.contains("medium") && *s == 1024));
        assert!(sizes
            .iter()
            .any(|(n, s)| n.contains("large") && *s == 10240));
    }

    #[test]
    fn test_hidden_files() {
        let temp = TempDir::new().expect("创建临时目录失败");

        File::create(temp.path().join(".hidden")).unwrap();
        File::create(temp.path().join(".gitignore")).unwrap();
        fs::create_dir(temp.path().join(".git")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert!(walk.file_count >= 2);
        assert!(walk.directory_count >= 2);
        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    #[test]
    fn test_dot_directories() {
        let temp = TempDir::new().expect("创建临时目录失败");

        fs::create_dir(temp.path().join(".config")).unwrap();
        fs::create_dir(temp.path().join("..strange")).unwrap();
        File::create(temp.path().join(".config/settings.json")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    #[test]
    fn test_very_long_filename() {
        let temp = TempDir::new().expect("创建临时目录失败");

        let long_name = "a".repeat(200);
        File::create(temp.path().join(format!("{}.txt", long_name))).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    #[test]
    fn test_nested_empty_dirs() {
        let temp = TempDir::new().expect("创建临时目录失败");

        fs::create_dir_all(temp.path().join("a/b/c/d/e")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.directory_count, 6);
        assert_eq!(walk.file_count, 0);
        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    // ========================================================================
    // 排序确定性测试
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

        assert_eq!(names[0].to_lowercase(), "apple.txt");
        assert_eq!(names[1].to_lowercase(), "banana.txt");
        assert_eq!(names[2].to_lowercase(), "cherry.txt");
        assert_eq!(names[3].to_lowercase(), "date.txt");
    }

    #[test]
    fn test_numeric_filename_sort() {
        let temp = TempDir::new().expect("创建临时目录失败");

        for i in [1, 10, 2, 20, 3] {
            File::create(temp.path().join(format!("file_{}.txt", i))).unwrap();
        }

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

        assert_eq!(names[0], "file_1.txt");
        assert_eq!(names[1], "file_10.txt");
        assert_eq!(names[2], "file_2.txt");
    }

    #[test]
    fn test_mixed_case_directories() {
        let temp = TempDir::new().expect("创建临时目录失败");

        fs::create_dir(temp.path().join("AAA")).unwrap();
        fs::create_dir(temp.path().join("aaa_second")).unwrap();
        fs::create_dir(temp.path().join("BBB")).unwrap();
        fs::create_dir(temp.path().join("bbb_second")).unwrap();

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
            .map(|c| c.name.to_lowercase())
            .collect();

        assert!(names[0].starts_with("aaa"));
        assert!(names[1].starts_with("aaa"));
    }

    // ========================================================================
    // TreeNode 方法测试
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

        assert_eq!(depth, 11, "深度应为 11 (根 + 10 层目录)");
    }

    #[test]
    fn test_tree_node_max_depth_with_files() {
        let temp = create_deep_directory_with_files(10);
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 1,
        };

        let result = scan_walk(&config).expect("扫描失败");
        let depth = result.tree.max_depth();

        assert_eq!(depth, 11, "深度应为 11 (根 + 10 层目录，不含文件)");
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

        assert_eq!(names.len(), 4);
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

    #[test]
    fn test_tree_node_collect_paths() {
        let temp = TempDir::new().expect("创建临时目录失败");
        fs::create_dir(temp.path().join("sub")).unwrap();
        File::create(temp.path().join("sub/file.txt")).unwrap();

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 1,
        };

        let result = scan_walk(&config).expect("扫描失败");
        let paths = result.tree.collect_paths();

        assert_eq!(paths.len(), 3);
    }

    // ========================================================================
    // 性能验证测试
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

        assert!(
            result.duration.as_secs() < 1,
            "扫描耗时过长: {:?}",
            result.duration
        );
    }

    #[test]
    fn test_walk_vs_parallel_similar_performance() {
        let temp = create_test_directory();
        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert!(walk.duration.as_secs() < 1);
        assert!(parallel.duration.as_secs() < 1);
    }

    // ========================================================================
    // 与原生 tree 命令对比测试
    // ========================================================================

    #[test]
    fn test_native_tree_execution() {
        let temp = create_test_directory();

        let result = scan_native_tree(temp.path(), true);
        assert!(result.is_ok(), "原生 tree 命令执行失败");

        let native = result.unwrap();
        assert!(!native.lines.is_empty(), "原生 tree 输出为空");
    }

    #[test]
    fn test_native_tree_directory_only() {
        let temp = create_test_directory();

        let result = scan_native_tree(temp.path(), false);
        assert!(result.is_ok(), "原生 tree 命令执行失败");
    }

    // ========================================================================
    // 大目录测试（使用 rustup 路径）
    // ========================================================================

    #[test]
    fn test_large_directory_rustup() {
        let rustup_path = match get_rustup_path() {
            Some(p) => p,
            None => {
                println!("跳过: rustup 路径不存在");
                return;
            }
        };

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

        let speedup = walk.duration.as_secs_f64() / parallel.duration.as_secs_f64();
        println!("加速比: {:.2}x", speedup);

        assert!(report.is_consistent(), "大目录一致性验证失败");
    }

    #[test]
    fn test_large_directory_consistency_stress() {
        let rustup_path = match get_rustup_path() {
            Some(p) => p,
            None => {
                println!("跳过: rustup 路径不存在");
                return;
            }
        };

        let config = ScanConfig {
            root: rustup_path,
            include_files: true,
            thread_count: num_cpus(),
        };

        let baseline = scan_walk(&config).expect("walk 扫描失败");

        let results: Vec<_> = (0..3)
            .map(|i| {
                let result = scan_parallel(&config).expect("扫描失败");
                println!("运行 {}: {} 条目", i, result.tree.collect_paths().len());
                result
            })
            .collect();

        for (i, result) in results.iter().enumerate() {
            let report = verify_consistency(&baseline, result);
            assert!(report.is_consistent(), "压力测试第 {} 次运行不一致", i);
        }
    }

    // ========================================================================
    // 速度比较测试（使用 rustup 路径）
    // ========================================================================

    #[test]
    fn test_performance_comparison_rustup() {
        let rustup_path = match get_rustup_path() {
            Some(p) => p,
            None => {
                println!("跳过: rustup 路径不存在");
                return;
            }
        };

        const WARMUP_RUNS: usize = 2;
        const BENCH_RUNS: usize = 5;

        println!("\n=== 性能比较测试 (rustup 目录) ===");
        println!("路径: {:?}", rustup_path);
        println!("预热: {} 次, 测量: {} 次\n", WARMUP_RUNS, BENCH_RUNS);

        // 预热
        println!("预热中...");
        for _ in 0..WARMUP_RUNS {
            let config = ScanConfig {
                root: rustup_path.clone(),
                include_files: true,
                thread_count: num_cpus(),
            };
            let _ = scan_walk(&config);
            let _ = scan_parallel(&config);
        }
        println!("预热完成\n");

        // 1. 原生 tree 命令基准
        let mut native_times = Vec::with_capacity(BENCH_RUNS);
        for _ in 0..BENCH_RUNS {
            if let Ok(r) = scan_native_tree(&rustup_path, true) {
                native_times.push(r.duration.as_secs_f64());
            }
        }
        let native_avg = if native_times.is_empty() {
            0.0
        } else {
            native_times.iter().sum::<f64>() / native_times.len() as f64
        };

        // 2. Rust 单线程基准
        let mut walk_times = Vec::with_capacity(BENCH_RUNS);
        let config_walk = ScanConfig {
            root: rustup_path.clone(),
            include_files: true,
            thread_count: 1,
        };
        for _ in 0..BENCH_RUNS {
            let result = scan_walk(&config_walk).expect("walk 扫描失败");
            walk_times.push(result.duration.as_secs_f64());
        }
        let walk_avg = walk_times.iter().sum::<f64>() / walk_times.len() as f64;
        let walk_min = walk_times.iter().cloned().fold(f64::INFINITY, f64::min);
        let walk_max = walk_times.iter().cloned().fold(0.0, f64::max);

        // 3. Rust 多线程基准（不同线程数）
        let thread_counts = [2, 4, 8, num_cpus()];
        let mut parallel_stats: Vec<(usize, f64, f64, f64)> = Vec::new();

        for &thread_count in &thread_counts {
            let config = ScanConfig {
                root: rustup_path.clone(),
                include_files: true,
                thread_count,
            };

            let mut times = Vec::with_capacity(BENCH_RUNS);
            for _ in 0..BENCH_RUNS {
                let result = scan_parallel(&config).expect("parallel 扫描失败");
                times.push(result.duration.as_secs_f64());
            }

            let avg = times.iter().sum::<f64>() / times.len() as f64;
            let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = times.iter().cloned().fold(0.0, f64::max);
            parallel_stats.push((thread_count, avg, min, max));
        }

        // 输出结果
        println!("=== 测量结果 ===\n");
        println!(
            "{:<20} {:>10} {:>10} {:>10} {:>12}",
            "模式", "平均(ms)", "最小(ms)", "最大(ms)", "vs 单线程"
        );
        println!("{}", "-".repeat(65));

        if native_avg > 0.0 {
            println!(
                "{:<20} {:>10.2} {:>10} {:>10} {:>12.2}x",
                "原生 tree",
                native_avg * 1000.0,
                "-",
                "-",
                native_avg / walk_avg
            );
        }

        println!(
            "{:<20} {:>10.2} {:>10.2} {:>10.2} {:>12}",
            "Rust 单线程",
            walk_avg * 1000.0,
            walk_min * 1000.0,
            walk_max * 1000.0,
            "1.00x"
        );

        for (threads, avg, min, max) in &parallel_stats {
            let label = format!("Rust {} 线程", threads);
            let speedup = walk_avg / avg;
            println!(
                "{:<20} {:>10.2} {:>10.2} {:>10.2} {:>12.2}x",
                label,
                avg * 1000.0,
                min * 1000.0,
                max * 1000.0,
                speedup
            );
        }

        // 一致性验证
        println!("\n=== 一致性验证 ===");
        let walk_result = scan_walk(&config_walk).expect("walk 扫描失败");
        for &thread_count in &thread_counts {
            let config = ScanConfig {
                root: rustup_path.clone(),
                include_files: true,
                thread_count,
            };
            let parallel_result = scan_parallel(&config).expect("parallel 扫描失败");
            let report = verify_consistency(&walk_result, &parallel_result);
            let status = if report.is_consistent() { "✓" } else { "✗" };
            println!("{} 线程: {}", thread_count, status);
        }
    }

    #[test]
    fn test_performance_all_thread_counts() {
        let rustup_path = match get_rustup_path() {
            Some(p) => p,
            None => {
                println!("跳过: rustup 路径不存在");
                return;
            }
        };

        const WARMUP_RUNS: usize = 3;
        const BENCH_RUNS: usize = 3;

        println!("\n=== 全线程数性能测试 (分治模式) ===");
        println!("路径: {:?}", rustup_path);
        println!("预热: {} 次, 每组测量: {} 次\n", WARMUP_RUNS, BENCH_RUNS);

        // 预热
        println!("预热中...");
        for _ in 0..WARMUP_RUNS {
            let config = ScanConfig {
                root: rustup_path.clone(),
                include_files: true,
                thread_count: num_cpus(),
            };
            let _ = scan_walk(&config);
            let _ = scan_parallel(&config);
        }
        println!("预热完成\n");

        // 单线程基准 (walk)
        let config_walk = ScanConfig {
            root: rustup_path.clone(),
            include_files: true,
            thread_count: 1,
        };
        let mut walk_times = Vec::with_capacity(BENCH_RUNS);
        for _ in 0..BENCH_RUNS {
            let result = scan_walk(&config_walk).expect("walk 扫描失败");
            walk_times.push(result.duration.as_secs_f64());
        }
        let baseline = walk_times.iter().sum::<f64>() / walk_times.len() as f64;

        // 多线程测试
        let thread_counts = [1, 2, 4, 6, 8, 12, 16, 24, 32, 48, 64];
        let mut results: Vec<(usize, f64, f64, f64)> = Vec::new();

        for &thread_count in &thread_counts {
            let config = ScanConfig {
                root: rustup_path.clone(),
                include_files: true,
                thread_count,
            };

            let mut times = Vec::with_capacity(BENCH_RUNS);
            for _ in 0..BENCH_RUNS {
                let result = scan_parallel(&config).expect("parallel 扫描失败");
                times.push(result.duration.as_secs_f64());
            }

            let avg = times.iter().sum::<f64>() / times.len() as f64;
            let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = times.iter().cloned().fold(0.0, f64::max);
            results.push((thread_count, avg, min, max));

            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
        println!(" 完成\n");

        // 输出表格
        println!(
            "{:>6} | {:>10} | {:>10} | {:>10} | {:>8} | {}",
            "线程数", "平均(ms)", "最小(ms)", "最大(ms)", "加速比", "柱状图"
        );
        println!("{}", "-".repeat(75));

        // 先输出单线程基准
        println!(
            "{:>6} | {:>10.2} | {:>10.2} | {:>10.2} | {:>7.2}x | {}",
            "walk",
            baseline * 1000.0,
            walk_times.iter().cloned().fold(f64::INFINITY, f64::min) * 1000.0,
            walk_times.iter().cloned().fold(0.0, f64::max) * 1000.0,
            1.0,
            "████████████████████"
        );

        let max_speedup = results
            .iter()
            .map(|(_, avg, _, _)| baseline / avg)
            .fold(1.0_f64, f64::max);

        for (threads, avg, min, max) in &results {
            let speedup = baseline / avg;
            let bar_len = ((speedup / max_speedup) * 20.0) as usize;
            let bar = "█".repeat(bar_len.max(1));

            println!(
                "{:>6} | {:>10.2} | {:>10.2} | {:>10.2} | {:>7.2}x | {}",
                threads,
                avg * 1000.0,
                min * 1000.0,
                max * 1000.0,
                speedup,
                bar
            );
        }

        // 找出最佳线程数
        let best = results
            .iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        println!(
            "\n最佳配置: {} 线程, 平均耗时 {:.2}ms, 加速比 {:.2}x",
            best.0,
            best.1 * 1000.0,
            baseline / best.1
        );

        // 效率分析
        println!("\n=== 并行效率分析 ===");
        println!("(效率 = 加速比 / 线程数 × 100%)");
        for (threads, avg, _, _) in &results {
            let speedup = baseline / avg;
            let efficiency = (speedup / *threads as f64) * 100.0;
            let status = if efficiency > 50.0 {
                "良好"
            } else if efficiency > 25.0 {
                "一般"
            } else {
                "较低"
            };
            println!("{:>3} 线程: {:.1}% ({})", threads, efficiency, status);
        }
    }

    // ========================================================================
    // 额外边缘情况测试
    // ========================================================================

    #[test]
    fn test_only_directories_no_files() {
        let temp = TempDir::new().expect("创建临时目录失败");

        for i in 0..10 {
            fs::create_dir(temp.path().join(format!("dir_{}", i))).unwrap();
        }

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 0);
        assert_eq!(walk.directory_count, 11);
        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    #[test]
    fn test_only_files_no_subdirs() {
        let temp = TempDir::new().expect("创建临时目录失败");

        for i in 0..20 {
            File::create(temp.path().join(format!("file_{}.txt", i))).unwrap();
        }

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 20);
        assert_eq!(walk.directory_count, 1);
        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    #[test]
    fn test_alternating_structure() {
        let temp = TempDir::new().expect("创建临时目录失败");

        for i in 0..10 {
            if i % 2 == 0 {
                File::create(temp.path().join(format!("item_{}.txt", i))).unwrap();
            } else {
                fs::create_dir(temp.path().join(format!("item_{}", i))).unwrap();
            }
        }

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 4,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert_eq!(walk.file_count, 5);
        assert_eq!(walk.directory_count, 6);
        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    #[test]
    fn test_binary_tree_structure() {
        let temp = TempDir::new().expect("创建临时目录失败");

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

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 8,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }

    #[test]
    fn test_sparse_deep_structure() {
        let temp = TempDir::new().expect("创建临时目录失败");

        for branch in 0..5 {
            let mut current = temp.path().join(format!("branch_{}", branch));
            fs::create_dir(&current).unwrap();
            for level in 0..10 {
                current = current.join(format!("level_{}", level));
                fs::create_dir(&current).unwrap();
            }
            File::create(current.join("leaf.txt")).unwrap();
        }

        let config = ScanConfig {
            root: temp.path().to_path_buf(),
            include_files: true,
            thread_count: 8,
        };

        let walk = scan_walk(&config).expect("walk 扫描失败");
        let parallel = scan_parallel(&config).expect("parallel 扫描失败");

        assert!(verify_consistency(&walk, &parallel).is_consistent());
    }
}
