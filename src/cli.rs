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
//! 作者: WaterRun
//! 更新于: 2025-01-05

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::env;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use crate::config::{CharsetMode, Config, PathMode, SortKey};
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
    // 排序类
    ArgDef {
        canonical: "sort",
        kind: ArgKind::Value,
        cmd_patterns: &["/SO"],
        short_patterns: &["-S"],
        long_patterns: &["--sort"],
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
        canonical: "ignore-case",
        kind: ArgKind::Flag,
        cmd_patterns: &["/IC"],
        short_patterns: &["-c"],
        long_patterns: &["--ignore-case"],
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

        while self.position < self.args.len() {
            let current_arg = self.args[self.position].clone();

            if Self::is_option_like(&current_arg) {
                let matched = self.try_match_argument(&current_arg)?;
                self.register_canonical_name(matched.definition.canonical)?;
                self.apply_to_config(&mut config, &matched)?;

                // 帮助和版本信息立即返回
                if matched.definition.canonical == "help" {
                    return Ok(ParseResult::Help);
                }
                if matched.definition.canonical == "version" {
                    return Ok(ParseResult::Version);
                }
            } else {
                // 非选项参数视为路径
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
            "ignore-case" => config.matching.ignore_case = true,
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
            "sort" => {
                let value = matched.value.as_ref().expect("sort 参数需要值");
                config.render.sort_key =
                    SortKey::from_str_loose(value).ok_or_else(|| CliError::InvalidValue {
                        option: canonical.to_string(),
                        value: value.clone(),
                        reason: format!("有效值: {}", SortKey::valid_keys().join(", ")),
                    })?;
            }

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
            0 => Ok(()),
            1 => {
                config.root_path = PathBuf::from(&paths[0]);
                Ok(())
            }
            _ => Err(CliError::ParseError {
                message: format!("只允许指定一个路径，但发现多个: {:?}", paths),
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
    r#"tree++ - 更好的 Windows tree 命令

用法:
  treepp [<PATH>] [<OPTIONS>...]

选项:
  --help, -h, /?              显示帮助信息
  --version, -v, /V           显示版本信息
  --ascii, -a, /A             使用 ASCII 字符绘制树
  --files, -f, /F             显示文件
  --full-path, -p, /FP        显示完整路径
  --human-readable, -H, /HR   以人类可读方式显示文件大小
  --no-indent, -i, /NI        不显示树形连接线
  --reverse, -r, /R           逆序排序
  --size, -s, /S              显示文件大小（字节）
  --date, -d, /DT             显示最后修改日期
  --exclude, -I, /X <PATTERN> 排除匹配的文件
  --level, -L, /L <N>         限制递归深度
  --include, -m, /M <PATTERN> 仅显示匹配的文件
  --disk-usage, -u, /DU       显示目录累计大小
  --ignore-case, -c, /IC      匹配时忽略大小写
  --report, -e, /RP           显示末尾统计信息
  --prune, -P, /P             修剪空目录
  --sort, -S, /SO <KEY>       指定排序方式（name, size, mtime, ctime）
  --no-win-banner, -N, /NB    不显示 Windows 原生 tree 的样板信息
  --silent, -l, /SI           终端静默（结合 --output 使用）
  --output, -o, /O <FILE>     将结果输出至文件（.txt, .json, .yml, .toml）
  --thread, -t, /T <N>        扫描线程数（默认 8）
  --gitignore, -g, /G         遵循 .gitignore

更多信息: https://github.com/Water-Run/treepp"#
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

A Better tree command for Windows.

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

    // ------------------------------------------------------------------------
    // 基础解析测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_empty_args_with_defaults() {
        let parser = CliParser::new(vec![]);
        let result = parser.parse();

        assert!(result.is_ok());
        if let Ok(ParseResult::Config(config)) = result {
            assert_eq!(config.root_path, PathBuf::from("."));
            assert!(!config.scan.show_files);
            assert_eq!(config.scan.thread_count.get(), 8);
        } else {
            panic!("期望 ParseResult::Config");
        }
    }

    #[test]
    fn should_parse_path_only() {
        let parser = CliParser::new(vec!["D:\\project".to_string()]);
        let result = parser.parse();

        assert!(result.is_ok());
        if let Ok(ParseResult::Config(config)) = result {
            assert_eq!(config.root_path, PathBuf::from("D:\\project"));
        } else {
            panic!("期望 ParseResult::Config");
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
    fn should_fail_on_missing_value() {
        let parser = CliParser::new(vec!["--level".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::MissingValue { .. })));
    }

    #[test]
    fn should_fail_on_invalid_value() {
        let parser = CliParser::new(vec!["--level".to_string(), "abc".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
    }

    // ------------------------------------------------------------------------
    // 排序键测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_parse_sort_key() {
        for (input, expected) in [
            ("name", SortKey::Name),
            ("size", SortKey::Size),
            ("mtime", SortKey::Mtime),
            ("ctime", SortKey::Ctime),
            ("NAME", SortKey::Name),
        ] {
            let parser = CliParser::new(vec!["--sort".to_string(), input.to_string()]);
            if let Ok(ParseResult::Config(config)) = parser.parse() {
                assert_eq!(config.render.sort_key, expected, "测试 {input}");
            } else {
                panic!("解析 sort={input} 失败");
            }
        }
    }

    #[test]
    fn should_fail_on_invalid_sort_key() {
        let parser = CliParser::new(vec!["--sort".to_string(), "invalid".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
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

    // ------------------------------------------------------------------------
    // 未知参数测试
    // ------------------------------------------------------------------------

    #[test]
    fn should_fail_on_unknown_option() {
        let parser = CliParser::new(vec!["/UNKNOWN".to_string()]);
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
    fn should_fail_on_zero_thread_count() {
        let parser = CliParser::new(vec!["--thread".to_string(), "0".to_string()]);
        let result = parser.parse();
        assert!(matches!(result, Err(CliError::InvalidValue { .. })));
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
    }

    #[test]
    fn should_contain_version_in_version_text() {
        let version = version_text();
        assert!(version.contains("0.1.0"));
        assert!(version.contains("WaterRun"));
    }
}