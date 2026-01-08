//! 命令行参数解析模块
//!
//! 本模块实现 `tree++` 命令行工具的参数解析功能，支持三种参数风格混用：
//!
//! - Windows CMD 风格 (`/F`)，大小写不敏感
//! - Unix 短参数风格 (`-f`)，大小写敏感
//! - GNU 长参数风格 (`--files`)，大小写敏感
//!
//! 解析完成后产出 [`Config`] 结构体，供后续扫描、匹配、渲染、输出模块使用。
//!
//! # 示例
//!
//! ```no_run
//! use treepp::cli::{CliParser, ParseResult};
//!
//! let args = vec!["D:\\project".to_string(), "/F".to_string(), "--ascii".to_string()];
//! let parser = CliParser::new(args);
//! match parser.parse() {
//!     Ok(ParseResult::Config(config)) => println!("{:?}", config),
//!     Ok(ParseResult::Help) => println!("显示帮助"),
//!     Ok(ParseResult::Version) => println!("显示版本"),
//!     Err(e) => eprintln!("错误: {}", e),
//! }
//! ```
//!
//! 文件: src/cli.rs
//! 作者: WaterRun
//! 更新于: 2026-01-06

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::env;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use crate::config::{CharsetMode, Config, PathMode};
pub(crate) use crate::error::CliError;

// ============================================================================
// 解析结果枚举
// ============================================================================

/// 解析结果
///
/// 表示命令行解析后的三种可能结果。
///
/// # Examples
///
/// ```no_run
/// use treepp::cli::{CliParser, ParseResult};
///
/// let parser = CliParser::new(vec!["--help".to_string()]);
/// match parser.parse() {
///     Ok(ParseResult::Help) => println!("显示帮助"),
///     Ok(ParseResult::Version) => println!("显示版本"),
///     Ok(ParseResult::Config(c)) => println!("配置: {:?}", c),
///     Err(e) => eprintln!("错误: {}", e),
/// }
/// ```
#[derive(Debug)]
pub enum ParseResult {
    /// 正常配置，需要执行扫描
    Config(Config),
    /// 用户请求显示帮助信息
    Help,
    /// 用户请求显示版本信息
    Version,
}

// ============================================================================
// 参数定义
// ============================================================================

/// 参数类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    /// 标志型参数（无需值）
    Flag,
    /// 带值型参数（需要后跟一个值）
    Value,
}

/// 参数定义结构体
struct ArgDef {
    /// 规范名称（用于重复检测和错误消息）
    canonical: &'static str,
    /// 参数类型
    kind: ArgKind,
    /// Windows CMD 风格 (`/X`)，大小写不敏感
    cmd_patterns: &'static [&'static str],
    /// Unix 短参数 (`-x`)，大小写敏感
    short_patterns: &'static [&'static str],
    /// GNU 长参数 (`--xxx`)，大小写敏感
    long_patterns: &'static [&'static str],
}

/// 所有支持的参数定义
///
/// 参数定义顺序与文档保持一致，便于维护。
const ARG_DEFINITIONS: &[ArgDef] = &[
    // 信息显示类
    ArgDef {
        canonical: "help",
        kind: ArgKind::Flag,
        cmd_patterns: &["/?"],
        short_patterns: &["-h"],
        long_patterns: &["--help"],
    },
    ArgDef {
        canonical: "version",
        kind: ArgKind::Flag,
        cmd_patterns: &["/V"],
        short_patterns: &["-v"],
        long_patterns: &["--version"],
    },
    // 显示内容类
    ArgDef {
        canonical: "files",
        kind: ArgKind::Flag,
        cmd_patterns: &["/F"],
        short_patterns: &["-f"],
        long_patterns: &["--files"],
    },
    ArgDef {
        canonical: "full-path",
        kind: ArgKind::Flag,
        cmd_patterns: &["/FP"],
        short_patterns: &["-p"],
        long_patterns: &["--full-path"],
    },
    ArgDef {
        canonical: "size",
        kind: ArgKind::Flag,
        cmd_patterns: &["/S"],
        short_patterns: &["-s"],
        long_patterns: &["--size"],
    },
    ArgDef {
        canonical: "human-readable",
        kind: ArgKind::Flag,
        cmd_patterns: &["/HR"],
        short_patterns: &["-H"],
        long_patterns: &["--human-readable"],
    },
    ArgDef {
        canonical: "date",
        kind: ArgKind::Flag,
        cmd_patterns: &["/DT"],
        short_patterns: &["-d"],
        long_patterns: &["--date"],
    },
    ArgDef {
        canonical: "disk-usage",
        kind: ArgKind::Flag,
        cmd_patterns: &["/DU"],
        short_patterns: &["-u"],
        long_patterns: &["--disk-usage"],
    },
    // 渲染样式类
    ArgDef {
        canonical: "ascii",
        kind: ArgKind::Flag,
        cmd_patterns: &["/A"],
        short_patterns: &["-a"],
        long_patterns: &["--ascii"],
    },
    ArgDef {
        canonical: "no-indent",
        kind: ArgKind::Flag,
        cmd_patterns: &["/NI"],
        short_patterns: &["-i"],
        long_patterns: &["--no-indent"],
    },
    ArgDef {
        canonical: "reverse",
        kind: ArgKind::Flag,
        cmd_patterns: &["/R"],
        short_patterns: &["-r"],
        long_patterns: &["--reverse"],
    },
    // 过滤类
    ArgDef {
        canonical: "level",
        kind: ArgKind::Value,
        cmd_patterns: &["/L"],
        short_patterns: &["-L"],
        long_patterns: &["--level"],
    },
    ArgDef {
        canonical: "include",
        kind: ArgKind::Value,
        cmd_patterns: &["/M"],
        short_patterns: &["-m"],
        long_patterns: &["--include"],
    },
    ArgDef {
        canonical: "exclude",
        kind: ArgKind::Value,
        cmd_patterns: &["/X"],
        short_patterns: &["-I"],
        long_patterns: &["--exclude"],
    },
    ArgDef {
        canonical: "prune",
        kind: ArgKind::Flag,
        cmd_patterns: &["/P"],
        short_patterns: &["-P"],
        long_patterns: &["--prune"],
    },
    ArgDef {
        canonical: "gitignore",
        kind: ArgKind::Flag,
        cmd_patterns: &["/G"],
        short_patterns: &["-g"],
        long_patterns: &["--gitignore"],
    },
    // 输出控制类
    ArgDef {
        canonical: "report",
        kind: ArgKind::Flag,
        cmd_patterns: &["/RP"],
        short_patterns: &["-e"],
        long_patterns: &["--report"],
    },
    ArgDef {
        canonical: "no-win-banner",
        kind: ArgKind::Flag,
        cmd_patterns: &["/NB"],
        short_patterns: &["-N"],
        long_patterns: &["--no-win-banner"],
    },
    ArgDef {
        canonical: "silent",
        kind: ArgKind::Flag,
        cmd_patterns: &["/SI"],
        short_patterns: &["-l"],
        long_patterns: &["--silent"],
    },
    ArgDef {
        canonical: "output",
        kind: ArgKind::Value,
        cmd_patterns: &["/O"],
        short_patterns: &["-o"],
        long_patterns: &["--output"],
    },
    // 性能类
    ArgDef {
        canonical: "thread",
        kind: ArgKind::Value,
        cmd_patterns: &["/T"],
        short_patterns: &["-t"],
        long_patterns: &["--thread"],
    },
];

