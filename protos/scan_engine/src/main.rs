use std::io;
use std::path::PathBuf;

use scan_engine::{
    num_cpus, scan_native_tree, scan_parallel, scan_walk, verify_consistency, ScanConfig,
};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir()?
    };

    let thread_count = args
        .iter()
        .position(|a| a == "-t" || a == "--threads")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(num_cpus);

    let include_files = args
        .iter()
        .any(|a| a == "-f" || a == "--files" || a == "/F");

    println!("扫描目录: {:?}", path);
    println!("线程数: {}", thread_count);
    println!("包含文件: {}", include_files);
    println!();

    let config = ScanConfig {
        root: path.clone(),
        include_files,
        thread_count,
    };

    println!("=== Walk 模式 (单线程) ===");
    let walk_result = scan_walk(&config)?;
    println!(
        "目录: {}, 文件: {}, 耗时: {:.3}s",
        walk_result.directory_count,
        walk_result.file_count,
        walk_result.duration.as_secs_f64()
    );

    println!("\n=== Parallel 模式 ({}线程, rayon 分治) ===", thread_count);
    let parallel_result = scan_parallel(&config)?;
    println!(
        "目录: {}, 文件: {}, 耗时: {:.3}s",
        parallel_result.directory_count,
        parallel_result.file_count,
        parallel_result.duration.as_secs_f64()
    );

    println!("\n=== 一致性验证 ===");
    let report = verify_consistency(&walk_result, &parallel_result);
    println!("{}", report);

    if walk_result.duration.as_secs_f64() > 0.0 {
        let speedup = walk_result.duration.as_secs_f64() / parallel_result.duration.as_secs_f64();
        println!("加速比: {:.2}x", speedup);
    }

    if include_files {
        println!("\n=== 原生 tree 命令对比 ===");
        match scan_native_tree(&path, include_files) {
            Ok(native) => {
                println!(
                    "原生 tree: 目录 {}, 文件 {}, 耗时: {:.3}s",
                    native.directory_count,
                    native.file_count,
                    native.duration.as_secs_f64()
                );

                let speedup_walk =
                    native.duration.as_secs_f64() / walk_result.duration.as_secs_f64();
                let speedup_parallel =
                    native.duration.as_secs_f64() / parallel_result.duration.as_secs_f64();

                println!("\n性能对比:");
                println!("  walk vs native: {:.2}x", speedup_walk);
                println!("  parallel vs native: {:.2}x", speedup_parallel);
            }
            Err(e) => {
                println!("无法执行原生 tree 命令: {}", e);
            }
        }
    }

    Ok(())
}