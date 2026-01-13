//! Performance tests for tree++.
//!
//! This module benchmarks tree++ performance across different configurations
//! and compares against the native Windows tree command.
//!
//! # Test Categories
//!
//! 1. **Feature Performance Tests**: Measure overhead of individual features
//! 2. **Comparative Performance Tests**: Compare treepp vs native tree
//!
//! # Methodology
//!
//! - Uses release build only
//! - Runs warmup iterations to eliminate cache effects
//! - Takes average of 3 runs for each measurement
//! - Outputs results in Markdown table format
//!
//! File: tests/performance_test.rs
//! Author: WaterRun
//! Date: 2026-01-13

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use tempfile::TempDir;

// ============================================================================
// Constants
// ============================================================================

/// Number of warmup iterations before measurement.
const WARMUP_ITERATIONS: usize = 2;

/// Number of measurement iterations to average.
const MEASURE_ITERATIONS: usize = 3;

// ============================================================================
// Types
// ============================================================================

/// Result of a performance measurement.
#[derive(Debug, Clone)]
struct BenchmarkResult {
    /// Description of the benchmark.
    description: String,
    /// Average duration in milliseconds.
    duration_ms: f64,
    /// Optional baseline duration for comparison.
    baseline_ms: Option<f64>,
}

impl BenchmarkResult {
    /// Creates a new benchmark result.
    fn new(description: &str, duration_ms: f64) -> Self {
        Self {
            description: description.to_string(),
            duration_ms,
            baseline_ms: None,
        }
    }

    /// Creates a benchmark result with baseline comparison.
    fn with_baseline(description: &str, duration_ms: f64, baseline_ms: f64) -> Self {
        Self {
            description: description.to_string(),
            duration_ms,
            baseline_ms: Some(baseline_ms),
        }
    }

    /// Calculates the multiplier compared to baseline.
    fn multiplier(&self) -> Option<f64> {
        self.baseline_ms.map(|base| base / self.duration_ms)
    }

    /// Calculates percentage change from baseline.
    fn percentage_change(&self) -> Option<f64> {
        self.baseline_ms
            .map(|base| ((self.duration_ms - base) / base) * 100.0)
    }
}

/// Collection of benchmark results for reporting.
#[derive(Debug, Default)]
struct BenchmarkReport {
    /// Title of the report.
    title: String,
    /// Individual benchmark results.
    results: Vec<BenchmarkResult>,
}

