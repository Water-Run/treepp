//! tree++ 主程序入口
//!
//! 本模块实现 `tree++` 命令行工具的主入口，串联以下流程：
//!
//! 1. **CLI 解析**：解析命令行参数，产出 `ParseResult`
//! 2. **配置验证**：验证配置有效性，补齐派生字段
//! 3. **目录扫描**：执行单线程或多线程扫描，产出 `ScanStats`
//! 4. **树形渲染**：将扫描结果渲染为文本，产出 `RenderResult`
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
//! 更新于: 2025-01-06

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
use error::TreeppError;

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

/// 执行主流程
///
/// 串联 CLI 解析 -> 配置验证 -> 扫描 -> 渲染 -> 输出的完整流程。
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
            // 配置已在 parse() 中调用 validate()，此处直接使用

            // 3. 目录扫描
            let stats = scan::scan(&config)?;

            // 4. 树形渲染
            let render_result = render::render(&stats, &config);

            // 5. 结果输出
            output::execute_output(&render_result, &stats.tree, &config)?;

            Ok(())
        }
    }
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
        TreeppError::Cli(_) => "参数错误",
        TreeppError::Config(_) => "配置错误",
        TreeppError::Scan(_) => "扫描错误",
        TreeppError::Match(_) => "匹配错误",
        TreeppError::Render(_) => "渲染错误",
        TreeppError::Output(_) => "输出错误",
    };

    eprintln!("tree++: {}: {}", prefix, err);

    // 对于特定错误类型，提供额外提示
    match err {
        TreeppError::Cli(CliError::UnknownOption { .. }) => {
            eprintln!("提示: 使用 treepp --help 查看可用选项");
        }
        TreeppError::Cli(CliError::MultiplePaths { .. }) => {
            eprintln!("提示: 只能指定一个目标路径");
        }
        _ => {}
    }
}