//! 配置模块：定义全量 Config 及其子配置结构
//!
//! 本模块是用户意图的**单一事实来源**（Single Source of Truth）。
//! 所有命令行参数经 CLI 层解析后，统一转换为 `Config` 结构，
//! 后续扫描、匹配、渲染、输出各层仅依赖此配置，不再直接访问原始参数。
//!
//! 作者: WaterRun
//! 更新于: 2025-01-06

#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::path::PathBuf;
use thiserror::Error;

// ============================================================================
// 错误类型
// ============================================================================

/// 配置验证错误
///
/// 表示用户输入的参数组合不合法或无法满足运行条件时产生的错误。
///
/// # Examples
///
/// ```
/// use treepp::config::ConfigError;
///
/// let err = ConfigError::ConflictingOptions {
///     opt_a: "--include".to_string(),
///     opt_b: "--exclude".to_string(),
///     reason: "不能同时指定包含和排除同一模式".to_string(),
/// };
/// assert!(err.to_string().contains("--include"));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// 选项之间存在冲突
    #[error("选项冲突: {opt_a} 与 {opt_b} 不能同时使用 ({reason})")]
    ConflictingOptions {
        /// 冲突选项 A
        opt_a: String,
        /// 冲突选项 B
        opt_b: String,
        /// 冲突原因
        reason: String,
    },

    /// 参数值无效
    #[error("无效参数值: {option} = {value} ({reason})")]
    InvalidValue {
        /// 选项名称
        option: String,
        /// 提供的值
        value: String,
        /// 无效原因
        reason: String,
    },

    /// 路径不存在或不可访问
    #[error("路径无效: {path} ({reason})")]
    InvalidPath {
        /// 路径
        path: PathBuf,
        /// 原因
        reason: String,
    },

    /// 输出格式无法推导
    #[error("无法推导输出格式: {path} (支持的扩展名: .txt, .json, .yml, .yaml, .toml)")]
    UnknownOutputFormat {
        /// 输出文件路径
        path: PathBuf,
    },
}

/// 配置验证结果类型
pub type ConfigResult<T> = Result<T, ConfigError>;

// ============================================================================
// 枚举类型定义
// ============================================================================

/// 排序键
///
/// 指定目录树条目的排序依据。
///
/// # Examples
///
/// ```
/// use treepp::config::SortKey;
///
/// let key = SortKey::from_str_loose("SIZE");
/// assert_eq!(key, Some(SortKey::Size));
///
/// let key = SortKey::from_str_loose("unknown");
/// assert_eq!(key, None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// 按名称字母序排序（默认）
    #[default]
    Name,
    /// 按文件大小排序
    Size,
    /// 按最后修改时间排序
    Mtime,
    /// 按创建时间排序
    Ctime,
}

impl SortKey {
    /// 从字符串松散解析排序键（大小写不敏感）
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::SortKey;
    ///
    /// assert_eq!(SortKey::from_str_loose("name"), Some(SortKey::Name));
    /// assert_eq!(SortKey::from_str_loose("MTIME"), Some(SortKey::Mtime));
    /// assert_eq!(SortKey::from_str_loose("invalid"), None);
    /// ```
    #[must_use]
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "name" => Some(Self::Name),
            "size" => Some(Self::Size),
            "mtime" => Some(Self::Mtime),
            "ctime" => Some(Self::Ctime),
            _ => None,
        }
    }

    /// 获取所有有效的排序键名称
    #[must_use]
    pub const fn valid_keys() -> &'static [&'static str] {
        &["name", "size", "mtime", "ctime"]
    }
}

/// 输出格式
///
/// 指定结果输出的文件格式。
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use treepp::config::OutputFormat;
///
/// let format = OutputFormat::from_extension(Path::new("tree.json"));
/// assert_eq!(format, Some(OutputFormat::Json));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// 纯文本格式（默认）
    #[default]
    Txt,
    /// JSON 格式
    Json,
    /// YAML 格式
    Yaml,
    /// TOML 格式
    Toml,
}

impl OutputFormat {
    /// 从文件扩展名推导输出格式
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use treepp::config::OutputFormat;
    ///
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.txt")), Some(OutputFormat::Txt));
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.yml")), Some(OutputFormat::Yaml));
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.yaml")), Some(OutputFormat::Yaml));
    /// assert_eq!(OutputFormat::from_extension(Path::new("out.unknown")), None);
    /// ```
    #[must_use]
    pub fn from_extension(path: &std::path::Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
                "txt" => Some(Self::Txt),
                "json" => Some(Self::Json),
                "yml" | "yaml" => Some(Self::Yaml),
                "toml" => Some(Self::Toml),
                _ => None,
            })
    }

    /// 获取格式对应的默认扩展名
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Json => "json",
            Self::Yaml => "yml",
            Self::Toml => "toml",
        }
    }
}

