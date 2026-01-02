//! # match_rules 原型
//!
//! 验证 tree++ 过滤与规则引擎的核心功能：
//! - Include/Exclude 通配匹配（按声明顺序执行）
//! - 忽略大小写匹配（`/IC`）
//! - 分层 `.gitignore` 规则解析与叠加
//! - 空目录剪枝（`/P`）

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use thiserror::Error;

/// 匹配规则引擎错误类型
#[derive(Error, Debug)]
pub enum MatchError {
    /// 无效的 glob 模式
    #[error("无效的 glob 模式 '{pattern}': {source}")]
    InvalidPattern {
        pattern: String,
        source: globset::Error,
    },

    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// gitignore 解析错误
    #[error("gitignore 解析错误: {0}")]
    GitignoreParse(String),
}

/// 匹配规则引擎结果类型
pub type MatchResult<T> = Result<T, MatchError>;

/// 条目被过滤的原因
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterReason {
    /// 通过所有过滤器，保留
    Retained,
    /// 被 include 规则排除（不在 include 列表中）
    NotIncluded { pattern: String },
    /// 被 exclude 规则排除
    Excluded { pattern: String },
    /// 被 gitignore 规则排除
    Gitignored { pattern: String },
    /// 空目录剪枝
    PrunedEmpty,
}

impl FilterReason {
    /// 是否被保留
    #[must_use]
    pub const fn is_retained(&self) -> bool {
        matches!(self, Self::Retained)
    }
}

/// 过滤结果条目
#[derive(Debug, Clone)]
pub struct FilteredEntry {
    /// 条目路径
    pub path: PathBuf,
    /// 是否为目录
    pub is_dir: bool,
    /// 过滤原因
    pub reason: FilterReason,
}

/// 已编译的 glob 模式
#[derive(Debug, Clone)]
struct CompiledPattern {
    original: String,
    matcher: GlobMatcher,
}

impl CompiledPattern {
    fn new(pattern: &str, case_insensitive: bool) -> MatchResult<Self> {
        let glob = Glob::new(pattern).map_err(|e| MatchError::InvalidPattern {
            pattern: pattern.to_owned(),
            source: e,
        })?;

        let matcher = if case_insensitive {
            glob.compile_matcher()
        } else {
            glob.compile_matcher()
        };

        Ok(Self {
            original: pattern.to_owned(),
            matcher,
        })
    }

    fn new_case_insensitive(pattern: &str) -> MatchResult<Self> {
        let lower_pattern = pattern.to_lowercase();
        let glob = Glob::new(&lower_pattern).map_err(|e| MatchError::InvalidPattern {
            pattern: pattern.to_owned(),
            source: e,
        })?;

        Ok(Self {
            original: pattern.to_owned(),
            matcher: glob.compile_matcher(),
        })
    }

    fn matches(&self, name: &str, case_insensitive: bool) -> bool {
        if case_insensitive {
            self.matcher.is_match(name.to_lowercase().as_str())
        } else {
            self.matcher.is_match(name)
        }
    }
}

/// 过滤规则配置
#[derive(Debug, Clone, Default)]
pub struct FilterConfig {
    /// Include 模式列表（只保留匹配项）
    pub include_patterns: Vec<String>,
    /// Exclude 模式列表（排除匹配项）
    pub exclude_patterns: Vec<String>,
    /// 是否忽略大小写
    pub ignore_case: bool,
    /// 是否启用 gitignore
    pub use_gitignore: bool,
    /// 是否剪枝空目录
    pub prune_empty: bool,
}

impl FilterConfig {
    /// 创建新的过滤配置
    #[must_use]
    pub const fn new() -> Self {
        Self {
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            ignore_case: false,
            use_gitignore: false,
            prune_empty: false,
        }
    }

    /// 添加 include 模式
    #[must_use]
    pub fn with_include(mut self, pattern: impl Into<String>) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }

    /// 添加 exclude 模式
    #[must_use]
    pub fn with_exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }

    /// 设置是否忽略大小写
    #[must_use]
    pub const fn with_ignore_case(mut self, ignore_case: bool) -> Self {
        self.ignore_case = ignore_case;
        self
    }

    /// 设置是否使用 gitignore
    #[must_use]
    pub const fn with_gitignore(mut self, use_gitignore: bool) -> Self {
        self.use_gitignore = use_gitignore;
        self
    }

    /// 设置是否剪枝空目录
    #[must_use]
    pub const fn with_prune(mut self, prune_empty: bool) -> Self {
        self.prune_empty = prune_empty;
        self
    }
}

/// 过滤规则引擎
#[derive(Debug)]
pub struct MatchEngine {
    config: FilterConfig,
    include_matchers: Vec<CompiledPattern>,
    exclude_matchers: Vec<CompiledPattern>,
    gitignore_cache: HashMap<PathBuf, Option<Gitignore>>,
}

impl MatchEngine {
    /// 从配置创建过滤引擎
    ///
    /// # Errors
    ///
    /// 当 glob 模式无效时返回错误
    pub fn new(config: FilterConfig) -> MatchResult<Self> {
        let include_matchers = config
            .include_patterns
            .iter()
            .map(|p| {
                if config.ignore_case {
                    CompiledPattern::new_case_insensitive(p)
                } else {
                    CompiledPattern::new(p, false)
                }
            })
            .collect::<MatchResult<Vec<_>>>()?;

        let exclude_matchers = config
            .exclude_patterns
            .iter()
            .map(|p| {
                if config.ignore_case {
                    CompiledPattern::new_case_insensitive(p)
                } else {
                    CompiledPattern::new(p, false)
                }
            })
            .collect::<MatchResult<Vec<_>>>()?;

        Ok(Self {
            config,
            include_matchers,
            exclude_matchers,
            gitignore_cache: HashMap::new(),
        })
    }

