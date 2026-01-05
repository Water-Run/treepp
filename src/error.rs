//! 错误处理模块：定义全局统一错误类型
//!
//! 本模块为 tree++ 提供统一的错误类型层次结构，覆盖以下场景：
//!
//! - **CLI 解析错误**：参数格式、冲突、未知选项等
//! - **配置错误**：重导出自 `config` 模块，保持 API 一致性
//! - **扫描错误**：文件系统访问、权限、路径不存在等
//! - **匹配错误**：通配符模式语法、gitignore 规则解析等
//! - **渲染错误**：输出格式化过程中的异常
//! - **输出错误**：文件写入、序列化失败等
//!
//! 所有错误类型实现 `std::error::Error`，支持错误链追溯。
//!
//! 作者: WaterRun
//! 更新于: 2025-01-05

#![forbid(unsafe_code)]

use std::io;
use std::path::PathBuf;
use thiserror::Error;

// 重导出配置错误，保持 API 一致性
pub use crate::config::ConfigError;

// ============================================================================
// 顶层错误类型
// ============================================================================

/// tree++ 全局错误类型
///
/// 聚合所有子模块错误，作为程序主入口的统一错误返回类型。
/// 支持从各子错误类型自动转换。
///
/// # Examples
///
/// ```
/// use treepp::error::{TreeppError, CliError};
///
/// fn example_cli_error() -> Result<(), TreeppError> {
///     Err(CliError::UnknownOption {
///         option: "/Z".to_string(),
///     }.into())
/// }
///
/// let err = example_cli_error().unwrap_err();
/// assert!(err.to_string().contains("/Z"));
/// ```
#[derive(Debug, Error)]
pub enum TreeppError {
    /// CLI 解析错误
    #[error(transparent)]
    Cli(#[from] CliError),

    /// 配置验证错误
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// 扫描错误
    #[error(transparent)]
    Scan(#[from] ScanError),

    /// 匹配规则错误
    #[error(transparent)]
    Match(#[from] MatchError),

    /// 渲染错误
    #[error(transparent)]
    Render(#[from] RenderError),

    /// 输出错误
    #[error(transparent)]
    Output(#[from] OutputError),
}

/// 全局结果类型别名
pub type TreeppResult<T> = Result<T, TreeppError>;

// ============================================================================
// CLI 解析错误
// ============================================================================

/// CLI 参数解析错误
///
/// 表示命令行参数解析阶段产生的错误，包括：
/// - 未知选项
/// - 缺少必需参数值
/// - 参数值格式错误
/// - 参数冲突
///
/// # Examples
///
/// ```
/// use treepp::error::CliError;
///
/// let err = CliError::MissingValue {
///     option: "--level".to_string(),
/// };
/// assert!(err.to_string().contains("--level"));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CliError {
    /// 未知选项
    #[error("未知选项: {option}")]
    UnknownOption {
        /// 未识别的选项名
        option: String,
    },

    /// 选项缺少必需的参数值
    #[error("选项 {option} 需要一个参数值")]
    MissingValue {
        /// 选项名
        option: String,
    },

    /// 参数值格式错误
    #[error("选项 {option} 的值 '{value}' 无效: {reason}")]
    InvalidValue {
        /// 选项名
        option: String,
        /// 提供的值
        value: String,
        /// 错误原因
        reason: String,
    },

    /// 选项重复指定
    #[error("选项 {option} 重复指定")]
    DuplicateOption {
        /// 选项名
        option: String,
    },

    /// 选项之间冲突
    #[error("选项冲突: {opt_a} 与 {opt_b} 不能同时使用")]
    ConflictingOptions {
        /// 冲突选项 A
        opt_a: String,
        /// 冲突选项 B
        opt_b: String,
    },

    /// 无法解析的路径参数
    #[error("无法解析路径参数: {arg}")]
    InvalidPath {
        /// 原始参数
        arg: String,
    },

    /// 底层 clap 解析错误
    #[error("参数解析错误: {message}")]
    ParseError {
        /// 错误消息
        message: String,
    },
}

// ============================================================================
// 扫描错误
// ============================================================================

/// 目录扫描错误
///
/// 表示目录遍历过程中产生的错误。
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::error::ScanError;
///
/// let err = ScanError::PathNotFound {
///     path: PathBuf::from("C:\\nonexistent"),
/// };
/// assert!(err.to_string().contains("nonexistent"));
/// ```
#[derive(Debug, Error)]
pub enum ScanError {
    /// 路径不存在
    #[error("路径不存在: {path}")]
    PathNotFound {
        /// 不存在的路径
        path: PathBuf,
    },

    /// 路径不是目录
    #[error("路径不是目录: {path}")]
    NotADirectory {
        /// 非目录路径
        path: PathBuf,
    },

    /// 权限不足
    #[error("权限不足，无法访问: {path}")]
    PermissionDenied {
        /// 无权访问的路径
        path: PathBuf,
    },

    /// 读取目录失败
    #[error("读取目录失败: {path}")]
    ReadDirFailed {
        /// 目录路径
        path: PathBuf,
        /// 底层 IO 错误
        #[source]
        source: io::Error,
    },

    /// 获取元数据失败
    #[error("获取元数据失败: {path}")]
    MetadataFailed {
        /// 文件路径
        path: PathBuf,
        /// 底层 IO 错误
        #[source]
        source: io::Error,
    },

    /// 路径规范化失败
    #[error("路径规范化失败: {path}")]
    CanonicalizeFailed {
        /// 原始路径
        path: PathBuf,
        /// 底层 IO 错误
        #[source]
        source: io::Error,
    },

    /// walkdir 遍历错误
    #[error("目录遍历错误: {message}")]
    WalkError {
        /// 错误消息
        message: String,
        /// 相关路径（如有）
        path: Option<PathBuf>,
    },
}

impl ScanError {
    /// 从 IO 错误和路径创建适当的扫描错误
    ///
    /// 根据 IO 错误类型自动选择合适的错误变体。
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{self, ErrorKind};
    /// use std::path::PathBuf;
    /// use treepp::error::ScanError;
    ///
    /// let io_err = io::Error::new(ErrorKind::NotFound, "not found");
    /// let scan_err = ScanError::from_io_error(io_err, PathBuf::from("/missing"));
    ///
    /// assert!(matches!(scan_err, ScanError::PathNotFound { .. }));
    /// ```
    #[must_use]
    pub fn from_io_error(err: io::Error, path: PathBuf) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::PathNotFound { path },
            io::ErrorKind::PermissionDenied => Self::PermissionDenied { path },
            _ => Self::ReadDirFailed { path, source: err },
        }
    }
}

// ============================================================================
// 匹配规则错误
// ============================================================================

/// 匹配规则错误
///
/// 表示模式匹配和 gitignore 规则解析过程中的错误。
///
/// # Examples
///
/// ```
/// use treepp::error::MatchError;
///
/// let err = MatchError::InvalidPattern {
///     pattern: "[invalid".to_string(),
///     reason: "未闭合的字符类".to_string(),
/// };
/// assert!(err.to_string().contains("[invalid"));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MatchError {
    /// 无效的通配符模式
    #[error("无效的匹配模式 '{pattern}': {reason}")]
    InvalidPattern {
        /// 无效的模式字符串
        pattern: String,
        /// 错误原因
        reason: String,
    },

    /// gitignore 文件解析失败
    #[error("解析 .gitignore 失败: {path}")]
    GitignoreParseError {
        /// gitignore 文件路径
        path: PathBuf,
        /// 错误详情
        detail: String,
    },

    /// gitignore 规则构建失败
    #[error("构建 gitignore 规则失败: {reason}")]
    GitignoreBuildError {
        /// 错误原因
        reason: String,
    },
}

impl MatchError {
    /// 从 glob 模式错误创建
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::error::MatchError;
    ///
    /// let err = MatchError::from_glob_error("[bad", "unclosed bracket");
    /// assert!(matches!(err, MatchError::InvalidPattern { .. }));
    /// ```
    #[must_use]
    pub fn from_glob_error(pattern: &str, reason: &str) -> Self {
        Self::InvalidPattern {
            pattern: pattern.to_string(),
            reason: reason.to_string(),
        }
    }
}

// ============================================================================
// 渲染错误
// ============================================================================

/// 渲染错误
///
/// 表示树形结构渲染过程中的错误。
/// 此类错误较少见，主要用于处理极端情况。
///
/// # Examples
///
/// ```
/// use treepp::error::RenderError;
///
/// let err = RenderError::FormatError {
///     context: "日期格式化".to_string(),
///     detail: "时间戳超出范围".to_string(),
/// };
/// assert!(err.to_string().contains("日期格式化"));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// 格式化错误
    #[error("格式化错误 ({context}): {detail}")]
    FormatError {
        /// 错误上下文
        context: String,
        /// 错误详情
        detail: String,
    },

