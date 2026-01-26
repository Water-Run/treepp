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
//! Date: 2026-01-26

#![forbid(unsafe_code)]

use std::fs::{self, File};
use std::io::{self, BufWriter, Stdout, StdoutLock, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{Config, OutputFormat};
use crate::error::OutputError;
use crate::render::RenderResult;
use crate::scan::{EntryKind, TreeNode};

// ============================================================================
// Constants
// ============================================================================

/// Schema version for structured output formats.
const SCHEMA_VERSION: &str = "treepp.pretty.v1";

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
// Structured Output Schema
// ============================================================================

/// Directory node in the structured output format.
///
/// Represents a directory with its files and subdirectories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirNode {
    /// Node type, always "dir" for directories.
    #[serde(rename = "type")]
    pub node_type: String,
    /// List of file names in this directory.
    pub files: Vec<String>,
    /// Map of subdirectory names to their nodes.
    pub dirs: std::collections::BTreeMap<String, DirNode>,
    /// File size in bytes (only when show_size is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Disk usage for directory (only when show_disk_usage is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_usage: Option<u64>,
    /// Last modification date (only when show_date is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

impl DirNode {
    /// Creates a new directory node.
    #[must_use]
    fn new() -> Self {
        Self {
            node_type: "dir".to_string(),
            files: Vec::new(),
            dirs: std::collections::BTreeMap::new(),
            size: None,
            disk_usage: None,
            modified: None,
        }
    }
}

/// File entry with optional metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    /// File name.
    pub name: String,
    /// File size in bytes (only when show_size is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Last modification date (only when show_date is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

/// Root node in the structured output format.
///
/// Contains the root path and directory structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RootNode {
    /// Root path display string.
    pub path: String,
    /// Node type, always "dir".
    #[serde(rename = "type")]
    pub node_type: String,
    /// List of file names or file entries in root directory.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<Value>,
    /// Map of subdirectory names to their nodes.
    pub dirs: std::collections::BTreeMap<String, DirNode>,
    /// Disk usage for root directory (only when show_disk_usage is enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk_usage: Option<u64>,
}

/// Top-level structure for structured output.
///
/// Contains schema version and root node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredOutput {
    /// Schema version identifier.
    pub schema: String,
    /// Root directory node.
    pub root: RootNode,
}

// ============================================================================
// Serialization Functions
// ============================================================================

/// Converts a `TreeNode` to a `DirNode` for structured output.
///
/// # Arguments
///
/// * `node` - The tree node to convert.
/// * `config` - Configuration controlling which metadata to include.
///
/// # Returns
///
/// A `DirNode` representing the directory structure.
fn tree_to_dir_node(node: &TreeNode, config: &Config) -> DirNode {
    let mut dir_node = DirNode::new();

    if config.render.show_disk_usage {
        dir_node.disk_usage = node.disk_usage;
    }

    if config.render.show_date {
        if let Some(ref modified) = node.metadata.modified {
            dir_node.modified = Some(crate::render::format_datetime(modified));
        }
    }

    let (files, dirs): (Vec<_>, Vec<_>) = node
        .children
        .iter()
        .partition(|c| c.kind == EntryKind::File);

    for file in files {
        if config.scan.show_files {
            dir_node.files.push(file.name.clone());
        }
    }

    for subdir in dirs {
        let sub_dir_node = tree_to_dir_node(subdir, config);
        dir_node.dirs.insert(subdir.name.clone(), sub_dir_node);
    }

    dir_node
}