impl BenchmarkReport {
    /// Creates a new benchmark report.
    fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            results: Vec::new(),
        }
    }

    /// Adds a result to the report.
    fn add(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    /// Generates a Markdown table from the results.
    fn to_markdown_table(&self) -> String {
        let mut output = format!("## {}\n\n", self.title);

        if self.results.is_empty() {
            return output;
        }

        // Check if we have baseline comparisons
        let has_baseline = self.results.iter().any(|r| r.baseline_ms.is_some());

        if has_baseline {
            output.push_str("| Type | Time (`ms`) | Multiplier |\n");
            output.push_str("| --- | ---:| ---:|\n");

            for result in &self.results {
                let multiplier = result
                    .multiplier()
                    .map(|m| format!("{:.2}x", m))
                    .unwrap_or_else(|| "1.00x".to_string());

                output.push_str(&format!(
                    "| `{}` | `{:.2}` | `{}` |\n",
                    result.description, result.duration_ms, multiplier
                ));
            }
        } else {
            output.push_str("| Feature | Time (`ms`) | Change |\n");
            output.push_str("| --- | ---:| ---:|\n");

            let baseline = self.results.first().map(|r| r.duration_ms).unwrap_or(1.0);

            for (i, result) in self.results.iter().enumerate() {
                let change = if i == 0 {
                    "baseline".to_string()
                } else {
                    let pct = ((result.duration_ms - baseline) / baseline) * 100.0;
                    if pct >= 0.0 {
                        format!("+{:.1}%", pct)
                    } else {
                        format!("{:.1}%", pct)
                    }
                };

                output.push_str(&format!(
                    "| `{}` | `{:.2}` | {} |\n",
                    result.description, result.duration_ms, change
                ));
            }
        }

        output
    }

    /// Generates a detailed Markdown report with percentage changes.
    fn to_detailed_markdown(&self) -> String {
        let mut output = format!("## {}\n\n", self.title);

        if self.results.is_empty() {
            return output;
        }

        output.push_str("| Feature Configuration | Time (`ms`) | vs Baseline |\n");
        output.push_str("| --- | ---:| ---:|\n");

        for result in &self.results {
            let change = match result.percentage_change() {
                Some(pct) if pct >= 0.0 => format!("+{:.1}%", pct),
                Some(pct) => format!("{:.1}%", pct),
                None => "baseline".to_string(),
            };

            output.push_str(&format!(
                "| `{}` | `{:.2}` | {} |\n",
                result.description, result.duration_ms, change
            ));
        }

        output
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Locates the release build of treepp executable.
///
/// # Returns
///
/// Path to the treepp release executable.
///
/// # Panics
///
/// Panics if release build is not found.
fn get_treepp_release_path() -> PathBuf {
    let release_path = PathBuf::from("target/release/treepp.exe");
    if release_path.exists() {
        return release_path;
    }
    panic!(
        "treepp release build not found. Please run: cargo build --release\n\
         Performance tests require release build for accurate measurements."
    );
}

/// Gets the user's home directory.
fn get_user_home() -> PathBuf {
    env::var("USERPROFILE")
        .map(PathBuf::from)
        .expect("USERPROFILE environment variable not set")
}

/// Gets the .cargo directory path.
fn get_cargo_dir() -> Option<PathBuf> {
    let cargo_dir = get_user_home().join(".cargo");
    if cargo_dir.exists() && cargo_dir.is_dir() {
        Some(cargo_dir)
    } else {
        None
    }
}

/// Gets the Windows directory path.
fn get_windows_dir() -> PathBuf {
    PathBuf::from("C:\\Windows")
}

/// Runs native tree command and returns execution duration.
fn time_native_tree(path: &Path, args: &[&str]) -> Duration {
    let mut cmd_args = vec!["/C", "tree"];
    cmd_args.extend(args);
    cmd_args.push(path.to_str().unwrap());

    let start = Instant::now();
    let _ = Command::new("cmd").args(&cmd_args).output();
    start.elapsed()
}

/// Runs treepp command and returns execution duration.
fn time_treepp(path: &Path, args: &[&str]) -> Duration {
    let treepp = get_treepp_release_path();
    let mut cmd_args: Vec<&str> = args.to_vec();
    let path_str = path.to_str().unwrap();
    cmd_args.push(path_str);

    let start = Instant::now();
    let _ = Command::new(&treepp).args(&cmd_args).output();
    start.elapsed()
}

/// Runs warmup iterations to prime file system cache.
fn warmup<F>(iterations: usize, mut f: F)
where
    F: FnMut(),
{
    for _ in 0..iterations {
        f();
    }
}

/// Measures average duration over multiple iterations.
fn measure_average<F>(iterations: usize, mut f: F) -> Duration
where
    F: FnMut() -> Duration,
{
    let total: Duration = (0..iterations).map(|_| f()).sum();
    total / iterations as u32
}

/// Benchmarks a treepp configuration.
fn benchmark_treepp(path: &Path, args: &[&str], description: &str) -> BenchmarkResult {
    // Warmup
    warmup(WARMUP_ITERATIONS, || {
        time_treepp(path, args);
    });

    // Measure
    let duration = measure_average(MEASURE_ITERATIONS, || time_treepp(path, args));

    BenchmarkResult::new(description, duration.as_secs_f64() * 1000.0)
}

/// Benchmarks native tree command.
fn benchmark_native_tree(path: &Path, args: &[&str], description: &str) -> BenchmarkResult {
    // Warmup
    warmup(WARMUP_ITERATIONS, || {
        time_native_tree(path, args);
    });

    // Measure
    let duration = measure_average(MEASURE_ITERATIONS, || time_native_tree(path, args));

    BenchmarkResult::new(description, duration.as_secs_f64() * 1000.0)
}

// ============================================================================
// Test Directory Creation
// ============================================================================

/// Creates a small test directory structure.
fn create_small_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create a realistic small project structure
    fs::create_dir_all(dir.path().join("src/utils")).unwrap();
    fs::create_dir_all(dir.path().join("tests")).unwrap();
    fs::create_dir_all(dir.path().join("docs")).unwrap();

    File::create(dir.path().join("Cargo.toml")).unwrap();
    File::create(dir.path().join("README.md")).unwrap();
    File::create(dir.path().join("src/main.rs")).unwrap();
    File::create(dir.path().join("src/lib.rs")).unwrap();
    File::create(dir.path().join("src/utils/helper.rs")).unwrap();
    File::create(dir.path().join("tests/test.rs")).unwrap();

    dir
}

/// Creates a medium test directory structure.
fn create_medium_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    for i in 0..10 {
        let subdir = dir.path().join(format!("module_{:02}", i));
        fs::create_dir_all(subdir.join("src")).unwrap();
        fs::create_dir_all(subdir.join("tests")).unwrap();

        for j in 0..5 {
            File::create(subdir.join("src").join(format!("file_{}.rs", j))).unwrap();
        }
        File::create(subdir.join("tests/test.rs")).unwrap();
        File::create(subdir.join("Cargo.toml")).unwrap();
    }

    dir
}

