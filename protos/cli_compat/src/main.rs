//! # CLI 兼容性原型
//!
//! 验证 `tree++` 命令行参数解析与等价映射的正确性。
//! 支持三种参数风格混用：Windows CMD (`/F`)、Unix 短参数 (`-f`)、GNU 长参数 (`--files`)。

use std::env;
use std::path::PathBuf;

use thiserror::Error;

// ============================================================================
// 错误定义
// ============================================================================

/// CLI 解析过程中可能发生的错误
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CliError {
    #[error("未知参数: {0}")]
    UnknownArgument(String),

    #[error("参数 {0} 需要一个值")]
    MissingValue(String),

    #[error("参数 {0} 的值无效: {1}")]
    InvalidValue(String, String),

    #[error("参数重复: {0}")]
    DuplicateArgument(String),

    #[error("只允许指定一个路径，但发现多个: {0:?}")]
    MultiplePaths(Vec<String>),

    #[error("路径必须在所有选项之前指定")]
    PathAfterOptions,

    #[error("MFT 模式下不支持参数: {0}")]
    MftIncompatible(String),
}

// ============================================================================
// 排序键枚举
// ============================================================================

/// 排序方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Mtime,
    Ctime,
}

impl SortKey {
    /// 从字符串解析排序键
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

    /// 遵循 .gitignore (`/G`, `-g`, `--gitignore`)
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
            thread_count: 24,
            use_mft: false,
            disk_usage: false,
        }
    }
}

// ============================================================================
// 参数类型定义
// ============================================================================

/// 参数类型：标志型或带值型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    Flag,
    Value,
}

/// 参数定义
struct ArgDef {
    /// 规范名称（用于重复检测和错误消息）
    canonical: &'static str,
    /// 参数类型
    kind: ArgKind,
    /// Windows 风格 (`/X`)，大小写不敏感
    cmd_style: &'static [&'static str],
    /// Unix 短参数 (`-x`)，大小写敏感
    short_style: &'static [&'static str],
    /// GNU 长参数 (`--xxx`)，大小写敏感
    long_style: &'static [&'static str],
}