// ============================================================================
// 参数匹配结果
// ============================================================================

/// 参数匹配结果
struct MatchedArg {
    /// 匹配到的参数定义
    definition: &'static ArgDef,
    /// 参数值（如果是带值参数）
    value: Option<String>,
}

// ============================================================================
// 命令行解析器
// ============================================================================

/// 命令行参数解析器
///
/// 支持三种参数风格混用：
/// - Windows CMD 风格 (`/F`)，大小写不敏感
/// - Unix 短参数风格 (`-f`)，大小写敏感
/// - GNU 长参数风格 (`--files`)，大小写敏感
///
/// # 路径位置规则
///
/// 路径参数可以出现在任意位置，包括选项之前、之后或之间。
///
/// # Examples
///
/// ```no_run
/// use treepp::cli::CliParser;
///
/// // 从命令行参数创建
/// let parser = CliParser::from_env();
///
/// // 或手动指定参数
/// let parser = CliParser::new(vec!["/F".to_string(), "--ascii".to_string()]);
/// ```
pub struct CliParser {
    /// 待解析的参数列表
    args: Vec<String>,
    /// 当前解析位置
    position: usize,
    /// 已使用的规范名称集合（用于重复检测）
    seen_canonical_names: HashSet<String>,
}

impl CliParser {
    /// 从参数列表创建解析器
    ///
    /// # 参数
    ///
    /// * `args` - 命令行参数列表（不包含程序名）
    ///
    /// # Examples
    ///
    /// ```
    /// use treepp::cli::CliParser;
    ///
    /// let parser = CliParser::new(vec!["/F".to_string(), "--level".to_string(), "3".to_string()]);
    /// ```
    #[must_use]
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            position: 0,
            seen_canonical_names: HashSet::new(),
        }
    }

    /// 从环境参数创建解析器
    ///
    /// 自动跳过程序名（第一个参数）。
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::cli::CliParser;
    ///
    /// let parser = CliParser::from_env();
    /// let result = parser.parse();
    /// ```
    #[must_use]
    pub fn from_env() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        Self::new(args)
    }

    /// 解析命令行参数
    ///
    /// 解析完成后调用 `Config::validate()` 验证配置有效性。
    ///
    /// # 路径位置规则
    ///
    /// 路径参数可以出现在任意位置。例如：
    /// - `treepp C:\dir /F` ✓ 正确
    /// - `treepp /F C:\dir` ✓ 正确
    /// - `treepp /F C:\dir --ascii` ✓ 正确
    ///
    /// # 返回值
    ///
    /// 成功返回 `ParseResult`，失败返回 `CliError`。
    ///
    /// # Errors
    ///
    /// - `CliError::UnknownOption` - 遇到未知参数
    /// - `CliError::MissingValue` - 需要值的参数缺少值
    /// - `CliError::InvalidValue` - 参数值无效
    /// - `CliError::DuplicateOption` - 参数重复
    /// - `CliError::MultiplePaths` - 指定了多个路径
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use treepp::cli::{CliParser, ParseResult};
    ///
    /// let parser = CliParser::new(vec!["/F".to_string()]);
    /// match parser.parse() {
    ///     Ok(ParseResult::Config(config)) => {
    ///         assert!(config.scan.show_files);
    ///     }
    ///     _ => panic!("解析失败"),
    /// }
    /// ```
    pub fn parse(mut self) -> Result<ParseResult, CliError> {
        let mut config = Config::default();
        let mut collected_paths: Vec<String> = Vec::new();

        // 可累积的参数（允许多次指定）
        const ACCUMULATIVE_OPTIONS: &[&str] = &["include", "exclude"];

        while self.position < self.args.len() {
            let current_arg = self.args[self.position].clone();

            if Self::is_option_like(&current_arg) {
                let matched = self.try_match_argument(&current_arg)?;

                // 非累积参数才检查重复
                if !ACCUMULATIVE_OPTIONS.contains(&matched.definition.canonical) {
                    self.register_canonical_name(matched.definition.canonical)?;
                }

                self.apply_to_config(&mut config, &matched)?;

                // 帮助和版本信息立即返回
                if matched.definition.canonical == "help" {
                    return Ok(ParseResult::Help);
                }
                if matched.definition.canonical == "version" {
                    return Ok(ParseResult::Version);
                }
            } else {
                // 非选项参数视为路径，直接收集
                collected_paths.push(current_arg);
            }

            self.position += 1;
        }

        // 验证并设置路径
        self.validate_paths(&collected_paths, &mut config)?;

        // 调用 Config::validate() 进行配置验证
        let validated_config = config.validate().map_err(|e| CliError::ParseError {
            message: e.to_string(),
        })?;

        Ok(ParseResult::Config(validated_config))
    }

    /// 判断字符串是否看起来像选项参数
    fn is_option_like(arg: &str) -> bool {
        arg.starts_with('-') || arg.starts_with('/')
    }

    /// 尝试匹配参数到已知定义
    fn try_match_argument(&mut self, arg: &str) -> Result<MatchedArg, CliError> {
        for def in ARG_DEFINITIONS {
            if let Some(matched) = self.try_match_definition(arg, def)? {
                return Ok(matched);
            }
        }
        Err(CliError::UnknownOption {
            option: arg.to_string(),
        })
    }

    /// 尝试将参数与特定定义匹配
    fn try_match_definition(
        &mut self,
        arg: &str,
        def: &'static ArgDef,
    ) -> Result<Option<MatchedArg>, CliError> {
        let arg_upper = arg.to_uppercase();

        // CMD 风格匹配（大小写不敏感）
        for pattern in def.cmd_patterns {
            if arg_upper == pattern.to_uppercase() {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }
        }

        // Unix 短参数匹配（大小写敏感）
        for pattern in def.short_patterns {
            if arg == *pattern {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }
        }

        // GNU 长参数匹配（大小写敏感）
        for pattern in def.long_patterns {
            if arg == *pattern {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }

            // 支持 --option=value 语法
            let equals_prefix = format!("{pattern}=");
            if arg.starts_with(&equals_prefix) && def.kind == ArgKind::Value {
                let value = arg[equals_prefix.len()..].to_string();
                return Ok(Some(MatchedArg {
                    definition: def,
                    value: Some(value),
                }));
            }
        }

        Ok(None)
    }

    /// 如果参数需要值，消费下一个参数作为值
    fn consume_value_if_required(
        &mut self,
        def: &ArgDef,
        arg: &str,
    ) -> Result<Option<String>, CliError> {
        if def.kind == ArgKind::Flag {
            return Ok(None);
        }

        let next_position = self.position + 1;
        if next_position >= self.args.len() {
            return Err(CliError::MissingValue {
                option: arg.to_string(),
            });
        }

        let next_arg = &self.args[next_position];
        if Self::is_option_like(next_arg) {
            return Err(CliError::MissingValue {
                option: arg.to_string(),
            });
        }

        self.position += 1;
        Ok(Some(next_arg.clone()))
    }

    /// 注册已使用的规范名称，检测重复
    fn register_canonical_name(&mut self, canonical: &str) -> Result<(), CliError> {
        if !self.seen_canonical_names.insert(canonical.to_string()) {
            return Err(CliError::DuplicateOption {
                option: canonical.to_string(),
            });
        }
        Ok(())
    }

    /// 将匹配的参数应用到配置
    fn apply_to_config(&self, config: &mut Config, matched: &MatchedArg) -> Result<(), CliError> {
        let canonical = matched.definition.canonical;

        match canonical {
            // 信息显示类
            "help" => config.show_help = true,
            "version" => config.show_version = true,

            // 扫描选项
            "files" => config.scan.show_files = true,
            "gitignore" => config.scan.respect_gitignore = true,
            "level" => {
                let value = matched.value.as_ref().expect("level 参数需要值");
                let depth: usize = value.parse().map_err(|_| CliError::InvalidValue {
                    option: canonical.to_string(),
                    value: value.clone(),
                    reason: "必须是正整数".to_string(),
                })?;
                config.scan.max_depth = Some(depth);
            }
            "thread" => {
                let value = matched.value.as_ref().expect("thread 参数需要值");
                let count: usize = value.parse().map_err(|_| CliError::InvalidValue {
                    option: canonical.to_string(),
                    value: value.clone(),
                    reason: "必须是正整数".to_string(),
                })?;
                config.scan.thread_count =
                    NonZeroUsize::new(count).ok_or_else(|| CliError::InvalidValue {
                        option: canonical.to_string(),
                        value: value.clone(),
                        reason: "线程数必须大于 0".to_string(),
                    })?;
            }

            // 匹配选项
            "include" => {
                if let Some(value) = matched.value.clone() {
                    config.matching.include_patterns.push(value);
                }
            }
            "exclude" => {
                if let Some(value) = matched.value.clone() {
                    config.matching.exclude_patterns.push(value);
                }
            }
            "prune" => config.matching.prune_empty = true,

            // 渲染选项
            "ascii" => config.render.charset = CharsetMode::Ascii,
            "full-path" => config.render.path_mode = PathMode::Full,
            "size" => config.render.show_size = true,
            "human-readable" => config.render.human_readable = true,
            "date" => config.render.show_date = true,
            "disk-usage" => config.render.show_disk_usage = true,
            "no-indent" => config.render.no_indent = true,
            "reverse" => config.render.reverse_sort = true,
            "report" => config.render.show_report = true,
            "no-win-banner" => config.render.no_win_banner = true,

            // 输出选项
            "output" => {
                if let Some(value) = matched.value.as_ref() {
                    config.output.output_path = Some(PathBuf::from(value));
                }
            }
            "silent" => config.output.silent = true,

            _ => {}
        }

        Ok(())
    }

    /// 验证路径参数
    fn validate_paths(&self, paths: &[String], config: &mut Config) -> Result<(), CliError> {
        match paths.len() {
            0 => {
                // 未指定路径，使用默认值，标记为未显式指定
                config.path_explicitly_set = false;
                Ok(())
            }
            1 => {
                config.root_path = PathBuf::from(&paths[0]);
                config.path_explicitly_set = true;
                Ok(())
            }
            _ => Err(CliError::MultiplePaths {
                paths: paths.to_vec(),
            }),
        }
    }
}