/// Creates a large test directory structure.
fn create_large_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    for i in 0..20 {
        let module = dir.path().join(format!("module_{:02}", i));
        fs::create_dir_all(module.join("src/core")).unwrap();
        fs::create_dir_all(module.join("src/utils")).unwrap();
        fs::create_dir_all(module.join("tests/unit")).unwrap();
        fs::create_dir_all(module.join("tests/integration")).unwrap();
        fs::create_dir_all(module.join("docs")).unwrap();

        for j in 0..10 {
            File::create(module.join("src/core").join(format!("core_{}.rs", j))).unwrap();
            File::create(module.join("src/utils").join(format!("util_{}.rs", j))).unwrap();
        }

        for j in 0..5 {
            File::create(
                module
                    .join("tests/unit")
                    .join(format!("test_unit_{}.rs", j)),
            )
                .unwrap();
            File::create(
                module
                    .join("tests/integration")
                    .join(format!("test_int_{}.rs", j)),
            )
                .unwrap();
        }

        File::create(module.join("Cargo.toml")).unwrap();
        File::create(module.join("README.md")).unwrap();
    }

    dir
}

/// Creates a deeply nested directory structure.
fn create_deep_nested_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    let mut current = dir.path().to_path_buf();
    for i in 0..15 {
        current = current.join(format!("level_{:02}", i));
        fs::create_dir_all(&current).unwrap();
        File::create(current.join("file.txt")).unwrap();
    }

    dir
}

/// Creates a wide directory structure.
fn create_wide_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    for i in 0..100 {
        fs::create_dir(dir.path().join(format!("dir_{:03}", i))).unwrap();
        File::create(dir.path().join(format!("file_{:03}.txt", i))).unwrap();
    }

    dir
}

/// Creates a directory with many files.
fn create_many_files_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.path().join("files")).unwrap();
    for i in 0..500 {
        File::create(
            dir.path()
                .join("files")
                .join(format!("file_{:04}.txt", i)),
        )
            .unwrap();
    }

    dir
}

/// Creates a directory with mixed content for gitignore testing.
fn create_gitignore_test_dir() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Create .gitignore
    let mut gitignore = File::create(dir.path().join(".gitignore")).unwrap();
    writeln!(gitignore, "target/").unwrap();
    writeln!(gitignore, "*.log").unwrap();
    writeln!(gitignore, "node_modules/").unwrap();

    // Create directories and files
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::create_dir_all(dir.path().join("target/debug")).unwrap();
    fs::create_dir_all(dir.path().join("target/release")).unwrap();
    fs::create_dir_all(dir.path().join("node_modules/package")).unwrap();

    File::create(dir.path().join("src/main.rs")).unwrap();
    File::create(dir.path().join("debug.log")).unwrap();
    File::create(dir.path().join("error.log")).unwrap();
    File::create(dir.path().join("target/debug/app.exe")).unwrap();
    File::create(dir.path().join("node_modules/package/index.js")).unwrap();

    dir
}

// ============================================================================
// Feature Performance Tests
// ============================================================================

#[cfg(test)]
mod feature_tests {
    use super::*;

    /// Tests baseline performance without any flags.
    #[test]
    fn benchmark_baseline_no_flags() {
        let dir = create_medium_test_dir();
        let result = benchmark_treepp(dir.path(), &[], "baseline (no flags)");
        println!("Baseline: {:.2} ms", result.duration_ms);
        assert!(result.duration_ms > 0.0);
    }

