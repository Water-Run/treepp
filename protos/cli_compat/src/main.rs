//! # `tree++` 命令行参数解析模块
//!
//! 本模块实现 `tree++` 命令行工具的参数解析功能，支持三种参数风格混用：
//!
//! - Windows CMD 风格 (`/F`)，大小写不敏感
//! - Unix 短参数风格 (`-f`)，大小写敏感
//! - GNU 长参数风格 (`--files`)，大小写敏感
//!
//! ## 示例
//!
//! ```rust
//! use cli_compat::{CliParser, ParseResult};
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

use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

use thiserror::Error;

// ============================================================================
// 常量定义
// ============================================================================

/// 默认扫描线程数
const DEFAULT_THREAD_COUNT: u32 = 8;

/// MFT 模式下不兼容的参数规范名称列表
const MFT_INCOMPATIBLE_ARGS: &[&str] = &[
    "prune",
    "level",
    "gitignore",
    "include",
    "exclude",
    "disk-usage",
    "sort",
    "reverse",
];

// ============================================================================
// 错误类型定义
// ============================================================================

/// 命令行解析错误
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum CliError {
    /// 遇到未知参数
    #[error("未知参数: {0}")]
    UnknownArgument(String),

    /// 需要值的参数缺少值
    #[error("参数 {0} 需要一个值")]
    MissingValue(String),

    /// 参数值无效
    #[error("参数 {0} 的值无效: {1}")]
    InvalidValue(String, String),

    /// 参数重复指定
    #[error("参数重复: {0}")]
    DuplicateArgument(String),

    /// 指定了多个路径
    #[error("只允许指定一个路径，但发现多个: {paths:?}")]
    MultiplePaths { paths: Vec<String> },

    /// 路径出现在选项之后
    #[error("路径必须在所有选项之前指定")]
    PathAfterOptions,

    /// MFT 模式下使用了不兼容的参数
    #[error("MFT 模式下不支持参数: {0}")]
    MftIncompatible(String),
}

// ============================================================================
// 排序键枚举
// ============================================================================

/// 排序方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// 按文件名字母表升序
    #[default]
    Name,
    /// 按文件大小升序
    Size,
    /// 按修改时间升序
    Mtime,
    /// 按创建时间升序
    Ctime,
}

impl SortKey {
    /// 从字符串解析排序键
    ///
    /// 解析时忽略大小写。
    ///
    /// # 参数
    ///
    /// * `s` - 待解析的字符串
    ///
    /// # 返回值
    ///
    /// 解析成功返回 `Some(SortKey)`，否则返回 `None`。
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "name" => Some(Self::Name),
            "size" => Some(Self::Size),
            "mtime" => Some(Self::Mtime),
            "ctime" => Some(Self::Ctime),
            _ => None,
        }
    }
}

// ============================================================================
// 配置结构体
// ============================================================================

/// 解析后的命令行配置
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// 目标路径
    pub path: PathBuf,

    /// 显示文件 (`/F`, `-f`, `--files`)
    pub show_files: bool,

    /// 显示完整路径 (`/FP`, `-p`, `--full-path`)
    pub show_full_path: bool,

    /// 显示文件大小（字节） (`/S`, `-s`, `--size`)
    pub show_size: bool,

    /// 人类可读的文件大小 (`/HR`, `-H`, `--human-readable`)
    pub human_readable: bool,

    /// 显示最后修改日期 (`/DT`, `-d`, `--date`)
    pub show_date: bool,

    /// 用双引号包裹文件名 (`/Q`, `-q`, `--quote`)
    pub quote_names: bool,

    /// 使用 ASCII 字符绘制树 (`/A`, `-a`, `--ascii`)
    pub use_ascii: bool,

    /// 不显示树形连接线 (`/NI`, `-i`, `--no-indent`)
    pub no_indent: bool,

    /// 逆序排序 (`/R`, `-r`, `--reverse`)
    pub reverse: bool,

    /// 目录优先显示 (`/DF`, `-D`, `--dirs-first`)
    pub dirs_first: bool,

    /// 排序方式 (`/SO`, `-S`, `--sort`)
    pub sort_by: Option<SortKey>,

    /// 匹配时忽略大小写 (`/IC`, `-c`, `--ignore-case`)
    pub ignore_case: bool,

    /// 递归深度限制 (`/L`, `-L`, `--level`)
    pub level: Option<u32>,

    /// 修剪空目录 (`/P`, `-P`, `--prune`)
    pub prune: bool,

    /// 仅显示匹配项 (`/M`, `-m`, `--include`)
    pub include_pattern: Option<String>,

    /// 排除匹配项 (`/X`, `-I`, `--exclude`)
    pub exclude_pattern: Option<String>,

    /// 遵循 `.gitignore` (`/G`, `-g`, `--gitignore`)
    pub gitignore: bool,

    /// 不显示末尾统计信息 (`/NR`, `-n`, `--no-report`)
    pub no_report: bool,

    /// 不显示卷信息与头部报告 (`/NH`, `-N`, `--no-header`)
    pub no_header: bool,

    /// 终端静默 (`/SI`, `-l`, `--silent`)
    pub silent: bool,

    /// 输出到文件 (`/O`, `-o`, `--output`)
    pub output_file: Option<PathBuf>,

    /// 扫描线程数 (`/T`, `-t`, `--thread`)
    pub thread_count: u32,

    /// 使用 MFT 模式 (`/MFT`, `-M`, `--mft`)
    pub use_mft: bool,

    /// 显示目录累计大小 (`/DU`, `-u`, `--disk-usage`)
    pub disk_usage: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            show_files: false,
            show_full_path: false,
            show_size: false,
            human_readable: false,
            show_date: false,
            quote_names: false,
            use_ascii: false,
            no_indent: false,
            reverse: false,
            dirs_first: false,
            sort_by: None,
            ignore_case: false,
            level: None,
            prune: false,
            include_pattern: None,
            exclude_pattern: None,
            gitignore: false,
            no_report: false,
            no_header: false,
            silent: false,
            output_file: None,
            thread_count: DEFAULT_THREAD_COUNT,
            use_mft: false,
            disk_usage: false,
        }
    }
}

// ============================================================================
// 解析结果枚举
// ============================================================================

