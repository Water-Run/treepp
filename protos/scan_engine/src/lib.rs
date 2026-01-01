//! 扫描引擎库
//!
//! 提供高性能的目录树扫描功能，支持单线程和多线程两种模式。
//!
//! # 功能特性
//!
//! - 单线程递归扫描 ([`scan_walk`])
//! - 多线程并行扫描 ([`scan_parallel`])
//! - 一致性验证 ([`verify_consistency`])
//! - 原生 tree 命令对比 ([`scan_native_tree`])
//!
//! # 示例
//!
//! ```no_run
//! use scan_engine::{ScanConfig, scan_parallel};
//! use std::path::PathBuf;
//!
//! let config = ScanConfig::builder()
//!     .root(PathBuf::from("."))
//!     .include_files(true)
//!     .thread_count(8)
//!     .build();
//!
//! let result = scan_parallel(&config).expect("扫描失败");
//! println!("目录: {}, 文件: {}", result.directory_count, result.file_count);
//! ```

mod engine;
mod error;

pub use engine::{
    ConsistencyReport, EntryKind, EntryMetadata, NativeTreeResult, ScanConfig, ScanConfigBuilder,
    ScanStats, TreeNode, available_parallelism, scan_native_tree, scan_parallel, scan_walk,
    verify_consistency, BENCHMARK_RUNS, WARMUP_RUNS,
};
pub use error::{ScanError, ScanResult as Result};