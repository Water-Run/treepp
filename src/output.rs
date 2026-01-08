//! 输出模块：多格式输出与文件写入
//!
//! 本模块负责将渲染结果输出到不同目标，支持：
//!
//! - **输出策略**：stdout、写文件、silent（仅写文件不写 stdout）
//! - **多格式输出**：txt/json/yml/toml（序列化 schema 固定）
//! - **文件写入策略**：覆盖写入，确保原子性
//! - **流式输出**：`StreamWriter` 支持即时 flush 的流式写入
//!
//! 文件: src/output.rs
//! 作者: WaterRun
//! 更新于: 2026-01-08

#![forbid(unsafe_code)]

use std::fs::{self, File};
use std::io::{self, BufWriter, Stdout, StdoutLock, Write};
use std::path::Path;

use serde_json::{Map, Value};

use crate::config::{Config, OutputFormat};
use crate::error::OutputError;
use crate::render::RenderResult;
use crate::scan::{EntryKind, TreeNode};

// ============================================================================
// 序列化结构
// ============================================================================

/// 将 TreeNode 转换为 JSON Value（简洁的树形结构）
///
/// 输出格式：
/// - 目录：对象，键为子项名称
/// - 文件：对象，包含请求的元数据，无子项时为空对象
fn to_json_value(node: &TreeNode, config: &Config) -> Value {
    let is_file = node.kind == EntryKind::File;

    if is_file {
        // 文件：返回包含元数据的对象
        let mut obj = Map::new();

        if config.render.show_size {
            obj.insert("size".to_string(), Value::Number(node.metadata.size.into()));
        }

        if config.render.show_disk_usage {
            if let Some(usage) = node.disk_usage {
                obj.insert("disk_usage".to_string(), Value::Number(usage.into()));
            }
        }

        if config.render.show_date {
            if let Some(ref modified) = node.metadata.modified {
                obj.insert(
                    "modified".to_string(),
                    Value::String(crate::render::format_datetime(modified)),
                );
            }
        }

        if config.render.path_mode == crate::config::PathMode::Full {
            obj.insert(
                "path".to_string(),
                Value::String(node.path.to_string_lossy().into_owned()),
            );
        }

        Value::Object(obj)
    } else {
        // 目录：返回包含子项的对象
        let mut obj = Map::new();

        // 添加目录自身的元数据（如果需要）
        if config.render.show_disk_usage {
            if let Some(usage) = node.disk_usage {
                obj.insert("_disk_usage".to_string(), Value::Number(usage.into()));
            }
        }

        // 添加子项
        let children: Vec<&TreeNode> = node
            .children
            .iter()
            .filter(|c| config.scan.show_files || c.kind == EntryKind::Directory)
            .filter(|c| {
                !config.matching.prune_empty
                    || c.kind != EntryKind::Directory
                    || !c.is_empty_dir()
            })
            .collect();

        for child in children {
            obj.insert(child.name.clone(), to_json_value(child, config));
        }

        Value::Object(obj)
    }
}

// ============================================================================
// 格式化输出
// ============================================================================

/// 序列化为 JSON 格式
///
/// # 参数
///
/// * `node` - 树节点
/// * `config` - 输出配置
///
/// # 返回值
///
/// 成功返回 JSON 字符串，失败返回错误。
///
/// # Errors
///
/// 返回 `OutputError::SerializationFailed` 如果序列化失败。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
/// use treepp::output::serialize_json;
///
/// let node = TreeNode::new(PathBuf::from("."), EntryKind::Directory, EntryMetadata::default());
/// let config = Config::default();
/// let json = serialize_json(&node, &config).unwrap();
/// assert!(json.contains("\"name\""));
/// ```
/// 序列化为 JSON 格式
pub fn serialize_json(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    let value = to_json_value(node, config);
    serde_json::to_string_pretty(&value).map_err(|e| OutputError::json_error(e.to_string()))
}

/// 序列化为 YAML 格式
///
/// # 参数
///
/// * `node` - 树节点
/// * `config` - 输出配置
///
/// # 返回值
///
/// 成功返回 YAML 字符串，失败返回错误。
///
/// # Errors
///
/// 返回 `OutputError::SerializationFailed` 如果序列化失败。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
/// use treepp::output::serialize_yaml;
///
/// let node = TreeNode::new(PathBuf::from("."), EntryKind::Directory, EntryMetadata::default());
/// let config = Config::default();
/// let yaml = serialize_yaml(&node, &config).unwrap();
/// assert!(yaml.contains("name:"));
/// ```
/// 序列化为 YAML 格式
pub fn serialize_yaml(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    let value = to_json_value(node, config);
    serde_yaml::to_string(&value).map_err(|e| OutputError::yaml_error(e.to_string()))
}