/// 解析结果
#[derive(Debug, PartialEq, Eq)]
pub enum ParseResult {
    /// 正常配置
    Config(Config),
    /// 显示帮助信息
    Help,
    /// 显示版本信息
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
const ARG_DEFINITIONS: &[ArgDef] = &[
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
        canonical: "quote",
        kind: ArgKind::Flag,
        cmd_patterns: &["/Q"],
        short_patterns: &["-q"],
        long_patterns: &["--quote"],
    },
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
    ArgDef {
        canonical: "dirs-first",
        kind: ArgKind::Flag,
        cmd_patterns: &["/DF"],
        short_patterns: &["-D"],
        long_patterns: &["--dirs-first"],
    },
    ArgDef {
        canonical: "sort",
        kind: ArgKind::Value,
        cmd_patterns: &["/SO"],
        short_patterns: &["-S"],
        long_patterns: &["--sort"],
    },
    ArgDef {
        canonical: "ignore-case",
        kind: ArgKind::Flag,
        cmd_patterns: &["/IC"],
        short_patterns: &["-c"],
        long_patterns: &["--ignore-case"],
    },
    ArgDef {
        canonical: "level",
        kind: ArgKind::Value,
        cmd_patterns: &["/L"],
        short_patterns: &["-L"],
        long_patterns: &["--level"],
    },
    ArgDef {
        canonical: "prune",
        kind: ArgKind::Flag,
        cmd_patterns: &["/P"],
        short_patterns: &["-P"],
        long_patterns: &["--prune"],
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
        canonical: "gitignore",
        kind: ArgKind::Flag,
        cmd_patterns: &["/G"],
        short_patterns: &["-g"],
        long_patterns: &["--gitignore"],
    },
    ArgDef {
        canonical: "no-report",
        kind: ArgKind::Flag,
        cmd_patterns: &["/NR"],
        short_patterns: &["-n"],
        long_patterns: &["--no-report"],
    },
    ArgDef {
        canonical: "no-header",
        kind: ArgKind::Flag,
        cmd_patterns: &["/NH"],
        short_patterns: &["-N"],
        long_patterns: &["--no-header"],
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
    ArgDef {
        canonical: "thread",
        kind: ArgKind::Value,
        cmd_patterns: &["/T"],
        short_patterns: &["-t"],
        long_patterns: &["--thread"],
    },
    ArgDef {
        canonical: "mft",
        kind: ArgKind::Flag,
        cmd_patterns: &["/MFT"],
        short_patterns: &["-M"],
        long_patterns: &["--mft"],
    },
    ArgDef {
        canonical: "disk-usage",
        kind: ArgKind::Flag,
        cmd_patterns: &["/DU"],
        short_patterns: &["-u"],
        long_patterns: &["--disk-usage"],
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
pub struct CliParser {
    args: Vec<String>,
    position: usize,
    has_seen_option: bool,
    seen_canonical_names: HashSet<String>,
}

impl CliParser {
    /// 从参数列表创建解析器
    ///
    /// # 参数
    ///
    /// * `args` - 命令行参数列表（不包含程序名）
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            position: 0,
            has_seen_option: false,
            seen_canonical_names: HashSet::new(),
        }
    }

    /// 从环境参数创建解析器
    ///
    /// 自动跳过程序名（第一个参数）。
    pub fn from_env() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        Self::new(args)
    }

    /// 解析命令行参数
    ///
    /// # 返回值
    ///
    /// 成功返回 `ParseResult`，失败返回 `CliError`。
    ///
    /// # 错误
    ///
    /// - `CliError::UnknownArgument` - 遇到未知参数
    /// - `CliError::MissingValue` - 需要值的参数缺少值
    /// - `CliError::InvalidValue` - 参数值无效
    /// - `CliError::DuplicateArgument` - 参数重复
    /// - `CliError::MultiplePaths` - 指定了多个路径
    /// - `CliError::PathAfterOptions` - 路径出现在选项之后
    /// - `CliError::MftIncompatible` - MFT 模式下使用了不兼容的参数
    pub fn parse(mut self) -> Result<ParseResult, CliError> {
        let mut config = Config::default();
        let mut collected_paths: Vec<String> = Vec::new();

        while self.position < self.args.len() {
            let current_arg = self.args[self.position].clone();

            if Self::is_option_like(&current_arg) {
                self.has_seen_option = true;

                let matched = self.try_match_argument(&current_arg)?;
                self.register_canonical_name(matched.definition.canonical)?;
                self.apply_to_config(&mut config, &matched)?;

                if matched.definition.canonical == "help" {
                    return Ok(ParseResult::Help);
                }
                if matched.definition.canonical == "version" {
                    return Ok(ParseResult::Version);
                }
            } else {
                if self.has_seen_option {
                    return Err(CliError::PathAfterOptions);
                }
                collected_paths.push(current_arg);
            }

            self.position += 1;
        }

        self.validate_paths(&collected_paths, &mut config)?;
        self.validate_mft_compatibility(&config)?;

        Ok(ParseResult::Config(config))
    }

    /// 判断字符串是否看起来像选项参数
    fn is_option_like(arg: &str) -> bool {
        arg.starts_with('-') || arg.starts_with('/')
    }

    /// 尝试匹配参数
    fn try_match_argument(&mut self, arg: &str) -> Result<MatchedArg, CliError> {
        for def in ARG_DEFINITIONS {
            if let Some(matched) = self.try_match_definition(arg, def)? {
                return Ok(matched);
            }
        }
        Err(CliError::UnknownArgument(arg.to_string()))
    }

    /// 尝试将参数与特定定义匹配
    fn try_match_definition(
        &mut self,
        arg: &str,
        def: &'static ArgDef,
    ) -> Result<Option<MatchedArg>, CliError> {
        let arg_upper = arg.to_uppercase();

        for pattern in def.cmd_patterns {
            if arg_upper == pattern.to_uppercase() {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }
        }

        for pattern in def.short_patterns {
            if arg == *pattern {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }
        }

        for pattern in def.long_patterns {
            if arg == *pattern {
                let value = self.consume_value_if_required(def, arg)?;
                return Ok(Some(MatchedArg {
                    definition: def,
                    value,
                }));
            }

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
            return Err(CliError::MissingValue(arg.to_string()));
        }

        let next_arg = &self.args[next_position];
        if Self::is_option_like(next_arg) {
            return Err(CliError::MissingValue(arg.to_string()));
        }

        self.position += 1;
        Ok(Some(next_arg.clone()))
    }

    /// 注册已使用的规范名称，检测重复
    fn register_canonical_name(&mut self, canonical: &str) -> Result<(), CliError> {
        if !self.seen_canonical_names.insert(canonical.to_string()) {
            return Err(CliError::DuplicateArgument(canonical.to_string()));
        }
        Ok(())
    }