/// 字符集模式
///
/// 控制树形符号使用 ASCII 还是 Unicode 字符。
///
/// # Examples
///
/// ```
/// use treepp::config::CharsetMode;
///
/// let mode = CharsetMode::Ascii;
/// assert_eq!(mode.branch(), "+--");
/// assert_eq!(mode.last_branch(), "\\--");
///
/// let mode = CharsetMode::Unicode;
/// assert_eq!(mode.branch(), "├─");
/// assert_eq!(mode.last_branch(), "└─");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharsetMode {
    /// 使用 Unicode 字符绘制树形（默认）
    #[default]
    Unicode,
    /// 使用 ASCII 字符绘制树形（兼容 `tree /A`）
    Ascii,
}

impl CharsetMode {
    /// 获取普通分支符号
    #[must_use]
    pub const fn branch(&self) -> &'static str {
        match self {
            Self::Unicode => "├─",
            Self::Ascii => "+--",
        }
    }

    /// 获取最后一个分支符号
    #[must_use]
    pub const fn last_branch(&self) -> &'static str {
        match self {
            Self::Unicode => "└─",
            Self::Ascii => "\\--",
        }
    }

    /// 获取纵向连接线符号
    #[must_use]
    pub const fn vertical(&self) -> &'static str {
        match self {
            Self::Unicode => "│  ",
            Self::Ascii => "|   ",
        }
    }

    /// 获取空白缩进
    #[must_use]
    pub const fn indent(&self) -> &'static str {
        match self {
            Self::Unicode => "   ",
            Self::Ascii => "    ",
        }
    }
}

/// 路径显示模式
///
/// 控制输出中显示完整路径还是相对名称。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathMode {
    /// 仅显示名称（默认）
    #[default]
    Relative,
    /// 显示完整绝对路径
    Full,
}

// ============================================================================
// 子配置结构
// ============================================================================

/// 扫描选项
///
/// 控制目录遍历行为的配置。
///
/// # Examples
///
/// ```
/// use treepp::config::ScanOptions;
///
/// let opts = ScanOptions::default();
/// assert_eq!(opts.max_depth, None);
/// assert!(opts.show_files);
/// assert_eq!(opts.thread_count.get(), 8);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanOptions {
    /// 最大递归深度（None 表示无限制）
    pub max_depth: Option<usize>,
    /// 是否显示文件（对应 `/F`）
    pub show_files: bool,
    /// 扫描线程数
    pub thread_count: NonZeroUsize,
    /// 是否遵循 `.gitignore` 规则
    pub respect_gitignore: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: None,
            show_files: false,
            // 安全：8 是非零常量
            thread_count: NonZeroUsize::new(8).expect("8 is non-zero"),
            respect_gitignore: false,
        }
    }
}

/// 匹配选项
///
/// 控制文件/目录过滤行为的配置。
///
/// # Examples
///
/// ```
/// use treepp::config::MatchOptions;
///
/// let opts = MatchOptions::default();
/// assert!(opts.include_patterns.is_empty());
/// assert!(opts.exclude_patterns.is_empty());
/// assert!(!opts.ignore_case);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MatchOptions {
    /// 包含模式列表（仅显示匹配项）
    pub include_patterns: Vec<String>,
    /// 排除模式列表（忽略匹配项）
    pub exclude_patterns: Vec<String>,
    /// 匹配时是否忽略大小写
    pub ignore_case: bool,
    /// 是否修剪空目录
    pub prune_empty: bool,
}

/// 渲染选项
///
/// 控制树形输出外观的配置。
///
/// # Examples
///
/// ```
/// use treepp::config::{RenderOptions, CharsetMode, PathMode, SortKey};
///
/// let opts = RenderOptions::default();
/// assert_eq!(opts.charset, CharsetMode::Unicode);
/// assert_eq!(opts.path_mode, PathMode::Relative);
/// assert!(!opts.show_size);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenderOptions {
    /// 字符集模式
    pub charset: CharsetMode,
    /// 路径显示模式
    pub path_mode: PathMode,
    /// 是否显示文件大小
    pub show_size: bool,
    /// 是否以人类可读格式显示大小
    pub human_readable: bool,
    /// 是否显示最后修改日期
    pub show_date: bool,
    /// 是否显示目录累计大小
    pub show_disk_usage: bool,
    /// 是否不显示树形连接线（仅缩进）
    pub no_indent: bool,
    /// 排序键
    pub sort_key: SortKey,
    /// 是否逆序排序
    pub reverse_sort: bool,
    /// 是否显示末尾统计报告
    pub show_report: bool,
    /// 是否隐藏 Windows 原生样板信息
    pub no_win_banner: bool,
    /// 是否用双引号包裹文件名
    pub quote_names: bool,
    /// 是否目录优先显示
    pub dirs_first: bool,
}

