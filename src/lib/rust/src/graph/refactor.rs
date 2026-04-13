use std::collections::HashSet;
use std::path::Path;

use anyhow::{Result, anyhow};
use regex::Regex;

use super::types::{RefactorOccurrence, RefactorPreview};
use crate::cache;

pub fn refactor_preview(
    repo_root: &Path,
    symbol: &str,
    new_name: &str,
    limit: i64,
) -> Result<RefactorPreview> {
    let payload = cache::load_cache(repo_root)
        .ok_or_else(|| anyhow!("No cache found. Run `context-cache refresh` first."))?;

    let escaped = regex::escape(symbol);
    let re = Regex::new(&format!(r"\b{}\b", escaped))?;
    let lim = limit.clamp(1, 500) as usize;

    let mut occurrences = Vec::new();
    let mut files = HashSet::new();

    for file in &payload.files {
        let content = match &file.content {
            Some(c) => c,
            None => continue,
        };
        for (idx, line) in content.lines().enumerate() {
            if re.is_match(line) {
                files.insert(file.path.clone());
                if occurrences.len() < lim {
                    occurrences.push(RefactorOccurrence {
                        file_path: file.path.clone(),
                        line: (idx + 1) as i64,
                        text: line.trim().chars().take(180).collect(),
                    });
                }
            }
        }
    }

    Ok(RefactorPreview {
        symbol: symbol.to_string(),
        new_name: new_name.to_string(),
        total_occurrences: occurrences.len() as i64,
        files_touched: files.len() as i64,
        occurrences,
    })
}

pub fn apply_refactor(
    repo_root: &Path,
    symbol: &str,
    new_name: &str,
    max_files: i64,
) -> Result<i64> {
    let payload = cache::load_cache(repo_root)
        .ok_or_else(|| anyhow!("No cache found. Run `context-cache refresh` first."))?;

    let escaped = regex::escape(symbol);
    let re = Regex::new(&format!(r"\b{}\b", escaped))?;
    let max_apply = max_files.clamp(1, 10000);
    let mut changed = 0i64;

    for file in &payload.files {
        if changed >= max_apply {
            break;
        }
        let abs = repo_root.join(&file.path);
        if !abs.exists() {
            continue;
        }
        let raw = match std::fs::read_to_string(&abs) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !re.is_match(&raw) {
            continue;
        }
        let replaced = re.replace_all(&raw, new_name).to_string();
        if replaced != raw {
            std::fs::write(&abs, replaced)?;
            changed += 1;
        }
    }

    Ok(changed)
}