    /// 编码错误
    #[error("编码错误: 路径包含无效 UTF-8 字符")]
    InvalidUtf8Path {
        /// 问题路径（尽可能转换）
        path_lossy: String,
    },
}

// ============================================================================
// 输出错误
// ============================================================================

/// 输出错误
///
/// 表示结果输出过程中的错误，包括文件写入和序列化。
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::error::OutputError;
///
/// let err = OutputError::SerializationFailed {
///     format: "JSON".to_string(),
///     reason: "循环引用".to_string(),
/// };
/// assert!(err.to_string().contains("JSON"));
/// ```
#[derive(Debug, Error)]
pub enum OutputError {
    /// 文件创建失败
    #[error("无法创建输出文件: {path}")]
    FileCreateFailed {
        /// 目标文件路径
        path: PathBuf,
        /// 底层 IO 错误
        #[source]
        source: io::Error,
    },

    /// 文件写入失败
    #[error("写入文件失败: {path}")]
    WriteFailed {
        /// 目标文件路径
        path: PathBuf,
        /// 底层 IO 错误
        #[source]
        source: io::Error,
    },

    /// 序列化失败
    #[error("{format} 序列化失败: {reason}")]
    SerializationFailed {
        /// 输出格式名称
        format: String,
        /// 失败原因
        reason: String,
    },