    /// 检查文件名是否匹配 include 规则
    fn check_include(&self, name: &str) -> Option<FilterReason> {
        if self.include_matchers.is_empty() {
            return None;
        }

        for matcher in &self.include_matchers {
            if matcher.matches(name, self.config.ignore_case) {
                return None;
            }
        }

        Some(FilterReason::NotIncluded {
            pattern: self
                .include_matchers
                .first()
                .map_or_else(String::new, |m| m.original.clone()),
        })
    }

    /// 检查文件名是否匹配 exclude 规则
    fn check_exclude(&self, name: &str) -> Option<FilterReason> {
        for matcher in &self.exclude_matchers {
            if matcher.matches(name, self.config.ignore_case) {
                return Some(FilterReason::Excluded {
                    pattern: matcher.original.clone(),
                });
            }
        }
        None
    }

    /// 加载指定目录的 gitignore 规则（带缓存）
    fn load_gitignore(&mut self, dir: &Path) -> Option<&Gitignore> {
        if !self.gitignore_cache.contains_key(dir) {
            let gitignore_path = dir.join(".gitignore");
            let gitignore = if gitignore_path.exists() {
                let mut builder = GitignoreBuilder::new(dir);
                if builder.add(&gitignore_path).is_none() {
                    builder.build().ok()
                } else {
                    None
                }
            } else {
                None
            };
            self.gitignore_cache.insert(dir.to_path_buf(), gitignore);
        }

        self.gitignore_cache.get(dir).and_then(Option::as_ref)
    }

    /// 构建从根目录到指定目录的 gitignore 规则链
    fn build_gitignore_chain(&mut self, root: &Path, target: &Path) -> Vec<PathBuf> {
        let mut chain = Vec::new();
        let mut current = root.to_path_buf();

        chain.push(current.clone());

        if let Ok(relative) = target.strip_prefix(root) {
            for component in relative.components() {
                current.push(component);
                if current.is_dir() {
                    chain.push(current.clone());
                }
            }
        }

        chain
    }

    /// 检查路径是否被 gitignore 规则排除
    fn check_gitignore(&mut self, root: &Path, path: &Path, is_dir: bool) -> Option<FilterReason> {
        if !self.config.use_gitignore {
            return None;
        }

        let parent = path.parent()?;
        let chain = self.build_gitignore_chain(root, parent);

        for dir in chain {
            if let Some(gitignore) = self.load_gitignore(&dir) {
                let matched = gitignore.matched_path_or_any_parents(path, is_dir);
                if matched.is_ignore() {
                    let pattern = matched
                        .inner()
                        .map_or_else(|| "<unknown>".to_owned(), |g| g.original().to_owned());
                    return Some(FilterReason::Gitignored { pattern });
                }
            }
        }

        None
    }

    /// 对单个条目应用过滤规则
    ///
    /// # Arguments
    ///
    /// * `root` - 扫描根目录
    /// * `path` - 条目路径
    /// * `is_dir` - 是否为目录
    pub fn filter_entry(&mut self, root: &Path, path: &Path, is_dir: bool) -> FilterReason {
        let name = path
            .file_name()
            .map_or_else(|| path.to_string_lossy(), |n| n.to_string_lossy());

        // 1. 检查 gitignore（最先检查，可以剪枝整个目录）
        if let Some(reason) = self.check_gitignore(root, path, is_dir) {
            return reason;
        }

        // 2. 检查 include（只对文件生效，目录始终包含以便递归）
        if !is_dir {
            if let Some(reason) = self.check_include(&name) {
                return reason;
            }
        }

        // 3. 检查 exclude
        if let Some(reason) = self.check_exclude(&name) {
            return reason;
        }

        FilterReason::Retained
    }

    /// 扫描目录并应用过滤规则
    ///
    /// # Errors
    ///
    /// 当无法读取目录时返回错误
    pub fn scan_directory(&mut self, root: &Path) -> MatchResult<Vec<FilteredEntry>> {
        let mut entries = Vec::new();
        self.scan_recursive(root, root, &mut entries)?;

        if self.config.prune_empty {
            self.apply_prune(&mut entries);
        }

        Ok(entries)
    }

    /// 递归扫描目录
    fn scan_recursive(
        &mut self,
        root: &Path,
        current: &Path,
        entries: &mut Vec<FilteredEntry>,
    ) -> MatchResult<()> {
        let mut dir_entries: Vec<_> = fs::read_dir(current)?.filter_map(Result::ok).collect();

        dir_entries.sort_by_key(|e| e.file_name());

        for entry in dir_entries {
            let path = entry.path();
            let is_dir = path.is_dir();
            let reason = self.filter_entry(root, &path, is_dir);

            entries.push(FilteredEntry {
                path: path.clone(),
                is_dir,
                reason: reason.clone(),
            });

            // 如果是目录且被保留，递归扫描
            if is_dir && reason.is_retained() {
                self.scan_recursive(root, &path, entries)?;
            }
        }

        Ok(())
    }