/// 所有支持的参数定义
const ARG_DEFS: &[ArgDef] = &[
    // 帮助与版本
    ArgDef {
        canonical: "help",
        kind: ArgKind::Flag,
        cmd_style: &["/?"],
        short_style: &["-h"],
        long_style: &["--help"],
    },
    ArgDef {
        canonical: "version",
        kind: ArgKind::Flag,
        cmd_style: &["/V"],
        short_style: &["-v"],
        long_style: &["--version"],
    },
    // 显示类
    ArgDef {
        canonical: "files",
        kind: ArgKind::Flag,
        cmd_style: &["/F"],
        short_style: &["-f"],
        long_style: &["--files"],
    },
    ArgDef {
        canonical: "full-path",
        kind: ArgKind::Flag,
        cmd_style: &["/FP"],
        short_style: &["-p"],
        long_style: &["--full-path"],
    },
    ArgDef {
        canonical: "size",
        kind: ArgKind::Flag,
        cmd_style: &["/S"],
        short_style: &["-s"],
        long_style: &["--size"],
    },
    ArgDef {
        canonical: "human-readable",
        kind: ArgKind::Flag,
        cmd_style: &["/HR"],
        short_style: &["-H"],
        long_style: &["--human-readable"],
    },
    ArgDef {
        canonical: "date",
        kind: ArgKind::Flag,
        cmd_style: &["/DT"],
        short_style: &["-d"],
        long_style: &["--date"],
    },
    ArgDef {
        canonical: "quote",
        kind: ArgKind::Flag,
        cmd_style: &["/Q"],
        short_style: &["-q"],
        long_style: &["--quote"],
    },
    // 树形渲染
    ArgDef {
        canonical: "ascii",
        kind: ArgKind::Flag,
        cmd_style: &["/A"],
        short_style: &["-a"],
        long_style: &["--ascii"],
    },
    ArgDef {
        canonical: "no-indent",
        kind: ArgKind::Flag,
        cmd_style: &["/NI"],
        short_style: &["-i"],
        long_style: &["--no-indent"],
    },
    // 排序与过滤
    ArgDef {
        canonical: "reverse",
        kind: ArgKind::Flag,
        cmd_style: &["/R"],
        short_style: &["-r"],
        long_style: &["--reverse"],
    },
    ArgDef {
        canonical: "dirs-first",
        kind: ArgKind::Flag,
        cmd_style: &["/DF"],
        short_style: &["-D"],
        long_style: &["--dirs-first"],
    },
    ArgDef {
        canonical: "sort",
        kind: ArgKind::Value,
        cmd_style: &["/SO"],
        short_style: &["-S"],
        long_style: &["--sort"],
    },
    ArgDef {
        canonical: "ignore-case",
        kind: ArgKind::Flag,
        cmd_style: &["/IC"],
        short_style: &["-c"],
        long_style: &["--ignore-case"],
    },
    // 深度与剪枝
    ArgDef {
        canonical: "level",
        kind: ArgKind::Value,
        cmd_style: &["/L"],
        short_style: &["-L"],
        long_style: &["--level"],
    },
    ArgDef {
        canonical: "prune",
        kind: ArgKind::Flag,
        cmd_style: &["/P"],
        short_style: &["-P"],
        long_style: &["--prune"],
    },
    // 匹配规则
    ArgDef {
        canonical: "include",
        kind: ArgKind::Value,
        cmd_style: &["/M"],
        short_style: &["-m"],
        long_style: &["--include"],
    },
    ArgDef {
        canonical: "exclude",
        kind: ArgKind::Value,
        cmd_style: &["/X"],
        short_style: &["-I"],
        long_style: &["--exclude"],
    },
    ArgDef {
        canonical: "gitignore",
        kind: ArgKind::Flag,
        cmd_style: &["/G"],
        short_style: &["-g"],
        long_style: &["--gitignore"],
    },
    // 输出控制
    ArgDef {
        canonical: "no-report",
        kind: ArgKind::Flag,
        cmd_style: &["/NR"],
        short_style: &["-n"],
        long_style: &["--no-report"],
    },
    ArgDef {
        canonical: "no-header",
        kind: ArgKind::Flag,
        cmd_style: &["/NH"],
        short_style: &["-N"],
        long_style: &["--no-header"],
    },
    ArgDef {
        canonical: "silent",
        kind: ArgKind::Flag,
        cmd_style: &["/SI"],
        short_style: &["-l"],
        long_style: &["--silent"],
    },
    ArgDef {
        canonical: "output",
        kind: ArgKind::Value,
        cmd_style: &["/O"],
        short_style: &["-o"],
        long_style: &["--output"],
    },
    // 性能
    ArgDef {
        canonical: "thread",
        kind: ArgKind::Value,
        cmd_style: &["/T"],
        short_style: &["-t"],
        long_style: &["--thread"],
    },
    ArgDef {
        canonical: "mft",
        kind: ArgKind::Flag,
        cmd_style: &["/MFT"],
        short_style: &["-M"],
        long_style: &["--mft"],
    },
    ArgDef {
        canonical: "disk-usage",
        kind: ArgKind::Flag,
        cmd_style: &["/DU"],
        short_style: &["-u"],
        long_style: &["--disk-usage"],
    },
];