    /// 标准输出写入失败
    #[error("写入标准输出失败")]
    StdoutFailed {
        /// 底层 IO 错误
        #[source]
        source: io::Error,
    },

    /// 输出路径无效
    #[error("输出路径无效: {path} ({reason})")]
    InvalidOutputPath {
        /// 输出路径
        path: PathBuf,
        /// 原因
        reason: String,
    },
}

impl OutputError {
    /// 创建 JSON 序列化错误
    #[must_use]
    pub fn json_error(reason: impl Into<String>) -> Self {
        Self::SerializationFailed {
            format: "JSON".to_string(),
            reason: reason.into(),
        }
    }

    /// 创建 YAML 序列化错误
    #[must_use]
    pub fn yaml_error(reason: impl Into<String>) -> Self {
        Self::SerializationFailed {
            format: "YAML".to_string(),
            reason: reason.into(),
        }
    }

    /// 创建 TOML 序列化错误
    #[must_use]
    pub fn toml_error(reason: impl Into<String>) -> Self {
        Self::SerializationFailed {
            format: "TOML".to_string(),
            reason: reason.into(),
        }
    }
}

// ============================================================================
// 便捷转换实现
// ============================================================================

impl From<io::Error> for OutputError {
    fn from(err: io::Error) -> Self {
        Self::StdoutFailed { source: err }
    }
}