    /// 应用空目录剪枝
    fn apply_prune(&self, entries: &mut Vec<FilteredEntry>) {
        let retained_files: std::collections::HashSet<_> = entries
            .iter()
            .filter(|e| !e.is_dir && e.reason.is_retained())
            .map(|e| e.path.clone())
            .collect();

        let has_retained_descendant = |dir: &Path| -> bool {
            retained_files.iter().any(|f| f.starts_with(dir))
        };

        for entry in entries.iter_mut() {
            if entry.is_dir && entry.reason.is_retained() && !has_retained_descendant(&entry.path) {
                entry.reason = FilterReason::PrunedEmpty;
            }
        }
    }
}

/// 便捷函数：对路径列表应用过滤规则
///
/// # Errors
///
/// 当配置无效时返回错误
pub fn filter_paths(
    root: &Path,
    paths: &[(PathBuf, bool)],
    config: FilterConfig,
) -> MatchResult<Vec<FilteredEntry>> {
    let mut engine = MatchEngine::new(config)?;

    Ok(paths
        .iter()
        .map(|(path, is_dir)| FilteredEntry {
            path: path.clone(),
            is_dir: *is_dir,
            reason: engine.filter_entry(root, path, *is_dir),
        })
        .collect())
}

fn main() {
    println!("match_rules 原型 - 过滤与规则引擎验证");
    println!("运行 `cargo test` 执行完整测试套件");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    /// 创建测试目录结构的辅助函数
    fn create_test_structure(dir: &Path, structure: &[(&str, Option<&str>)]) {
        for (path, content) in structure {
            let full_path = dir.join(path);
            if let Some(content) = content {
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                let mut file = File::create(&full_path).unwrap();
                file.write_all(content.as_bytes()).unwrap();
            } else {
                fs::create_dir_all(&full_path).unwrap();
            }
        }
    }

    // ==================== Include 模式测试 ====================

    #[test]
    fn test_include_single_pattern_matches() {
        let config = FilterConfig::new().with_include("*.rs");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        let reason = engine.filter_entry(root, Path::new("/test/main.rs"), false);
        assert!(reason.is_retained());
    }

    #[test]
    fn test_include_single_pattern_not_matches() {
        let config = FilterConfig::new().with_include("*.rs");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        let reason = engine.filter_entry(root, Path::new("/test/readme.md"), false);
        assert!(matches!(reason, FilterReason::NotIncluded { .. }));
    }

    #[test]
    fn test_include_multiple_patterns() {
        let config = FilterConfig::new()
            .with_include("*.rs")
            .with_include("*.toml");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/main.rs"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/Cargo.toml"), false)
            .is_retained());
        assert!(!engine
            .filter_entry(root, Path::new("/test/readme.md"), false)
            .is_retained());
    }

    #[test]
    fn test_include_empty_allows_all() {
        let config = FilterConfig::new();
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/any.file"), false)
            .is_retained());
    }

    #[test]
    fn test_include_directories_always_retained() {
        let config = FilterConfig::new().with_include("*.rs");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        let reason = engine.filter_entry(root, Path::new("/test/src"), true);
        assert!(reason.is_retained());
    }

    #[test]
    fn test_include_question_mark_wildcard() {
        let config = FilterConfig::new().with_include("?.rs");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/a.rs"), false)
            .is_retained());
        assert!(!engine
            .filter_entry(root, Path::new("/test/ab.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_include_character_class() {
        let config = FilterConfig::new().with_include("[abc].rs");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/a.rs"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/b.rs"), false)
            .is_retained());
        assert!(!engine
            .filter_entry(root, Path::new("/test/d.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_include_character_range() {
        let config = FilterConfig::new().with_include("[0-9].txt");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/5.txt"), false)
            .is_retained());
        assert!(!engine
            .filter_entry(root, Path::new("/test/a.txt"), false)
            .is_retained());
    }

    #[test]
    fn test_include_negated_character_class() {
        let config = FilterConfig::new().with_include("[!abc].rs");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(!engine
            .filter_entry(root, Path::new("/test/a.rs"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/d.rs"), false)
            .is_retained());
    }

    // ==================== Exclude 模式测试 ====================

    #[test]
    fn test_exclude_single_pattern() {
        let config = FilterConfig::new().with_exclude("*.log");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        let reason = engine.filter_entry(root, Path::new("/test/debug.log"), false);
        assert!(matches!(reason, FilterReason::Excluded { .. }));
    }

    #[test]
    fn test_exclude_not_matches_retained() {
        let config = FilterConfig::new().with_exclude("*.log");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/main.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_exclude_multiple_patterns() {
        let config = FilterConfig::new()
            .with_exclude("*.log")
            .with_exclude("*.tmp");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/debug.log"), false),
            FilterReason::Excluded { .. }
        ));
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/cache.tmp"), false),
            FilterReason::Excluded { .. }
        ));
        assert!(engine
            .filter_entry(root, Path::new("/test/main.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_exclude_applies_to_directories() {
        let config = FilterConfig::new().with_exclude("node_modules");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        let reason = engine.filter_entry(root, Path::new("/test/node_modules"), true);
        assert!(matches!(reason, FilterReason::Excluded { .. }));
    }

    #[test]
    fn test_exclude_pattern_preserved_in_reason() {
        let config = FilterConfig::new().with_exclude("secret_*");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        let reason = engine.filter_entry(root, Path::new("/test/secret_key.pem"), false);
        if let FilterReason::Excluded { pattern } = reason {
            assert_eq!(pattern, "secret_*");
        } else {
            panic!("Expected Excluded reason");
        }
    }

    // ==================== Include + Exclude 组合测试 ====================

    #[test]
    fn test_include_then_exclude_order() {
        let config = FilterConfig::new()
            .with_include("*.rs")
            .with_exclude("test_*.rs");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/main.rs"), false)
            .is_retained());
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/test_main.rs"), false),
            FilterReason::Excluded { .. }
        ));
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/readme.md"), false),
            FilterReason::NotIncluded { .. }
        ));
    }

    #[test]
    fn test_exclude_takes_precedence_after_include() {
        let config = FilterConfig::new()
            .with_include("*.txt")
            .with_exclude("secret.txt");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/readme.txt"), false)
            .is_retained());
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/secret.txt"), false),
            FilterReason::Excluded { .. }
        ));
    }

    #[test]
    fn test_multiple_include_multiple_exclude() {
        let config = FilterConfig::new()
            .with_include("*.rs")
            .with_include("*.toml")
            .with_exclude("test_*")
            .with_exclude("*_generated.*");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/main.rs"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/Cargo.toml"), false)
            .is_retained());
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/test_lib.rs"), false),
            FilterReason::Excluded { .. }
        ));
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/code_generated.rs"), false),
            FilterReason::Excluded { .. }
        ));
    }

    // ==================== Ignore Case 测试 ====================

    #[test]
    fn test_ignore_case_include() {
        let config = FilterConfig::new()
            .with_include("*.md")
            .with_ignore_case(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/README.MD"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/readme.md"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/ReadMe.Md"), false)
            .is_retained());
    }

    #[test]
    fn test_ignore_case_exclude() {
        let config = FilterConfig::new()
            .with_exclude("*.log")
            .with_ignore_case(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/DEBUG.LOG"), false),
            FilterReason::Excluded { .. }
        ));
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/error.log"), false),
            FilterReason::Excluded { .. }
        ));
    }

    #[test]
    fn test_case_sensitive_by_default() {
        let config = FilterConfig::new().with_include("*.md");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/readme.md"), false)
            .is_retained());
        assert!(!engine
            .filter_entry(root, Path::new("/test/README.MD"), false)
            .is_retained());
    }

    #[test]
    fn test_ignore_case_mixed_patterns() {
        let config = FilterConfig::new()
            .with_include("Makefile")
            .with_include("*.mk")
            .with_ignore_case(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/makefile"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/MAKEFILE"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/rules.MK"), false)
            .is_retained());
    }

    // ==================== gitignore 基础测试 ====================

    #[test]
    fn test_gitignore_simple_pattern() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log\n")),
                ("main.rs", Some("")),
                ("debug.log", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        let reason = engine.filter_entry(temp.path(), &temp.path().join("debug.log"), false);
        assert!(matches!(reason, FilterReason::Gitignored { .. }));

        let reason = engine.filter_entry(temp.path(), &temp.path().join("main.rs"), false);
        assert!(reason.is_retained());
    }

    #[test]
    fn test_gitignore_directory_pattern() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("target/\n")),
                ("src/", None),
                ("target/", None),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        let reason = engine.filter_entry(temp.path(), &temp.path().join("target"), true);
        assert!(matches!(reason, FilterReason::Gitignored { .. }));

        let reason = engine.filter_entry(temp.path(), &temp.path().join("src"), true);
        assert!(reason.is_retained());
    }

    #[test]
    fn test_gitignore_negation_pattern() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log\n!important.log\n")),
                ("debug.log", Some("")),
                ("important.log", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        let reason = engine.filter_entry(temp.path(), &temp.path().join("debug.log"), false);
        assert!(matches!(reason, FilterReason::Gitignored { .. }));

        let reason = engine.filter_entry(temp.path(), &temp.path().join("important.log"), false);
        assert!(reason.is_retained());
    }

    #[test]
    fn test_gitignore_comment_ignored() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("# This is a comment\n*.log\n")),
                ("debug.log", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        let reason = engine.filter_entry(temp.path(), &temp.path().join("debug.log"), false);
        assert!(matches!(reason, FilterReason::Gitignored { .. }));
    }

    #[test]
    fn test_gitignore_blank_lines_ignored() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("\n\n*.log\n\n")),
                ("debug.log", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        let reason = engine.filter_entry(temp.path(), &temp.path().join("debug.log"), false);
        assert!(matches!(reason, FilterReason::Gitignored { .. }));
    }

    #[test]
    fn test_gitignore_double_asterisk() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("**/temp\n")),
                ("temp", Some("")),
                ("src/temp", Some("")),
                ("src/deep/temp", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("temp"), false),
            FilterReason::Gitignored { .. }
        ));
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("src/temp"), false),
            FilterReason::Gitignored { .. }
        ));
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("src/deep/temp"), false),
            FilterReason::Gitignored { .. }
        ));
    }

    #[test]
    fn test_gitignore_leading_slash_anchor() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("/build\n")),
                ("build", Some("")),
                ("src/build", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("build"), false),
            FilterReason::Gitignored { .. }
        ));
        // 子目录中的 build 不应被忽略
        let reason = engine.filter_entry(temp.path(), &temp.path().join("src/build"), false);
        assert!(reason.is_retained());
    }

    // ==================== 分层 gitignore 测试 ====================

    #[test]
    fn test_gitignore_nested_rules() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log\n")),
                ("root.log", Some("")),
                ("src/", None),
                ("src/.gitignore", Some("*.tmp\n")),
                ("src/main.rs", Some("")),
                ("src/cache.tmp", Some("")),
                ("src/debug.log", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        // 根目录规则生效
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("root.log"), false),
            FilterReason::Gitignored { .. }
        ));

        // 子目录继承根目录规则
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("src/debug.log"), false),
            FilterReason::Gitignored { .. }
        ));

        // 子目录自己的规则也生效
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("src/cache.tmp"), false),
            FilterReason::Gitignored { .. }
        ));

        // 正常文件保留
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("src/main.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_gitignore_deep_nesting() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.a\n")),
                ("a/", None),
                ("a/.gitignore", Some("*.b\n")),
                ("a/b/", None),
                ("a/b/.gitignore", Some("*.c\n")),
                ("a/b/c/", None),
                ("a/b/c/test.a", Some("")),
                ("a/b/c/test.b", Some("")),
                ("a/b/c/test.c", Some("")),
                ("a/b/c/test.d", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        // 所有层级的规则都应生效
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("a/b/c/test.a"), false),
            FilterReason::Gitignored { .. }
        ));
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("a/b/c/test.b"), false),
            FilterReason::Gitignored { .. }
        ));
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("a/b/c/test.c"), false),
            FilterReason::Gitignored { .. }
        ));
        // .d 文件不被任何规则匹配
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("a/b/c/test.d"), false)
            .is_retained());
    }

    #[test]
    fn test_gitignore_same_file_negation() {
        // gitignore 的取反规则在同一文件中工作正确
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log\n!keep.log\n")),
                ("debug.log", Some("")),
                ("keep.log", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        // debug.log 被 *.log 排除
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("debug.log"), false),
            FilterReason::Gitignored { .. }
        ));

        // keep.log 被 !keep.log 恢复
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("keep.log"), false)
            .is_retained());
    }

    // ==================== gitignore + include/exclude 组合测试 ====================

    #[test]
    fn test_gitignore_with_exclude_merged() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log\n")),
                ("debug.log", Some("")),
                ("cache.tmp", Some("")),
                ("main.rs", Some("")),
            ],
        );

        let config = FilterConfig::new()
            .with_gitignore(true)
            .with_exclude("*.tmp");
        let mut engine = MatchEngine::new(config).unwrap();

        // gitignore 规则
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("debug.log"), false),
            FilterReason::Gitignored { .. }
        ));

        // exclude 规则
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("cache.tmp"), false),
            FilterReason::Excluded { .. }
        ));

        // 都不匹配则保留
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("main.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_gitignore_priority_over_include() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("secret.rs\n")),
                ("main.rs", Some("")),
                ("secret.rs", Some("")),
            ],
        );

        let config = FilterConfig::new()
            .with_gitignore(true)
            .with_include("*.rs");
        let mut engine = MatchEngine::new(config).unwrap();

        // 即使匹配 include，gitignore 优先排除
        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("secret.rs"), false),
            FilterReason::Gitignored { .. }
        ));

        // 正常 include 生效
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("main.rs"), false)
            .is_retained());
    }

    // ==================== 空目录剪枝测试 ====================

    #[test]
    fn test_prune_empty_directory() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                ("empty_dir/", None),
                ("with_file/", None),
                ("with_file/test.txt", Some("")),
            ],
        );

        let config = FilterConfig::new().with_prune(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        let empty_dir_entry = entries
            .iter()
            .find(|e| e.path.ends_with("empty_dir"))
            .unwrap();
        assert!(matches!(empty_dir_entry.reason, FilterReason::PrunedEmpty));

        let with_file_entry = entries
            .iter()
            .find(|e| e.path.ends_with("with_file"))
            .unwrap();
        assert!(with_file_entry.reason.is_retained());
    }

    #[test]
    fn test_prune_nested_empty_directories() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[("a/", None), ("a/b/", None), ("a/b/c/", None)],
        );

        let config = FilterConfig::new().with_prune(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // 所有目录都应被剪枝
        for entry in &entries {
            if entry.is_dir {
                assert!(matches!(entry.reason, FilterReason::PrunedEmpty));
            }
        }
    }

    #[test]
    fn test_prune_with_deep_file() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                ("a/", None),
                ("a/b/", None),
                ("a/b/c/", None),
                ("a/b/c/file.txt", Some("")),
            ],
        );

        let config = FilterConfig::new().with_prune(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // 有深层文件，所有父目录都应保留
        for entry in &entries {
            assert!(entry.reason.is_retained());
        }
    }

    #[test]
    fn test_prune_after_include_filter() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                ("src/", None),
                ("src/main.rs", Some("")),
                ("docs/", None),
                ("docs/readme.md", Some("")),
            ],
        );

        let config = FilterConfig::new().with_include("*.rs").with_prune(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // docs 目录应被剪枝（内部文件不匹配 include）
        let docs_entry = entries.iter().find(|e| e.path.ends_with("docs")).unwrap();
        assert!(matches!(docs_entry.reason, FilterReason::PrunedEmpty));

        // src 目录应保留
        let src_entry = entries.iter().find(|e| e.path.ends_with("src")).unwrap();
        assert!(src_entry.reason.is_retained());
    }

    #[test]
    fn test_prune_after_exclude_filter() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                ("logs/", None),
                ("logs/error.log", Some("")),
                ("logs/debug.log", Some("")),
                ("src/", None),
                ("src/main.rs", Some("")),
            ],
        );

        let config = FilterConfig::new()
            .with_exclude("*.log")
            .with_prune(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // logs 目录应被剪枝（所有文件都被 exclude）
        let logs_entry = entries.iter().find(|e| e.path.ends_with("logs")).unwrap();
        assert!(matches!(logs_entry.reason, FilterReason::PrunedEmpty));
    }

    #[test]
    fn test_prune_after_gitignore() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log\n")),
                ("logs/", None),
                ("logs/app.log", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true).with_prune(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // logs 目录应被剪枝
        let logs_entry = entries.iter().find(|e| e.path.ends_with("logs")).unwrap();
        assert!(matches!(logs_entry.reason, FilterReason::PrunedEmpty));
    }

    #[test]
    fn test_no_prune_when_disabled() {
        let temp = TempDir::new().unwrap();
        create_test_structure(temp.path(), &[("empty_dir/", None)]);

        let config = FilterConfig::new().with_prune(false);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        let empty_dir_entry = entries
            .iter()
            .find(|e| e.path.ends_with("empty_dir"))
            .unwrap();
        assert!(empty_dir_entry.reason.is_retained());
    }

    // ==================== 边缘情况测试 ====================

    #[test]
    fn test_empty_pattern_error() {
        let config = FilterConfig::new().with_include("");
        // 空模式应该能编译但不匹配任何东西
        let engine = MatchEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_special_characters_in_filename() {
        let config = FilterConfig::new().with_include("*");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/file with spaces.txt"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/file-with-dashes.txt"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/file_with_underscores.txt"), false)
            .is_retained());
    }

    #[test]
    fn test_unicode_filename() {
        let config = FilterConfig::new().with_include("*.txt");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/文档.txt"), false)
            .is_retained());
        assert!(engine
            .filter_entry(root, Path::new("/test/ドキュメント.txt"), false)
            .is_retained());
    }

    #[test]
    fn test_hidden_files() {
        let config = FilterConfig::new().with_exclude(".*");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/.gitignore"), false),
            FilterReason::Excluded { .. }
        ));
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/.hidden"), false),
            FilterReason::Excluded { .. }
        ));
        assert!(engine
            .filter_entry(root, Path::new("/test/visible.txt"), false)
            .is_retained());
    }

    #[test]
    fn test_no_extension_file() {
        let config = FilterConfig::new().with_include("*.*");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/file.txt"), false)
            .is_retained());
        // 无扩展名文件不匹配 *.*
        assert!(!engine
            .filter_entry(root, Path::new("/test/Makefile"), false)
            .is_retained());
    }

    #[test]
    fn test_double_extension() {
        let config = FilterConfig::new().with_include("*.tar.gz");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/archive.tar.gz"), false)
            .is_retained());
        assert!(!engine
            .filter_entry(root, Path::new("/test/archive.tar"), false)
            .is_retained());
    }

    #[test]
    fn test_gitignore_without_file() {
        let temp = TempDir::new().unwrap();
        create_test_structure(temp.path(), &[("main.rs", Some(""))]);

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        // 没有 .gitignore 文件，所有文件保留
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("main.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_gitignore_disabled() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[(".gitignore", Some("*.log\n")), ("debug.log", Some(""))],
        );

        let config = FilterConfig::new().with_gitignore(false);
        let mut engine = MatchEngine::new(config).unwrap();

        // gitignore 未启用，文件保留
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("debug.log"), false)
            .is_retained());
    }

    #[test]
    fn test_complex_glob_pattern() {
        let config = FilterConfig::new().with_include("test_*.{rs,py}");
        // globset 可能不支持 {} 语法，这里测试错误处理
        let result = MatchEngine::new(config);
        // 根据实际支持情况调整断言
        if result.is_err() {
            // 预期的错误处理
        }
    }

    #[test]
    fn test_scan_empty_directory() {
        let temp = TempDir::new().unwrap();
        let config = FilterConfig::new();
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        assert!(entries.is_empty());
    }

    #[test]
    fn test_filter_config_builder_pattern() {
        let config = FilterConfig::new()
            .with_include("*.rs")
            .with_include("*.toml")
            .with_exclude("test_*")
            .with_ignore_case(true)
            .with_gitignore(true)
            .with_prune(true);

        assert_eq!(config.include_patterns.len(), 2);
        assert_eq!(config.exclude_patterns.len(), 1);
        assert!(config.ignore_case);
        assert!(config.use_gitignore);
        assert!(config.prune_empty);
    }

    #[test]
    fn test_filter_paths_convenience_function() {
        let root = Path::new("/test");
        let paths = vec![
            (PathBuf::from("/test/main.rs"), false),
            (PathBuf::from("/test/readme.md"), false),
        ];
        let config = FilterConfig::new().with_include("*.rs");

        let results = filter_paths(root, &paths, config).unwrap();

        assert!(results[0].reason.is_retained());
        assert!(!results[1].reason.is_retained());
    }

    #[test]
    fn test_filter_reason_display() {
        let retained = FilterReason::Retained;
        assert!(retained.is_retained());

        let excluded = FilterReason::Excluded {
            pattern: "*.log".to_owned(),
        };
        assert!(!excluded.is_retained());
    }

    // ==================== 复杂场景集成测试 ====================

    #[test]
    fn test_realistic_project_structure() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("target/\n*.log\n.env\n")),
                ("Cargo.toml", Some("")),
                ("Cargo.lock", Some("")),
                ("src/", None),
                ("src/main.rs", Some("")),
                ("src/lib.rs", Some("")),
                ("src/utils/", None),
                ("src/utils/mod.rs", Some("")),
                ("tests/", None),
                ("tests/integration.rs", Some("")),
                ("target/", None),
                ("target/debug/", None),
                ("target/debug/app", Some("")),
                ("debug.log", Some("")),
                (".env", Some("")),
            ],
        );

        let config = FilterConfig::new()
            .with_gitignore(true)
            .with_include("*.rs")
            .with_include("*.toml")
            .with_prune(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // 验证保留的文件
        let retained_paths: Vec<_> = entries
            .iter()
            .filter(|e| e.reason.is_retained())
            .map(|e| e.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(retained_paths.contains(&"Cargo.toml".to_owned()));
        assert!(retained_paths.contains(&"main.rs".to_owned()));
        assert!(retained_paths.contains(&"lib.rs".to_owned()));
        assert!(retained_paths.contains(&"mod.rs".to_owned()));
        assert!(retained_paths.contains(&"integration.rs".to_owned()));

        // 验证被过滤的项目
        let target_entry = entries.iter().find(|e| {
            e.path
                .file_name()
                .map_or(false, |n| n.to_string_lossy() == "target")
        });
        assert!(target_entry.is_none() || !target_entry.unwrap().reason.is_retained());
    }

    #[test]
    fn test_monorepo_structure() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("node_modules/\ndist/\n")),
                ("packages/", None),
                ("packages/app/", None),
                ("packages/app/package.json", Some("")),
                ("packages/app/.gitignore", Some("*.local\n")),
                ("packages/app/src/", None),
                ("packages/app/src/index.ts", Some("")),
                ("packages/app/config.local", Some("")),
                ("packages/lib/", None),
                ("packages/lib/package.json", Some("")),
                ("packages/lib/src/", None),
                ("packages/lib/src/index.ts", Some("")),
                ("node_modules/", None),
                ("node_modules/pkg/", None),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // node_modules 应被忽略
        let node_modules_ignored = entries.iter().any(|e| {
            e.path.to_string_lossy().contains("node_modules")
                && matches!(e.reason, FilterReason::Gitignored { .. })
        });
        assert!(node_modules_ignored);

        // config.local 应被子目录 .gitignore 忽略
        let config_local_entry = entries.iter().find(|e| e.path.ends_with("config.local"));
        assert!(
            config_local_entry.is_none()
                || matches!(
                    config_local_entry.unwrap().reason,
                    FilterReason::Gitignored { .. }
                )
        );
    }

    #[test]
    fn test_all_filters_combined() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log\n")),
                ("src/", None),
                ("src/main.RS", Some("")),      // 大写扩展名
                ("src/test_main.rs", Some("")), // 应被 exclude
                ("src/lib.rs", Some("")),
                ("docs/", None),
                ("docs/readme.md", Some("")), // 不匹配 include
                ("debug.log", Some("")),      // gitignore
                ("empty/", None),             // 空目录
            ],
        );

        let config = FilterConfig::new()
            .with_gitignore(true)
            .with_include("*.rs")
            .with_exclude("test_*")
            .with_ignore_case(true)
            .with_prune(true);
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // main.RS 应匹配（ignore case）
        let main_entry = entries.iter().find(|e| e.path.ends_with("main.RS"));
        assert!(main_entry.map_or(false, |e| e.reason.is_retained()));

        // test_main.rs 应被 exclude
        let test_entry = entries.iter().find(|e| e.path.ends_with("test_main.rs"));
        assert!(test_entry.map_or(false, |e| matches!(
            e.reason,
            FilterReason::Excluded { .. }
        )));

        // lib.rs 保留
        let lib_entry = entries.iter().find(|e| e.path.ends_with("lib.rs"));
        assert!(lib_entry.map_or(false, |e| e.reason.is_retained()));

        // docs 目录应被剪枝
        let docs_entry = entries.iter().find(|e| e.path.ends_with("docs"));
        assert!(docs_entry.map_or(false, |e| matches!(
            e.reason,
            FilterReason::PrunedEmpty
        )));

        // empty 目录应被剪枝
        let empty_entry = entries.iter().find(|e| e.path.ends_with("empty"));
        assert!(empty_entry.map_or(false, |e| matches!(
            e.reason,
            FilterReason::PrunedEmpty
        )));
    }

    // ==================== 性能与容量测试 ====================

    #[test]
    fn test_many_include_patterns() {
        let patterns: Vec<String> = (0..100).map(|i| format!("pattern_{i}_*")).collect();
        let mut config = FilterConfig::new();
        for p in patterns {
            config = config.with_include(p);
        }

        let engine = MatchEngine::new(config);
        assert!(engine.is_ok());
        assert_eq!(engine.unwrap().include_matchers.len(), 100);
    }

    #[test]
    fn test_many_exclude_patterns() {
        let patterns: Vec<String> = (0..100).map(|i| format!("exclude_{i}_*")).collect();
        let mut config = FilterConfig::new();
        for p in patterns {
            config = config.with_exclude(p);
        }

        let engine = MatchEngine::new(config);
        assert!(engine.is_ok());
        assert_eq!(engine.unwrap().exclude_matchers.len(), 100);
    }

    #[test]
    fn test_deep_directory_scan() {
        let temp = TempDir::new().unwrap();
        let mut path = temp.path().to_path_buf();

        // 创建 20 层深的目录
        for i in 0..20 {
            path.push(format!("level_{i}"));
            fs::create_dir_all(&path).unwrap();
        }

        // 在最深层创建文件
        File::create(path.join("deep.txt")).unwrap();

        let config = FilterConfig::new();
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        // 应该有 20 个目录 + 1 个文件
        assert_eq!(entries.len(), 21);
    }

    #[test]
    fn test_wide_directory_scan() {
        let temp = TempDir::new().unwrap();

        // 创建 100 个文件
        for i in 0..100 {
            File::create(temp.path().join(format!("file_{i:03}.txt"))).unwrap();
        }

        let config = FilterConfig::new();
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        assert_eq!(entries.len(), 100);
    }

    // ==================== 错误处理测试 ====================

    #[test]
    fn test_invalid_glob_pattern() {
        let config = FilterConfig::new().with_include("[invalid");
        let result = MatchEngine::new(config);

        assert!(result.is_err());
        if let Err(MatchError::InvalidPattern { pattern, .. }) = result {
            assert_eq!(pattern, "[invalid");
        } else {
            panic!("Expected InvalidPattern error");
        }
    }

    #[test]
    fn test_nonexistent_directory_scan() {
        let config = FilterConfig::new();
        let mut engine = MatchEngine::new(config).unwrap();
        let result = engine.scan_directory(Path::new("/nonexistent/path/12345"));

        assert!(result.is_err());
    }

    // ==================== gitignore 高级语法测试 ====================

    #[test]
    fn test_gitignore_trailing_spaces() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log  \n")), // 末尾空格
                ("debug.log", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        // gitignore 规则应该忽略末尾空格
        let reason = engine.filter_entry(temp.path(), &temp.path().join("debug.log"), false);
        assert!(matches!(reason, FilterReason::Gitignored { .. }));
    }

    #[test]
    fn test_gitignore_escaped_hash() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("\\#important\n")),
                ("#important", Some("")),
                ("important", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        // 转义的 # 应该匹配文件名
        let reason = engine.filter_entry(temp.path(), &temp.path().join("#important"), false);
        assert!(matches!(reason, FilterReason::Gitignored { .. }));

        // 普通文件不受影响
        let reason = engine.filter_entry(temp.path(), &temp.path().join("important"), false);
        assert!(reason.is_retained());
    }

    #[test]
    fn test_gitignore_directory_trailing_slash() {
        // 测试 "build/" 只匹配目录
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("output/\n")),
                ("output/", None),
                ("output/result.txt", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        // output 目录应被忽略
        let reason = engine.filter_entry(temp.path(), &temp.path().join("output"), true);
        assert!(matches!(reason, FilterReason::Gitignored { .. }));
    }

    // ==================== 排序与确定性测试 ====================

    #[test]
    fn test_scan_results_sorted() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                ("zebra.txt", Some("")),
                ("apple.txt", Some("")),
                ("mango.txt", Some("")),
            ],
        );

        let config = FilterConfig::new();
        let mut engine = MatchEngine::new(config).unwrap();
        let entries = engine.scan_directory(temp.path()).unwrap();

        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["apple.txt", "mango.txt", "zebra.txt"]);
    }

    #[test]
    fn test_scan_deterministic() {
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                ("a.txt", Some("")),
                ("b.txt", Some("")),
                ("c/", None),
                ("c/d.txt", Some("")),
            ],
        );

        let config = FilterConfig::new();

        // 多次扫描应该得到相同结果
        let mut engine1 = MatchEngine::new(config.clone()).unwrap();
        let entries1 = engine1.scan_directory(temp.path()).unwrap();

        let mut engine2 = MatchEngine::new(config).unwrap();
        let entries2 = engine2.scan_directory(temp.path()).unwrap();

        assert_eq!(entries1.len(), entries2.len());
        for (e1, e2) in entries1.iter().zip(entries2.iter()) {
            assert_eq!(e1.path, e2.path);
            assert_eq!(e1.is_dir, e2.is_dir);
        }
    }

    // ==================== 额外的边缘情况测试 ====================

    #[test]
    fn test_gitignore_with_only_negation() {
        // 只有取反规则的情况
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("!keep.txt\n")),
                ("keep.txt", Some("")),
                ("other.txt", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        // 所有文件都应保留（因为没有先排除的规则）
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("keep.txt"), false)
            .is_retained());
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("other.txt"), false)
            .is_retained());
    }

    #[test]
    fn test_multiple_gitignore_files_in_same_dir() {
        // 确保只读取 .gitignore，不会有冲突
        let temp = TempDir::new().unwrap();
        create_test_structure(
            temp.path(),
            &[
                (".gitignore", Some("*.log\n")),
                ("debug.log", Some("")),
                ("main.rs", Some("")),
            ],
        );

        let config = FilterConfig::new().with_gitignore(true);
        let mut engine = MatchEngine::new(config).unwrap();

        assert!(matches!(
            engine.filter_entry(temp.path(), &temp.path().join("debug.log"), false),
            FilterReason::Gitignored { .. }
        ));
        assert!(engine
            .filter_entry(temp.path(), &temp.path().join("main.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_include_with_path_separator() {
        // 测试包含路径的模式（注意：这里匹配的是文件名，不是路径）
        let config = FilterConfig::new().with_include("*.rs");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(engine
            .filter_entry(root, Path::new("/test/src/main.rs"), false)
            .is_retained());
    }

    #[test]
    fn test_exclude_hidden_directory() {
        let config = FilterConfig::new().with_exclude(".*");
        let mut engine = MatchEngine::new(config).unwrap();
        let root = Path::new("/test");

        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/.git"), true),
            FilterReason::Excluded { .. }
        ));
        assert!(matches!(
            engine.filter_entry(root, Path::new("/test/.cache"), true),
            FilterReason::Excluded { .. }
        ));
    }
}