    /// Tests performance impact of /F flag.
    #[test]
    fn benchmark_files_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &[], "baseline");
        let with_files = benchmark_treepp(dir.path(), &["/F"], "with /F");

        let change = ((with_files.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/F flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_files.duration_ms, change
        );
    }

    /// Tests performance impact of /A flag.
    #[test]
    fn benchmark_ascii_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_ascii = benchmark_treepp(dir.path(), &["/F", "/A"], "with /F /A");

        let change = ((with_ascii.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/A flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_ascii.duration_ms, change
        );
    }

    /// Tests performance impact of /S (size) flag.
    #[test]
    fn benchmark_size_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_size = benchmark_treepp(dir.path(), &["/F", "/S"], "with /F /S");

        let change = ((with_size.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/S flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_size.duration_ms, change
        );
    }

    /// Tests performance impact of /DT (date) flag.
    #[test]
    fn benchmark_date_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_date = benchmark_treepp(dir.path(), &["/F", "/DT"], "with /F /DT");

        let change = ((with_date.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/DT flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_date.duration_ms, change
        );
    }

    /// Tests performance impact of /HR (human-readable) flag.
    #[test]
    fn benchmark_human_readable_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F", "/S"], "baseline /F /S");
        let with_hr = benchmark_treepp(dir.path(), &["/F", "/S", "/HR"], "with /F /S /HR");

        let change = ((with_hr.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/HR flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_hr.duration_ms, change
        );
    }

    /// Tests performance impact of /L (level) flag.
    #[test]
    fn benchmark_level_flag() {
        let dir = create_deep_nested_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_level = benchmark_treepp(dir.path(), &["/F", "/L", "3"], "with /F /L 3");

        let change = ((with_level.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/L flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_level.duration_ms, change
        );
    }

    /// Tests performance impact of /DU (disk usage) flag.
    #[test]
    fn benchmark_disk_usage_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_du = benchmark_treepp(dir.path(), &["/F", "/DU"], "with /F /DU");

        let change = ((with_du.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/DU flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_du.duration_ms, change
        );
    }

    /// Tests performance impact of /RP (report) flag.
    #[test]
    fn benchmark_report_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_report = benchmark_treepp(dir.path(), &["/F", "/RP"], "with /F /RP");

        let change =
            ((with_report.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/RP flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_report.duration_ms, change
        );
    }

    /// Tests performance impact of /P (prune) flag.
    #[test]
    fn benchmark_prune_flag() {
        let dir = create_medium_test_dir();

        // Add some empty directories
        fs::create_dir_all(dir.path().join("empty1/nested")).unwrap();
        fs::create_dir_all(dir.path().join("empty2")).unwrap();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_prune = benchmark_treepp(dir.path(), &["/F", "/P"], "with /F /P");

        let change = ((with_prune.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/P flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_prune.duration_ms, change
        );
    }

    /// Tests performance impact of /NB (no banner) flag.
    #[test]
    fn benchmark_no_banner_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_nb = benchmark_treepp(dir.path(), &["/F", "/NB"], "with /F /NB");

        let change = ((with_nb.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/NB flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_nb.duration_ms, change
        );
    }

    /// Tests performance impact of /NI (no indent) flag.
    #[test]
    fn benchmark_no_indent_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_ni = benchmark_treepp(dir.path(), &["/F", "/NI"], "with /F /NI");

        let change = ((with_ni.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/NI flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_ni.duration_ms, change
        );
    }

    /// Tests performance impact of /FP (full path) flag.
    #[test]
    fn benchmark_full_path_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_fp = benchmark_treepp(dir.path(), &["/F", "/FP"], "with /F /FP");

        let change = ((with_fp.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/FP flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_fp.duration_ms, change
        );
    }

    /// Tests performance impact of /R (reverse) flag.
    #[test]
    fn benchmark_reverse_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_reverse = benchmark_treepp(dir.path(), &["/F", "/R"], "with /F /R");

        let change =
            ((with_reverse.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/R flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_reverse.duration_ms, change
        );
    }

    /// Tests performance impact of /G (gitignore) flag.
    #[test]
    fn benchmark_gitignore_flag() {
        let dir = create_gitignore_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_gitignore = benchmark_treepp(dir.path(), &["/F", "/G"], "with /F /G");

        let change =
            ((with_gitignore.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/G flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_gitignore.duration_ms, change
        );
    }

    /// Tests performance impact of /X (exclude) flag.
    #[test]
    fn benchmark_exclude_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_exclude = benchmark_treepp(dir.path(), &["/F", "/X", "*.rs"], "with /F /X *.rs");

        let change =
            ((with_exclude.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/X flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_exclude.duration_ms, change
        );
    }

    /// Tests performance impact of /M (include/match) flag.
    #[test]
    fn benchmark_include_flag() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let with_include = benchmark_treepp(dir.path(), &["/F", "/M", "*.rs"], "with /F /M *.rs");

        let change =
            ((with_include.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/M flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_include.duration_ms, change
        );
    }

    /// Tests performance impact of batch mode.
    #[test]
    fn benchmark_batch_mode() {
        let dir = create_large_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F", "/NB"], "baseline /F /NB");
        let with_batch = benchmark_treepp(dir.path(), &["/F", "/NB", "/B"], "with /F /NB /B");

        let change = ((with_batch.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "/B flag impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, with_batch.duration_ms, change
        );
    }

    /// Tests performance with different thread counts.
    #[test]
    fn benchmark_thread_counts() {
        let dir = create_large_test_dir();

        let thread_counts = [1, 2, 4, 8, 16];
        let baseline = benchmark_treepp(dir.path(), &["/F", "/NB", "/B"], "baseline /F /NB /B");

        println!("Thread count performance comparison:");
        println!("Baseline (default): {:.2} ms", baseline.duration_ms);

        for threads in thread_counts {
            let result = benchmark_treepp(
                dir.path(),
                &["/F", "/NB", "/B", "/T", &threads.to_string()],
                &format!("/T {}", threads),
            );
            let change = ((result.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
            println!("  {} threads: {:.2} ms ({:+.1}%)", threads, result.duration_ms, change);
        }
    }

    /// Tests combined feature overhead.
    #[test]
    fn benchmark_combined_features() {
        let dir = create_medium_test_dir();

        let baseline = benchmark_treepp(dir.path(), &["/F"], "baseline /F");
        let combined = benchmark_treepp(
            dir.path(),
            &["/F", "/S", "/DT", "/HR", "/RP"],
            "combined /F /S /DT /HR /RP",
        );

        let change = ((combined.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "Combined features impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, combined.duration_ms, change
        );
    }

    /// Tests all features enabled.
    #[test]
    fn benchmark_all_features() {
        let dir = create_gitignore_test_dir();

        let baseline = benchmark_treepp(dir.path(), &[], "baseline");
        let all_features = benchmark_treepp(
            dir.path(),
            &["/F", "/S", "/DT", "/HR", "/RP", "/FP", "/G"],
            "all features",
        );

        let change =
            ((all_features.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100.0;
        println!(
            "All features impact: {:.2} ms -> {:.2} ms ({:+.1}%)",
            baseline.duration_ms, all_features.duration_ms, change
        );
    }
}

// ============================================================================
// Comparative Performance Tests
// ============================================================================

#[cfg(test)]
mod comparative_tests {
    use super::*;

    /// Compares treepp vs native tree on small directory.
    #[test]
    fn compare_small_directory() {
        let dir = create_small_test_dir();

        let native = benchmark_native_tree(dir.path(), &[], "tree (native)");
        let treepp = benchmark_treepp(dir.path(), &[], "treepp");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "Small directory: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares treepp vs native tree on medium directory.
    #[test]
    fn compare_medium_directory() {
        let dir = create_medium_test_dir();

        let native = benchmark_native_tree(dir.path(), &[], "tree (native)");
        let treepp = benchmark_treepp(dir.path(), &[], "treepp");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "Medium directory: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares treepp vs native tree on large directory.
    #[test]
    fn compare_large_directory() {
        let dir = create_large_test_dir();

        let native = benchmark_native_tree(dir.path(), &[], "tree (native)");
        let treepp = benchmark_treepp(dir.path(), &[], "treepp");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "Large directory: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares treepp vs native tree with /F flag.
    #[test]
    fn compare_with_files_flag() {
        let dir = create_medium_test_dir();

        let native = benchmark_native_tree(dir.path(), &["/F"], "tree /F (native)");
        let treepp = benchmark_treepp(dir.path(), &["/F"], "treepp /F");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "With /F: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares treepp vs native tree with /A flag.
    #[test]
    fn compare_with_ascii_flag() {
        let dir = create_medium_test_dir();

        let native = benchmark_native_tree(dir.path(), &["/A"], "tree /A (native)");
        let treepp = benchmark_treepp(dir.path(), &["/A"], "treepp /A");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "With /A: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares treepp vs native tree with /F /A flags.
    #[test]
    fn compare_with_files_and_ascii() {
        let dir = create_medium_test_dir();

        let native = benchmark_native_tree(dir.path(), &["/F", "/A"], "tree /F /A (native)");
        let treepp = benchmark_treepp(dir.path(), &["/F", "/A"], "treepp /F /A");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "With /F /A: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares on deep nested structure.
    #[test]
    fn compare_deep_nested() {
        let dir = create_deep_nested_dir();

        let native = benchmark_native_tree(dir.path(), &["/F"], "tree /F (native)");
        let treepp = benchmark_treepp(dir.path(), &["/F"], "treepp /F");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "Deep nested: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares on wide structure.
    #[test]
    fn compare_wide_structure() {
        let dir = create_wide_dir();

        let native = benchmark_native_tree(dir.path(), &["/F"], "tree /F (native)");
        let treepp = benchmark_treepp(dir.path(), &["/F"], "treepp /F");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "Wide structure: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares on many files structure.
    #[test]
    fn compare_many_files() {
        let dir = create_many_files_dir();

        let native = benchmark_native_tree(dir.path(), &["/F"], "tree /F (native)");
        let treepp = benchmark_treepp(dir.path(), &["/F"], "treepp /F");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            "Many files: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares on .cargo directory (real world).
    #[test]
    fn compare_cargo_directory() {
        let Some(cargo_dir) = get_cargo_dir() else {
            eprintln!("Skipping: .cargo directory not found");
            return;
        };

        let native = benchmark_native_tree(&cargo_dir, &["/F"], "tree /F (native)");
        let treepp = benchmark_treepp(&cargo_dir, &["/F"], "treepp /F");

        let speedup = native.duration_ms / treepp.duration_ms;
        println!(
            ".cargo directory: native={:.2}ms, treepp={:.2}ms, speedup={:.2}x",
            native.duration_ms, treepp.duration_ms, speedup
        );
    }

    /// Compares treepp optimizations on .cargo directory.
    #[test]
    fn compare_treepp_optimizations_cargo() {
        let Some(cargo_dir) = get_cargo_dir() else {
            eprintln!("Skipping: .cargo directory not found");
            return;
        };

        println!("\n=== treepp optimization comparison on .cargo ===\n");

        let native = benchmark_native_tree(&cargo_dir, &["/F"], "tree /f (Windows Native)");
        let base = benchmark_treepp(&cargo_dir, &["/F"], "treepp /f");
        let nb = benchmark_treepp(&cargo_dir, &["/F", "/NB"], "treepp /f /nb");
        let batch = benchmark_treepp(&cargo_dir, &["/F", "/NB", "/B"], "treepp /f /nb /b");

        let mut report = BenchmarkReport::new("treepp Optimization Comparison (.cargo)");
        report.add(BenchmarkResult::with_baseline(
            "tree /f (Windows Native)",
            native.duration_ms,
            native.duration_ms,
        ));
        report.add(BenchmarkResult::with_baseline(
            "treepp /f",
            base.duration_ms,
            native.duration_ms,
        ));
        report.add(BenchmarkResult::with_baseline(
            "treepp /f /nb",
            nb.duration_ms,
            native.duration_ms,
        ));
        report.add(BenchmarkResult::with_baseline(
            "treepp /f /nb /b",
            batch.duration_ms,
            native.duration_ms,
        ));

        println!("{}", report.to_markdown_table());
    }

    /// Compares thread scaling on .cargo directory.
    #[test]
    fn compare_thread_scaling_cargo() {
        let Some(cargo_dir) = get_cargo_dir() else {
            eprintln!("Skipping: .cargo directory not found");
            return;
        };

        println!("\n=== Thread scaling comparison on .cargo ===\n");

        let native = benchmark_native_tree(&cargo_dir, &["/F"], "tree /f (Windows Native)");

        let mut report = BenchmarkReport::new("Thread Scaling (.cargo)");
        report.add(BenchmarkResult::with_baseline(
            "tree /f (Windows Native)",
            native.duration_ms,
            native.duration_ms,
        ));

        for threads in [1, 2, 4, 8, 16, 32] {
            let result = benchmark_treepp(
                &cargo_dir,
                &["/F", "/NB", "/B", "/T", &threads.to_string()],
                &format!("treepp /f /nb /b /t {}", threads),
            );
            report.add(BenchmarkResult::with_baseline(
                &format!("treepp /f /nb /b /t {}", threads),
                result.duration_ms,
                native.duration_ms,
            ));
        }

        println!("{}", report.to_markdown_table());
    }
}

// ============================================================================
// Full Benchmark Report Tests (Windows Directory)
// ============================================================================

#[cfg(test)]
mod full_benchmark {
    use super::*;

    /// Generates the full performance comparison report using C:\Windows.
    ///
    /// This test is marked as ignored by default since it takes a long time.
    /// Run with: cargo test --release full_windows_benchmark -- --ignored --nocapture
    #[test]
    #[ignore]
    fn full_windows_benchmark() {
        let windows_dir = get_windows_dir();

        if !windows_dir.exists() {
            eprintln!("Skipping: C:\\Windows not found");
            return;
        }

        println!("\n=== Full Performance Benchmark (C:\\Windows) ===\n");
        println!("This benchmark may take several minutes...\n");

        let mut report = BenchmarkReport::new("Performance Comparison (C:\\Windows)");

        // Native tree baseline
        println!("Benchmarking: tree /f (Windows Native)...");
        let native = benchmark_native_tree(&windows_dir, &["/F"], "tree /f (Windows Native)");
        report.add(BenchmarkResult::with_baseline(
            "tree /f (Windows Native)",
            native.duration_ms,
            native.duration_ms,
        ));

        // treepp basic
        println!("Benchmarking: treepp /f...");
        let treepp_basic = benchmark_treepp(&windows_dir, &["/F"], "treepp /f");
        report.add(BenchmarkResult::with_baseline(
            "treepp /f",
            treepp_basic.duration_ms,
            native.duration_ms,
        ));

        // treepp with no banner
        println!("Benchmarking: treepp /f /nb...");
        let treepp_nb = benchmark_treepp(&windows_dir, &["/F", "/NB"], "treepp /f /nb");
        report.add(BenchmarkResult::with_baseline(
            "treepp /f /nb",
            treepp_nb.duration_ms,
            native.duration_ms,
        ));

        // treepp with batch mode
        println!("Benchmarking: treepp /f /nb /b...");
        let treepp_batch = benchmark_treepp(&windows_dir, &["/F", "/NB", "/B"], "treepp /f /nb /b");
        report.add(BenchmarkResult::with_baseline(
            "treepp /f /nb /b",
            treepp_batch.duration_ms,
            native.duration_ms,
        ));

        // Thread variations
        for threads in [1, 2, 4, 8, 16, 32] {
            println!("Benchmarking: treepp /f /nb /b /t {}...", threads);
            let result = benchmark_treepp(
                &windows_dir,
                &["/F", "/NB", "/B", "/T", &threads.to_string()],
                &format!("treepp /f /nb /b /t {}", threads),
            );
            report.add(BenchmarkResult::with_baseline(
                &format!("treepp /f /nb /b /t {}", threads),
                result.duration_ms,
                native.duration_ms,
            ));
        }

        println!("\n{}", report.to_markdown_table());
    }

    /// Generates feature performance report.
    #[test]
    #[ignore]
    fn full_feature_benchmark() {
        let Some(cargo_dir) = get_cargo_dir() else {
            eprintln!("Skipping: .cargo directory not found");
            return;
        };

        println!("\n=== Feature Performance Benchmark (.cargo) ===\n");

        let baseline = benchmark_treepp(&cargo_dir, &["/F"], "/F (baseline)");

        let mut report = BenchmarkReport::new("Feature Performance Impact");

        // Add baseline
        let mut baseline_result = BenchmarkResult::new("/F (baseline)", baseline.duration_ms);
        baseline_result.baseline_ms = Some(baseline.duration_ms);
        report.add(baseline_result);

        // Test each feature
        let features = [
            (&["/F", "/A"][..], "/F /A"),
            (&["/F", "/S"][..], "/F /S"),
            (&["/F", "/DT"][..], "/F /DT"),
            (&["/F", "/HR", "/S"][..], "/F /S /HR"),
            (&["/F", "/FP"][..], "/F /FP"),
            (&["/F", "/R"][..], "/F /R"),
            (&["/F", "/DU"][..], "/F /DU"),
            (&["/F", "/RP"][..], "/F /RP"),
            (&["/F", "/NI"][..], "/F /NI"),
            (&["/F", "/NB"][..], "/F /NB"),
            (&["/F", "/L", "3"][..], "/F /L 3"),
            (&["/F", "/P"][..], "/F /P"),
            (&["/F", "/G"][..], "/F /G"),
            (&["/F", "/X", "*.rs"][..], "/F /X *.rs"),
            (&["/F", "/M", "*.toml"][..], "/F /M *.toml"),
        ];

        for (args, desc) in features {
            println!("Benchmarking: {}...", desc);
            let result = benchmark_treepp(&cargo_dir, args, desc);
            report.add(BenchmarkResult::with_baseline(
                desc,
                result.duration_ms,
                baseline.duration_ms,
            ));
        }

        println!("\n{}", report.to_detailed_markdown());
    }

    /// Combined features benchmark.
    #[test]
    #[ignore]
    fn full_combined_benchmark() {
        let Some(cargo_dir) = get_cargo_dir() else {
            eprintln!("Skipping: .cargo directory not found");
            return;
        };

        println!("\n=== Combined Features Benchmark (.cargo) ===\n");

        let baseline = benchmark_treepp(&cargo_dir, &["/F"], "/F (baseline)");

        let combinations = [
            (&["/F", "/S", "/DT"][..], "/F /S /DT"),
            (&["/F", "/S", "/DT", "/HR"][..], "/F /S /DT /HR"),
            (&["/F", "/S", "/DT", "/HR", "/RP"][..], "/F /S /DT /HR /RP"),
            (&["/F", "/S", "/DT", "/HR", "/RP", "/FP"][..], "/F /S /DT /HR /RP /FP"),
            (&["/F", "/NB", "/B"][..], "/F /NB /B"),
            (&["/F", "/NB", "/B", "/S", "/DT"][..], "/F /NB /B /S /DT"),
        ];

        let mut report = BenchmarkReport::new("Combined Features Performance");

        let mut baseline_result = BenchmarkResult::new("/F (baseline)", baseline.duration_ms);
        baseline_result.baseline_ms = Some(baseline.duration_ms);
        report.add(baseline_result);

        for (args, desc) in combinations {
            println!("Benchmarking: {}...", desc);
            let result = benchmark_treepp(&cargo_dir, args, desc);
            report.add(BenchmarkResult::with_baseline(
                desc,
                result.duration_ms,
                baseline.duration_ms,
            ));
        }

        println!("\n{}", report.to_detailed_markdown());
    }
}

// ============================================================================
// Stress Tests
// ============================================================================

#[cfg(test)]
mod stress_tests {
    use super::*;

    /// Tests performance on very deep nesting.
    #[test]
    fn stress_deep_nesting() {
        let dir = TempDir::new().unwrap();

        let mut current = dir.path().to_path_buf();
        for i in 0..25 {
            current = current.join(format!("level_{:02}", i));
            fs::create_dir_all(&current).unwrap();
            File::create(current.join("file.txt")).unwrap();
        }

        let result = benchmark_treepp(dir.path(), &["/F"], "25 levels deep");
        println!("Deep nesting (25 levels): {:.2} ms", result.duration_ms);
    }

    /// Tests performance on very wide directory.
    #[test]
    fn stress_wide_directory() {
        let dir = TempDir::new().unwrap();

        for i in 0..500 {
            fs::create_dir(dir.path().join(format!("dir_{:04}", i))).unwrap();
        }

        let result = benchmark_treepp(dir.path(), &[], "500 directories");
        println!("Wide directory (500 dirs): {:.2} ms", result.duration_ms);
    }

    /// Tests performance on many files.
    #[test]
    fn stress_many_files() {
        let dir = TempDir::new().unwrap();

        for i in 0..1000 {
            File::create(dir.path().join(format!("file_{:04}.txt", i))).unwrap();
        }

        let result = benchmark_treepp(dir.path(), &["/F"], "1000 files");
        println!("Many files (1000): {:.2} ms", result.duration_ms);
    }

    /// Tests performance on complex mixed structure.
    #[test]
    fn stress_complex_structure() {
        let dir = TempDir::new().unwrap();

        for i in 0..30 {
            let module = dir.path().join(format!("module_{:02}", i));
            fs::create_dir_all(module.join("src/core/utils")).unwrap();
            fs::create_dir_all(module.join("tests/unit")).unwrap();
            fs::create_dir_all(module.join("tests/integration")).unwrap();
            fs::create_dir_all(module.join("docs/api")).unwrap();

            for j in 0..20 {
                File::create(module.join("src/core").join(format!("file_{}.rs", j))).unwrap();
                File::create(module.join("src/core/utils").join(format!("util_{}.rs", j))).unwrap();
            }

            for j in 0..10 {
                File::create(module.join("tests/unit").join(format!("test_{}.rs", j))).unwrap();
                File::create(
                    module
                        .join("tests/integration")
                        .join(format!("int_test_{}.rs", j)),
                )
                    .unwrap();
            }

            File::create(module.join("Cargo.toml")).unwrap();
            File::create(module.join("README.md")).unwrap();
        }

        let result = benchmark_treepp(dir.path(), &["/F"], "complex structure");
        println!("Complex structure: {:.2} ms", result.duration_ms);

        // Also test with batch mode
        let batch_result = benchmark_treepp(dir.path(), &["/F", "/NB", "/B"], "complex with batch");
        let speedup = result.duration_ms / batch_result.duration_ms;
        println!(
            "Complex with batch: {:.2} ms (speedup: {:.2}x)",
            batch_result.duration_ms, speedup
        );
    }
}