    /// 将匹配的参数应用到配置
    fn apply_to_config(&self, config: &mut Config, matched: &MatchedArg) -> Result<(), CliError> {
        let canonical = matched.definition.canonical;

        match canonical {
            "help" | "version" => {}
            "files" => config.show_files = true,
            "full-path" => config.show_full_path = true,
            "size" => config.show_size = true,
            "human-readable" => config.human_readable = true,
            "date" => config.show_date = true,
            "quote" => config.quote_names = true,
            "ascii" => config.use_ascii = true,
            "no-indent" => config.no_indent = true,
            "reverse" => config.reverse = true,
            "dirs-first" => config.dirs_first = true,
            "ignore-case" => config.ignore_case = true,
            "prune" => config.prune = true,
            "gitignore" => config.gitignore = true,
            "no-report" => config.no_report = true,
            "no-header" => config.no_header = true,
            "silent" => config.silent = true,
            "mft" => config.use_mft = true,
            "disk-usage" => config.disk_usage = true,
            "sort" => {
                let value = matched.value.as_ref().expect("sort 参数需要值");
                config.sort_by = Some(SortKey::from_str(value).ok_or_else(|| {
                    CliError::InvalidValue(canonical.to_string(), value.clone())
                })?);
            }
            "level" => {
                let value = matched.value.as_ref().expect("level 参数需要值");
                config.level = Some(value.parse::<u32>().map_err(|_| {
                    CliError::InvalidValue(canonical.to_string(), value.clone())
                })?);
            }
            "include" => {
                config.include_pattern = matched.value.clone();
            }
            "exclude" => {
                config.exclude_pattern = matched.value.clone();
            }
            "output" => {
                config.output_file = matched.value.as_ref().map(PathBuf::from);
            }
            "thread" => {
                let value = matched.value.as_ref().expect("thread 参数需要值");
                config.thread_count = value.parse::<u32>().map_err(|_| {
                    CliError::InvalidValue(canonical.to_string(), value.clone())
                })?;
            }
            _ => {}
        }

        Ok(())
    }

    /// 验证路径参数
    fn validate_paths(
        &self,
        paths: &[String],
        config: &mut Config,
    ) -> Result<(), CliError> {
        match paths.len() {
            0 => Ok(()),
            1 => {
                config.path = PathBuf::from(&paths[0]);
                Ok(())
            }
            _ => Err(CliError::MultiplePaths {
                paths: paths.to_vec(),
            }),
        }
    }

    /// 验证 MFT 模式兼容性
    fn validate_mft_compatibility(&self, config: &Config) -> Result<(), CliError> {
        if !config.use_mft {
            return Ok(());
        }

        for canonical in &self.seen_canonical_names {
            if MFT_INCOMPATIBLE_ARGS.contains(&canonical.as_str()) {
                return Err(CliError::MftIncompatible(canonical.clone()));
            }
        }

        Ok(())
    }
}

// ============================================================================
// 帮助与版本信息
// ============================================================================

/// 打印帮助信息
pub fn print_help() {
    println!(
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
  --quote, -q, /Q             用双引号包裹文件名
  --dirs-first, -D, /DF       目录优先显示
  --disk-usage, -u, /DU       显示目录累计大小
  --ignore-case, -c, /IC      匹配时忽略大小写
  --no-report, -n, /NR        不显示末尾统计信息
  --prune, -P, /P             修剪空目录
  --sort, -S, /SO <KEY>       指定排序方式（name, size, mtime, ctime）
  --no-header, -N, /NH        不显示卷信息与头部报告
  --silent, -l, /SI           终端静默
  --output, -o, /O <FILE>     将结果输出至文件
  --thread, -t, /T <N>        扫描线程数（默认 8）
  --mft, -M, /MFT             使用 MFT（需管理员权限）
  --gitignore, -g, /G         遵循 .gitignore"#
    );
}

/// 打印版本信息
pub fn print_version() {
    println!(
        r#"tree++ version 0.1.0

A Better tree command for Windows.

author: WaterRun
link: https://github.com/Water-Run/treepp"#
    );
}

// ============================================================================
// 主函数
// ============================================================================