/// Converts a `TreeNode` to a `DirNode` with detailed file metadata.
///
/// # Arguments
///
/// * `node` - The tree node to convert.
/// * `config` - Configuration controlling which metadata to include.
///
/// # Returns
///
/// A tuple of (files as Value array, dirs as BTreeMap).
fn tree_to_detailed_content(
    node: &TreeNode,
    config: &Config,
) -> (Vec<Value>, std::collections::BTreeMap<String, DirNode>) {
    let mut files = Vec::new();
    let mut dirs = std::collections::BTreeMap::new();

    let (file_nodes, dir_nodes): (Vec<_>, Vec<_>) = node
        .children
        .iter()
        .partition(|c| c.kind == EntryKind::File);

    let needs_file_metadata =
        (config.render.show_size || config.render.show_date) && config.scan.show_files;

    for file in file_nodes {
        if config.scan.show_files {
            if needs_file_metadata {
                let mut file_obj = serde_json::Map::new();
                file_obj.insert("name".to_string(), Value::String(file.name.clone()));

                if config.render.show_size {
                    file_obj.insert(
                        "size".to_string(),
                        Value::Number(file.metadata.size.into()),
                    );
                }

                if config.render.show_date {
                    if let Some(ref modified) = file.metadata.modified {
                        file_obj.insert(
                            "modified".to_string(),
                            Value::String(crate::render::format_datetime(modified)),
                        );
                    }
                }

                files.push(Value::Object(file_obj));
            } else {
                files.push(Value::String(file.name.clone()));
            }
        }
    }

    for subdir in dir_nodes {
        let sub_dir_node = tree_to_dir_node(subdir, config);
        dirs.insert(subdir.name.clone(), sub_dir_node);
    }

    (files, dirs)
}

/// Creates the structured output from a tree node.
///
/// # Arguments
///
/// * `node` - The root tree node.
/// * `config` - Configuration controlling serialization options.
///
/// # Returns
///
/// A `StructuredOutput` structure ready for serialization.
fn create_structured_output(node: &TreeNode, config: &Config) -> StructuredOutput {
    let root_path = format_root_path(&config.root_path);
    let (files, dirs) = tree_to_detailed_content(node, config);

    let mut root = RootNode {
        path: root_path,
        node_type: "dir".to_string(),
        files,
        dirs,
        disk_usage: None,
    };

    if config.render.show_disk_usage {
        root.disk_usage = node.disk_usage;
    }

    StructuredOutput {
        schema: SCHEMA_VERSION.to_string(),
        root,
    }
}

/// Formats the root path for display in structured output.
///
/// # Arguments
///
/// * `path` - The root path.
///
/// # Returns
///
/// A formatted path string.
fn format_root_path(path: &Path) -> String {
    use std::path::Component;

    if let Some(Component::Prefix(prefix)) = path.components().next() {
        let prefix_str = prefix.as_os_str().to_string_lossy();
        let chars: Vec<char> = prefix_str.chars().collect();

        if chars.len() >= 2 && chars[1] == ':' {
            return format!("{}:.", chars[0].to_ascii_uppercase());
        }

        if prefix_str.starts_with(r"\\?\") && chars.len() >= 6 && chars[5] == ':' {
            return format!("{}:.", chars[4].to_ascii_uppercase());
        }
    }

    path.to_string_lossy().into_owned()
}

/// Serializes a tree node to JSON format.
///
/// Produces a pretty-printed JSON string with the tree structure using
/// the treepp.pretty.v1 schema.
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
/// assert!(json.contains("treepp.pretty.v1"));
/// ```
pub fn serialize_json(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    let output = create_structured_output(node, config);
    serde_json::to_string_pretty(&output).map_err(|e| OutputError::json_error(e.to_string()))
}

/// Serializes a tree node to YAML format.
///
/// Produces a YAML string with the tree structure using the treepp.pretty.v1 schema.
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
/// assert!(yaml.contains("treepp.pretty.v1"));
/// ```
pub fn serialize_yaml(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    let output = create_structured_output(node, config);
    serde_yaml::to_string(&output).map_err(|e| OutputError::yaml_error(e.to_string()))
}

/// Serializes a tree node to TOML format.
///
/// Produces a TOML string with the tree structure. Uses the treepp.pretty.v1 schema.
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
/// assert!(toml_str.contains("treepp.pretty.v1"));
/// ```
pub fn serialize_toml(node: &TreeNode, config: &Config) -> Result<String, OutputError> {
    let output = create_structured_output(node, config);

    // Convert to TOML-compatible structure
    let toml_output = TomlOutput::from_structured(&output);

    toml::to_string_pretty(&toml_output).map_err(|e| OutputError::toml_error(e.to_string()))
}