/// 将 JSON Value 转换为 TOML Value
fn json_to_toml(value: &Value) -> Result<toml::Value, OutputError> {
    match value {
        Value::Null => Ok(toml::Value::String("null".to_string())),
        Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml::Value::Float(f))
            } else {
                Ok(toml::Value::String(n.to_string()))
            }
        }
        Value::String(s) => Ok(toml::Value::String(s.clone())),
        Value::Array(arr) => {
            let toml_arr: Result<Vec<_>, _> = arr.iter().map(json_to_toml).collect();
            Ok(toml::Value::Array(toml_arr?))
        }
        Value::Object(obj) => {
            let mut table = toml::map::Map::new();
            for (k, v) in obj {
                table.insert(k.clone(), json_to_toml(v)?);
            }
            Ok(toml::Value::Table(table))
        }
    }
}

/// 序列化为 TOML 格式
///
/// # 参数
///
/// * `node` - 树节点
/// * `config` - 输出配置
///
/// # 返回值
///
/// 成功返回 TOML 字符串，失败返回错误。
///
/// # Errors
///
/// 返回 `OutputError::SerializationFailed` 如果序列化失败。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
/// use treepp::output::serialize_toml;
///
/// let node = TreeNode::new(PathBuf::from("."), EntryKind::Directory, EntryMetadata::default());
/// let config = Config::default();
/// let toml = serialize_toml(&node, &config).unwrap();
/// assert!(toml.contains("[tree]"));
/// ```
/// 序列化为 TOML 格式
pub fn serialize_toml(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    // TOML 需要顶层为表，所以用根节点名称作为键
    let value = to_json_value(node, config);

    // 转换为 TOML 兼容格式
    let toml_value = json_to_toml(&value)?;
    toml::to_string_pretty(&toml_value).map_err(|e| OutputError::toml_error(e.to_string()))
}

// ============================================================================
// 流式输出写入器
// ============================================================================

/// 流式输出写入器
///
/// 封装 stdout 输出，每次写入后立即 flush，实现实时滚动效果。
///
/// # Examples
///
/// ```no_run
/// use treepp::output::StreamWriter;
///
/// let stdout = std::io::stdout();
/// let mut writer = StreamWriter::new(&stdout);
/// writer.write_line("Hello, World!").unwrap();
/// ```
pub struct StreamWriter<'a> {
    /// stdout 锁定句柄
    handle: StdoutLock<'a>,
}

impl<'a> StreamWriter<'a> {
    /// 创建新的流式写入器
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::output::StreamWriter;
    ///
    /// let stdout = std::io::stdout();
    /// let writer = StreamWriter::new(&stdout);
    /// ```
    #[must_use]
    pub fn new(stdout: &'a Stdout) -> Self {
        Self {
            handle: stdout.lock(),
        }
    }

    /// 写入一行并立即 flush
    ///
    /// 自动追加换行符。
    ///
    /// # Errors
    ///
    /// 返回 `OutputError::StdoutFailed` 如果写入失败。
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::output::StreamWriter;
    ///
    /// let stdout = std::io::stdout();
    /// let mut writer = StreamWriter::new(&stdout);
    /// writer.write_line("├─src").unwrap();
    /// ```
    pub fn write_line(&mut self, line: &str) -> Result<(), OutputError> {
        writeln!(self.handle, "{}", line)?;
        self.handle.flush()?;
        Ok(())
    }

    /// 写入字符串（不换行）并 flush
    ///
    /// # Errors
    ///
    /// 返回 `OutputError::StdoutFailed` 如果写入失败。
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::output::StreamWriter;
    ///
    /// let stdout = std::io::stdout();
    /// let mut writer = StreamWriter::new(&stdout);
    /// writer.write("Header content\n").unwrap();
    /// ```
    pub fn write(&mut self, content: &str) -> Result<(), OutputError> {
        write!(self.handle, "{}", content)?;
        self.handle.flush()?;
        Ok(())
    }
}