/// MFT 模式下不兼容的参数
const MFT_INCOMPATIBLE: &[&str] = &[
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
// 解析器实现
// ============================================================================

/// 命令行解析器
pub struct CliParser {
    args: Vec<String>,
    position: usize,
    seen_option: bool,
    seen_args: Vec<String>,
}

impl CliParser {
    /// 从参数列表创建解析器
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            position: 0,
            seen_option: false,
            seen_args: Vec::new(),
        }
    }

    /// 从环境参数创建解析器（跳过程序名）
    pub fn from_env() -> Self {
        let args: Vec<String> = env::args().skip(1).collect();
        Self::new(args)
    }

    /// 解析命令行参数，返回配置或特殊操作
    pub fn parse(mut self) -> Result<ParseResult, CliError> {
        let mut config = Config::default();
        let mut paths: Vec<String> = Vec::new();

        while self.position < self.args.len() {
            let arg = self.args[self.position].clone();

            if Self::is_option(&arg) {
                self.seen_option = true;

                if let Some((def, value)) = self.match_argument(&arg)? {
                    self.check_duplicate(def.canonical)?;
                    self.apply_argument(&mut config, def, value)?;

                    if def.canonical == "help" {
                        return Ok(ParseResult::Help);
                    }
                    if def.canonical == "version" {
                        return Ok(ParseResult::Version);
                    }
                } else {
                    return Err(CliError::UnknownArgument(arg));
                }
            } else {
                // 非选项参数，视为路径
                if self.seen_option {
                    return Err(CliError::PathAfterOptions);
                }
                paths.push(arg);
            }

            self.position += 1;
        }

        // 处理路径
        match paths.len() {
            0 => {}
            1 => config.path = PathBuf::from(&paths[0]),
            _ => return Err(CliError::MultiplePaths(paths)),
        }

        // MFT 兼容性检查
        self.check_mft_compatibility(&config)?;

        Ok(ParseResult::Config(config))
    }

    /// 判断是否为选项参数
    fn is_option(arg: &str) -> bool {
        arg.starts_with('-') || arg.starts_with('/')
    }

    /// 匹配参数定义，返回定义和可能的值
    fn match_argument(&mut self, arg: &str) -> Result<Option<(&'static ArgDef, Option<String>)>, CliError> {
        for def in ARG_DEFS {
            // 检查 CMD 风格（大小写不敏感）
            let arg_upper = arg.to_uppercase();
            for pattern in def.cmd_style {
                if arg_upper == pattern.to_uppercase() {
                    let value = self.consume_value_if_needed(def, arg)?;
                    return Ok(Some((def, value)));
                }
            }

            // 检查 Unix 短参数（大小写敏感）
            for pattern in def.short_style {
                if arg == *pattern {
                    let value = self.consume_value_if_needed(def, arg)?;
                    return Ok(Some((def, value)));
                }
            }

            // 检查 GNU 长参数（大小写敏感）
            for pattern in def.long_style {
                if arg == *pattern {
                    let value = self.consume_value_if_needed(def, arg)?;
                    return Ok(Some((def, value)));
                }
                // 支持 `--option=value` 格式
                let prefix = format!("{pattern}=");
                if arg.starts_with(&prefix) {
                    if def.kind == ArgKind::Value {
                        let value = arg[prefix.len()..].to_string();
                        return Ok(Some((def, Some(value))));
                    }
                }
            }
        }

        Ok(None)
    }

    /// 如果参数需要值，消费下一个参数作为值
    fn consume_value_if_needed(
        &mut self,
        def: &ArgDef,
        arg: &str,
    ) -> Result<Option<String>, CliError> {
        if def.kind == ArgKind::Flag {
            return Ok(None);
        }

        // 尝试获取下一个参数作为值
        if self.position + 1 >= self.args.len() {
            return Err(CliError::MissingValue(arg.to_string()));
        }

        let next = &self.args[self.position + 1];
        if Self::is_option(next) {
            return Err(CliError::MissingValue(arg.to_string()));
        }

        self.position += 1;
        Ok(Some(next.clone()))
    }

    /// 检查参数是否重复
    fn check_duplicate(&mut self, canonical: &str) -> Result<(), CliError> {
        if self.seen_args.contains(&canonical.to_string()) {
            return Err(CliError::DuplicateArgument(canonical.to_string()));
        }
        self.seen_args.push(canonical.to_string());
        Ok(())
    }

    /// 将参数应用到配置
    fn apply_argument(
        &self,
        config: &mut Config,
        def: &ArgDef,
        value: Option<String>,
    ) -> Result<(), CliError> {
        match def.canonical {
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
                let v = value.as_ref().unwrap();
                config.sort_by = Some(
                    SortKey::from_str(v)
                        .ok_or_else(|| CliError::InvalidValue("sort".to_string(), v.clone()))?,
                );
            }
            "level" => {
                let v = value.as_ref().unwrap();
                config.level = Some(
                    v.parse::<u32>()
                        .map_err(|_| CliError::InvalidValue("level".to_string(), v.clone()))?,
                );
            }
            "include" => {
                config.include_pattern = value;
            }
            "exclude" => {
                config.exclude_pattern = value;
            }
            "output" => {
                config.output_file = value.map(PathBuf::from);
            }
            "thread" => {
                let v = value.as_ref().unwrap();
                config.thread_count = v
                    .parse::<u32>()
                    .map_err(|_| CliError::InvalidValue("thread".to_string(), v.clone()))?;
            }
            _ => {}
        }
        Ok(())
    }

    /// 检查 MFT 模式兼容性
    fn check_mft_compatibility(&self, config: &Config) -> Result<(), CliError> {
        if !config.use_mft {
            return Ok(());
        }

        for arg in &self.seen_args {
            if MFT_INCOMPATIBLE.contains(&arg.as_str()) {
                return Err(CliError::MftIncompatible(arg.clone()));
            }
        }

        Ok(())
    }
}

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
// 帮助与版本信息
// ============================================================================