fn main() {
    let parser = CliParser::from_env();

    match parser.parse() {
        Ok(ParseResult::Help) => print_help(),
        Ok(ParseResult::Version) => print_version(),
        Ok(ParseResult::Config(config)) => {
            println!("解析成功！");
            println!("{config:#?}");
        }
        Err(e) => {
            eprintln!("错误: {e}");
            std::process::exit(1);
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 从字符串切片创建解析器并解析
    fn parse_args(args: &[&str]) -> Result<ParseResult, CliError> {
        let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        CliParser::new(args).parse()
    }

    /// 解析并期望返回 Config
    fn parse_to_config(args: &[&str]) -> Result<Config, CliError> {
        match parse_args(args)? {
            ParseResult::Config(c) => Ok(c),
            other => panic!("期望返回 Config，实际返回 {other:?}"),
        }
    }

    mod basic_functionality {
        use super::*;

        #[test]
        fn empty_args_returns_default_config() {
            let config = parse_to_config(&[]).unwrap();
            assert_eq!(config, Config::default());
        }

        #[test]
        fn path_only() {
            let config = parse_to_config(&["D:\\test\\path"]).unwrap();
            assert_eq!(config.path, PathBuf::from("D:\\test\\path"));
        }

        #[test]
        fn relative_path() {
            let config = parse_to_config(&["./src"]).unwrap();
            assert_eq!(config.path, PathBuf::from("./src"));
        }

        #[test]
        fn path_with_dots() {
            let config = parse_to_config(&["../parent/child"]).unwrap();
            assert_eq!(config.path, PathBuf::from("../parent/child"));
        }

        #[test]
        fn path_with_spaces() {
            let config = parse_to_config(&["C:\\Program Files\\Test"]).unwrap();
            assert_eq!(config.path, PathBuf::from("C:\\Program Files\\Test"));
        }

        #[test]
        fn unicode_path() {
            let config = parse_to_config(&["D:\\项目\\测试"]).unwrap();
            assert_eq!(config.path, PathBuf::from("D:\\项目\\测试"));
        }

        #[test]
        fn default_thread_count() {
            let config = parse_to_config(&[]).unwrap();
            assert_eq!(config.thread_count, DEFAULT_THREAD_COUNT);
        }

        #[test]
        fn default_path_is_current_dir() {
            let config = parse_to_config(&[]).unwrap();
            assert_eq!(config.path, PathBuf::from("."));
        }
    }

    mod help_and_version {
        use super::*;

        #[test]
        fn help_gnu_long() {
            assert_eq!(parse_args(&["--help"]).unwrap(), ParseResult::Help);
        }

        #[test]
        fn help_unix_short() {
            assert_eq!(parse_args(&["-h"]).unwrap(), ParseResult::Help);
        }

        #[test]
        fn help_cmd_style() {
            assert_eq!(parse_args(&["/?"]).unwrap(), ParseResult::Help);
        }

        #[test]
        fn version_gnu_long() {
            assert_eq!(parse_args(&["--version"]).unwrap(), ParseResult::Version);
        }

        #[test]
        fn version_unix_short() {
            assert_eq!(parse_args(&["-v"]).unwrap(), ParseResult::Version);
        }

        #[test]
        fn version_cmd_style_upper() {
            assert_eq!(parse_args(&["/V"]).unwrap(), ParseResult::Version);
        }

        #[test]
        fn version_cmd_style_lower() {
            assert_eq!(parse_args(&["/v"]).unwrap(), ParseResult::Version);
        }
    }

    mod style_equivalence {
        use super::*;

        #[test]
        fn files_equivalence() {
            let c1 = parse_to_config(&["--files"]).unwrap();
            let c2 = parse_to_config(&["-f"]).unwrap();
            let c3 = parse_to_config(&["/F"]).unwrap();
            let c4 = parse_to_config(&["/f"]).unwrap();

            assert!(c1.show_files);
            assert!(c2.show_files);
            assert!(c3.show_files);
            assert!(c4.show_files);
        }

        #[test]
        fn ascii_equivalence() {
            let c1 = parse_to_config(&["--ascii"]).unwrap();
            let c2 = parse_to_config(&["-a"]).unwrap();
            let c3 = parse_to_config(&["/A"]).unwrap();
            let c4 = parse_to_config(&["/a"]).unwrap();

            assert!(c1.use_ascii);
            assert!(c2.use_ascii);
            assert!(c3.use_ascii);
            assert!(c4.use_ascii);
        }

        #[test]
        fn level_equivalence() {
            let c1 = parse_to_config(&["--level", "3"]).unwrap();
            let c2 = parse_to_config(&["-L", "3"]).unwrap();
            let c3 = parse_to_config(&["/L", "3"]).unwrap();
            let c4 = parse_to_config(&["/l", "3"]).unwrap();

            assert_eq!(c1.level, Some(3));
            assert_eq!(c2.level, Some(3));
            assert_eq!(c3.level, Some(3));
            assert_eq!(c4.level, Some(3));
        }

        #[test]
        fn sort_equivalence() {
            let c1 = parse_to_config(&["--sort", "size"]).unwrap();
            let c2 = parse_to_config(&["-S", "size"]).unwrap();
            let c3 = parse_to_config(&["/SO", "size"]).unwrap();
            let c4 = parse_to_config(&["/so", "SIZE"]).unwrap();

            assert_eq!(c1.sort_by, Some(SortKey::Size));
            assert_eq!(c2.sort_by, Some(SortKey::Size));
            assert_eq!(c3.sort_by, Some(SortKey::Size));
            assert_eq!(c4.sort_by, Some(SortKey::Size));
        }

        #[test]
        fn full_path_equivalence() {
            let c1 = parse_to_config(&["--full-path"]).unwrap();
            let c2 = parse_to_config(&["-p"]).unwrap();
            let c3 = parse_to_config(&["/FP"]).unwrap();

            assert!(c1.show_full_path);
            assert!(c2.show_full_path);
            assert!(c3.show_full_path);
        }

        #[test]
        fn human_readable_equivalence() {
            let c1 = parse_to_config(&["--human-readable"]).unwrap();
            let c2 = parse_to_config(&["-H"]).unwrap();
            let c3 = parse_to_config(&["/HR"]).unwrap();

            assert!(c1.human_readable);
            assert!(c2.human_readable);
            assert!(c3.human_readable);
        }

        #[test]
        fn date_equivalence() {
            let c1 = parse_to_config(&["--date"]).unwrap();
            let c2 = parse_to_config(&["-d"]).unwrap();
            let c3 = parse_to_config(&["/DT"]).unwrap();

            assert!(c1.show_date);
            assert!(c2.show_date);
            assert!(c3.show_date);
        }

        #[test]
        fn quote_equivalence() {
            let c1 = parse_to_config(&["--quote"]).unwrap();
            let c2 = parse_to_config(&["-q"]).unwrap();
            let c3 = parse_to_config(&["/Q"]).unwrap();

            assert!(c1.quote_names);
            assert!(c2.quote_names);
            assert!(c3.quote_names);
        }

        #[test]
        fn no_indent_equivalence() {
            let c1 = parse_to_config(&["--no-indent"]).unwrap();
            let c2 = parse_to_config(&["-i"]).unwrap();
            let c3 = parse_to_config(&["/NI"]).unwrap();

            assert!(c1.no_indent);
            assert!(c2.no_indent);
            assert!(c3.no_indent);
        }

        #[test]
        fn reverse_equivalence() {
            let c1 = parse_to_config(&["--reverse"]).unwrap();
            let c2 = parse_to_config(&["-r"]).unwrap();
            let c3 = parse_to_config(&["/R"]).unwrap();

            assert!(c1.reverse);
            assert!(c2.reverse);
            assert!(c3.reverse);
        }

        #[test]
        fn dirs_first_equivalence() {
            let c1 = parse_to_config(&["--dirs-first"]).unwrap();
            let c2 = parse_to_config(&["-D"]).unwrap();
            let c3 = parse_to_config(&["/DF"]).unwrap();

            assert!(c1.dirs_first);
            assert!(c2.dirs_first);
            assert!(c3.dirs_first);
        }

        #[test]
        fn ignore_case_equivalence() {
            let c1 = parse_to_config(&["--ignore-case"]).unwrap();
            let c2 = parse_to_config(&["-c"]).unwrap();
            let c3 = parse_to_config(&["/IC"]).unwrap();

            assert!(c1.ignore_case);
            assert!(c2.ignore_case);
            assert!(c3.ignore_case);
        }

        #[test]
        fn prune_equivalence() {
            let c1 = parse_to_config(&["--prune"]).unwrap();
            let c2 = parse_to_config(&["-P"]).unwrap();
            let c3 = parse_to_config(&["/P"]).unwrap();

            assert!(c1.prune);
            assert!(c2.prune);
            assert!(c3.prune);
        }

        #[test]
        fn gitignore_equivalence() {
            let c1 = parse_to_config(&["--gitignore"]).unwrap();
            let c2 = parse_to_config(&["-g"]).unwrap();
            let c3 = parse_to_config(&["/G"]).unwrap();

            assert!(c1.gitignore);
            assert!(c2.gitignore);
            assert!(c3.gitignore);
        }

        #[test]
        fn no_report_equivalence() {
            let c1 = parse_to_config(&["--no-report"]).unwrap();
            let c2 = parse_to_config(&["-n"]).unwrap();
            let c3 = parse_to_config(&["/NR"]).unwrap();

            assert!(c1.no_report);
            assert!(c2.no_report);
            assert!(c3.no_report);
        }

        #[test]
        fn no_header_equivalence() {
            let c1 = parse_to_config(&["--no-header"]).unwrap();
            let c2 = parse_to_config(&["-N"]).unwrap();
            let c3 = parse_to_config(&["/NH"]).unwrap();

            assert!(c1.no_header);
            assert!(c2.no_header);
            assert!(c3.no_header);
        }

        #[test]
        fn silent_equivalence() {
            let c1 = parse_to_config(&["--silent"]).unwrap();
            let c2 = parse_to_config(&["-l"]).unwrap();
            let c3 = parse_to_config(&["/SI"]).unwrap();

            assert!(c1.silent);
            assert!(c2.silent);
            assert!(c3.silent);
        }

        #[test]
        fn mft_equivalence() {
            let c1 = parse_to_config(&["--mft"]).unwrap();
            let c2 = parse_to_config(&["-M"]).unwrap();
            let c3 = parse_to_config(&["/MFT"]).unwrap();

            assert!(c1.use_mft);
            assert!(c2.use_mft);
            assert!(c3.use_mft);
        }

        #[test]
        fn disk_usage_equivalence() {
            let c1 = parse_to_config(&["--disk-usage"]).unwrap();
            let c2 = parse_to_config(&["-u"]).unwrap();
            let c3 = parse_to_config(&["/DU"]).unwrap();

            assert!(c1.disk_usage);
            assert!(c2.disk_usage);
            assert!(c3.disk_usage);
        }

        #[test]
        fn size_equivalence() {
            let c1 = parse_to_config(&["--size"]).unwrap();
            let c2 = parse_to_config(&["-s"]).unwrap();
            let c3 = parse_to_config(&["/S"]).unwrap();

            assert!(c1.show_size);
            assert!(c2.show_size);
            assert!(c3.show_size);
        }
    }

    mod case_sensitivity {
        use super::*;

        #[test]
        fn cmd_style_case_insensitive() {
            let c1 = parse_to_config(&["/FP"]).unwrap();
            let c2 = parse_to_config(&["/fp"]).unwrap();
            let c3 = parse_to_config(&["/Fp"]).unwrap();
            let c4 = parse_to_config(&["/fP"]).unwrap();

            assert!(c1.show_full_path);
            assert!(c2.show_full_path);
            assert!(c3.show_full_path);
            assert!(c4.show_full_path);
        }

        #[test]
        fn cmd_style_multi_char_case_insensitive() {
            let c1 = parse_to_config(&["/HR"]).unwrap();
            let c2 = parse_to_config(&["/hr"]).unwrap();
            let c3 = parse_to_config(&["/Hr"]).unwrap();

            assert!(c1.human_readable);
            assert!(c2.human_readable);
            assert!(c3.human_readable);
        }

        #[test]
        fn unix_short_case_sensitive_valid() {
            let config = parse_to_config(&["-f"]).unwrap();
            assert!(config.show_files);
        }

        #[test]
        fn unix_short_case_sensitive_invalid() {
            let result = parse_args(&["-F"]);
            assert!(matches!(result, Err(CliError::UnknownArgument(_))));
        }

        #[test]
        fn gnu_long_case_sensitive_valid() {
            let config = parse_to_config(&["--files"]).unwrap();
            assert!(config.show_files);
        }

        #[test]
        fn gnu_long_case_sensitive_invalid() {
            let result = parse_args(&["--FILES"]);
            assert!(matches!(result, Err(CliError::UnknownArgument(_))));
        }

        #[test]
        fn gnu_long_case_sensitive_mixed_case() {
            let result = parse_args(&["--Files"]);
            assert!(matches!(result, Err(CliError::UnknownArgument(_))));
        }
    }

    mod valued_arguments {
        use super::*;

        #[test]
        fn level_with_value() {
            let config = parse_to_config(&["/L", "5"]).unwrap();
            assert_eq!(config.level, Some(5));
        }

        #[test]
        fn level_with_equals_syntax() {
            let config = parse_to_config(&["--level=7"]).unwrap();
            assert_eq!(config.level, Some(7));
        }

        #[test]
        fn level_zero() {
            let config = parse_to_config(&["/L", "0"]).unwrap();
            assert_eq!(config.level, Some(0));
        }

        #[test]
        fn output_with_value() {
            let config = parse_to_config(&["--output", "tree.json"]).unwrap();
            assert_eq!(config.output_file, Some(PathBuf::from("tree.json")));
        }

        #[test]
        fn output_various_extensions() {
            assert_eq!(
                parse_to_config(&["-o", "output.txt"]).unwrap().output_file,
                Some(PathBuf::from("output.txt"))
            );
            assert_eq!(
                parse_to_config(&["-o", "data.json"]).unwrap().output_file,
                Some(PathBuf::from("data.json"))
            );
            assert_eq!(
                parse_to_config(&["-o", "config.yml"]).unwrap().output_file,
                Some(PathBuf::from("config.yml"))
            );
            assert_eq!(
                parse_to_config(&["-o", "settings.toml"])
                    .unwrap()
                    .output_file,
                Some(PathBuf::from("settings.toml"))
            );
        }

        #[test]
        fn thread_with_value() {
            let config = parse_to_config(&["/T", "32"]).unwrap();
            assert_eq!(config.thread_count, 32);
        }

        #[test]
        fn thread_one() {
            let config = parse_to_config(&["/T", "1"]).unwrap();
            assert_eq!(config.thread_count, 1);
        }

        #[test]
        fn include_pattern() {
            let config = parse_to_config(&["--include", "*.rs"]).unwrap();
            assert_eq!(config.include_pattern, Some("*.rs".to_string()));
        }

        #[test]
        fn include_empty_pattern() {
            let config = parse_to_config(&["--include", ""]).unwrap();
            assert_eq!(config.include_pattern, Some(String::new()));
        }

        #[test]
        fn include_pattern_with_spaces() {
            let config = parse_to_config(&["--include", "my file*.txt"]).unwrap();
            assert_eq!(config.include_pattern, Some("my file*.txt".to_string()));
        }

        #[test]
        fn exclude_pattern() {
            let config = parse_to_config(&["/X", "*.md"]).unwrap();
            assert_eq!(config.exclude_pattern, Some("*.md".to_string()));
        }

        #[test]
        fn sort_name() {
            let config = parse_to_config(&["--sort", "name"]).unwrap();
            assert_eq!(config.sort_by, Some(SortKey::Name));
        }

        #[test]
        fn sort_size() {
            let config = parse_to_config(&["--sort", "size"]).unwrap();
            assert_eq!(config.sort_by, Some(SortKey::Size));
        }

        #[test]
        fn sort_mtime() {
            let config = parse_to_config(&["--sort", "mtime"]).unwrap();
            assert_eq!(config.sort_by, Some(SortKey::Mtime));
        }

        #[test]
        fn sort_ctime() {
            let config = parse_to_config(&["--sort", "ctime"]).unwrap();
            assert_eq!(config.sort_by, Some(SortKey::Ctime));
        }

        #[test]
        fn sort_keys_case_insensitive() {
            assert_eq!(
                parse_to_config(&["--sort", "NAME"]).unwrap().sort_by,
                Some(SortKey::Name)
            );
            assert_eq!(
                parse_to_config(&["--sort", "Size"]).unwrap().sort_by,
                Some(SortKey::Size)
            );
            assert_eq!(
                parse_to_config(&["--sort", "MTIME"]).unwrap().sort_by,
                Some(SortKey::Mtime)
            );
            assert_eq!(
                parse_to_config(&["--sort", "CTime"]).unwrap().sort_by,
                Some(SortKey::Ctime)
            );
        }

        #[test]
        fn output_with_equals_syntax() {
            let config = parse_to_config(&["--output=result.json"]).unwrap();
            assert_eq!(config.output_file, Some(PathBuf::from("result.json")));
        }
    }

    mod mixed_arguments {
        use super::*;

        #[test]
        fn mixed_styles() {
            let config = parse_to_config(&["/F", "-a", "--no-report"]).unwrap();
            assert!(config.show_files);
            assert!(config.use_ascii);
            assert!(config.no_report);
        }

        #[test]
        fn path_with_options() {
            let config = parse_to_config(&["D:\\project", "/F", "--ascii"]).unwrap();
            assert_eq!(config.path, PathBuf::from("D:\\project"));
            assert!(config.show_files);
            assert!(config.use_ascii);
        }

        #[test]
        fn multiple_flags() {
            let config = parse_to_config(&[
                "/F", "/A", "/S", "/HR", "/DT", "/Q", "/R", "/DF", "/NR", "/NH",
            ])
                .unwrap();

            assert!(config.show_files);
            assert!(config.use_ascii);
            assert!(config.show_size);
            assert!(config.human_readable);
            assert!(config.show_date);
            assert!(config.quote_names);
            assert!(config.reverse);
            assert!(config.dirs_first);
            assert!(config.no_report);
            assert!(config.no_header);
        }

        #[test]
        fn complex_combination() {
            let config = parse_to_config(&[
                "C:\\Users",
                "--files",
                "/L",
                "3",
                "-a",
                "--exclude",
                "node_modules",
                "/T",
                "16",
            ])
                .unwrap();

            assert_eq!(config.path, PathBuf::from("C:\\Users"));
            assert!(config.show_files);
            assert_eq!(config.level, Some(3));
            assert!(config.use_ascii);
            assert_eq!(config.exclude_pattern, Some("node_modules".to_string()));
            assert_eq!(config.thread_count, 16);
        }

        #[test]
        fn all_display_flags() {
            let config =
                parse_to_config(&["/F", "/FP", "/S", "/HR", "/DT", "/Q", "/NI"]).unwrap();

            assert!(config.show_files);
            assert!(config.show_full_path);
            assert!(config.show_size);
            assert!(config.human_readable);
            assert!(config.show_date);
            assert!(config.quote_names);
            assert!(config.no_indent);
        }

        #[test]
        fn sorting_and_filtering_combination() {
            let config =
                parse_to_config(&["/SO", "size", "/R", "/DF", "/IC", "/P"]).unwrap();

            assert_eq!(config.sort_by, Some(SortKey::Size));
            assert!(config.reverse);
            assert!(config.dirs_first);
            assert!(config.ignore_case);
            assert!(config.prune);
        }

        #[test]
        fn output_control_combination() {
            let config = parse_to_config(&["/NR", "/NH", "/SI", "/O", "output.json"]).unwrap();

            assert!(config.no_report);
            assert!(config.no_header);
            assert!(config.silent);
            assert_eq!(config.output_file, Some(PathBuf::from("output.json")));
        }

        #[test]
        fn pattern_matching_combination() {
            let config = parse_to_config(&["/M", "*.rs", "/X", "test_*", "/IC", "/G"]).unwrap();

            assert_eq!(config.include_pattern, Some("*.rs".to_string()));
            assert_eq!(config.exclude_pattern, Some("test_*".to_string()));
            assert!(config.ignore_case);
            assert!(config.gitignore);
        }
    }

    mod error_handling {
        use super::*;

        #[test]
        fn unknown_cmd_argument() {
            let result = parse_args(&["/Z"]);
            assert!(matches!(result, Err(CliError::UnknownArgument(ref s)) if s == "/Z"));
        }

        #[test]
        fn unknown_gnu_argument() {
            let result = parse_args(&["--unknown"]);
            assert!(matches!(result, Err(CliError::UnknownArgument(ref s)) if s == "--unknown"));
        }

        #[test]
        fn unknown_unix_argument() {
            let result = parse_args(&["-z"]);
            assert!(matches!(result, Err(CliError::UnknownArgument(ref s)) if s == "-z"));
        }

        #[test]
        fn missing_value_cmd_style() {
            let result = parse_args(&["/L"]);
            assert!(matches!(result, Err(CliError::MissingValue(ref s)) if s == "/L"));
        }

        #[test]
        fn missing_value_gnu_style() {
            let result = parse_args(&["--level"]);
            assert!(matches!(result, Err(CliError::MissingValue(ref s)) if s == "--level"));
        }

        #[test]
        fn missing_value_output() {
            let result = parse_args(&["--output"]);
            assert!(matches!(result, Err(CliError::MissingValue(ref s)) if s == "--output"));
        }

        #[test]
        fn missing_value_when_next_is_option() {
            let result = parse_args(&["/L", "/F"]);
            assert!(matches!(result, Err(CliError::MissingValue(ref s)) if s == "/L"));
        }

        #[test]
        fn missing_value_thread() {
            let result = parse_args(&["--thread"]);
            assert!(matches!(result, Err(CliError::MissingValue(_))));
        }

        #[test]
        fn invalid_level_value() {
            let result = parse_args(&["/L", "abc"]);
            assert!(matches!(
                result,
                Err(CliError::InvalidValue(ref name, ref val)) if name == "level" && val == "abc"
            ));
        }

        #[test]
        fn invalid_thread_value_negative_looking() {
            let result = parse_args(&["--thread", "-5"]);
            assert!(matches!(result, Err(CliError::MissingValue(_))));
        }

        #[test]
        fn invalid_sort_value() {
            let result = parse_args(&["--sort", "invalid"]);
            assert!(matches!(
                result,
                Err(CliError::InvalidValue(ref name, ref val)) if name == "sort" && val == "invalid"
            ));
        }

        #[test]
        fn invalid_thread_value_non_numeric() {
            let result = parse_args(&["/T", "fast"]);
            assert!(matches!(
                result,
                Err(CliError::InvalidValue(ref name, ref val)) if name == "thread" && val == "fast"
            ));
        }

        #[test]
        fn duplicate_same_style() {
            let result = parse_args(&["-f", "-f"]);
            assert!(matches!(
                result,
                Err(CliError::DuplicateArgument(ref s)) if s == "files"
            ));
        }

        #[test]
        fn duplicate_different_styles() {
            let result = parse_args(&["/F", "--files"]);
            assert!(matches!(
                result,
                Err(CliError::DuplicateArgument(ref s)) if s == "files"
            ));
        }

        #[test]
        fn duplicate_valued_argument() {
            let result = parse_args(&["/L", "3", "-L", "5"]);
            assert!(matches!(
                result,
                Err(CliError::DuplicateArgument(ref s)) if s == "level"
            ));
        }

        #[test]
        fn multiple_paths_two() {
            let result = parse_args(&["D:\\a", "D:\\b"]);
            assert!(matches!(result, Err(CliError::MultiplePaths { ref paths }) if paths.len() == 2));
        }

        #[test]
        fn multiple_paths_three() {
            let result = parse_args(&["path1", "path2", "path3"]);
            assert!(matches!(result, Err(CliError::MultiplePaths { ref paths }) if paths.len() == 3));
        }

        #[test]
        fn path_after_options_cmd_style() {
            let result = parse_args(&["/F", "D:\\path"]);
            assert!(matches!(result, Err(CliError::PathAfterOptions)));
        }

        #[test]
        fn path_after_options_gnu_style() {
            let result = parse_args(&["--files", "C:\\test"]);
            assert!(matches!(result, Err(CliError::PathAfterOptions)));
        }

        #[test]
        fn path_after_valued_option() {
            let result = parse_args(&["/L", "3", "D:\\path"]);
            assert!(matches!(result, Err(CliError::PathAfterOptions)));
        }
    }

    mod mft_compatibility {
        use super::*;

        #[test]
        fn mft_alone_works() {
            let config = parse_to_config(&["/MFT"]).unwrap();
            assert!(config.use_mft);
        }

        #[test]
        fn mft_with_compatible_options() {
            let config = parse_to_config(&["/MFT", "/F", "/A", "/S"]).unwrap();
            assert!(config.use_mft);
            assert!(config.show_files);
            assert!(config.use_ascii);
            assert!(config.show_size);
        }

        #[test]
        fn mft_with_display_options() {
            let config = parse_to_config(&["/MFT", "/HR", "/DT", "/Q"]).unwrap();
            assert!(config.use_mft);
            assert!(config.human_readable);
            assert!(config.show_date);
            assert!(config.quote_names);
        }

        #[test]
        fn mft_incompatible_prune() {
            let result = parse_args(&["/MFT", "/P"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "prune"
            ));
        }

        #[test]
        fn mft_incompatible_level() {
            let result = parse_args(&["/MFT", "/L", "3"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "level"
            ));
        }

        #[test]
        fn mft_incompatible_gitignore() {
            let result = parse_args(&["/MFT", "/G"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "gitignore"
            ));
        }

        #[test]
        fn mft_incompatible_include() {
            let result = parse_args(&["/MFT", "/M", "*.rs"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "include"
            ));
        }

        #[test]
        fn mft_incompatible_exclude() {
            let result = parse_args(&["/MFT", "/X", "*.md"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "exclude"
            ));
        }

        #[test]
        fn mft_incompatible_disk_usage() {
            let result = parse_args(&["/MFT", "/DU"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "disk-usage"
            ));
        }

        #[test]
        fn mft_incompatible_sort() {
            let result = parse_args(&["/MFT", "/SO", "name"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "sort"
            ));
        }

        #[test]
        fn mft_incompatible_reverse() {
            let result = parse_args(&["/MFT", "/R"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "reverse"
            ));
        }

        #[test]
        fn mft_at_end_with_incompatible() {
            let result = parse_args(&["/P", "/MFT"]);
            assert!(matches!(
                result,
                Err(CliError::MftIncompatible(ref s)) if s == "prune"
            ));
        }

        #[test]
        fn mft_multiple_incompatible() {
            let result = parse_args(&["/MFT", "/P", "/L", "3"]);
            assert!(matches!(result, Err(CliError::MftIncompatible(_))));
        }
    }

    mod all_flags_individual {
        use super::*;

        #[test]
        fn files() {
            assert!(parse_to_config(&["/F"]).unwrap().show_files);
        }

        #[test]
        fn full_path() {
            assert!(parse_to_config(&["/FP"]).unwrap().show_full_path);
        }

        #[test]
        fn size() {
            assert!(parse_to_config(&["/S"]).unwrap().show_size);
        }

        #[test]
        fn human_readable() {
            assert!(parse_to_config(&["/HR"]).unwrap().human_readable);
        }

        #[test]
        fn date() {
            assert!(parse_to_config(&["/DT"]).unwrap().show_date);
        }

        #[test]
        fn quote() {
            assert!(parse_to_config(&["/Q"]).unwrap().quote_names);
        }

        #[test]
        fn ascii() {
            assert!(parse_to_config(&["/A"]).unwrap().use_ascii);
        }

        #[test]
        fn no_indent() {
            assert!(parse_to_config(&["/NI"]).unwrap().no_indent);
        }

        #[test]
        fn reverse() {
            assert!(parse_to_config(&["/R"]).unwrap().reverse);
        }

        #[test]
        fn dirs_first() {
            assert!(parse_to_config(&["/DF"]).unwrap().dirs_first);
        }

        #[test]
        fn ignore_case() {
            assert!(parse_to_config(&["/IC"]).unwrap().ignore_case);
        }

        #[test]
        fn prune() {
            assert!(parse_to_config(&["/P"]).unwrap().prune);
        }

        #[test]
        fn gitignore() {
            assert!(parse_to_config(&["/G"]).unwrap().gitignore);
        }

        #[test]
        fn no_report() {
            assert!(parse_to_config(&["/NR"]).unwrap().no_report);
        }

        #[test]
        fn no_header() {
            assert!(parse_to_config(&["/NH"]).unwrap().no_header);
        }

        #[test]
        fn silent() {
            assert!(parse_to_config(&["/SI"]).unwrap().silent);
        }

        #[test]
        fn mft() {
            assert!(parse_to_config(&["/MFT"]).unwrap().use_mft);
        }

        #[test]
        fn disk_usage() {
            assert!(parse_to_config(&["/DU"]).unwrap().disk_usage);
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn level_large_value() {
            let config = parse_to_config(&["/L", "999999"]).unwrap();
            assert_eq!(config.level, Some(999_999));
        }

        #[test]
        fn thread_large_value() {
            let config = parse_to_config(&["/T", "1024"]).unwrap();
            assert_eq!(config.thread_count, 1024);
        }

        #[test]
        fn pattern_with_special_chars() {
            let config = parse_to_config(&["--include", "*.{rs,toml}"]).unwrap();
            assert_eq!(config.include_pattern, Some("*.{rs,toml}".to_string()));
        }

        #[test]
        fn pattern_with_glob_double_star() {
            let config = parse_to_config(&["--exclude", "**/node_modules/**"]).unwrap();
            assert_eq!(
                config.exclude_pattern,
                Some("**/node_modules/**".to_string())
            );
        }

        #[test]
        fn output_path_with_directory() {
            let config = parse_to_config(&["-o", "output/dir/file.json"]).unwrap();
            assert_eq!(
                config.output_file,
                Some(PathBuf::from("output/dir/file.json"))
            );
        }

        #[test]
        fn unicode_pattern() {
            let config = parse_to_config(&["--include", "文档*.txt"]).unwrap();
            assert_eq!(config.include_pattern, Some("文档*.txt".to_string()));
        }

        #[test]
        fn equals_syntax_with_empty_value() {
            let config = parse_to_config(&["--include="]).unwrap();
            assert_eq!(config.include_pattern, Some(String::new()));
        }

        #[test]
        fn equals_syntax_with_equals_in_value() {
            let config = parse_to_config(&["--include=a=b"]).unwrap();
            assert_eq!(config.include_pattern, Some("a=b".to_string()));
        }

        #[test]
        fn many_flags_at_once() {
            let config = parse_to_config(&[
                "/F", "/FP", "/S", "/HR", "/DT", "/Q", "/A", "/NI", "/R", "/DF", "/IC", "/P", "/G",
                "/NR", "/NH", "/SI", "/DU",
            ])
                .unwrap();

            assert!(config.show_files);
            assert!(config.show_full_path);
            assert!(config.show_size);
            assert!(config.human_readable);
            assert!(config.show_date);
            assert!(config.quote_names);
            assert!(config.use_ascii);
            assert!(config.no_indent);
            assert!(config.reverse);
            assert!(config.dirs_first);
            assert!(config.ignore_case);
            assert!(config.prune);
            assert!(config.gitignore);
            assert!(config.no_report);
            assert!(config.no_header);
            assert!(config.silent);
            assert!(config.disk_usage);
        }

        #[test]
        fn complex_real_world_scenario() {
            let config = parse_to_config(&[
                "D:\\Projects\\my-app",
                "--files",
                "-a",
                "/L",
                "5",
                "--exclude",
                "node_modules",
                "--exclude=target",
            ]);

            assert!(matches!(
                config,
                Err(CliError::DuplicateArgument(ref s)) if s == "exclude"
            ));
        }

        #[test]
        fn valid_complex_scenario_without_duplicates() {
            let config = parse_to_config(&[
                "D:\\Projects\\my-app",
                "--files",
                "-a",
                "/L",
                "5",
                "--exclude",
                "node_modules",
                "/SO",
                "mtime",
                "/R",
            ])
                .unwrap();

            assert_eq!(config.path, PathBuf::from("D:\\Projects\\my-app"));
            assert!(config.show_files);
            assert!(config.use_ascii);
            assert_eq!(config.level, Some(5));
            assert_eq!(config.exclude_pattern, Some("node_modules".to_string()));
            assert_eq!(config.sort_by, Some(SortKey::Mtime));
            assert!(config.reverse);
        }
    }

    mod sort_key {
        use super::*;

        #[test]
        fn from_str_valid_lowercase() {
            assert_eq!(SortKey::from_str("name"), Some(SortKey::Name));
            assert_eq!(SortKey::from_str("size"), Some(SortKey::Size));
            assert_eq!(SortKey::from_str("mtime"), Some(SortKey::Mtime));
            assert_eq!(SortKey::from_str("ctime"), Some(SortKey::Ctime));
        }

        #[test]
        fn from_str_valid_uppercase() {
            assert_eq!(SortKey::from_str("NAME"), Some(SortKey::Name));
            assert_eq!(SortKey::from_str("SIZE"), Some(SortKey::Size));
            assert_eq!(SortKey::from_str("MTIME"), Some(SortKey::Mtime));
            assert_eq!(SortKey::from_str("CTIME"), Some(SortKey::Ctime));
        }

        #[test]
        fn from_str_valid_mixed_case() {
            assert_eq!(SortKey::from_str("Name"), Some(SortKey::Name));
            assert_eq!(SortKey::from_str("SiZe"), Some(SortKey::Size));
            assert_eq!(SortKey::from_str("MTime"), Some(SortKey::Mtime));
            assert_eq!(SortKey::from_str("cTIME"), Some(SortKey::Ctime));
        }

        #[test]
        fn from_str_invalid() {
            assert_eq!(SortKey::from_str("invalid"), None);
            assert_eq!(SortKey::from_str(""), None);
            assert_eq!(SortKey::from_str("date"), None);
            assert_eq!(SortKey::from_str("atime"), None);
        }

        #[test]
        fn default_is_name() {
            assert_eq!(SortKey::default(), SortKey::Name);
        }
    }

    mod config_default {
        use super::*;

        #[test]
        fn all_booleans_false() {
            let config = Config::default();
            assert!(!config.show_files);
            assert!(!config.show_full_path);
            assert!(!config.show_size);
            assert!(!config.human_readable);
            assert!(!config.show_date);
            assert!(!config.quote_names);
            assert!(!config.use_ascii);
            assert!(!config.no_indent);
            assert!(!config.reverse);
            assert!(!config.dirs_first);
            assert!(!config.ignore_case);
            assert!(!config.prune);
            assert!(!config.gitignore);
            assert!(!config.no_report);
            assert!(!config.no_header);
            assert!(!config.silent);
            assert!(!config.use_mft);
            assert!(!config.disk_usage);
        }

        #[test]
        fn all_optionals_none() {
            let config = Config::default();
            assert!(config.sort_by.is_none());
            assert!(config.level.is_none());
            assert!(config.include_pattern.is_none());
            assert!(config.exclude_pattern.is_none());
            assert!(config.output_file.is_none());
        }

        #[test]
        fn default_values() {
            let config = Config::default();
            assert_eq!(config.path, PathBuf::from("."));
            assert_eq!(config.thread_count, DEFAULT_THREAD_COUNT);
        }
    }

    mod cli_error {
        use super::*;

        #[test]
        fn error_display_unknown_argument() {
            let err = CliError::UnknownArgument("/Z".to_string());
            assert_eq!(format!("{err}"), "未知参数: /Z");
        }

        #[test]
        fn error_display_missing_value() {
            let err = CliError::MissingValue("/L".to_string());
            assert_eq!(format!("{err}"), "参数 /L 需要一个值");
        }

        #[test]
        fn error_display_invalid_value() {
            let err = CliError::InvalidValue("level".to_string(), "abc".to_string());
            assert_eq!(format!("{err}"), "参数 level 的值无效: abc");
        }

        #[test]
        fn error_display_duplicate_argument() {
            let err = CliError::DuplicateArgument("files".to_string());
            assert_eq!(format!("{err}"), "参数重复: files");
        }

        #[test]
        fn error_display_multiple_paths() {
            let err = CliError::MultiplePaths {
                paths: vec!["a".to_string(), "b".to_string()],
            };
            assert!(format!("{err}").contains("只允许指定一个路径"));
        }

        #[test]
        fn error_display_path_after_options() {
            let err = CliError::PathAfterOptions;
            assert_eq!(format!("{err}"), "路径必须在所有选项之前指定");
        }

        #[test]
        fn error_display_mft_incompatible() {
            let err = CliError::MftIncompatible("prune".to_string());
            assert_eq!(format!("{err}"), "MFT 模式下不支持参数: prune");
        }

        #[test]
        fn errors_are_eq() {
            let err1 = CliError::UnknownArgument("/Z".to_string());
            let err2 = CliError::UnknownArgument("/Z".to_string());
            assert_eq!(err1, err2);
        }

        #[test]
        fn errors_are_clone() {
            let err = CliError::UnknownArgument("/Z".to_string());
            let cloned = err.clone();
            assert_eq!(err, cloned);
        }
    }
}