// ============================================================================
// 帮助与版本信息
// ============================================================================

/// 获取帮助信息字符串
///
/// # Examples
///
/// ```
/// use treepp::cli::help_text;
///
/// let help = help_text();
/// assert!(help.contains("tree++"));
/// assert!(help.contains("--help"));
/// ```
#[must_use]
pub fn help_text() -> &'static str {
    r#"tree++: A much better Windows tree command.

Usage:
  treepp [<PATH>] [<OPTIONS>...]

Options:
  --help, -h, /?              Show help information
  --version, -v, /V           Show version information
  --ascii, -a, /A             Draw the tree using ASCII characters
  --files, -f, /F             Show files
  --full-path, -p, /FP        Show full paths
  --human-readable, -H, /HR   Show file sizes in human-readable format
  --no-indent, -i, /NI        Do not display tree connector lines
  --reverse, -r, /R           Sort in reverse order
  --size, -s, /S              Show file size (bytes)
  --date, -d, /DT             Show last modified date
  --exclude, -I, /X <PATTERN> Exclude files matching the pattern
  --level, -L, /L <N>         Limit recursion depth
  --include, -m, /M <PATTERN> Show only files matching the pattern
  --disk-usage, -u, /DU       Show cumulative directory sizes
  --report, -e, /RP           Show summary statistics at the end
  --prune, -P, /P             Prune empty directories
  --no-win-banner, -N, /NB    Do not show the Windows native tree banner/header
  --silent, -l, /SI           Silent mode (use with --output)
  --output, -o, /O <FILE>     Write output to a file (.txt, .json, .yml, .toml)
  --thread, -t, /T <N>        Number of scanning threads (default: 8)
  --gitignore, -g, /G         Respect .gitignore