/// 打印帮助信息
fn print_help() {
    println!(
        r#"tree++ - 更好的 Windows tree 命令

用法:
  treepp [<PATH>] [<OPTIONS>...]

选项:
  --help, -h, /?           显示帮助信息
  --version, -v, /V        显示版本信息
  --ascii, -a, /A          使用 ASCII 字符绘制树
  --files, -f, /F          显示文件
  --full-path, -p, /FP     显示完整路径
  --human-readable, -H, /HR  以人类可读方式显示文件大小
  --no-indent, -i, /NI     不显示树形连接线
  --reverse, -r, /R        逆序排序
  --size, -s, /S           显示文件大小（字节）
  --date, -d, /DT          显示最后修改日期
  --exclude, -I, /X <PATTERN>  排除匹配的文件
  --level, -L, /L <N>      限制递归深度
  --include, -m, /M <PATTERN>  仅显示匹配的文件
  --quote, -q, /Q          用双引号包裹文件名
  --dirs-first, -D, /DF    目录优先显示
  --disk-usage, -u, /DU    显示目录累计大小
  --ignore-case, -c, /IC   匹配时忽略大小写
  --no-report, -n, /NR     不显示末尾统计信息
  --prune, -P, /P          修剪空目录
  --sort, -S, /SO <KEY>    指定排序方式（name, size, mtime, ctime）
  --no-header, -N, /NH     不显示卷信息与头部报告
  --silent, -l, /SI        终端静默
  --output, -o, /O <FILE>  将结果输出至文件
  --thread, -t, /T <N>     扫描线程数（默认 24）
  --mft, -M, /MFT          使用 MFT（需管理员权限）
  --gitignore, -g, /G      遵循 .gitignore"#
    );
}