/// 输出选项
///
/// 控制结果输出方式的配置。
///
/// # Examples
///
/// ```
/// use treepp::config::OutputOptions;
///
/// let opts = OutputOptions::default();
/// assert!(opts.output_path.is_none());
/// assert!(!opts.silent);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OutputOptions {
    /// 输出文件路径（None 表示仅输出到终端）
    pub output_path: Option<PathBuf>,
    /// 输出格式（从 output_path 扩展名推导，或默认 Txt）
    pub format: OutputFormat,
    /// 是否静默（不输出到终端）
    pub silent: bool,
}

// ============================================================================
// 主配置结构
// ============================================================================

/// 全量配置
///
/// 用户意图的单一事实来源。CLI 解析后生成此结构，
/// 后续所有模块（扫描、匹配、渲染、输出）均依赖此配置运行。
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use treepp::config::Config;
///
/// let config = Config::default();
/// assert_eq!(config.root_path, PathBuf::from("."));
/// assert!(!config.scan.show_files);
/// ```
///
/// ```
/// use std::path::PathBuf;
/// use treepp::config::{Config, ScanOptions, OutputOptions, OutputFormat};
/// use std::num::NonZeroUsize;
///
/// let mut config = Config::default();
/// config.root_path = PathBuf::from("C:\\Windows");
/// config.scan.show_files = true;
/// config.scan.thread_count = NonZeroUsize::new(16).unwrap();
/// config.output.output_path = Some(PathBuf::from("tree.json"));
///
/// let validated = config.validate().expect("验证应通过");
/// assert_eq!(validated.output.format, OutputFormat::Json);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// 根路径（起始目录）
    pub root_path: PathBuf,
    /// 是否显示帮助信息
    pub show_help: bool,
    /// 是否显示版本信息
    pub show_version: bool,
    /// 扫描选项
    pub scan: ScanOptions,
    /// 匹配选项
    pub matching: MatchOptions,
    /// 渲染选项
    pub render: RenderOptions,
    /// 输出选项
    pub output: OutputOptions,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("."),
            show_help: false,
            show_version: false,
            scan: ScanOptions::default(),
            matching: MatchOptions::default(),
            render: RenderOptions::default(),
            output: OutputOptions::default(),
        }
    }
}

impl Config {
    /// 创建具有指定根路径的配置
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::config::Config;
    ///
    /// let config = Config::with_root(PathBuf::from("C:\\Users"));
    /// assert_eq!(config.root_path, PathBuf::from("C:\\Users"));
    /// ```
    #[must_use]
    pub fn with_root(root_path: PathBuf) -> Self {
        Self {
            root_path,
            ..Self::default()
        }
    }

    /// 验证配置并补齐派生字段
    ///
    /// 执行以下操作：
    /// - 检查选项冲突
    /// - 验证根路径存在性并规范化
    /// - 验证参数值合法性
    /// - 从输出路径扩展名推导输出格式
    /// - 应用默认值
    ///
    /// # Errors
    ///
    /// 返回 `ConfigError` 如果：
    /// - 选项之间存在不可调和的冲突
    /// - 根路径不存在或不是目录
    /// - 参数值无效（如未知的排序键）
    /// - 输出路径扩展名无法识别
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::config::{Config, OutputFormat};
    ///
    /// let mut config = Config::default();
    /// config.output.output_path = Some(PathBuf::from("result.json"));
    ///
    /// let validated = config.validate().unwrap();
    /// assert_eq!(validated.output.format, OutputFormat::Json);
    /// ```
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::config::{Config, ConfigError};
    ///
    /// let mut config = Config::default();
    /// config.output.output_path = Some(PathBuf::from("result.xyz"));
    ///
    /// let err = config.validate().unwrap_err();
    /// assert!(matches!(err, ConfigError::UnknownOutputFormat { .. }));
    /// ```
    pub fn validate(mut self) -> ConfigResult<Self> {
        // 1. 选项冲突检查
        self.check_conflicts()?;

        // 2. 根路径验证与规范化
        self.validate_and_canonicalize_root_path()?;

        // 3. 派生字段：从输出路径推导格式
        if let Some(ref path) = self.output.output_path {
            if let Some(format) = OutputFormat::from_extension(path) {
                self.output.format = format;
            } else {
                return Err(ConfigError::UnknownOutputFormat { path: path.clone() });
            }
        }

        // 4. 隐含依赖：human_readable 隐含 show_size
        if self.render.human_readable {
            self.render.show_size = true;
        }

        // 5. 隐含依赖：show_disk_usage 需要 show_size 语义支持
        // （但 show_disk_usage 是目录级别统计，与 show_size 不冲突）

        // 6. 线程数下限校验（NonZeroUsize 已保证 >= 1，无需额外检查）

        Ok(self)
    }