More info: https://github.com/Water-Run/treepp"#
}

/// 获取版本信息字符串
///
/// # Examples
///
/// ```
/// use treepp::cli::version_text;
///
/// let version = version_text();
/// assert!(version.contains("0.1.0"));
/// ```
#[must_use]
pub fn version_text() -> &'static str {
    r#"tree++ version 0.1.0

A much better Windows tree command.

author: WaterRun
link: https://github.com/Water-Run/treepp"#
}

/// 打印帮助信息到标准输出
///
/// # Examples
///
/// ```no_run
/// use treepp::cli::print_help;
///
/// print_help();
/// ```
pub fn print_help() {
    println!("{}", help_text());
}

/// 打印版本信息到标准输出
///
/// # Examples
///
/// ```no_run
/// use treepp::cli::print_version;
///
/// print_version();
/// ```
pub fn print_version() {
    println!("{}", version_text());
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OutputFormat;
    use tempfile::TempDir;

    // ------------------------------------------------------------------------
    // 辅助函数
    // ------------------------------------------------------------------------

    /// 创建临时目录用于测试
    fn create_temp_dir() -> TempDir {
        TempDir::new().expect("创建临时目录失败")
    }

    /// 创建带有指定路径的解析器
    fn parser_with_temp_dir(temp_dir: &TempDir, extra_args: Vec<&str>) -> CliParser {
        let path_str = temp_dir.path().to_string_lossy().to_string();
        let mut args = vec![path_str];
        args.extend(extra_args.into_iter().map(String::from));
        CliParser::new(args)
    }

    // ------------------------------------------------------------------------
    // 基础解析测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_empty_args_with_defaults() {
        let parser = CliParser::new(vec![]);
        let result = parser.parse();

        assert!(result.is_ok(), "解析应成功: {:?}", result);
        if let Ok(ParseResult::Config(config)) = result {
            // 验证后的路径是规范化的绝对路径
            assert!(
                config.root_path.is_absolute(),
                "默认路径应被规范化为绝对路径"
            );
            // 验证其他默认值
            assert!(!config.scan.show_files);
            assert_eq!(config.scan.thread_count.get(), 8);
            assert_eq!(config.scan.max_depth, None);
            assert!(!config.scan.respect_gitignore);
            assert!(config.matching.include_patterns.is_empty());
            assert!(config.matching.exclude_patterns.is_empty());
            assert!(!config.matching.prune_empty);
            assert_eq!(config.render.charset, CharsetMode::Unicode);
            assert_eq!(config.render.path_mode, PathMode::Relative);
            assert!(!config.render.show_size);
            assert!(!config.render.human_readable);
            assert!(!config.render.show_date);
            assert!(!config.render.show_disk_usage);
            assert!(!config.render.no_indent);
            assert!(!config.render.reverse_sort);
            assert!(!config.render.show_report);
            assert!(!config.render.no_win_banner);
            assert!(config.output.output_path.is_none());
            assert!(!config.output.silent);
        } else {
            panic!("期望 ParseResult::Config");
        }
    }

    #[test]
    fn should_parse_path_only() {
        let temp_dir = create_temp_dir();
        let path_str = temp_dir.path().to_string_lossy().to_string();

        let parser = CliParser::new(vec![path_str]);
        let result = parser.parse();

        assert!(result.is_ok(), "解析应成功: {:?}", result);
        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.root_path.is_absolute());
            let expected = dunce::canonicalize(temp_dir.path()).expect("规范化失败");
            assert_eq!(config.root_path, expected);
        } else {
            panic!("期望 ParseResult::Config");
        }
    }

    #[test]
    fn should_parse_path_with_spaces() {
        let temp_dir = create_temp_dir();
        let sub_dir = temp_dir.path().join("path with spaces");
        std::fs::create_dir(&sub_dir).expect("创建子目录失败");

        let parser = CliParser::new(vec![sub_dir.to_string_lossy().to_string()]);
        let result = parser.parse();

        assert!(result.is_ok(), "带空格路径解析应成功: {:?}", result);
    }

    #[test]
    fn should_parse_relative_path() {
        // 使用当前目录的相对路径 "."
        let parser = CliParser::new(vec![".".to_string()]);
        let result = parser.parse();

        assert!(result.is_ok(), "相对路径解析应成功: {:?}", result);
        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.root_path.is_absolute());
        }
    }

    // ------------------------------------------------------------------------
    // 帮助与版本测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_return_help_for_help_flags() {
        for flag in &["--help", "-h", "/?"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            let result = parser.parse();
            assert!(matches!(result, Ok(ParseResult::Help)), "测试 {flag}");
        }
    }

    #[test]
    fn should_return_version_for_version_flags() {
        for flag in &["--version", "-v", "/V", "/v"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            let result = parser.parse();
            assert!(matches!(result, Ok(ParseResult::Version)), "测试 {flag}");
        }
    }

    #[test]
    fn should_return_help_even_with_other_options() {
        // 帮助选项应该优先返回
        let parser = CliParser::new(vec!["/F".to_string(), "--help".to_string()]);
        let result = parser.parse();
        // 注意：由于重复检测，这里会检测到 help，但 /F 在前面先处理
        // 实际上会解析 /F，然后解析 --help 并返回 Help
        assert!(matches!(result, Ok(ParseResult::Help)));
    }

    #[test]
    fn should_return_version_even_with_other_options() {
        let parser = CliParser::new(vec!["/F".to_string(), "--version".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Ok(ParseResult::Version)));
    }

    // ------------------------------------------------------------------------
    // 三风格混用测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_cmd_style_case_insensitive() {
        let parser1 = CliParser::new(vec!["/F".to_string()]);
        let parser2 = CliParser::new(vec!["/f".to_string()]);

        if let (Ok(ParseResult::Config(c1)), Ok(ParseResult::Config(c2))) =
            (parser1.parse(), parser2.parse())
        {
            assert!(c1.scan.show_files);
            assert!(c2.scan.show_files);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_cmd_style_various_cases() {
        // 测试更多 CMD 风格的大小写变体
        for flag in &["/A", "/a", "/HR", "/hr", "/Hr", "/hR", "/NI", "/ni", "/Ni"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            let result = parser.parse();
            assert!(
                matches!(result, Ok(ParseResult::Config(_))),
                "测试 {flag} 失败"
            );
        }
    }

    #[test]
    fn should_parse_unix_short_style_case_sensitive() {
        let parser1 = CliParser::new(vec!["-f".to_string()]);
        let result1 = parser1.parse();
        assert!(matches!(result1, Ok(ParseResult::Config(_))));

        let parser2 = CliParser::new(vec!["-F".to_string()]);
        let result2 = parser2.parse();
        assert!(matches!(result2, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn should_parse_gnu_long_style() {
        let parser = CliParser::new(vec!["--files".to_string()]);
        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_reject_gnu_long_style_wrong_case() {
        let parser = CliParser::new(vec!["--FILES".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn should_parse_mixed_styles() {
        let parser = CliParser::new(vec![
            "/F".to_string(),
            "-a".to_string(),
            "--level".to_string(),
            "3".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert_eq!(config.scan.max_depth, Some(3));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_all_three_styles_together() {
        let temp_dir = create_temp_dir();
        let parser = parser_with_temp_dir(
            &temp_dir,
            vec![
                "/F",
                "-a",
                "--size",
                "/HR",
                "-d",
                "--reverse",
                "--prune",
            ],
        );

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert!(config.render.show_size);
            assert!(config.render.human_readable);
            assert!(config.render.show_date);
            assert!(config.render.reverse_sort);
            assert!(config.matching.prune_empty);
        } else {
            panic!("解析失败");
        }
    }

    // ------------------------------------------------------------------------
    // 等价映射测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_map_equivalent_options() {
        let cases = vec!["/F", "-f", "--files"];

        for flag in cases {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.scan.show_files, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_map_all_equivalent_ascii_options() {
        for flag in &["/A", "/a", "-a", "--ascii"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(
                    config.render.charset,
                    CharsetMode::Ascii,
                    "测试 {flag} 失败"
                );
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_map_all_equivalent_level_options() {
        for (flag, value) in &[("/L", "5"), ("-L", "5"), ("--level", "5")] {
            let parser = CliParser::new(vec![flag.to_string(), value.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(config.scan.max_depth, Some(5), "测试 {flag} 失败");
            } else {
                panic!("解析 {flag} {value} 失败");
            }
        }
    }

    // ------------------------------------------------------------------------
    // 带值参数测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_value_arguments() {
        let parser = CliParser::new(vec!["--level".to_string(), "5".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.max_depth, Some(5));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_equals_syntax() {
        let parser = CliParser::new(vec!["--level=10".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.max_depth, Some(10));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_equals_syntax_with_various_values() {
        let cases = vec![
            ("--level=1", 1usize),
            ("--level=100", 100),
            ("--level=999", 999),
        ];

        for (arg, expected) in cases {
            let parser = CliParser::new(vec![arg.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(config.scan.max_depth, Some(expected), "测试 {arg}");
            } else {
                panic!("解析 {arg} 失败");
            }
        }
    }

    #[test]
    fn should_parse_equals_syntax_for_output() {
        let parser = CliParser::new(vec!["--output=tree.json".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.output.output_path, Some(PathBuf::from("tree.json")));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_fail_on_missing_value() {
        let parser = CliParser::new(vec!["--level".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::MissingValue { .. })));
    }

    #[test]
    fn should_fail_on_missing_value_when_followed_by_option() {
        let parser = CliParser::new(vec!["--level".to_string(), "--files".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::MissingValue { .. })));
    }

    #[test]
    fn should_fail_on_invalid_value() {
        let parser = CliParser::new(vec!["--level".to_string(), "abc".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn should_fail_on_negative_level() {
        let parser = CliParser::new(vec!["--level".to_string(), "-5".to_string()]);
        let result = parser.parse();
        // -5 看起来像选项，所以会报 MissingValue
        assert!(matches!(result, Err(CliError::MissingValue { .. })));
    }

    #[test]
    fn should_parse_level_zero() {
        let parser = CliParser::new(vec!["--level".to_string(), "0".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.max_depth, Some(0));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_large_level() {
        let parser = CliParser::new(vec!["--level".to_string(), "1000000".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.max_depth, Some(1_000_000));
        } else {
            panic!("解析失败");
        }
    }

    // ------------------------------------------------------------------------
    // 重复参数测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_fail_on_duplicate_option() {
        let parser = CliParser::new(vec!["/F".to_string(), "--files".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::DuplicateOption { .. })));
    }

    #[test]
    fn should_fail_on_duplicate_same_style() {
        let parser = CliParser::new(vec!["/F".to_string(), "/F".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::DuplicateOption { .. })));
    }

    #[test]
    fn should_fail_on_duplicate_different_case_cmd() {
        let parser = CliParser::new(vec!["/F".to_string(), "/f".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::DuplicateOption { .. })));
    }

    #[test]
    fn should_fail_on_duplicate_level() {
        let parser = CliParser::new(vec![
            "--level".to_string(),
            "3".to_string(),
            "-L".to_string(),
            "5".to_string(),
        ]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::DuplicateOption { .. })));
    }

    // ------------------------------------------------------------------------
    // 未知参数测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_fail_on_unknown_option() {
        let parser = CliParser::new(vec!["/UNKNOWN".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn should_fail_on_unknown_short_option() {
        let parser = CliParser::new(vec!["-z".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn should_fail_on_unknown_long_option() {
        let parser = CliParser::new(vec!["--unknown-option".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn should_fail_on_typo_option() {
        let parser = CliParser::new(vec!["--fies".to_string()]); // typo for --files
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    // ------------------------------------------------------------------------
    // 线程数测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_thread_count() {
        let parser = CliParser::new(vec!["--thread".to_string(), "16".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.thread_count.get(), 16);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_thread_count_one() {
        let parser = CliParser::new(vec!["--thread".to_string(), "1".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.thread_count.get(), 1);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_fail_on_zero_thread_count() {
        let parser = CliParser::new(vec!["--thread".to_string(), "0".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn should_fail_on_invalid_thread_count() {
        let parser = CliParser::new(vec!["--thread".to_string(), "abc".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    #[test]
    fn should_parse_thread_with_cmd_style() {
        let parser = CliParser::new(vec!["/T".to_string(), "4".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.scan.thread_count.get(), 4);
        } else {
            panic!("解析失败");
        }
    }

    // ------------------------------------------------------------------------
    // 输出选项测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_output_path() {
        let parser = CliParser::new(vec!["--output".to_string(), "tree.json".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.output.output_path, Some(PathBuf::from("tree.json")));
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_output_with_various_extensions() {
        let cases = vec![
            ("tree.txt", OutputFormat::Txt),
            ("tree.json", OutputFormat::Json),
            ("tree.yml", OutputFormat::Yaml),
            ("tree.yaml", OutputFormat::Yaml),
            ("tree.toml", OutputFormat::Toml),
        ];

        for (filename, expected_format) in cases {
            let parser = CliParser::new(vec!["--output".to_string(), filename.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(
                    config.output.format, expected_format,
                    "测试 {filename} 格式推断"
                );
            } else {
                panic!("解析 --output {filename} 失败");
            }
        }
    }

    #[test]
    fn should_parse_output_with_path() {
        let parser = CliParser::new(vec![
            "--output".to_string(),
            "C:\\output\\tree.json".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.output.output_path,
                Some(PathBuf::from("C:\\output\\tree.json"))
            );
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_silent_with_output() {
        let parser = CliParser::new(vec![
            "--silent".to_string(),
            "--output".to_string(),
            "tree.txt".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.output.silent);
            assert!(config.output.output_path.is_some());
        } else {
            panic!("解析失败");
        }
    }

    // ------------------------------------------------------------------------
    // 模式匹配测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_include_pattern() {
        let parser = CliParser::new(vec!["--include".to_string(), "*.rs".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.matching.include_patterns, vec!["*.rs".to_string()]);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_exclude_pattern() {
        let parser = CliParser::new(vec!["--exclude".to_string(), "node_modules".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.matching.exclude_patterns,
                vec!["node_modules".to_string()]
            );
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_multiple_include_patterns() {
        let parser = CliParser::new(vec![
            "--include".to_string(),
            "*.rs".to_string(),
            "-m".to_string(),
            "*.toml".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.matching.include_patterns,
                vec!["*.rs".to_string(), "*.toml".to_string()]
            );
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_multiple_exclude_patterns() {
        let parser = CliParser::new(vec![
            "--exclude".to_string(),
            "target".to_string(),
            "/X".to_string(),
            "node_modules".to_string(),
        ]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(
                config.matching.exclude_patterns,
                vec!["target".to_string(), "node_modules".to_string()]
            );
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_parse_prune() {
        for flag in &["--prune", "-P", "/P"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.matching.prune_empty, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_gitignore() {
        for flag in &["--gitignore", "-g", "/G", "/g"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.scan.respect_gitignore, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    // ------------------------------------------------------------------------
    // 配置验证集成测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_fail_silent_without_output() {
        let parser = CliParser::new(vec!["--silent".to_string()]);
        let result = parser.parse();
        assert!(result.is_err());
    }

    #[test]
    fn should_infer_output_format_from_extension() {
        let parser = CliParser::new(vec!["--output".to_string(), "tree.json".to_string()]);

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert_eq!(config.output.format, OutputFormat::Json);
        } else {
            panic!("解析失败");
        }
    }

    #[test]
    fn should_fail_on_nonexistent_path() {
        let parser = CliParser::new(vec!["C:\\nonexistent\\path\\12345".to_string()]);
        let result = parser.parse();
        assert!(result.is_err(), "不存在的路径应该失败");
    }

    // ------------------------------------------------------------------------
    // 渲染选项测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_full_path() {
        for flag in &["--full-path", "-p", "/FP", "/fp"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(config.render.path_mode, PathMode::Full, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_size() {
        for flag in &["--size", "-s", "/S"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.show_size, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_human_readable() {
        for flag in &["--human-readable", "-H", "/HR", "/hr"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.human_readable, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_date() {
        for flag in &["--date", "-d", "/DT", "/dt"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.show_date, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_disk_usage() {
        for flag in &["--disk-usage", "-u", "/DU", "/du"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.show_disk_usage, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_no_indent() {
        for flag in &["--no-indent", "-i", "/NI", "/ni"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.no_indent, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_reverse() {
        for flag in &["--reverse", "-r", "/R", "/r"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.reverse_sort, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_report() {
        for flag in &["--report", "-e", "/RP", "/rp"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.show_report, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    #[test]
    fn should_parse_no_win_banner() {
        for flag in &["--no-win-banner", "-N", "/NB", "/nb"] {
            let parser = CliParser::new(vec![flag.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert!(config.render.no_win_banner, "测试 {flag}");
            } else {
                panic!("解析 {flag} 失败");
            }
        }
    }

    // ------------------------------------------------------------------------
    // 帮助文本测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_contain_all_options_in_help() {
        let help = help_text();
        assert!(help.contains("--help"));
        assert!(help.contains("--files"));
        assert!(help.contains("--ascii"));
        assert!(help.contains("--level"));
        assert!(help.contains("--output"));
        assert!(help.contains("--thread"));
        assert!(help.contains("--gitignore"));
        assert!(help.contains("--include"));
        assert!(help.contains("--exclude"));
        assert!(help.contains("--silent"));
        assert!(help.contains("--prune"));
        assert!(help.contains("--reverse"));
        assert!(help.contains("--no-win-banner"));
        assert!(help.contains("--disk-usage"));
        assert!(help.contains("--human-readable"));
        assert!(help.contains("--date"));
        assert!(help.contains("--size"));
        assert!(help.contains("--full-path"));
        assert!(help.contains("--no-indent"));
        assert!(help.contains("--report"));
    }

    #[test]
    fn should_contain_version_in_version_text() {
        let version = version_text();
        assert!(version.contains("0.1.0"));
        assert!(version.contains("WaterRun"));
        assert!(version.contains("github.com"));
    }

    #[test]
    fn should_contain_usage_in_help() {
        let help = help_text();
        assert!(help.contains("Usage"));
        assert!(help.contains("treepp"));
        assert!(help.contains("PATH"));
        assert!(help.contains("OPTIONS"));
    }

    // ------------------------------------------------------------------------
    // 路径位置测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_fail_multiple_paths() {
        let parser = CliParser::new(vec!["path1".to_string(), "path2".to_string()]);
        let result = parser.parse();

        assert!(matches!(result, Err(CliError::MultiplePaths { .. })));

        if let Err(CliError::MultiplePaths { paths }) = result {
            assert_eq!(paths.len(), 2);
            assert_eq!(paths[0], "path1");
            assert_eq!(paths[1], "path2");
        }
    }

    #[test]
    fn should_fail_three_paths() {
        let parser = CliParser::new(vec![
            "path1".to_string(),
            "path2".to_string(),
            "path3".to_string(),
        ]);
        let result = parser.parse();

        assert!(matches!(result, Err(CliError::MultiplePaths { .. })));

        if let Err(CliError::MultiplePaths { paths }) = result {
            assert_eq!(paths.len(), 3);
        }
    }

    #[test]
    fn should_accept_only_options_no_path() {
        let parser = CliParser::new(vec!["/F".to_string()]);
        let result = parser.parse();

        // 当前目录应该存在，所以解析应该成功
        assert!(result.is_ok(), "只有选项无路径应该成功（使用当前目录）");
    }

    // ------------------------------------------------------------------------
    // 边缘情况测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_handle_empty_string_arg() {
        // 空字符串被视为路径
        let parser = CliParser::new(vec!["".to_string()]);
        let result = parser.parse();
        // 空路径应该导致验证失败
        assert!(result.is_err());
    }

    #[test]
    fn should_handle_whitespace_only_arg() {
        let parser = CliParser::new(vec!["   ".to_string()]);
        let result = parser.parse();
        // 纯空白路径应该导致验证失败
        assert!(result.is_err());
    }

    #[test]
    fn should_handle_dash_only() {
        let parser = CliParser::new(vec!["-".to_string()]);
        let result = parser.parse();
        // 单独的 "-" 应该被视为未知选项
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn should_handle_double_dash_only() {
        let parser = CliParser::new(vec!["--".to_string()]);
        let result = parser.parse();
        // 单独的 "--" 应该被视为未知选项
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn should_handle_slash_only() {
        let parser = CliParser::new(vec!["/".to_string()]);
        let result = parser.parse();
        // 单独的 "/" 应该被视为未知选项
        assert!(matches!(result, Err(CliError::UnknownOption { .. })));
    }

    #[test]
    fn should_parse_option_with_unicode() {
        // 带有 Unicode 字符的路径
        let temp_dir = create_temp_dir();
        let unicode_dir = temp_dir.path().join("测试目录");
        std::fs::create_dir(&unicode_dir).expect("创建目录失败");

        let parser = CliParser::new(vec![
            unicode_dir.to_string_lossy().to_string(),
            "/F".to_string(),
        ]);
        let result = parser.parse();

        assert!(result.is_ok(), "Unicode 路径应该成功: {:?}", result);
    }

    // ------------------------------------------------------------------------
    // 复合场景测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_complex_command_line() {
        let temp_dir = create_temp_dir();
        let parser = parser_with_temp_dir(
            &temp_dir,
            vec![
                "/F",
                "-a",
                "--level",
                "5",
                "-s",
                "-H",
                "-r",
                "--include",
                "*.rs",
                "--exclude",
                "target",
                "--prune",
                "-g",
                "--report",
                "-N",
                "--thread",
                "4",
            ],
        );

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.scan.show_files);
            assert_eq!(config.render.charset, CharsetMode::Ascii);
            assert_eq!(config.scan.max_depth, Some(5));
            assert!(config.render.show_size);
            assert!(config.render.human_readable);
            assert!(config.render.reverse_sort);
            assert_eq!(config.matching.include_patterns, vec!["*.rs"]);
            assert_eq!(config.matching.exclude_patterns, vec!["target"]);
            assert!(config.matching.prune_empty);
            assert!(config.scan.respect_gitignore);
            assert!(config.render.show_report);
            assert!(config.render.no_win_banner);
            assert_eq!(config.scan.thread_count.get(), 4);
        } else {
            panic!("复杂命令行解析失败");
        }
    }

    #[test]
    fn should_parse_minimal_output_scenario() {
        let temp_dir = create_temp_dir();
        let output_file = temp_dir.path().join("output.json");

        let parser = parser_with_temp_dir(
            &temp_dir,
            vec!["--output", output_file.to_str().unwrap(), "--silent", "/F"],
        );

        if let Ok(ParseResult::Config(config)) = parser.parse() {
            assert!(config.output.silent);
            assert!(config.output.output_path.is_some());
            assert_eq!(config.output.format, OutputFormat::Json);
        } else {
            panic!("解析失败");
        }
    }

    // ------------------------------------------------------------------------
    // is_option_like 测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_identify_option_like_strings() {
        assert!(CliParser::is_option_like("-f"));
        assert!(CliParser::is_option_like("--files"));
        assert!(CliParser::is_option_like("/F"));
        assert!(CliParser::is_option_like("/?"));
        assert!(CliParser::is_option_like("-"));
        assert!(CliParser::is_option_like("--"));
        assert!(CliParser::is_option_like("/"));
    }

    #[test]
    fn should_identify_non_option_strings() {
        assert!(!CliParser::is_option_like("path"));
        assert!(!CliParser::is_option_like("C:\\dir"));
        assert!(!CliParser::is_option_like("file.txt"));
        assert!(!CliParser::is_option_like(""));
        assert!(!CliParser::is_option_like("123"));
    }

    // ------------------------------------------------------------------------
    // from_env 测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_create_parser_from_env() {
        // 这个测试主要验证 from_env 不会 panic
        let _parser = CliParser::from_env();
        // 不调用 parse()，因为实际的命令行参数可能导致各种结果
    }

    #[test]
    fn test_path_explicitly_set_when_specified() {
        let temp_dir = create_temp_dir();
        let path_str = temp_dir.path().to_string_lossy().to_string();

        let parser = CliParser::new(vec![path_str]);
        let result = parser.parse();

        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.path_explicitly_set);
        } else {
            panic!("解析应成功");
        }
    }

    #[test]
    fn test_path_not_explicitly_set_when_omitted() {
        let parser = CliParser::new(vec![]);
        let result = parser.parse();

        if let Ok(ParseResult::Config(config)) = result {
            assert!(!config.path_explicitly_set);
        } else {
            panic!("解析应成功");
        }
    }

    #[test]
    fn test_path_explicitly_set_with_dot() {
        let parser = CliParser::new(vec![".".to_string()]);
        let result = parser.parse();

        if let Ok(ParseResult::Config(config)) = result {
            assert!(config.path_explicitly_set);
        } else {
            panic!("解析应成功");
        }
    }
}