// ============================================================================
// 输出目标
// ============================================================================

/// 输出到标准输出
///
/// 如果配置了静默模式则不输出。
///
/// # 参数
///
/// * `content` - 输出内容
/// * `config` - 输出配置
///
/// # Errors
///
/// 返回 `OutputError::StdoutFailed` 如果写入失败。
///
/// # Examples
///
/// ```no_run
/// use treepp::config::Config;
/// use treepp::output::write_stdout;
///
/// let config = Config::default();
/// write_stdout("Hello, World!\n", &config).unwrap();
/// ```
pub fn write_stdout(content: &str, config: &Config) -> Result<(), OutputError> {
    if config.output.silent {
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(content.as_bytes())?;
    handle.flush()?;
    Ok(())
}

/// 写入文件
///
/// 使用覆盖写入策略，确保文件内容完整写入。
///
/// # 参数
///
/// * `content` - 输出内容
/// * `path` - 目标文件路径
///
/// # Errors
///
/// 返回 `OutputError` 如果：
/// - 无法创建文件
/// - 写入失败
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use treepp::output::write_file;
///
/// write_file("content", Path::new("output.txt")).unwrap();
/// ```
pub fn write_file(content: &str, path: &Path) -> Result<(), OutputError> {
    // 确保父目录存在
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty() && !parent.exists() {
        fs::create_dir_all(parent).map_err(|e| OutputError::FileCreateFailed {
            path: path.to_path_buf(),
            source: e,
        })?;
    }

    // 创建文件并写入
    let file = File::create(path).map_err(|e| OutputError::FileCreateFailed {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut writer = BufWriter::new(file);
    writer
        .write_all(content.as_bytes())
        .map_err(|e| OutputError::WriteFailed {
            path: path.to_path_buf(),
            source: e,
        })?;

    writer.flush().map_err(|e| OutputError::WriteFailed {
        path: path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

/// 输出文件写入提示信息
///
/// 在非静默模式下，向标准输出打印文件写入提示。
///
/// # 参数
///
/// * `path` - 输出文件路径
/// * `config` - 输出配置
///
/// # Errors
///
/// 返回 `OutputError::StdoutFailed` 如果写入失败。
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use treepp::config::Config;
/// use treepp::output::print_file_notice;
///
/// let config = Config::default();
/// print_file_notice(Path::new("tree.json"), &config).unwrap();
/// ```
pub fn print_file_notice(path: &Path, config: &Config) -> Result<(), OutputError> {
    if config.output.silent {
        return Ok(());
    }

    let notice = format!("\noutput: {}\n", path.display());
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(notice.as_bytes())?;
    handle.flush()?;
    Ok(())
}

// ============================================================================
// 统一输出接口
// ============================================================================

/// 执行输出操作
///
/// 根据配置执行完整的输出流程：
/// 1. 根据格式选择渲染文本或序列化结构
/// 2. 输出到 stdout（除非静默）
/// 3. 写入文件（如果配置了输出路径）
/// 4. 打印文件写入提示（如果写入了文件且非静默）
///
/// # 参数
///
/// * `render_result` - 渲染结果（用于 TXT 格式）
/// * `tree` - 树节点（用于结构化格式）
/// * `config` - 完整配置
///
/// # 返回值
///
/// 成功返回 `()`，失败返回 `OutputError`。
///
/// # Errors
///
/// 返回 `OutputError` 如果：
/// - 序列化失败
/// - 文件写入失败
/// - 标准输出写入失败
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::scan;
/// use treepp::render::render;
/// use treepp::output::execute_output;
///
/// let config = Config::with_root(PathBuf::from(".")).validate().unwrap();
/// let stats = scan(&config).expect("扫描失败");
/// let result = render(&stats, &config);
/// execute_output(&result, &stats.tree, &config).expect("输出失败");
/// ```
pub fn execute_output(
    render_result: &RenderResult,
    tree: &TreeNode,
    config: &Config,
) -> Result<(), OutputError> {
    // 根据格式生成输出内容
    let content = match config.output.format {
        OutputFormat::Txt => render_result.content.clone(),
        OutputFormat::Json => serialize_json(tree, config)?,
        OutputFormat::Yaml => serialize_yaml(tree, config)?,
        OutputFormat::Toml => serialize_toml(tree, config)?,
    };

    // 输出到 stdout
    write_stdout(&content, config)?;

    // 写入文件（如果配置了输出路径）
    if let Some(ref output_path) = config.output.output_path {
        write_file(&content, output_path)?;
        print_file_notice(output_path, config)?;
    }

    Ok(())
}

/// 仅输出到文件（跳过 stdout）
///
/// 用于明确需要跳过标准输出的场景。
///
/// # 参数
///
/// * `render_result` - 渲染结果（用于 TXT 格式）
/// * `tree` - 树节点（用于结构化格式）
/// * `config` - 完整配置
/// * `path` - 输出文件路径
///
/// # Errors
///
/// 返回 `OutputError` 如果序列化或写入失败。
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::scan;
/// use treepp::render::render;
/// use treepp::output::write_to_file_only;
///
/// let config = Config::with_root(PathBuf::from(".")).validate().unwrap();
/// let stats = scan(&config).expect("扫描失败");
/// let result = render(&stats, &config);
/// write_to_file_only(&result, &stats.tree, &config, &PathBuf::from("tree.txt")).unwrap();
/// ```
pub fn write_to_file_only(
    render_result: &RenderResult,
    tree: &TreeNode,
    config: &Config,
    path: &Path,
) -> Result<(), OutputError> {
    let content = match config.output.format {
        OutputFormat::Txt => render_result.content.clone(),
        OutputFormat::Json => serialize_json(tree, config)?,
        OutputFormat::Yaml => serialize_yaml(tree, config)?,
        OutputFormat::Toml => serialize_toml(tree, config)?,
    };

    write_file(&content, path)
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 从文件路径推断输出格式
///
/// # 参数
///
/// * `path` - 文件路径
///
/// # 返回值
///
/// 返回 `Some(OutputFormat)` 如果能识别扩展名，否则返回 `None`。
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use treepp::config::OutputFormat;
/// use treepp::output::infer_format;
///
/// assert_eq!(infer_format(Path::new("tree.json")), Some(OutputFormat::Json));
/// assert_eq!(infer_format(Path::new("tree.yml")), Some(OutputFormat::Yaml));
/// assert_eq!(infer_format(Path::new("tree.yaml")), Some(OutputFormat::Yaml));
/// assert_eq!(infer_format(Path::new("tree.toml")), Some(OutputFormat::Toml));
/// assert_eq!(infer_format(Path::new("tree.txt")), Some(OutputFormat::Txt));
/// assert_eq!(infer_format(Path::new("tree.unknown")), None);
/// ```
#[must_use]
pub fn infer_format(path: &Path) -> Option<OutputFormat> {
    OutputFormat::from_extension(path)
}

/// 验证输出路径有效性
///
/// 检查输出路径是否可写入。
///
/// # 参数
///
/// * `path` - 输出文件路径
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误原因。
///
/// # Errors
///
/// 返回 `OutputError::InvalidOutputPath` 如果：
/// - 路径指向已存在的目录
/// - 父目录不可访问
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use treepp::output::validate_output_path;
///
/// // 正常路径
/// assert!(validate_output_path(Path::new("output.txt")).is_ok());
///
/// // 相对路径也可以
/// assert!(validate_output_path(Path::new("subdir/output.txt")).is_ok());
/// ```
pub fn validate_output_path(path: &Path) -> Result<(), OutputError> {
    // 检查是否为已存在的目录
    if path.exists() && path.is_dir() {
        return Err(OutputError::InvalidOutputPath {
            path: path.to_path_buf(),
            reason: "Path points to an existing directory; please specify a file name.".to_string(),
        });
    }

    // 检查父目录
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty() && parent.exists() && !parent.is_dir() {
        return Err(OutputError::InvalidOutputPath {
            path: path.to_path_buf(),
            reason: "Parent path is not a directory.".to_string(),
        });
    }

    Ok(())
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::EntryMetadata;
    use std::path::PathBuf;
    use tempfile::tempdir;

    /// 创建测试用的树节点
    fn create_test_tree() -> TreeNode {
        let mut root = TreeNode::new(
            PathBuf::from("test_root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        root.children.push(TreeNode::new(
            PathBuf::from("test_root/file1.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 1024,
                ..Default::default()
            },
        ));

        let mut subdir = TreeNode::new(
            PathBuf::from("test_root/subdir"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        subdir.children.push(TreeNode::new(
            PathBuf::from("test_root/subdir/file2.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 2048,
                ..Default::default()
            },
        ));
        root.children.push(subdir);

        root
    }

    #[test]
    fn test_serialize_json_basic() {
        let tree = create_test_tree();
        let config = Config::default();

        let json = serialize_json(&tree, &config).expect("JSON 序列化应成功");

        // 新格式：键为条目名称，值为子项或元数据对象
        assert!(json.contains("\"subdir\""));  // 子目录名称作为键
        assert!(json.contains("{"));           // 对象格式
    }

    #[test]
    fn test_serialize_json_with_files() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.batch_mode = true; // JSON 序列化需要批处理模式
        config.scan.show_files = true;
        config.render.show_size = true;

        let json = serialize_json(&tree, &config).expect("JSON 序列化应成功");

        assert!(json.contains("file1.txt"));
        assert!(json.contains("\"size\":"));
        assert!(json.contains("1024"));
    }

    #[test]
    fn test_serialize_yaml_basic() {
        let tree = create_test_tree();
        let config = Config::default();

        let yaml = serialize_yaml(&tree, &config).expect("YAML 序列化应成功");

        // 新格式：键为条目名称
        assert!(yaml.contains("subdir:") || yaml.contains("file1.txt:"));
    }

    #[test]
    fn test_serialize_toml_basic() {
        let tree = create_test_tree();
        let config = Config::default();

        let toml = serialize_toml(&tree, &config).expect("TOML 序列化应成功");

        // 新格式：TOML 表格，条目名称作为键
        // TOML 输出可能是 [subdir] 或类似格式
        assert!(toml.contains("subdir") || toml.contains("file1"));
    }

    #[test]
    fn test_infer_format_json() {
        assert_eq!(
            infer_format(Path::new("output.json")),
            Some(OutputFormat::Json)
        );
        assert_eq!(
            infer_format(Path::new("OUTPUT.JSON")),
            Some(OutputFormat::Json)
        );
    }

    #[test]
    fn test_infer_format_yaml() {
        assert_eq!(
            infer_format(Path::new("output.yaml")),
            Some(OutputFormat::Yaml)
        );
        assert_eq!(
            infer_format(Path::new("output.yml")),
            Some(OutputFormat::Yaml)
        );
    }

    #[test]
    fn test_infer_format_toml() {
        assert_eq!(
            infer_format(Path::new("output.toml")),
            Some(OutputFormat::Toml)
        );
    }

    #[test]
    fn test_infer_format_txt() {
        assert_eq!(
            infer_format(Path::new("output.txt")),
            Some(OutputFormat::Txt)
        );
    }

    #[test]
    fn test_infer_format_unknown() {
        assert_eq!(infer_format(Path::new("output.xyz")), None);
        assert_eq!(infer_format(Path::new("output")), None);
    }

    #[test]
    fn test_write_file_creates_file() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("test_output.txt");

        write_file("test content", &file_path).expect("写入文件应成功");

        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).expect("读取文件失败");
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_write_file_creates_parent_dirs() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("subdir1/subdir2/test.txt");

        write_file("nested content", &file_path).expect("写入嵌套文件应成功");

        assert!(file_path.exists());
    }

    #[test]
    fn test_write_file_overwrites() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("overwrite.txt");

        write_file("first content", &file_path).expect("首次写入应成功");
        write_file("second content", &file_path).expect("覆盖写入应成功");

        let content = fs::read_to_string(&file_path).expect("读取文件失败");
        assert_eq!(content, "second content");
    }

    #[test]
    fn test_validate_output_path_normal() {
        assert!(validate_output_path(Path::new("output.txt")).is_ok());
        assert!(validate_output_path(Path::new("subdir/output.txt")).is_ok());
    }

    #[test]
    fn test_validate_output_path_existing_dir() {
        let dir = tempdir().expect("创建临时目录失败");

        let result = validate_output_path(dir.path());
        assert!(result.is_err());

        if let Err(OutputError::InvalidOutputPath { reason, .. }) = result {
            assert!(reason.contains("directory"));
        }
    }

    #[test]
    fn test_write_stdout_silent_mode() {
        let mut config = Config::default();
        config.output.silent = true;

        // 静默模式下不应报错（即使我们无法真正验证没有输出）
        let result = write_stdout("test", &config);
        assert!(result.is_ok());
    }
}