/// TOML-specific output structure.
///
/// TOML has limitations with heterogeneous arrays, so we use a slightly
/// different structure for TOML output.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlOutput {
    schema: String,
    root: TomlRootNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlRootNode {
    path: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk_usage: Option<u64>,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    dirs: std::collections::BTreeMap<String, TomlDirNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TomlDirNode {
    #[serde(rename = "type")]
    node_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk_usage: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    dirs: std::collections::BTreeMap<String, TomlDirNode>,
}

impl TomlOutput {
    fn from_structured(output: &StructuredOutput) -> Self {
        let files: Vec<String> = output
            .root
            .files
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Object(obj) => obj.get("name").and_then(|n| n.as_str()).map(String::from),
                _ => None,
            })
            .collect();

        let dirs = output
            .root
            .dirs
            .iter()
            .map(|(k, v)| (k.clone(), TomlDirNode::from_dir_node(v)))
            .collect();

        Self {
            schema: output.schema.clone(),
            root: TomlRootNode {
                path: output.root.path.clone(),
                node_type: output.root.node_type.clone(),
                files,
                disk_usage: output.root.disk_usage,
                dirs,
            },
        }
    }
}

impl TomlDirNode {
    fn from_dir_node(node: &DirNode) -> Self {
        Self {
            node_type: node.node_type.clone(),
            files: node.files.clone(),
            disk_usage: node.disk_usage,
            modified: node.modified.clone(),
            dirs: node
                .dirs
                .iter()
                .map(|(k, v)| (k.clone(), Self::from_dir_node(v)))
                .collect(),
        }
    }
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
    use std::time::SystemTime;
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

    // ========================================================================
    // Schema Structure Tests
    // ========================================================================

