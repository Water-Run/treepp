//! tree++ 主程序入口
//!
//! 本模块实现 `tree++` 命令行工具的主入口，串联以下流程：
//!
//! 1. **CLI 解析**：解析命令行参数，产出 `ParseResult`
//! 2. **配置验证**：验证配置有效性，补齐派生字段
//! 3. **目录扫描**：在可流式时采用“边扫边渲染边输出”，否则构建完整树
//! 4. **树形渲染**：根据扫描模式选择流式渲染或批处理渲染
//! 5. **结果输出**：输出到 stdout 和/或文件
//!
//! # 退出码
//!
//! - `0`：成功
//! - `1`：参数错误
//! - `2`：扫描错误
//! - `3`：输出错误
//!
//! 文件: src/main.rs
//! 作者: WaterRun
//! 更新于: 2026-01-07

#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(missing_docs)]
#![allow(dead_code)]

mod cli;
mod config;
mod error;
mod output;
mod render;
mod scan;

use std::process::ExitCode;

use cli::{CliError, CliParser, ParseResult};
use config::OutputFormat;
use error::TreeppError;
use output::StreamWriter;
use render::{StreamRenderConfig, StreamRenderer};

/// 退出码：成功
const EXIT_SUCCESS: u8 = 0;
/// 退出码：参数错误
const EXIT_CLI_ERROR: u8 = 1;
/// 退出码：扫描错误
const EXIT_SCAN_ERROR: u8 = 2;
/// 退出码：输出错误
const EXIT_OUTPUT_ERROR: u8 = 3;

/// 程序主入口
///
/// 解析命令行参数并执行相应操作。
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::from(EXIT_SUCCESS),
        Err(e) => {
            let code = error_to_exit_code(&e);
            print_error(&e);
            ExitCode::from(code)
        }
    }
}

/// 执行主流程：能流式则流式，否则回退批处理。
fn run() -> Result<(), TreeppError> {
    // 1. CLI 解析
    let parser = CliParser::from_env();
    let parse_result = parser.parse()?;

    // 2. 根据解析结果执行相应操作
    match parse_result {
        ParseResult::Help => {
            cli::print_help();
            Ok(())
        }
        ParseResult::Version => {
            cli::print_version();
            Ok(())
        }
        ParseResult::Config(config) => {
            // 配置已在 parse() 中 validate
            let streaming_eligible = config.output.format == OutputFormat::Txt
                && !config.render.show_disk_usage
                && config.output.output_path.is_none()
                && !config.output.silent;
            if streaming_eligible {
                stream_mode(&config)
            } else {
                batch_mode(&config)
            }
        }
    }
}

/// 批处理管线：完整扫描 -> 渲染 -> 输出。
fn batch_mode(config: &config::Config) -> Result<(), TreeppError> {
    let stats = scan::scan(config)?;
    let render_result = render::render(&stats, config);
    output::execute_output(&render_result, &stats.tree, config)?;
    Ok(())
}

/// 流式管线：边扫描边渲染边输出（仅 TXT、未开启 /DU、无输出文件且非静默时）。
fn stream_mode(config: &config::Config) -> Result<(), TreeppError> {
    use crate::error::ScanError;
    use render::WinBanner;
    use scan::StreamEvent;

    // 静默模式已在 streaming_eligible 中被排除，这里仍防御性处理
    if config.output.silent {
        // 静默且无输出文件时，直接扫描即可（无终端输出）
        let _ = scan::scan_streaming(config, |_| Ok(()))?;
        return Ok(());
    }

    let mut renderer = StreamRenderer::new(StreamRenderConfig::from_config(config));
    let stdout = std::io::stdout();
    let mut writer = StreamWriter::new(&stdout);

    // 头部（可含 banner）立即输出
    let header = renderer.render_header(&config.root_path, config.path_explicitly_set);
    writer.write(&header)?;

    // 流式扫描 + 渲染
    let stats = scan::scan_streaming(config, |event| {
        match event {
            StreamEvent::Entry(entry) => {
                let line = renderer.render_entry(&entry);
                writer
                    .write_line(&line)
                    .map_err(|e| ScanError::WalkError {
                        message: e.to_string(),
                        path: None,
                    })?;
            }
            StreamEvent::EnterDir { is_last } => {
                renderer.push_level(!is_last);
            }
            StreamEvent::LeaveDir => {
                renderer.pop_level();
            }
        }
        Ok(())
    })?;

    // 空目录时，按原生 tree 行为输出“没有子文件夹”（仅在未禁用 banner 且获取成功时）
    if stats.directory_count == 0 && !config.render.no_win_banner {
        if let Some(drive) = drive_letter_from_path(&config.root_path) {
            if let Ok(banner) = WinBanner::fetch_for_drive(drive) {
                if !banner.no_subfolder.is_empty() {
                    writer.write_line(&banner.no_subfolder)?;
                }
            }
        }
    }

    // 末尾统计
    if config.render.show_report {
        let report = renderer.render_report(stats.directory_count, stats.file_count, stats.duration);
        writer.write(&report)?;
    }

    Ok(())
}

/// 从路径提取盘符（大写）。无法提取时返回 None。
fn drive_letter_from_path(path: &std::path::Path) -> Option<char> {
    use std::path::Component;

    if let Some(Component::Prefix(prefix)) = path.components().next() {
        let s = prefix.as_os_str().to_string_lossy();
        let chars: Vec<char> = s.chars().collect();
        // 普通格式 "C:"
        if chars.len() >= 2 && chars[1] == ':' {
            return Some(chars[0].to_ascii_uppercase());
        }
        // 长路径格式 "\\?\C:"
        if s.starts_with(r"\\?\") && chars.len() >= 6 && chars[5] == ':' {
            return Some(chars[4].to_ascii_uppercase());
        }
    }
    None
}

/// 将错误映射为退出码
fn error_to_exit_code(err: &TreeppError) -> u8 {
    match err {
        TreeppError::Cli(_) | TreeppError::Config(_) => EXIT_CLI_ERROR,
        TreeppError::Scan(_) | TreeppError::Match(_) => EXIT_SCAN_ERROR,
        TreeppError::Render(_) | TreeppError::Output(_) => EXIT_OUTPUT_ERROR,
    }
}

/// 打印错误信息到 stderr
///
/// 根据错误类型格式化输出，提供用户友好的错误提示。
fn print_error(err: &TreeppError) {
    let prefix = match err {
        TreeppError::Cli(_) => "CLI error",
        TreeppError::Config(_) => "Config error",
        TreeppError::Scan(_) => "Scan error",
        TreeppError::Match(_) => "Match error",
        TreeppError::Render(_) => "Render error",
        TreeppError::Output(_) => "Output error",
    };

    eprintln!("tree++: {}: {}", prefix, err);

    // 对于特定错误类型，提供额外提示
    match err {
        TreeppError::Cli(CliError::UnknownOption { .. }) => {
            eprintln!("Hint: run `treepp --help` to list available options");
        }
        TreeppError::Cli(CliError::MultiplePaths { .. }) => {
            eprintln!("Hint: only one target path can be specified.");
        }
        _ => {}
    }
}