impl From<walkdir::Error> for ScanError {
    fn from(err: walkdir::Error) -> Self {
        let path = err.path().map(PathBuf::from);
        if let Some(io_err) = err.io_error() {
            match io_err.kind() {
                io::ErrorKind::PermissionDenied => {
                    if let Some(p) = path {
                        return Self::PermissionDenied { path: p };
                    }
                }
                io::ErrorKind::NotFound => {
                    if let Some(p) = path {
                        return Self::PathNotFound { path: p };
                    }
                }
                _ => {}
            }
        }
        Self::WalkError {
            message: err.to_string(),
            path,
        }
    }
}

impl From<glob::PatternError> for MatchError {
    fn from(err: glob::PatternError) -> Self {
        Self::InvalidPattern {
            pattern: err.msg.to_string(),
            reason: format!("位置 {}", err.pos),
        }
    }
}

impl From<ignore::Error> for MatchError {
    fn from(err: ignore::Error) -> Self {
        Self::GitignoreBuildError {
            reason: err.to_string(),
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 将路径转换为可显示字符串
///
/// 处理可能包含无效 UTF-8 的路径，使用有损转换确保总能输出。
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use treepp::error::path_display;
///
/// let path = Path::new("C:\\Users\\test");
/// assert_eq!(path_display(path), "C:\\Users\\test");
/// ```
#[must_use]
pub fn path_display(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

/// 判断错误是否可恢复（可继续处理其他项）
///
/// 某些扫描错误（如单个文件权限不足）不应中断整个遍历。
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::error::{ScanError, is_recoverable};
///
/// let err = ScanError::PermissionDenied {
///     path: PathBuf::from("/protected"),
/// };
/// assert!(is_recoverable(&err));
///
/// let err = ScanError::PathNotFound {
///     path: PathBuf::from("/root"),
/// };
/// // 根路径不存在通常不可恢复，但此函数仅判断错误类型
/// assert!(is_recoverable(&err));
/// ```
#[must_use]
pub const fn is_recoverable(err: &ScanError) -> bool {
    matches!(
        err,
        ScanError::PermissionDenied { .. }
            | ScanError::MetadataFailed { .. }
            | ScanError::PathNotFound { .. }
    )
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    // ------------------------------------------------------------------------
    // TreeppError 转换测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_convert_cli_error_to_treepp_error() {
        let cli_err = CliError::UnknownOption {
            option: "/X".to_string(),
        };
        let treepp_err: TreeppError = cli_err.into();

        assert!(matches!(treepp_err, TreeppError::Cli(_)));
        assert!(treepp_err.to_string().contains("/X"));
    }

    #[test]
    fn should_convert_scan_error_to_treepp_error() {
        let scan_err = ScanError::PathNotFound {
            path: PathBuf::from("/missing"),
        };
        let treepp_err: TreeppError = scan_err.into();

        assert!(matches!(treepp_err, TreeppError::Scan(_)));
    }

    #[test]
    fn should_convert_match_error_to_treepp_error() {
        let match_err = MatchError::InvalidPattern {
            pattern: "[bad".to_string(),
            reason: "unclosed".to_string(),
        };
        let treepp_err: TreeppError = match_err.into();

        assert!(matches!(treepp_err, TreeppError::Match(_)));
    }

    #[test]
    fn should_convert_output_error_to_treepp_error() {
        let output_err = OutputError::SerializationFailed {
            format: "JSON".to_string(),
            reason: "test".to_string(),
        };
        let treepp_err: TreeppError = output_err.into();

        assert!(matches!(treepp_err, TreeppError::Output(_)));
    }

    // ------------------------------------------------------------------------
    // CliError 测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_format_unknown_option_error() {
        let err = CliError::UnknownOption {
            option: "--unknown".to_string(),
        };
        assert!(err.to_string().contains("--unknown"));
        assert!(err.to_string().contains("未知选项"));
    }

    #[test]
    fn should_format_missing_value_error() {
        let err = CliError::MissingValue {
            option: "--level".to_string(),
        };
        assert!(err.to_string().contains("--level"));
        assert!(err.to_string().contains("需要一个参数值"));
    }

    #[test]
    fn should_format_invalid_value_error() {
        let err = CliError::InvalidValue {
            option: "--thread".to_string(),
            value: "abc".to_string(),
            reason: "必须是正整数".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("--thread"));
        assert!(msg.contains("abc"));
        assert!(msg.contains("必须是正整数"));
    }

    #[test]
    fn should_compare_cli_errors_for_equality() {
        let err1 = CliError::UnknownOption {
            option: "/Z".to_string(),
        };
        let err2 = CliError::UnknownOption {
            option: "/Z".to_string(),
        };
        let err3 = CliError::UnknownOption {
            option: "/Y".to_string(),
        };

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    // ------------------------------------------------------------------------
    // ScanError 测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_create_scan_error_from_io_not_found() {
        let io_err = io::Error::new(ErrorKind::NotFound, "file not found");
        let path = PathBuf::from("/test/path");
        let scan_err = ScanError::from_io_error(io_err, path.clone());

        assert!(matches!(scan_err, ScanError::PathNotFound { path: p } if p == path));
    }

    #[test]
    fn should_create_scan_error_from_io_permission_denied() {
        let io_err = io::Error::new(ErrorKind::PermissionDenied, "access denied");
        let path = PathBuf::from("/protected");
        let scan_err = ScanError::from_io_error(io_err, path.clone());

        assert!(matches!(scan_err, ScanError::PermissionDenied { path: p } if p == path));
    }

    #[test]
    fn should_create_scan_error_from_io_other() {
        let io_err = io::Error::new(ErrorKind::Other, "some error");
        let path = PathBuf::from("/some/path");
        let scan_err = ScanError::from_io_error(io_err, path.clone());

        assert!(matches!(scan_err, ScanError::ReadDirFailed { path: p, .. } if p == path));
    }

    #[test]
    fn should_format_path_not_found_error() {
        let err = ScanError::PathNotFound {
            path: PathBuf::from("C:\\missing\\dir"),
        };
        let msg = err.to_string();
        assert!(msg.contains("路径不存在"));
        assert!(msg.contains("C:\\missing\\dir"));
    }

    #[test]
    fn should_format_permission_denied_error() {
        let err = ScanError::PermissionDenied {
            path: PathBuf::from("/root/secret"),
        };
        let msg = err.to_string();
        assert!(msg.contains("权限不足"));
    }

    // ------------------------------------------------------------------------
    // MatchError 测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_create_match_error_from_glob_error() {
        let err = MatchError::from_glob_error("[invalid", "未闭合的括号");

        match err {
            MatchError::InvalidPattern { pattern, reason } => {
                assert_eq!(pattern, "[invalid");
                assert!(reason.contains("未闭合的括号"));
            }
            _ => panic!("期望 InvalidPattern 变体"),
        }
    }

    #[test]
    fn should_format_invalid_pattern_error() {
        let err = MatchError::InvalidPattern {
            pattern: "**[".to_string(),
            reason: "语法错误".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("**["));
        assert!(msg.contains("无效的匹配模式"));
    }

    #[test]
    fn should_format_gitignore_parse_error() {
        let err = MatchError::GitignoreParseError {
            path: PathBuf::from(".gitignore"),
            detail: "第 5 行语法错误".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains(".gitignore"));
    }

    #[test]
    fn should_compare_match_errors_for_equality() {
        let err1 = MatchError::InvalidPattern {
            pattern: "*.txt".to_string(),
            reason: "test".to_string(),
        };
        let err2 = MatchError::InvalidPattern {
            pattern: "*.txt".to_string(),
            reason: "test".to_string(),
        };
        assert_eq!(err1, err2);
    }

    // ------------------------------------------------------------------------
    // RenderError 测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_format_render_format_error() {
        let err = RenderError::FormatError {
            context: "大小格式化".to_string(),
            detail: "数值溢出".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("大小格式化"));
        assert!(msg.contains("数值溢出"));
    }