    #[test]
    fn should_serialize_json_with_schema_version() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("\"schema\": \"treepp.pretty.v1\""));
    }

    #[test]
    fn should_serialize_json_with_root_structure() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("\"root\""));
        assert!(json.contains("\"type\": \"dir\""));
        assert!(json.contains("\"files\""));
        assert!(json.contains("\"dirs\""));
    }

    #[test]
    fn should_serialize_json_with_files_array() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("\"file1.txt\""));
        assert!(json.contains("\"file2.txt\""));
    }

    #[test]
    fn should_serialize_json_with_nested_dirs() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("\"subdir\""));
    }

    #[test]
    fn should_serialize_json_for_empty_tree() {
        let tree = create_empty_tree();
        let config = Config::default();

        let json = serialize_json(&tree, &config).expect("空树JSON序列化应成功");

        assert!(json.contains("treepp.pretty.v1"));
        assert!(json.contains("\"dirs\": {}"));
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
    fn should_serialize_json_with_file_size_when_enabled() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.batch_mode = true;
        config.scan.show_files = true;
        config.render.show_size = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("\"size\""));
        assert!(json.contains("1024"));
    }

    #[test]
    fn should_serialize_json_with_disk_usage_when_enabled() {
        let mut tree = create_test_tree();
        tree.compute_disk_usage();

        let mut config = Config::default();
        config.batch_mode = true;
        config.scan.show_files = true;
        config.render.show_disk_usage = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("\"disk_usage\""));
    }

    #[test]
    fn should_serialize_json_with_modified_date_when_enabled() {
        let mut tree = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        tree.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 100,
                modified: Some(SystemTime::now()),
                ..Default::default()
            },
        ));

        let mut config = Config::default();
        config.scan.show_files = true;
        config.render.show_date = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");

        assert!(json.contains("\"modified\""));
    }

    // ========================================================================
    // YAML Serialization Tests
    // ========================================================================

    #[test]
    fn should_serialize_yaml_with_schema_version() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let yaml = serialize_yaml(&tree, &config).expect("YAML序列化应成功");

        assert!(yaml.contains("schema: treepp.pretty.v1"));
    }

    #[test]
    fn should_serialize_yaml_with_root_structure() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let yaml = serialize_yaml(&tree, &config).expect("YAML序列化应成功");

        assert!(yaml.contains("root:"));
        assert!(yaml.contains("type: dir"));
        assert!(yaml.contains("files:"));
        assert!(yaml.contains("dirs:"));
    }

    #[test]
    fn should_serialize_yaml_with_files_list() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let yaml = serialize_yaml(&tree, &config).expect("YAML序列化应成功");

        assert!(yaml.contains("file1.txt"));
        assert!(yaml.contains("file2.txt"));
    }

    #[test]
    fn should_serialize_yaml_for_empty_tree() {
        let tree = create_empty_tree();
        let config = Config::default();

        let yaml = serialize_yaml(&tree, &config).expect("空树YAML序列化应成功");

        assert!(yaml.contains("treepp.pretty.v1"));
        assert!(yaml.contains("dirs: {}"));
    }

    // ========================================================================
    // TOML Serialization Tests
    // ========================================================================

    #[test]
    fn should_serialize_toml_with_schema_version() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let toml = serialize_toml(&tree, &config).expect("TOML序列化应成功");

        assert!(toml.contains("schema = \"treepp.pretty.v1\""));
    }

    #[test]
    fn should_serialize_toml_with_root_section() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let toml = serialize_toml(&tree, &config).expect("TOML序列化应成功");

        assert!(toml.contains("[root]"));
        assert!(toml.contains("type = \"dir\""));
    }

    #[test]
    fn should_serialize_toml_with_files_array() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let toml = serialize_toml(&tree, &config).expect("TOML序列化应成功");

        assert!(toml.contains("files = ["));
        assert!(toml.contains("\"file1.txt\""));
    }

    #[test]
    fn should_serialize_toml_with_nested_dirs() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let toml = serialize_toml(&tree, &config).expect("TOML序列化应成功");

        assert!(toml.contains("[root.dirs.subdir]"));
    }

    #[test]
    fn should_serialize_toml_for_empty_tree() {
        let tree = create_empty_tree();
        let config = Config::default();

        let result = serialize_toml(&tree, &config);

        assert!(result.is_ok());
        let toml = result.unwrap();
        assert!(toml.contains("treepp.pretty.v1"));
    }

    // ========================================================================
    // Format Inference Tests
    // ========================================================================

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

    // ========================================================================
    // File Writing Tests
    // ========================================================================

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

    // ========================================================================
    // Path Validation Tests
    // ========================================================================

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

    // ========================================================================
    // Silent Mode Tests
    // ========================================================================

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

    // ========================================================================
    // Root Path Formatting Tests
    // ========================================================================

    #[test]
    fn should_format_root_path_with_drive_letter() {
        let path = PathBuf::from(r"C:\Users\Test");
        let formatted = format_root_path(&path);
        assert_eq!(formatted, "C:.");
    }

    #[test]
    fn should_format_root_path_lowercase_drive() {
        let path = PathBuf::from(r"d:\data");
        let formatted = format_root_path(&path);
        assert_eq!(formatted, "D:.");
    }

    #[test]
    fn should_format_relative_path() {
        let path = PathBuf::from("relative/path");
        let formatted = format_root_path(&path);
        assert_eq!(formatted, "relative/path");
    }

    // ========================================================================
    // DirNode Tests
    // ========================================================================

    #[test]
    fn should_create_empty_dir_node() {
        let node = DirNode::new();
        assert_eq!(node.node_type, "dir");
        assert!(node.files.is_empty());
        assert!(node.dirs.is_empty());
        assert!(node.size.is_none());
        assert!(node.disk_usage.is_none());
        assert!(node.modified.is_none());
    }

    #[test]
    fn should_convert_tree_to_dir_node() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let dir_node = tree_to_dir_node(&tree, &config);

        assert_eq!(dir_node.node_type, "dir");
        assert!(dir_node.files.contains(&"file1.txt".to_string()));
        assert!(dir_node.dirs.contains_key("subdir"));
    }

    #[test]
    fn should_convert_tree_to_dir_node_without_files() {
        let tree = create_test_tree();
        let config = Config::default();

        let dir_node = tree_to_dir_node(&tree, &config);

        assert!(dir_node.files.is_empty());
        assert!(dir_node.dirs.contains_key("subdir"));
    }

    // ========================================================================
    // StructuredOutput Tests
    // ========================================================================

    #[test]
    fn should_create_structured_output() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let output = create_structured_output(&tree, &config);

        assert_eq!(output.schema, SCHEMA_VERSION);
        assert_eq!(output.root.node_type, "dir");
    }

    #[test]
    fn should_include_disk_usage_in_structured_output() {
        let mut tree = create_test_tree();
        tree.compute_disk_usage();

        let mut config = Config::default();
        config.batch_mode = true;
        config.render.show_disk_usage = true;

        let output = create_structured_output(&tree, &config);

        assert!(output.root.disk_usage.is_some());
    }

    // ========================================================================
    // Round-Trip Tests
    // ========================================================================

    #[test]
    fn should_deserialize_json_output() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("序列化应成功");
        let parsed: StructuredOutput = serde_json::from_str(&json).expect("反序列化应成功");

        assert_eq!(parsed.schema, SCHEMA_VERSION);
        assert_eq!(parsed.root.node_type, "dir");
    }

    #[test]
    fn should_deserialize_yaml_output() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let yaml = serialize_yaml(&tree, &config).expect("序列化应成功");
        let parsed: StructuredOutput = serde_yaml::from_str(&yaml).expect("反序列化应成功");

        assert_eq!(parsed.schema, SCHEMA_VERSION);
        assert_eq!(parsed.root.node_type, "dir");
    }

    // ========================================================================
    // Edge Cases Tests
    // ========================================================================

    #[test]
    fn should_handle_special_characters_in_filenames() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file with spaces.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));
        root.children.push(TreeNode::new(
            PathBuf::from("root/文件名.txt"),
            EntryKind::File,
            EntryMetadata::default(),
        ));

        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&root, &config).expect("序列化应成功");

        assert!(json.contains("file with spaces.txt"));
        assert!(json.contains("文件名.txt"));
    }

    #[test]
    fn should_handle_deeply_nested_structure() {
        let tree = create_deep_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("序列化应成功");
        let yaml = serialize_yaml(&tree, &config).expect("序列化应成功");
        let toml = serialize_toml(&tree, &config).expect("序列化应成功");

        assert!(json.contains("level2"));
        assert!(yaml.contains("level2"));
        assert!(toml.contains("level2"));
    }

    #[test]
    fn should_handle_many_files() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        for i in 0..100 {
            root.children.push(TreeNode::new(
                PathBuf::from(format!("root/file{}.txt", i)),
                EntryKind::File,
                EntryMetadata::default(),
            ));
        }

        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&root, &config).expect("序列化应成功");

        assert!(json.contains("file0.txt"));
        assert!(json.contains("file99.txt"));
    }

    #[test]
    fn should_handle_many_directories() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );

        for i in 0..50 {
            root.children.push(TreeNode::new(
                PathBuf::from(format!("root/dir{}", i)),
                EntryKind::Directory,
                EntryMetadata::default(),
            ));
        }

        let config = Config::default();

        let json = serialize_json(&root, &config).expect("序列化应成功");

        assert!(json.contains("dir0"));
        assert!(json.contains("dir49"));
    }

    // ========================================================================
    // Metadata Tests
    // ========================================================================

    #[test]
    fn should_include_file_metadata_in_json_when_enabled() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 12345,
                modified: Some(SystemTime::now()),
                ..Default::default()
            },
        ));

        let mut config = Config::default();
        config.scan.show_files = true;
        config.render.show_size = true;
        config.render.show_date = true;

        let json = serialize_json(&root, &config).expect("序列化应成功");

        assert!(json.contains("\"name\": \"file.txt\""));
        assert!(json.contains("\"size\": 12345"));
        assert!(json.contains("\"modified\""));
    }

    #[test]
    fn should_not_include_file_metadata_when_disabled() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 12345,
                ..Default::default()
            },
        ));

        let mut config = Config::default();
        config.scan.show_files = true;
        config.render.show_size = false;
        config.render.show_date = false;

        let json = serialize_json(&root, &config).expect("序列化应成功");

        // Files should be simple strings when no metadata is requested
        assert!(json.contains("\"file.txt\""));
        // Should not contain size as a separate field
        assert!(!json.contains("\"size\": 12345"));
    }

    // ========================================================================
    // TomlOutput Tests
    // ========================================================================

    #[test]
    fn should_convert_structured_to_toml_output() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let structured = create_structured_output(&tree, &config);
        let toml_output = TomlOutput::from_structured(&structured);

        assert_eq!(toml_output.schema, SCHEMA_VERSION);
        assert_eq!(toml_output.root.node_type, "dir");
        assert!(toml_output.root.files.contains(&"file1.txt".to_string()));
    }

    #[test]
    fn should_extract_file_names_from_objects() {
        let mut root = TreeNode::new(
            PathBuf::from("root"),
            EntryKind::Directory,
            EntryMetadata::default(),
        );
        root.children.push(TreeNode::new(
            PathBuf::from("root/file.txt"),
            EntryKind::File,
            EntryMetadata {
                size: 100,
                ..Default::default()
            },
        ));

        let mut config = Config::default();
        config.scan.show_files = true;
        config.render.show_size = true;

        let structured = create_structured_output(&root, &config);
        let toml_output = TomlOutput::from_structured(&structured);

        assert!(toml_output.root.files.contains(&"file.txt".to_string()));
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn should_serialize_all_formats_consistently() {
        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("JSON序列化应成功");
        let yaml = serialize_yaml(&tree, &config).expect("YAML序列化应成功");
        let toml = serialize_toml(&tree, &config).expect("TOML序列化应成功");

        // All formats should contain the schema version
        assert!(json.contains("treepp.pretty.v1"));
        assert!(yaml.contains("treepp.pretty.v1"));
        assert!(toml.contains("treepp.pretty.v1"));

        // All formats should contain file names
        assert!(json.contains("file1.txt"));
        assert!(yaml.contains("file1.txt"));
        assert!(toml.contains("file1.txt"));

        // All formats should contain directory names
        assert!(json.contains("subdir"));
        assert!(yaml.contains("subdir"));
        assert!(toml.contains("subdir"));
    }

    #[test]
    fn should_write_json_to_file() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("output.json");

        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let json = serialize_json(&tree, &config).expect("序列化应成功");
        write_file(&json, &file_path).expect("写入应成功");

        let content = fs::read_to_string(&file_path).expect("读取失败");
        assert!(content.contains("treepp.pretty.v1"));
    }

    #[test]
    fn should_write_yaml_to_file() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("output.yml");

        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let yaml = serialize_yaml(&tree, &config).expect("序列化应成功");
        write_file(&yaml, &file_path).expect("写入应成功");

        let content = fs::read_to_string(&file_path).expect("读取失败");
        assert!(content.contains("treepp.pretty.v1"));
    }

    #[test]
    fn should_write_toml_to_file() {
        let dir = tempdir().expect("创建临时目录失败");
        let file_path = dir.path().join("output.toml");

        let tree = create_test_tree();
        let mut config = Config::default();
        config.scan.show_files = true;

        let toml = serialize_toml(&tree, &config).expect("序列化应成功");
        write_file(&toml, &file_path).expect("写入应成功");

        let content = fs::read_to_string(&file_path).expect("读取失败");
        assert!(content.contains("treepp.pretty.v1"));
    }
}
