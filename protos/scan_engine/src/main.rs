//! 扫描引擎命令行入口
//!
//! 提供目录扫描的命令行接口，支持性能基准测试模式。

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

use scan_engine::{
    ScanConfig, available_parallelism, scan_native_tree, scan_parallel, scan_walk,
    verify_consistency, BENCHMARK_RUNS, WARMUP_RUNS,
};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let path = args
        .get(1)
        .filter(|a| !a.starts_with('-') && !a.starts_with('/'))
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let is_performance_mode = args.iter().any(|a| a == "--performance" || a == "-p");

    let thread_count = args
        .iter()
        .position(|a| a == "-t" || a == "--threads")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(available_parallelism);

    let include_files = args
        .iter()
        .any(|a| a == "-f" || a == "--files" || a == "/F");

    if is_performance_mode {
        run_performance_benchmark(&path, include_files)?;
    } else {
        run_normal_scan(&path, include_files, thread_count)?;
    }

    Ok(())
}

fn run_normal_scan(path: &PathBuf, include_files: bool, thread_count: usize) -> io::Result<()> {
    println!("扫描目录: {}", path.display());
    println!("线程数: {thread_count}");
    println!("包含文件: {include_files}");
    println!();

    let config = ScanConfig::builder()
        .root(path.clone())
        .include_files(include_files)
        .thread_count(thread_count)
        .build();

    println!("=== Walk 模式 (单线程) ===");
    let walk_result = scan_walk(&config).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    println!(
        "目录: {}, 文件: {}, 耗时: {:.3}s",
        walk_result.directory_count,
        walk_result.file_count,
        walk_result.duration.as_secs_f64()
    );

    println!("\n=== Parallel 模式 ({thread_count}线程) ===");
    let parallel_result =
        scan_parallel(&config).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    println!(
        "目录: {}, 文件: {}, 耗时: {:.3}s",
        parallel_result.directory_count,
        parallel_result.file_count,
        parallel_result.duration.as_secs_f64()
    );

    println!("\n=== 一致性验证 ===");
    let report = verify_consistency(&walk_result, &parallel_result);
    println!("{report}");

    if walk_result.duration.as_secs_f64() > 0.0 {
        let speedup = walk_result.duration.as_secs_f64() / parallel_result.duration.as_secs_f64();
        println!("加速比: {speedup:.2}x");
    }

    if include_files {
        println!("\n=== 原生 tree 命令对比 ===");
        match scan_native_tree(path, include_files) {
            Ok(native) => {
                println!(
                    "原生 tree: 目录 {}, 文件 {}, 耗时: {:.3}s",
                    native.directory_count,
                    native.file_count,
                    native.duration.as_secs_f64()
                );

                let speedup_parallel =
                    native.duration.as_secs_f64() / parallel_result.duration.as_secs_f64();
                println!("parallel vs native: {speedup_parallel:.2}x");
            }
            Err(e) => {
                println!("无法执行原生 tree 命令: {e}");
            }
        }
    }

    Ok(())
}

fn run_performance_benchmark(path: &PathBuf, include_files: bool) -> io::Result<()> {
    println!("=== 性能对比测试 ===");
    println!("路径: {}", path.display());
    println!("预热: {WARMUP_RUNS} 次, 测量: {BENCHMARK_RUNS} 次\n");

    print!("预热中");
    io::stdout().flush()?;
    for _ in 0..WARMUP_RUNS {
        let config = ScanConfig::builder()
            .root(path.clone())
            .include_files(include_files)
            .thread_count(available_parallelism())
            .build();
        let _ = scan_parallel(&config);
        print!(".");
        io::stdout().flush()?;
    }
    println!(" 完成\n");

    print!("测量原生 tree");
    io::stdout().flush()?;
    let mut native_times = Vec::with_capacity(BENCHMARK_RUNS);
    for _ in 0..BENCHMARK_RUNS {
        if let Ok(r) = scan_native_tree(path, include_files) {
            native_times.push(r.duration.as_secs_f64() * 1000.0);
        }
        print!(".");
        io::stdout().flush()?;
    }
    println!(" 完成");
    let native_avg = if native_times.is_empty() {
        0.0
    } else {
        native_times.iter().sum::<f64>() / native_times.len() as f64
    };

    print!("测量单线程");
    io::stdout().flush()?;
    let mut walk_times = Vec::with_capacity(BENCHMARK_RUNS);
    let config_walk = ScanConfig::builder()
        .root(path.clone())
        .include_files(include_files)
        .thread_count(1)
        .build();
    for _ in 0..BENCHMARK_RUNS {
        let result =
            scan_walk(&config_walk).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        walk_times.push(result.duration.as_secs_f64() * 1000.0);
        print!(".");
        io::stdout().flush()?;
    }
    println!(" 完成");
    let walk_avg = walk_times.iter().sum::<f64>() / walk_times.len() as f64;

    let thread_count = available_parallelism();
    print!("测量多线程({thread_count}线程)");
    io::stdout().flush()?;
    let mut parallel_times = Vec::with_capacity(BENCHMARK_RUNS);
    let config_parallel = ScanConfig::builder()
        .root(path.clone())
        .include_files(include_files)
        .thread_count(thread_count)
        .build();
    for _ in 0..BENCHMARK_RUNS {
        let result =
            scan_parallel(&config_parallel).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        parallel_times.push(result.duration.as_secs_f64() * 1000.0);
        print!(".");
        io::stdout().flush()?;
    }
    println!(" 完成\n");
    let parallel_avg = parallel_times.iter().sum::<f64>() / parallel_times.len() as f64;

    println!("| {:<25} | {:>12} | {:>8} |", "类型", "耗时(`ms`)", "倍率");
    println!("|{:-<27}|{:-<14}|{:-<10}|", "", "", "");

    if native_avg > 0.0 {
        println!(
            "| {:<25} | {:>12.2} | {:>7.2}x |",
            "原生`tree`", native_avg, 1.0
        );
        println!(
            "| {:<25} | {:>12.2} | {:>7.2}x |",
            format!("`treepp`(默认, {thread_count}线程)"),
            parallel_avg,
            native_avg / parallel_avg
        );
        println!(
            "| {:<25} | {:>12.2} | {:>7.2}x |",
            "`treepp`(1线程)",
            walk_avg,
            native_avg / walk_avg
        );
    } else {
        println!(
            "| {:<25} | {:>12.2} | {:>7.2}x |",
            "`treepp`(1线程)", walk_avg, 1.0
        );
        println!(
            "| {:<25} | {:>12.2} | {:>7.2}x |",
            format!("`treepp`(默认, {thread_count}线程)"),
            parallel_avg,
            walk_avg / parallel_avg
        );
    }

    println!("\n=== 一致性验证 ===");
    let walk_result =
        scan_walk(&config_walk).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let parallel_result =
        scan_parallel(&config_parallel).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let report = verify_consistency(&walk_result, &parallel_result);
    println!(
        "结果: {}",
        if report.is_consistent() {
            "一致 ✓"
        } else {
            "不一致 ✗"
        }
    );

    Ok(())
}