    /// 验证根路径并规范化
    ///
    /// 使用 dunce 规范化路径，避免 Windows 上的 `\\?\` 前缀问题。
    fn validate_and_canonicalize_root_path(&mut self) -> ConfigResult<()> {
        // 检查路径是否存在
        if !self.root_path.exists() {
            return Err(ConfigError::InvalidPath {
                path: self.root_path.clone(),
                reason: "路径不存在".to_string(),
            });
        }

        // 检查是否为目录
        if !self.root_path.is_dir() {
            return Err(ConfigError::InvalidPath {
                path: self.root_path.clone(),
                reason: "路径不是目录".to_string(),
            });
        }

        // 使用 dunce 规范化路径，避免 Windows 上的 \\?\ 前缀
        match dunce::canonicalize(&self.root_path) {
            Ok(canonical) => {
                self.root_path = canonical;
                Ok(())
            }
            Err(e) => Err(ConfigError::InvalidPath {
                path: self.root_path.clone(),
                reason: format!("无法规范化路径: {}", e),
            }),
        }
    }

    /// 检查选项冲突
    fn check_conflicts(&self) -> ConfigResult<()> {
        // 冲突：silent 必须配合 output_path 使用
        if self.output.silent && self.output.output_path.is_none() {
            return Err(ConfigError::ConflictingOptions {
                opt_a: "--silent".to_string(),
                opt_b: "(无 --output)".to_string(),
                reason: "静默模式必须指定输出文件，否则无任何输出".to_string(),
            });
        }

        // 冲突：no_indent 与 charset 的视觉效果说明（非阻断性，允许组合）
        // 此处不阻断，仅作为设计说明

        // 冲突：human_readable 无意义时的提示（非阻断）
        // 当 show_size 和 show_disk_usage 均为 false 时，human_readable 无效
        // validate() 会自动开启 show_size，故此处无需检查

        Ok(())
    }

    /// 判断是否为"仅信息显示"模式（帮助或版本）
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::Config;
    ///
    /// let mut config = Config::default();
    /// assert!(!config.is_info_only());
    ///
    /// config.show_help = true;
    /// assert!(config.is_info_only());
    /// ```
    #[must_use]
    pub const fn is_info_only(&self) -> bool {
        self.show_help || self.show_version
    }

    /// 判断是否需要计算文件大小信息
    ///
    /// 当 show_size、human_readable 或 show_disk_usage 任一启用时返回 true。
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::Config;
    ///
    /// let mut config = Config::default();
    /// assert!(!config.needs_size_info());
    ///
    /// config.render.show_size = true;
    /// assert!(config.needs_size_info());
    /// ```
    #[must_use]
    pub const fn needs_size_info(&self) -> bool {
        self.render.show_size || self.render.human_readable || self.render.show_disk_usage
    }

    /// 判断是否需要计算时间信息
    ///
    /// 当 show_date 启用或排序依据为时间相关键时返回 true。
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::{Config, SortKey};
    ///
    /// let mut config = Config::default();
    /// assert!(!config.needs_time_info());
    ///
    /// config.render.sort_key = SortKey::Mtime;
    /// assert!(config.needs_time_info());
    /// ```
    #[must_use]
    pub const fn needs_time_info(&self) -> bool {
        self.render.show_date
            || matches!(self.render.sort_key, SortKey::Mtime | SortKey::Ctime)
    }

    /// 判断是否应使用流式输出模式
    ///
    /// 满足以下所有条件时使用流式输出：
    /// - 输出格式为 TXT
    /// - 未指定输出文件（仅终端输出）
    /// - 未启用 disk_usage（需要完整树计算）
    /// - 非静默模式
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::config::Config;
    ///
    /// let config = Config::default();
    /// assert!(config.should_stream());
    /// ```
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use treepp::config::Config;
    ///
    /// let mut config = Config::default();
    /// config.output.output_path = Some(PathBuf::from("tree.txt"));
    /// assert!(!config.should_stream());
    /// ```
    #[must_use]
    pub fn should_stream(&self) -> bool {
        self.output.output_path.is_none()
            && self.output.format == OutputFormat::Txt
            && !self.render.show_disk_usage
            && !self.output.silent
    }
}

