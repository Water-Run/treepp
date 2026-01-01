//! 错误类型定义
//!
//! 使用 `thiserror` 提供结构化的错误处理。

use std::path::PathBuf;

use thiserror::Error;

/// 扫描操作的结果类型别名
pub type ScanResult<T> = std::result::Result<T, ScanError>;

/// 扫描引擎错误类型
#[derive(Debug, Error)]
pub enum ScanError {
    /// 路径不存在
    #[error("路径不存在: {path}")]
    PathNotFound { path: PathBuf },

    /// 无法读取目录
    #[error("无法读取目录 {path}: {source}")]
    DirectoryReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// 无法获取元数据
    #[error("无法获取元数据 {path}: {source}")]
    MetadataError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// 线程池创建失败
    #[error("线程池创建失败: {0}")]
    ThreadPoolError(String),

    /// 原生命令执行失败
    #[error("原生命令执行失败: {0}")]
    NativeCommandError(#[from] std::io::Error),

    /// 配置无效
    #[error("配置无效: {0}")]
    InvalidConfig(String),
}