/// 打印版本信息
fn print_version() {
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

    /// 辅助函数：从字符串列表创建解析器并解析
    fn parse_args(args: &[&str]) -> Result<ParseResult, CliError> {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        CliParser::new(args).parse()
    }

    /// 辅助函数：解析并期望返回 Config
    fn parse_to_config(args: &[&str]) -> Result<Config, CliError> {
        match parse_args(args)? {
            ParseResult::Config(c) => Ok(c),
            _ => panic!("期望返回 Config"),
        }
    }

    // ------------------------------------------------------------------------
    // 基础功能测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_empty_args_returns_default_config() {
        let config = parse_to_config(&[]).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_path_only() {
        let config = parse_to_config(&["D:\\test\\path"]).unwrap();
        assert_eq!(config.path, PathBuf::from("D:\\test\\path"));
    }

    #[test]
    fn test_help_flag_variants() {
        assert_eq!(parse_args(&["--help"]).unwrap(), ParseResult::Help);
        assert_eq!(parse_args(&["-h"]).unwrap(), ParseResult::Help);
        assert_eq!(parse_args(&["/?"]).unwrap(), ParseResult::Help);
    }

    #[test]
    fn test_version_flag_variants() {
        assert_eq!(parse_args(&["--version"]).unwrap(), ParseResult::Version);
        assert_eq!(parse_args(&["-v"]).unwrap(), ParseResult::Version);
        assert_eq!(parse_args(&["/V"]).unwrap(), ParseResult::Version);
        assert_eq!(parse_args(&["/v"]).unwrap(), ParseResult::Version); // CMD 大小写不敏感
    }

    // ------------------------------------------------------------------------
    // 三种风格等价性测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_files_flag_equivalence() {
        let c1 = parse_to_config(&["--files"]).unwrap();
        let c2 = parse_to_config(&["-f"]).unwrap();
        let c3 = parse_to_config(&["/F"]).unwrap();
        let c4 = parse_to_config(&["/f"]).unwrap(); // CMD 大小写不敏感

        assert!(c1.show_files);
        assert!(c2.show_files);
        assert!(c3.show_files);
        assert!(c4.show_files);
    }

    #[test]
    fn test_ascii_flag_equivalence() {
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
    fn test_level_flag_equivalence() {
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
    fn test_sort_flag_equivalence() {
        let c1 = parse_to_config(&["--sort", "size"]).unwrap();
        let c2 = parse_to_config(&["-S", "size"]).unwrap();
        let c3 = parse_to_config(&["/SO", "size"]).unwrap();
        let c4 = parse_to_config(&["/so", "SIZE"]).unwrap(); // 值也大小写不敏感

        assert_eq!(c1.sort_by, Some(SortKey::Size));
        assert_eq!(c2.sort_by, Some(SortKey::Size));
        assert_eq!(c3.sort_by, Some(SortKey::Size));
        assert_eq!(c4.sort_by, Some(SortKey::Size));
    }

    // ------------------------------------------------------------------------
    // 大小写敏感性测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_cmd_style_case_insensitive() {
        let c1 = parse_to_config(&["/FP"]).unwrap();
        let c2 = parse_to_config(&["/fp"]).unwrap();
        let c3 = parse_to_config(&["/Fp"]).unwrap();

        assert!(c1.show_full_path);
        assert!(c2.show_full_path);
        assert!(c3.show_full_path);
    }

    #[test]
    fn test_unix_short_case_sensitive() {
        // -f 是 files，-F 不存在
        let c = parse_to_config(&["-f"]).unwrap();
        assert!(c.show_files);

        // -F 应该是未知参数
        let err = parse_args(&["-F"]);
        assert!(matches!(err, Err(CliError::UnknownArgument(_))));
    }

    #[test]
    fn test_gnu_long_case_sensitive() {
        let c = parse_to_config(&["--files"]).unwrap();
        assert!(c.show_files);

        let err = parse_args(&["--FILES"]);
        assert!(matches!(err, Err(CliError::UnknownArgument(_))));
    }

    // ------------------------------------------------------------------------
    // 带值参数测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_level_with_value() {
        let c = parse_to_config(&["/L", "5"]).unwrap();
        assert_eq!(c.level, Some(5));
    }

    #[test]
    fn test_level_with_equals_syntax() {
        let c = parse_to_config(&["--level=7"]).unwrap();
        assert_eq!(c.level, Some(7));
    }

    #[test]
    fn test_output_with_value() {
        let c = parse_to_config(&["--output", "tree.json"]).unwrap();
        assert_eq!(c.output_file, Some(PathBuf::from("tree.json")));
    }

    #[test]
    fn test_thread_with_value() {
        let c = parse_to_config(&["/T", "32"]).unwrap();
        assert_eq!(c.thread_count, 32);
    }

    #[test]
    fn test_include_pattern() {
        let c = parse_to_config(&["--include", "*.rs"]).unwrap();
        assert_eq!(c.include_pattern, Some("*.rs".to_string()));
    }

    #[test]
    fn test_exclude_pattern() {
        let c = parse_to_config(&["/X", "*.md"]).unwrap();
        assert_eq!(c.exclude_pattern, Some("*.md".to_string()));
    }

    #[test]
    fn test_sort_keys() {
        assert_eq!(
            parse_to_config(&["--sort", "name"]).unwrap().sort_by,
            Some(SortKey::Name)
        );
        assert_eq!(
            parse_to_config(&["--sort", "size"]).unwrap().sort_by,
            Some(SortKey::Size)
        );
        assert_eq!(
            parse_to_config(&["--sort", "mtime"]).unwrap().sort_by,
            Some(SortKey::Mtime)
        );
        assert_eq!(
            parse_to_config(&["--sort", "ctime"]).unwrap().sort_by,
            Some(SortKey::Ctime)
        );
    }

    // ------------------------------------------------------------------------
    // 混合参数测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_mixed_styles() {
        let c = parse_to_config(&["/F", "-a", "--no-report"]).unwrap();
        assert!(c.show_files);
        assert!(c.use_ascii);
        assert!(c.no_report);
    }

    #[test]
    fn test_path_with_options() {
        let c = parse_to_config(&["D:\\project", "/F", "--ascii"]).unwrap();
        assert_eq!(c.path, PathBuf::from("D:\\project"));
        assert!(c.show_files);
        assert!(c.use_ascii);
    }

    #[test]
    fn test_multiple_flags() {
        let c = parse_to_config(&[
            "/F", "/A", "/S", "/HR", "/DT", "/Q", "/R", "/DF", "/NR", "/NH",
        ])
            .unwrap();

        assert!(c.show_files);
        assert!(c.use_ascii);
        assert!(c.show_size);
        assert!(c.human_readable);
        assert!(c.show_date);
        assert!(c.quote_names);
        assert!(c.reverse);
        assert!(c.dirs_first);
        assert!(c.no_report);
        assert!(c.no_header);
    }

    #[test]
    fn test_complex_combination() {
        let c = parse_to_config(&[
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

        assert_eq!(c.path, PathBuf::from("C:\\Users"));
        assert!(c.show_files);
        assert_eq!(c.level, Some(3));
        assert!(c.use_ascii);
        assert_eq!(c.exclude_pattern, Some("node_modules".to_string()));
        assert_eq!(c.thread_count, 16);
    }

    // ------------------------------------------------------------------------
    // 错误处理测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_unknown_argument_error() {
        let err = parse_args(&["/Z"]).unwrap_err();
        assert!(matches!(err, CliError::UnknownArgument(_)));

        let err = parse_args(&["--unknown"]).unwrap_err();
        assert!(matches!(err, CliError::UnknownArgument(_)));

        let err = parse_args(&["-z"]).unwrap_err();
        assert!(matches!(err, CliError::UnknownArgument(_)));
    }

    #[test]
    fn test_missing_value_error() {
        let err = parse_args(&["/L"]).unwrap_err();
        assert!(matches!(err, CliError::MissingValue(_)));

        let err = parse_args(&["--level"]).unwrap_err();
        assert!(matches!(err, CliError::MissingValue(_)));

        let err = parse_args(&["--output"]).unwrap_err();
        assert!(matches!(err, CliError::MissingValue(_)));
    }

    #[test]
    fn test_missing_value_when_next_is_option() {
        let err = parse_args(&["/L", "/F"]).unwrap_err();
        assert!(matches!(err, CliError::MissingValue(_)));
    }

    #[test]
    fn test_invalid_value_error() {
        let err = parse_args(&["/L", "abc"]).unwrap_err();
        assert!(matches!(err, CliError::InvalidValue(_, _)));

        let err = parse_args(&["--thread", "-5"]).unwrap_err();
        assert!(matches!(err, CliError::MissingValue(_))); // -5 被视为选项

        let err = parse_args(&["--sort", "invalid"]).unwrap_err();
        assert!(matches!(err, CliError::InvalidValue(_, _)));
    }

    #[test]
    fn test_duplicate_argument_error() {
        let err = parse_args(&["/F", "--files"]).unwrap_err();
        assert!(matches!(err, CliError::DuplicateArgument(_)));

        let err = parse_args(&["/L", "3", "-L", "5"]).unwrap_err();
        assert!(matches!(err, CliError::DuplicateArgument(_)));

        let err = parse_args(&["-f", "-f"]).unwrap_err();
        assert!(matches!(err, CliError::DuplicateArgument(_)));
    }

    #[test]
    fn test_multiple_paths_error() {
        let err = parse_args(&["D:\\a", "D:\\b"]).unwrap_err();
        assert!(matches!(err, CliError::MultiplePaths(_)));

        let err = parse_args(&["path1", "path2", "path3"]).unwrap_err();
        assert!(matches!(err, CliError::MultiplePaths(_)));
    }

    #[test]
    fn test_path_after_options_error() {
        let err = parse_args(&["/F", "D:\\path"]).unwrap_err();
        assert!(matches!(err, CliError::PathAfterOptions));

        let err = parse_args(&["--files", "C:\\test"]).unwrap_err();
        assert!(matches!(err, CliError::PathAfterOptions));
    }

    // ------------------------------------------------------------------------
    // MFT 兼容性测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_mft_alone_works() {
        let c = parse_to_config(&["/MFT"]).unwrap();
        assert!(c.use_mft);
    }

    #[test]
    fn test_mft_with_compatible_options() {
        let c = parse_to_config(&["/MFT", "/F", "/A", "/S"]).unwrap();
        assert!(c.use_mft);
        assert!(c.show_files);
        assert!(c.use_ascii);
        assert!(c.show_size);
    }

    #[test]
    fn test_mft_incompatible_prune() {
        let err = parse_args(&["/MFT", "/P"]).unwrap_err();
        assert!(matches!(err, CliError::MftIncompatible(s) if s == "prune"));
    }

    #[test]
    fn test_mft_incompatible_level() {
        let err = parse_args(&["/MFT", "/L", "3"]).unwrap_err();
        assert!(matches!(err, CliError::MftIncompatible(s) if s == "level"));
    }

    #[test]
    fn test_mft_incompatible_gitignore() {
        let err = parse_args(&["/MFT", "/G"]).unwrap_err();
        assert!(matches!(err, CliError::MftIncompatible(s) if s == "gitignore"));
    }

    #[test]
    fn test_mft_incompatible_include() {
        let err = parse_args(&["/MFT", "/M", "*.rs"]).unwrap_err();
        assert!(matches!(err, CliError::MftIncompatible(s) if s == "include"));
    }

    #[test]
    fn test_mft_incompatible_exclude() {
        let err = parse_args(&["/MFT", "/X", "*.md"]).unwrap_err();
        assert!(matches!(err, CliError::MftIncompatible(s) if s == "exclude"));
    }

    #[test]
    fn test_mft_incompatible_disk_usage() {
        let err = parse_args(&["/MFT", "/DU"]).unwrap_err();
        assert!(matches!(err, CliError::MftIncompatible(s) if s == "disk-usage"));
    }

    #[test]
    fn test_mft_incompatible_sort() {
        let err = parse_args(&["/MFT", "/SO", "name"]).unwrap_err();
        assert!(matches!(err, CliError::MftIncompatible(s) if s == "sort"));
    }

    #[test]
    fn test_mft_incompatible_reverse() {
        let err = parse_args(&["/MFT", "/R"]).unwrap_err();
        assert!(matches!(err, CliError::MftIncompatible(s) if s == "reverse"));
    }

    // ------------------------------------------------------------------------
    // 边界情况测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_level_zero() {
        let c = parse_to_config(&["/L", "0"]).unwrap();
        assert_eq!(c.level, Some(0));
    }

    #[test]
    fn test_thread_one() {
        let c = parse_to_config(&["/T", "1"]).unwrap();
        assert_eq!(c.thread_count, 1);
    }

    #[test]
    fn test_empty_pattern_include() {
        let c = parse_to_config(&["--include", ""]).unwrap();
        assert_eq!(c.include_pattern, Some(String::new()));
    }

    #[test]
    fn test_pattern_with_spaces() {
        let c = parse_to_config(&["--include", "my file*.txt"]).unwrap();
        assert_eq!(c.include_pattern, Some("my file*.txt".to_string()));
    }

    #[test]
    fn test_path_with_spaces() {
        let c = parse_to_config(&["C:\\Program Files\\Test"]).unwrap();
        assert_eq!(c.path, PathBuf::from("C:\\Program Files\\Test"));
    }

    #[test]
    fn test_relative_path() {
        let c = parse_to_config(&["./src"]).unwrap();
        assert_eq!(c.path, PathBuf::from("./src"));
    }

    #[test]
    fn test_output_various_extensions() {
        let c = parse_to_config(&["-o", "output.txt"]).unwrap();
        assert_eq!(c.output_file, Some(PathBuf::from("output.txt")));

        let c = parse_to_config(&["-o", "data.json"]).unwrap();
        assert_eq!(c.output_file, Some(PathBuf::from("data.json")));

        let c = parse_to_config(&["-o", "config.yml"]).unwrap();
        assert_eq!(c.output_file, Some(PathBuf::from("config.yml")));

        let c = parse_to_config(&["-o", "settings.toml"]).unwrap();
        assert_eq!(c.output_file, Some(PathBuf::from("settings.toml")));
    }

    #[test]
    fn test_all_sort_keys_case_insensitive() {
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

    // ------------------------------------------------------------------------
    // 全参数覆盖测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_all_flags_individually() {
        // 逐个测试所有 flag 类型参数
        assert!(parse_to_config(&["/F"]).unwrap().show_files);
        assert!(parse_to_config(&["/FP"]).unwrap().show_full_path);
        assert!(parse_to_config(&["/S"]).unwrap().show_size);
        assert!(parse_to_config(&["/HR"]).unwrap().human_readable);
        assert!(parse_to_config(&["/DT"]).unwrap().show_date);
        assert!(parse_to_config(&["/Q"]).unwrap().quote_names);
        assert!(parse_to_config(&["/A"]).unwrap().use_ascii);
        assert!(parse_to_config(&["/NI"]).unwrap().no_indent);
        assert!(parse_to_config(&["/R"]).unwrap().reverse);
        assert!(parse_to_config(&["/DF"]).unwrap().dirs_first);
        assert!(parse_to_config(&["/IC"]).unwrap().ignore_case);
        assert!(parse_to_config(&["/P"]).unwrap().prune);
        assert!(parse_to_config(&["/G"]).unwrap().gitignore);
        assert!(parse_to_config(&["/NR"]).unwrap().no_report);
        assert!(parse_to_config(&["/NH"]).unwrap().no_header);
        assert!(parse_to_config(&["/SI"]).unwrap().silent);
        assert!(parse_to_config(&["/MFT"]).unwrap().use_mft);
        assert!(parse_to_config(&["/DU"]).unwrap().disk_usage);
    }

    #[test]
    fn test_default_thread_count() {
        let c = parse_to_config(&[]).unwrap();
        assert_eq!(c.thread_count, 24);
    }

    #[test]
    fn test_default_path_is_current_dir() {
        let c = parse_to_config(&[]).unwrap();
        assert_eq!(c.path, PathBuf::from("."));
    }

    // ------------------------------------------------------------------------
    // 特殊字符路径测试
    // ------------------------------------------------------------------------

    #[test]
    fn test_unicode_path() {
        let c = parse_to_config(&["D:\\项目\\测试"]).unwrap();
        assert_eq!(c.path, PathBuf::from("D:\\项目\\测试"));
    }

    #[test]
    fn test_path_with_dots() {
        let c = parse_to_config(&["../parent/child"]).unwrap();
        assert_eq!(c.path, PathBuf::from("../parent/child"));
    }
}