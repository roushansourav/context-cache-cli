use std::path::{Path, PathBuf};
use anyhow::Result;
use ignore::WalkBuilder;

use crate::config::CacheConfig;

pub struct CandidateFile {
    pub absolute_path: PathBuf,
    pub relative_path: String,
}

/// Walk the repository using .gitignore-aware traversal, then filter to text
/// files matching the include/exclude globs from config.
pub fn collect_files(repo_root: &Path, config: &CacheConfig) -> Result<Vec<CandidateFile>> {
    let include_patterns = config.compiled_include();
    let exclude_patterns = config.compiled_exclude();

    let walker = WalkBuilder::new(repo_root)
        .hidden(false)
        .ignore(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false)
        .build();

    let mut candidates: Vec<CandidateFile> = walker
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type()?.is_dir() {
                return None;
            }

            let abs = entry.into_path();
            if !is_text_file(&abs, &config.text_extensions) {
                return None;
            }

            let rel = abs
                .strip_prefix(repo_root)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");

            // must match at least one include
            let included = include_patterns.iter().any(|p| p.matches(&rel));
            if !included {
                return None;
            }

            // must not match any exclude
            let excluded = exclude_patterns.iter().any(|p| p.matches(&rel));
            if excluded {
                return None;
            }

            Some(CandidateFile {
                absolute_path: abs,
                relative_path: rel,
            })
        })
        .collect();

    candidates.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    if config.max_files > 0 {
        candidates.truncate(config.max_files);
    }

    Ok(candidates)
}

pub fn is_text_file(path: &Path, text_extensions: &[String]) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()));

    match ext {
        Some(e) => text_extensions.iter().any(|t| t == &e),
        None => false,
    }
}