    #[test]
    fn should_format_invalid_utf8_path_error() {
        let err = RenderError::InvalidUtf8Path {
            path_lossy: "some\u{FFFD}path".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("无效 UTF-8"));
    }

    // ------------------------------------------------------------------------
    // OutputError 测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_create_json_error() {
        let err = OutputError::json_error("无效结构");
        match err {
            OutputError::SerializationFailed { format, reason } => {
                assert_eq!(format, "JSON");
                assert!(reason.contains("无效结构"));
            }
            _ => panic!("期望 SerializationFailed 变体"),
        }
    }

    #[test]
    fn should_create_yaml_error() {
        let err = OutputError::yaml_error("缩进错误");
        match err {
            OutputError::SerializationFailed { format, reason } => {
                assert_eq!(format, "YAML");
                assert!(reason.contains("缩进错误"));
            }
            _ => panic!("期望 SerializationFailed 变体"),
        }
    }

    #[test]
    fn should_create_toml_error() {
        let err = OutputError::toml_error("键值对错误");
        match err {
            OutputError::SerializationFailed { format, reason } => {
                assert_eq!(format, "TOML");
                assert!(reason.contains("键值对错误"));
            }
            _ => panic!("期望 SerializationFailed 变体"),
        }
    }

    #[test]
    fn should_convert_io_error_to_output_error() {
        let io_err = io::Error::new(ErrorKind::BrokenPipe, "pipe broken");
        let output_err: OutputError = io_err.into();

        assert!(matches!(output_err, OutputError::StdoutFailed { .. }));
    }

    #[test]
    fn should_format_file_create_failed_error() {
        let err = OutputError::FileCreateFailed {
            path: PathBuf::from("output.json"),
            source: io::Error::new(ErrorKind::PermissionDenied, "denied"),
        };
        let msg = err.to_string();
        assert!(msg.contains("output.json"));
        assert!(msg.contains("无法创建"));
    }

    // ------------------------------------------------------------------------
    // 辅助函数测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_display_path_correctly() {
        let path = std::path::Path::new("C:\\Users\\test\\file.txt");
        let display = path_display(path);
        assert!(display.contains("test"));
        assert!(display.contains("file.txt"));
    }

    #[test]
    fn should_identify_recoverable_permission_denied() {
        let err = ScanError::PermissionDenied {
            path: PathBuf::from("/test"),
        };
        assert!(is_recoverable(&err));
    }

    #[test]
    fn should_identify_recoverable_metadata_failed() {
        let err = ScanError::MetadataFailed {
            path: PathBuf::from("/test"),
            source: io::Error::new(ErrorKind::Other, "test"),
        };
        assert!(is_recoverable(&err));
    }

    #[test]
    fn should_identify_recoverable_path_not_found() {
        let err = ScanError::PathNotFound {
            path: PathBuf::from("/test"),
        };
        assert!(is_recoverable(&err));
    }

    #[test]
    fn should_not_identify_not_a_directory_as_recoverable() {
        let err = ScanError::NotADirectory {
            path: PathBuf::from("/test"),
        };
        assert!(!is_recoverable(&err));
    }

    #[test]
    fn should_not_identify_walk_error_as_recoverable() {
        let err = ScanError::WalkError {
            message: "test error".to_string(),
            path: None,
        };
        assert!(!is_recoverable(&err));
    }

    // ------------------------------------------------------------------------
    // 错误链测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_preserve_source_in_read_dir_failed() {
        let io_err = io::Error::new(ErrorKind::Other, "underlying error");
        let err = ScanError::ReadDirFailed {
            path: PathBuf::from("/test"),
            source: io_err,
        };

        // 验证 source 可访问
        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }

    #[test]
    fn should_preserve_source_in_file_create_failed() {
        let io_err = io::Error::new(ErrorKind::PermissionDenied, "no permission");
        let err = OutputError::FileCreateFailed {
            path: PathBuf::from("test.txt"),
            source: io_err,
        };

        let source = std::error::Error::source(&err);
        assert!(source.is_some());
    }
}