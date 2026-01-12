//! Output module: multi-format output and file writing.
//!
//! This module handles outputting render results to various destinations:
//!
//! - **Output strategies**: stdout, file writing, silent mode (file only)
//! - **Multiple formats**: txt/json/yml/toml with fixed serialization schema
//! - **File writing**: overwrite strategy with atomic semantics
//! - **Streaming output**: `StreamWriter` for immediate flush streaming
//!
//! File: src/output.rs
//! Author: WaterRun
//! Date: 2026-01-12

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
// Streaming Writer
// ============================================================================

/// A streaming writer that flushes immediately after each write.
///
/// Wraps stdout with automatic flushing to provide real-time scrolling output.
///
/// # Examples
///
/// ```no_run
/// use treepp::output::StreamWriter;
///
/// let stdout = std::io::stdout();
/// let mut writer = StreamWriter::new(&stdout);
/// writer.write_line("├─src").unwrap();
/// writer.write("Header content\n").unwrap();
/// ```
pub struct StreamWriter<'a> {
    handle: StdoutLock<'a>,
}

impl<'a> StreamWriter<'a> {
    /// Creates a new streaming writer from a stdout reference.
    ///
    /// # Arguments
    ///
    /// * `stdout` - Reference to the standard output handle.
    ///
    /// # Returns
    ///
    /// A new `StreamWriter` instance with the stdout locked.
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

    /// Writes a line and flushes immediately.
    ///
    /// Automatically appends a newline character.
    ///
    /// # Arguments
    ///
    /// * `line` - The line content to write (without trailing newline).
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns `OutputError::StdoutFailed` if writing or flushing fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::output::StreamWriter;
    ///
    /// let stdout = std::io::stdout();
    /// let mut writer = StreamWriter::new(&stdout);
    /// writer.write_line("├─src").unwrap();
    /// writer.write_line("└─tests").unwrap();
    /// ```
    pub fn write_line(&mut self, line: &str) -> Result<(), OutputError> {
        writeln!(self.handle, "{}", line)?;
        self.handle.flush()?;
        Ok(())
    }

    /// Writes a string without appending a newline and flushes.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to write.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns `OutputError::StdoutFailed` if writing or flushing fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::output::StreamWriter;
    ///
    /// let stdout = std::io::stdout();
    /// let mut writer = StreamWriter::new(&stdout);
    /// writer.write("Header: ").unwrap();
    /// writer.write("value\n").unwrap();
    /// ```
    pub fn write(&mut self, content: &str) -> Result<(), OutputError> {
        write!(self.handle, "{}", content)?;
        self.handle.flush()?;
        Ok(())
    }
}

// ============================================================================
// Serialization Functions
// ============================================================================