// ============================================================================
// 单元测试
// ============================================================================
// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // SortKey 测试
    // ------------------------------------------------------------------------

    #[test]
    fn sort_key_from_str_loose_should_parse_valid_keys() {
        assert_eq!(SortKey::from_str_loose("name"), Some(SortKey::Name));
        assert_eq!(SortKey::from_str_loose("NAME"), Some(SortKey::Name));
        assert_eq!(SortKey::from_str_loose("Name"), Some(SortKey::Name));
        assert_eq!(SortKey::from_str_loose("size"), Some(SortKey::Size));
        assert_eq!(SortKey::from_str_loose("SIZE"), Some(SortKey::Size));
        assert_eq!(SortKey::from_str_loose("mtime"), Some(SortKey::Mtime));
        assert_eq!(SortKey::from_str_loose("MTIME"), Some(SortKey::Mtime));
        assert_eq!(SortKey::from_str_loose("ctime"), Some(SortKey::Ctime));
        assert_eq!(SortKey::from_str_loose("CTIME"), Some(SortKey::Ctime));
    }

    #[test]
    fn sort_key_from_str_loose_should_return_none_for_invalid_keys() {
        assert_eq!(SortKey::from_str_loose(""), None);
        assert_eq!(SortKey::from_str_loose("invalid"), None);
        assert_eq!(SortKey::from_str_loose("date"), None);
        assert_eq!(SortKey::from_str_loose("time"), None);
    }

    #[test]
    fn sort_key_default_should_be_name() {
        assert_eq!(SortKey::default(), SortKey::Name);
    }

    #[test]
    fn sort_key_valid_keys_should_contain_all_variants() {
        let keys = SortKey::valid_keys();
        assert!(keys.contains(&"name"));
        assert!(keys.contains(&"size"));
        assert!(keys.contains(&"mtime"));
        assert!(keys.contains(&"ctime"));
        assert_eq!(keys.len(), 4);
    }

    // ------------------------------------------------------------------------
    // OutputFormat 测试
    // ------------------------------------------------------------------------

    #[test]
    fn output_format_from_extension_should_recognize_valid_extensions() {
        use std::path::Path;

        assert_eq!(
            OutputFormat::from_extension(Path::new("file.txt")),
            Some(OutputFormat::Txt)
        );
        assert_eq!(
            OutputFormat::from_extension(Path::new("file.TXT")),
            Some(OutputFormat::Txt)
        );
        assert_eq!(
            OutputFormat::from_extension(Path::new("file.json")),
            Some(OutputFormat::Json)
        );
        assert_eq!(
            OutputFormat::from_extension(Path::new("file.JSON")),
            Some(OutputFormat::Json)
        );
        assert_eq!(
            OutputFormat::from_extension(Path::new("file.yml")),
            Some(OutputFormat::Yaml)
        );
        assert_eq!(
            OutputFormat::from_extension(Path::new("file.yaml")),
            Some(OutputFormat::Yaml)
        );
        assert_eq!(
            OutputFormat::from_extension(Path::new("file.YAML")),
            Some(OutputFormat::Yaml)
        );
        assert_eq!(
            OutputFormat::from_extension(Path::new("file.toml")),
            Some(OutputFormat::Toml)
        );
        assert_eq!(
            OutputFormat::from_extension(Path::new("file.TOML")),
            Some(OutputFormat::Toml)
        );
    }

    #[test]
    fn output_format_from_extension_should_return_none_for_unknown() {
        use std::path::Path;

        assert_eq!(OutputFormat::from_extension(Path::new("file.xyz")), None);
        assert_eq!(OutputFormat::from_extension(Path::new("file")), None);
        assert_eq!(OutputFormat::from_extension(Path::new("")), None);
        assert_eq!(OutputFormat::from_extension(Path::new("file.md")), None);
    }

    #[test]
    fn output_format_extension_should_return_correct_string() {
        assert_eq!(OutputFormat::Txt.extension(), "txt");
        assert_eq!(OutputFormat::Json.extension(), "json");
        assert_eq!(OutputFormat::Yaml.extension(), "yml");
        assert_eq!(OutputFormat::Toml.extension(), "toml");
    }

    #[test]
    fn output_format_default_should_be_txt() {
        assert_eq!(OutputFormat::default(), OutputFormat::Txt);
    }

    // ------------------------------------------------------------------------
    // CharsetMode 测试
    // ------------------------------------------------------------------------

    #[test]
    fn charset_mode_unicode_should_return_unicode_symbols() {
        let mode = CharsetMode::Unicode;
        assert_eq!(mode.branch(), "├─");
        assert_eq!(mode.last_branch(), "└─");
        assert_eq!(mode.vertical(), "│  ");
        assert_eq!(mode.indent(), "   ");
    }

    #[test]
    fn charset_mode_ascii_should_return_ascii_symbols() {
        let mode = CharsetMode::Ascii;
        assert_eq!(mode.branch(), "+--");
        assert_eq!(mode.last_branch(), "\\--");
        assert_eq!(mode.vertical(), "|   ");
        assert_eq!(mode.indent(), "    ");
    }

    #[test]
    fn charset_mode_default_should_be_unicode() {
        assert_eq!(CharsetMode::default(), CharsetMode::Unicode);
    }

    // ------------------------------------------------------------------------
    // PathMode 测试
    // ------------------------------------------------------------------------

    #[test]
    fn path_mode_default_should_be_relative() {
        assert_eq!(PathMode::default(), PathMode::Relative);
    }

    // ------------------------------------------------------------------------
    // ScanOptions 测试
    // ------------------------------------------------------------------------

    #[test]
    fn scan_options_default_should_have_expected_values() {
        let opts = ScanOptions::default();
        assert_eq!(opts.max_depth, None);
        assert!(!opts.show_files);
        assert_eq!(opts.thread_count.get(), 8);
        assert!(!opts.respect_gitignore);
    }

    // ------------------------------------------------------------------------
    // MatchOptions 测试
    // ------------------------------------------------------------------------

    #[test]
    fn match_options_default_should_be_empty() {
        let opts = MatchOptions::default();
        assert!(opts.include_patterns.is_empty());
        assert!(opts.exclude_patterns.is_empty());
        assert!(!opts.ignore_case);
        assert!(!opts.prune_empty);
    }

    // ------------------------------------------------------------------------
    // RenderOptions 测试
    // ------------------------------------------------------------------------

    #[test]
    fn render_options_default_should_have_expected_values() {
        let opts = RenderOptions::default();
        assert_eq!(opts.charset, CharsetMode::Unicode);
        assert_eq!(opts.path_mode, PathMode::Relative);
        assert!(!opts.show_size);
        assert!(!opts.human_readable);
        assert!(!opts.show_date);
        assert!(!opts.show_disk_usage);
        assert!(!opts.no_indent);
        assert_eq!(opts.sort_key, SortKey::Name);
        assert!(!opts.reverse_sort);
        assert!(!opts.show_report);
        assert!(!opts.no_win_banner);
        assert!(!opts.quote_names);
        assert!(!opts.dirs_first);
    }

    // ------------------------------------------------------------------------
    // OutputOptions 测试
    // ------------------------------------------------------------------------

    #[test]
    fn output_options_default_should_have_expected_values() {
        let opts = OutputOptions::default();
        assert!(opts.output_path.is_none());
        assert_eq!(opts.format, OutputFormat::Txt);
        assert!(!opts.silent);
    }

    // ------------------------------------------------------------------------
    // Config 基本测试
    // ------------------------------------------------------------------------

    #[test]
    fn config_default_should_have_expected_values() {
        let config = Config::default();
        assert_eq!(config.root_path, PathBuf::from("."));
        assert!(!config.show_help);
        assert!(!config.show_version);
    }

    #[test]
    fn config_with_root_should_set_root_path() {
        let config = Config::with_root(PathBuf::from("/some/path"));
        assert_eq!(config.root_path, PathBuf::from("/some/path"));
        assert!(!config.show_help);
        assert!(!config.show_version);
    }

    #[test]
    fn config_is_info_only_should_return_true_for_help() {
        let mut config = Config::default();
        assert!(!config.is_info_only());

        config.show_help = true;
        assert!(config.is_info_only());
    }

    #[test]
    fn config_is_info_only_should_return_true_for_version() {
        let mut config = Config::default();
        config.show_version = true;
        assert!(config.is_info_only());
    }

    #[test]
    fn config_is_info_only_should_return_true_for_both() {
        let mut config = Config::default();
        config.show_help = true;
        config.show_version = true;
        assert!(config.is_info_only());
    }

    // ------------------------------------------------------------------------
    // Config::needs_size_info 测试
    // ------------------------------------------------------------------------

    #[test]
    fn config_needs_size_info_should_return_false_by_default() {
        let config = Config::default();
        assert!(!config.needs_size_info());
    }

    #[test]
    fn config_needs_size_info_should_return_true_when_show_size() {
        let mut config = Config::default();
        config.render.show_size = true;
        assert!(config.needs_size_info());
    }

    #[test]
    fn config_needs_size_info_should_return_true_when_human_readable() {
        let mut config = Config::default();
        config.render.human_readable = true;
        assert!(config.needs_size_info());
    }

    #[test]
    fn config_needs_size_info_should_return_true_when_show_disk_usage() {
        let mut config = Config::default();
        config.render.show_disk_usage = true;
        assert!(config.needs_size_info());
    }

    // ------------------------------------------------------------------------
    // Config::needs_time_info 测试
    // ------------------------------------------------------------------------

    #[test]
    fn config_needs_time_info_should_return_false_by_default() {
        let config = Config::default();
        assert!(!config.needs_time_info());
    }

    #[test]
    fn config_needs_time_info_should_return_true_when_show_date() {
        let mut config = Config::default();
        config.render.show_date = true;
        assert!(config.needs_time_info());
    }

    #[test]
    fn config_needs_time_info_should_return_true_when_sort_by_mtime() {
        let mut config = Config::default();
        config.render.sort_key = SortKey::Mtime;
        assert!(config.needs_time_info());
    }

    #[test]
    fn config_needs_time_info_should_return_true_when_sort_by_ctime() {
        let mut config = Config::default();
        config.render.sort_key = SortKey::Ctime;
        assert!(config.needs_time_info());
    }

    #[test]
    fn config_needs_time_info_should_return_false_when_sort_by_name() {
        let mut config = Config::default();
        config.render.sort_key = SortKey::Name;
        assert!(!config.needs_time_info());
    }

    #[test]
    fn config_needs_time_info_should_return_false_when_sort_by_size() {
        let mut config = Config::default();
        config.render.sort_key = SortKey::Size;
        assert!(!config.needs_time_info());
    }

    // ------------------------------------------------------------------------
    // Config::should_stream 测试
    // ------------------------------------------------------------------------

    #[test]
    fn config_should_stream_should_return_true_by_default() {
        let config = Config::default();
        assert!(config.should_stream());
    }

    #[test]
    fn config_should_stream_should_return_false_when_output_path_set() {
        let mut config = Config::default();
        config.output.output_path = Some(PathBuf::from("output.txt"));
        assert!(!config.should_stream());
    }

    #[test]
    fn config_should_stream_should_return_false_when_format_is_json() {
        let mut config = Config::default();
        config.output.format = OutputFormat::Json;
        assert!(!config.should_stream());
    }

    #[test]
    fn config_should_stream_should_return_false_when_show_disk_usage() {
        let mut config = Config::default();
        config.render.show_disk_usage = true;
        assert!(!config.should_stream());
    }

    #[test]
    fn config_should_stream_should_return_false_when_silent() {
        let mut config = Config::default();
        config.output.silent = true;
        assert!(!config.should_stream());
    }

    // ------------------------------------------------------------------------
    // Config::validate 测试
    // ------------------------------------------------------------------------

    #[test]
    fn config_validate_should_fail_for_nonexistent_path() {
        let config = Config::with_root(PathBuf::from("/nonexistent/path/that/does/not/exist"));
        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::InvalidPath { path, reason } => {
                assert_eq!(path, PathBuf::from("/nonexistent/path/that/does/not/exist"));
                assert!(reason.contains("不存在"));
            }
            _ => panic!("应返回 InvalidPath 错误"),
        }
    }

    #[test]
    fn config_validate_should_fail_for_file_as_root() {
        // 使用 Cargo.toml 作为测试文件（它肯定存在于项目根目录）
        let config = Config::with_root(PathBuf::from("Cargo.toml"));
        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::InvalidPath { reason, .. } => {
                assert!(reason.contains("不是目录"));
            }
            _ => panic!("应返回 InvalidPath 错误"),
        }
    }

    #[test]
    fn config_validate_should_succeed_for_current_directory() {
        let config = Config::default();
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn config_validate_should_canonicalize_path() {
        let config = Config::with_root(PathBuf::from("."));
        let validated = config.validate().unwrap();
        // 规范化后的路径应该是绝对路径
        assert!(validated.root_path.is_absolute());
    }

    #[test]
    fn config_validate_should_infer_json_format_from_extension() {
        let mut config = Config::default();
        config.output.output_path = Some(PathBuf::from("tree.json"));
        let validated = config.validate().unwrap();
        assert_eq!(validated.output.format, OutputFormat::Json);
    }

    #[test]
    fn config_validate_should_infer_yaml_format_from_yml_extension() {
        let mut config = Config::default();
        config.output.output_path = Some(PathBuf::from("tree.yml"));
        let validated = config.validate().unwrap();
        assert_eq!(validated.output.format, OutputFormat::Yaml);
    }

    #[test]
    fn config_validate_should_infer_yaml_format_from_yaml_extension() {
        let mut config = Config::default();
        config.output.output_path = Some(PathBuf::from("tree.yaml"));
        let validated = config.validate().unwrap();
        assert_eq!(validated.output.format, OutputFormat::Yaml);
    }

    #[test]
    fn config_validate_should_infer_toml_format_from_extension() {
        let mut config = Config::default();
        config.output.output_path = Some(PathBuf::from("tree.toml"));
        let validated = config.validate().unwrap();
        assert_eq!(validated.output.format, OutputFormat::Toml);
    }

    #[test]
    fn config_validate_should_infer_txt_format_from_extension() {
        let mut config = Config::default();
        config.output.output_path = Some(PathBuf::from("tree.txt"));
        let validated = config.validate().unwrap();
        assert_eq!(validated.output.format, OutputFormat::Txt);
    }

    #[test]
    fn config_validate_should_fail_for_unknown_extension() {
        let mut config = Config::default();
        config.output.output_path = Some(PathBuf::from("tree.xyz"));
        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::UnknownOutputFormat { path } => {
                assert_eq!(path, PathBuf::from("tree.xyz"));
            }
            _ => panic!("应返回 UnknownOutputFormat 错误"),
        }
    }

    #[test]
    fn config_validate_should_fail_for_silent_without_output() {
        let mut config = Config::default();
        config.output.silent = true;
        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ConflictingOptions { opt_a, opt_b, .. } => {
                assert!(opt_a.contains("silent"));
                assert!(opt_b.contains("output"));
            }
            _ => panic!("应返回 ConflictingOptions 错误"),
        }
    }

    #[test]
    fn config_validate_should_succeed_for_silent_with_output() {
        let mut config = Config::default();
        config.output.silent = true;
        config.output.output_path = Some(PathBuf::from("tree.txt"));
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn config_validate_should_enable_show_size_when_human_readable() {
        let mut config = Config::default();
        config.render.human_readable = true;
        config.render.show_size = false;
        let validated = config.validate().unwrap();
        assert!(validated.render.show_size);
    }

    // ------------------------------------------------------------------------
    // ConfigError 测试
    // ------------------------------------------------------------------------

    #[test]
    fn config_error_conflicting_options_should_display_correctly() {
        let err = ConfigError::ConflictingOptions {
            opt_a: "--foo".to_string(),
            opt_b: "--bar".to_string(),
            reason: "互斥选项".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("--foo"));
        assert!(msg.contains("--bar"));
        assert!(msg.contains("互斥选项"));
    }

    #[test]
    fn config_error_invalid_value_should_display_correctly() {
        let err = ConfigError::InvalidValue {
            option: "--depth".to_string(),
            value: "-1".to_string(),
            reason: "必须为正整数".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("--depth"));
        assert!(msg.contains("-1"));
        assert!(msg.contains("必须为正整数"));
    }

    #[test]
    fn config_error_invalid_path_should_display_correctly() {
        let err = ConfigError::InvalidPath {
            path: PathBuf::from("/invalid/path"),
            reason: "路径不存在".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/invalid/path") || msg.contains("\\invalid\\path"));
        assert!(msg.contains("路径不存在"));
    }

    #[test]
    fn config_error_unknown_output_format_should_display_correctly() {
        let err = ConfigError::UnknownOutputFormat {
            path: PathBuf::from("output.xyz"),
        };
        let msg = err.to_string();
        assert!(msg.contains("output.xyz"));
        assert!(msg.contains(".txt"));
        assert!(msg.contains(".json"));
    }

    #[test]
    fn config_error_should_be_clone_and_eq() {
        let err1 = ConfigError::ConflictingOptions {
            opt_a: "a".to_string(),
            opt_b: "b".to_string(),
            reason: "r".to_string(),
        };
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // ------------------------------------------------------------------------
    // 边界条件测试
    // ------------------------------------------------------------------------

    #[test]
    fn config_with_all_options_enabled_should_validate() {
        let mut config = Config::default();
        config.scan.show_files = true;
        config.scan.max_depth = Some(10);
        config.scan.respect_gitignore = true;
        config.matching.include_patterns = vec!["*.rs".to_string()];
        config.matching.exclude_patterns = vec!["target".to_string()];
        config.matching.ignore_case = true;
        config.matching.prune_empty = true;
        config.render.charset = CharsetMode::Ascii;
        config.render.path_mode = PathMode::Full;
        config.render.show_size = true;
        config.render.human_readable = true;
        config.render.show_date = true;
        config.render.show_disk_usage = true;
        config.render.sort_key = SortKey::Mtime;
        config.render.reverse_sort = true;
        config.render.show_report = true;
        config.render.dirs_first = true;
        config.render.quote_names = true;
        config.output.output_path = Some(PathBuf::from("tree.json"));

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn scan_options_thread_count_should_be_non_zero() {
        let opts = ScanOptions::default();
        assert!(opts.thread_count.get() > 0);
    }

    #[test]
    fn config_clone_should_produce_equal_copy() {
        let mut config = Config::default();
        config.scan.show_files = true;
        config.render.show_size = true;

        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn config_debug_should_be_implemented() {
        let config = Config::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("root_path"));
    }
}