/// Converts a `TreeNode` to a JSON `Value` with a concise tree structure.
///
/// The output format is:
/// - Directory: object with child names as keys
/// - File: object containing requested metadata, empty if no metadata
///
/// # Arguments
///
/// * `node` - The tree node to convert.
/// * `config` - Configuration controlling which metadata to include.
///
/// # Returns
///
/// A `serde_json::Value` representing the node.
fn to_json_value(node: &TreeNode, config: &Config) -> Value {
    let is_file = node.kind == EntryKind::File;

    if is_file {
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
        let mut obj = Map::new();

        if config.render.show_disk_usage {
            if let Some(usage) = node.disk_usage {
                obj.insert("_disk_usage".to_string(), Value::Number(usage.into()));
            }
        }

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

/// Converts a JSON `Value` to a TOML `Value`.
///
/// # Arguments
///
/// * `value` - The JSON value to convert.
///
/// # Returns
///
/// The equivalent TOML value.
///
/// # Errors
///
/// Returns `OutputError` if conversion fails (currently infallible for valid JSON).
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

/// Serializes a tree node to JSON format.
///
/// Produces a pretty-printed JSON string with the tree structure where
/// directory and file names are used as object keys.
///
/// # Arguments
///
/// * `node` - The root tree node to serialize.
/// * `config` - Configuration controlling serialization options.
///
/// # Returns
///
/// A pretty-printed JSON string on success.
///
/// # Errors
///
/// Returns `OutputError::SerializationFailed` if JSON serialization fails.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
/// use treepp::output::serialize_json;
///
/// let node = TreeNode::new(
///     PathBuf::from("."),
///     EntryKind::Directory,
///     EntryMetadata::default(),
/// );
/// let config = Config::default();
/// let json = serialize_json(&node, &config).unwrap();
/// assert!(json.starts_with("{"));
/// ```
pub fn serialize_json(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    let value = to_json_value(node, config);
    serde_json::to_string_pretty(&value).map_err(|e| OutputError::json_error(e.to_string()))
}

/// Serializes a tree node to YAML format.
///
/// Produces a YAML string with the tree structure where directory and
/// file names are used as mapping keys.
///
/// # Arguments
///
/// * `node` - The root tree node to serialize.
/// * `config` - Configuration controlling serialization options.
///
/// # Returns
///
/// A YAML string on success.
///
/// # Errors
///
/// Returns `OutputError::SerializationFailed` if YAML serialization fails.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
/// use treepp::output::serialize_yaml;
///
/// let node = TreeNode::new(
///     PathBuf::from("."),
///     EntryKind::Directory,
///     EntryMetadata::default(),
/// );
/// let config = Config::default();
/// let yaml = serialize_yaml(&node, &config).unwrap();
/// assert!(!yaml.is_empty());
/// ```
pub fn serialize_yaml(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    let value = to_json_value(node, config);
    serde_yaml::to_string(&value).map_err(|e| OutputError::yaml_error(e.to_string()))
}

/// Serializes a tree node to TOML format.
///
/// Produces a TOML string with the tree structure. Since TOML requires
/// a top-level table, the root node's contents are serialized directly.
///
/// # Arguments
///
/// * `node` - The root tree node to serialize.
/// * `config` - Configuration controlling serialization options.
///
/// # Returns
///
/// A pretty-printed TOML string on success.
///
/// # Errors
///
/// Returns `OutputError::SerializationFailed` if TOML serialization fails.
///
/// # Examples
///
/// ```no_run
/// use std::path::PathBuf;
/// use treepp::config::Config;
/// use treepp::scan::{TreeNode, EntryKind, EntryMetadata};
/// use treepp::output::serialize_toml;
///
/// let node = TreeNode::new(
///     PathBuf::from("."),
///     EntryKind::Directory,
///     EntryMetadata::default(),
/// );
/// let config = Config::default();
/// let toml_str = serialize_toml(&node, &config).unwrap();
/// assert!(!toml_str.is_empty());
/// ```
pub fn serialize_toml(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    let value = to_json_value(node, config);
    let toml_value = json_to_toml(&value)?;
    toml::to_string_pretty(&toml_value).map_err(|e| OutputError::toml_error(e.to_string()))
}

// ============================================================================
// Output Functions
// ============================================================================

/// Writes content to standard output.
///
/// Respects the silent mode configuration; if silent is enabled,
/// no output is written.
///
/// # Arguments
///
/// * `content` - The content to write.
/// * `config` - Configuration containing the silent mode flag.
///
/// # Returns
///
/// `Ok(())` on success or if silent mode is enabled.
///
/// # Errors
///
/// Returns `OutputError::StdoutFailed` if writing to stdout fails.
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

/// Writes content to a file.
///
/// Uses overwrite strategy and creates parent directories if needed.
/// The write is buffered for performance.
///
/// # Arguments
///
/// * `content` - The content to write.
/// * `path` - The destination file path.
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns `OutputError::FileCreateFailed` if the file cannot be created,
/// or `OutputError::WriteFailed` if writing fails.
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
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(|e| OutputError::FileCreateFailed {
            path: path.to_path_buf(),
            source: e,
        })?;
    }

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

/// Prints a file output notice to stdout.
///
/// Displays the path where output was written, unless silent mode is enabled.
///
/// # Arguments
///
/// * `path` - The output file path to display.
/// * `config` - Configuration containing the silent mode flag.
///
/// # Returns
///
/// `Ok(())` on success or if silent mode is enabled.
///
/// # Errors
///
/// Returns `OutputError::StdoutFailed` if writing to stdout fails.
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
// Unified Output Interface
// ============================================================================

/// Executes the complete output workflow.
///
/// Performs the full output process based on configuration:
/// 1. Selects rendered text or serializes structure based on format
/// 2. Outputs to stdout (unless silent)
/// 3. Writes to file (if output path is configured)
/// 4. Prints file notice (if file was written and not silent)
///
/// # Arguments
///
/// * `render_result` - The render result (used for TXT format).
/// * `tree` - The tree node (used for structured formats).
/// * `config` - The complete configuration.
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns `OutputError` if serialization, file writing, or stdout fails.
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
/// let stats = scan(&config).expect("Scan failed");
/// let result = render(&stats, &config);
/// execute_output(&result, &stats.tree, &config).expect("Output failed");
/// ```
pub fn execute_output(
    render_result: &RenderResult,
    tree: &TreeNode,
    config: &Config,
) -> Result<(), OutputError> {
    let content = match config.output.format {
        OutputFormat::Txt => render_result.content.clone(),
        OutputFormat::Json => serialize_json(tree, config)?,
        OutputFormat::Yaml => serialize_yaml(tree, config)?,
        OutputFormat::Toml => serialize_toml(tree, config)?,
    };

    write_stdout(&content, config)?;

    if let Some(ref output_path) = config.output.output_path {
        write_file(&content, output_path)?;
        print_file_notice(output_path, config)?;
    }

    Ok(())
}

/// Writes output to a file only, skipping stdout.
///
/// Used when stdout output should be explicitly bypassed.
///
/// # Arguments
///
/// * `render_result` - The render result (used for TXT format).
/// * `tree` - The tree node (used for structured formats).
/// * `config` - The complete configuration.
/// * `path` - The output file path.
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns `OutputError` if serialization or file writing fails.
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
/// let stats = scan(&config).expect("Scan failed");
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
// Helper Functions
// ============================================================================

/// Infers the output format from a file path extension.
///
/// Recognizes the following extensions (case-insensitive):
/// - `.json` → JSON
/// - `.yaml`, `.yml` → YAML
/// - `.toml` → TOML
/// - `.txt` → TXT
///
/// # Arguments
///
/// * `path` - The file path to examine.
///
/// # Returns
///
/// `Some(OutputFormat)` if the extension is recognized, `None` otherwise.
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
/// assert_eq!(infer_format(Path::new("no_extension")), None);
/// ```
#[must_use]
pub fn infer_format(path: &Path) -> Option<OutputFormat> {
    OutputFormat::from_extension(path)
}

/// Validates that an output path is writable.
///
/// Checks that the path does not point to an existing directory and
/// that the parent path (if it exists) is a directory.
///
/// # Arguments
///
/// * `path` - The output file path to validate.
///
/// # Returns
///
/// `Ok(())` if the path is valid for writing.
///
/// # Errors
///
/// Returns `OutputError::InvalidOutputPath` if:
/// - The path points to an existing directory
/// - The parent path exists but is not a directory
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use treepp::output::validate_output_path;
///
/// assert!(validate_output_path(Path::new("output.txt")).is_ok());
/// assert!(validate_output_path(Path::new("subdir/output.txt")).is_ok());
/// ```
pub fn validate_output_path(path: &Path) -> Result<(), OutputError> {
    if path.exists() && path.is_dir() {
        return Err(OutputError::InvalidOutputPath {
            path: path.to_path_buf(),
            reason: "Path points to an existing directory; please specify a file name.".to_string(),
        });
    }

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && parent.exists()
        && !parent.is_dir()
    {
        return Err(OutputError::InvalidOutputPath {
            path: path.to_path_buf(),
            reason: "Parent path is not a directory.".to_string(),
        });
    }

    Ok(())
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::EntryMetadata;
    use std::path::PathBuf;
    use tempfile::tempdir;

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

    fn create_empty_tree() -> TreeNode {
        TreeNode::new(
            PathBuf::from("empty_root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        )
    }

    fn create_deep_tree() -> TreeNode {
        let mut root = TreeNode::new(
            PathBuf::from("deep"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut level1 = TreeNode::new(
            PathBuf::from("deep/level1"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        let mut level2 = TreeNode::new(
            PathBuf::from("deep/level1/level2"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        level2.children.push(TreeNode::new(
            PathBuf::from("deep/level1/level2/deep_file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 512,
                ..Default::default()
            },
        ));

        level1.children.push(level2);
        root.children.push(level1);

        root
    }

    #[test]
    fn should_serialize_json_with_directory_structure() {
        let tree = create_test_tree();
        let config = Config::default();

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("\"subdir\""));
        assert!(json.contains("{"));
    }

    #[test]
    fn should_serialize_json_with_file_metadata_when_enabled() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.batch_mode = true;
        config.scan.show_files = true;
        config.render.show_size = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("file1.txt"));
        assert!(json.contains("\"size\":"));
        assert!(json.contains("1024"));
    }

    #[test]
    fn should_serialize_json_for_empty_tree() {
        let tree = create_empty_tree();
        let config = Config::default();

        let json = serialize_json(&tree, &config).expect("空树JSON序列化应成功");

        assert!(json.contains("{"));
        assert!(json.contains("}"));
    }

    #[test]
    fn should_serialize_json_for_deep_tree() {
        let tree = create_deep_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("深层树JSON序列化应成功");

        assert!(json.contains("level1"));
        assert!(json.contains("level2"));
        assert!(json.contains("deep_file.txt"));
    }

    #[test]
    fn should_serialize_yaml_with_directory_structure() {
        let tree = create_test_tree();
        let config = Config::default();

        let yaml = serialize_yaml(&tree, &config).expect("YAML序列化应成功");

        assert!(yaml.contains("subdir:") || yaml.contains("file1.txt:"));
    }

    #[test]
    fn should_serialize_yaml_for_empty_tree() {
        let tree = create_empty_tree();
        let config = Config::default();

        let yaml = serialize_yaml(&tree, &config).expect("空树YAML序列化应成功");

        assert!(!yaml.is_empty());
    }

    #[test]
    fn should_serialize_toml_with_directory_structure() {
        let tree = create_test_tree();
        let config = Config::default();

        let toml = serialize_toml(&tree, &config).expect("TOML序列化应成功");

        assert!(toml.contains("subdir") || toml.contains("file1"));
    }

    #[test]
    fn should_serialize_toml_for_empty_tree() {
        let tree = create_empty_tree();
        let config = Config::default();

        let result = serialize_toml(&tree, &config);

        assert!(result.is_ok());
    }

    #[test]
    fn should_infer_json_format_from_extension() {
        assert_eq!(
            infer_format(Path::new("output.json")),
            Some(OutputFormat::Json)
        );
        assert_eq!(
            infer_format(Path::new("OUTPUT.JSON")),
            Some(OutputFormat::Json)
        );
        assert_eq!(
            infer_format(Path::new("path/to/file.json")),
            Some(OutputFormat::Json)
        );
    }

    #[test]
    fn should_infer_yaml_format_from_extension() {
        assert_eq!(
            infer_format(Path::new("output.yaml")),
            Some(OutputFormat::Yaml)
        );
        assert_eq!(
            infer_format(Path::new("output.yml")),
            Some(OutputFormat::Yaml)
        );
        assert_eq!(
            infer_format(Path::new("OUTPUT.YML")),
            Some(OutputFormat::Yaml)
        );
    }

    #[test]
    fn should_infer_toml_format_from_extension() {
        assert_eq!(
            infer_format(Path::new("output.toml")),
            Some(OutputFormat::Toml)
        );
        assert_eq!(
            infer_format(Path::new("config.TOML")),
            Some(OutputFormat::Toml)
        );
    }

    #[test]
    fn should_infer_txt_format_from_extension() {
        assert_eq!(
            infer_format(Path::new("output.txt")),
            Some(OutputFormat::Txt)
        );
        assert_eq!(
            infer_format(Path::new("README.TXT")),
            Some(OutputFormat::Txt)
        );
    }

    #[test]
    fn should_return_none_for_unknown_extension() {
        assert_eq!(infer_format(Path::new("output.xyz")), None);
        assert_eq!(infer_format(Path::new("output")), None);
        assert_eq!(infer_format(Path::new(".hidden")), None);
        assert_eq!(infer_format(Path::new("file.doc")), None);
    }

    #[test]
    fn should_create_file_and_write_content() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("test_output.txt");

        write_file("test content", &file_path).expect("写入文件应成功");

        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).expect("读取文件失败");
        assert_eq!(content, "test content");
    }

    #[test]
    fn should_create_parent_directories_when_writing() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("subdir1/subdir2/test.txt");

        write_file("nested content", &file_path).expect("写入嵌套文件应成功");

        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).expect("读取文件失败");
        assert_eq!(content, "nested content");
    }

    #[test]
    fn should_overwrite_existing_file() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("overwrite.txt");

        write_file("first content", &file_path).expect("首次写入应成功");
        write_file("second content", &file_path).expect("覆盖写入应成功");

        let content = fs::read_to_string(&file_path).expect("读取文件失败");
        assert_eq!(content, "second content");
    }

    #[test]
    fn should_write_empty_content() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("empty.txt");

        write_file("", &file_path).expect("写入空内容应成功");

        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).expect("读取文件失败");
        assert!(content.is_empty());
    }

    #[test]
    fn should_write_unicode_content() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("unicode.txt");

        write_file("你好世界 🌍 émoji", &file_path).expect("写入Unicode内容应成功");

        let content = fs::read_to_string(&file_path).expect("读取文件失败");
        assert_eq!(content, "你好世界 🌍 émoji");
    }

    #[test]
    fn should_validate_normal_output_path() {
        assert!(validate_output_path(Path::new("output.txt")).is_ok());
        assert!(validate_output_path(Path::new("subdir/output.txt")).is_ok());
        assert!(validate_output_path(Path::new("a/b/c/d/output.json")).is_ok());
    }

    #[test]
    fn should_reject_existing_directory_as_output_path() {
        let dir = tempdir().expect("创建临时目录失败");

        let result = validate_output_path(dir.path());
        assert!(result.is_err());

        if let Err(OutputError::InvalidOutputPath { reason, .. }) = result {
            assert!(reason.contains("directory"));
        }
    }

    #[test]
    fn should_skip_output_in_silent_mode() {
        let mut config = Config::default();
        config.output.silent = true;

        let result = write_stdout("test", &config);
        assert!(result.is_ok());
    }

    #[test]
    fn should_skip_file_notice_in_silent_mode() {
        let mut config = Config::default();
        config.output.silent = true;

        let result = print_file_notice(Path::new("test.txt"), &config);
        assert!(result.is_ok());
    }

    #[test]
    fn should_convert_json_null_to_toml() {
        let result = json_to_toml(&Value::Null).expect("转换应成功");
        assert_eq!(result, toml::Value::String("null".to_string()));
    }

    #[test]
    fn should_convert_json_bool_to_toml() {
        let result = json_to_toml(&Value::Bool(true)).expect("转换应成功");
        assert_eq!(result, toml::Value::Boolean(true));
    }

    #[test]
    fn should_convert_json_number_to_toml() {
        let result = json_to_toml(&Value::Number(42.into())).expect("转换应成功");
        assert_eq!(result, toml::Value::Integer(42));
    }

    #[test]
    fn should_convert_json_string_to_toml() {
        let result = json_to_toml(&Value::String("test".to_string())).expect("转换应成功");
        assert_eq!(result, toml::Value::String("test".to_string()));
    }

    #[test]
    fn should_convert_json_array_to_toml() {
        let arr = Value::Array(vec![Value::Number(1.into()), Value::Number(2.into())]);
        let result = json_to_toml(&arr).expect("转换应成功");
        if let toml::Value::Array(arr) = result {
            assert_eq!(arr.len(), 2);
        } else {
            panic!("应该是数组");
        }
    }

    #[test]
    fn should_convert_json_object_to_toml() {
        let mut obj = Map::new();
        obj.insert("key".to_string(), Value::String("value".to_string()));
        let result = json_to_toml(&Value::Object(obj)).expect("转换应成功");
        if let toml::Value::Table(table) = result {
            assert!(table.contains_key("key"));
        } else {
            panic!("应该是表格");
        }
    }
}
