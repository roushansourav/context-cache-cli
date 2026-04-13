use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::CacheConfig;
use crate::hasher::{file_changed, hash_bytes};
use crate::summarize::extract_summary;
use crate::walker::{CandidateFile, collect_files};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub hash: String,
    pub mtime_ms: i64,
    pub size: u64,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachePayload {
    pub repo_root: String,
    pub updated_at: String,
    pub config_hash: String,
    pub file_count: usize,
    pub changed_count: usize,
    pub files: Vec<FileEntry>,
}

/// Stores cache in ~/.context-cache-store/repos/<blake3(repo_root)>.json
pub fn cache_path(repo_root: &Path) -> PathBuf {
    let key = hash_bytes(repo_root.to_string_lossy().as_bytes());
    let store = cache_store_dir();
    store.join(format!("{}.json", key))
}

fn cache_store_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let p = PathBuf::from(home)
        .join(".context-cache-store")
        .join("repos");
    std::fs::create_dir_all(&p).ok();
    p
}

pub fn load_cache(repo_root: &Path) -> Option<CachePayload> {
    let path = cache_path(repo_root);
    if !path.exists() {
        return None;
    }

    let raw = std::fs::read(&path).ok()?;
    serde_json::from_slice(&raw).ok()
}

pub fn save_cache(repo_root: &Path, payload: &CachePayload) -> Result<()> {
    let path = cache_path(repo_root);
    // write atomically: write to .tmp then rename
    let tmp = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(payload)?;
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub struct RefreshResult {
    pub payload: CachePayload,
    pub cache_path: PathBuf,
}

pub fn refresh(repo_root: &Path) -> Result<RefreshResult> {
    let config = CacheConfig::load(repo_root);
    let config_hash = hash_bytes(serde_json::to_string(&config)?.as_bytes());
    let previous = load_cache(repo_root);
    let cp = cache_path(repo_root);

    // Build previous lookup by path
    let prev_map: HashMap<String, &FileEntry> = previous
        .as_ref()
        .map(|p| p.files.iter().map(|e| (e.path.clone(), e)).collect())
        .unwrap_or_default();

    // Collect candidate files (gitignore-aware)
    let candidates = collect_files(repo_root, &config)?;

    // Process in parallel with rayon
    use rayon::prelude::*;

    let results: Vec<Result<(FileEntry, bool)>> = candidates
        .par_iter()
        .map(|candidate| {
            build_entry(
                candidate,
                prev_map.get(&candidate.relative_path).copied(),
                &config,
            )
        })
        .collect();

    let mut files = Vec::with_capacity(results.len());
    let mut changed_count = 0usize;

    for result in results {
        let (entry, changed) = result?;
        if changed {
            changed_count += 1;
        }
        files.push(entry);
    }

    // Re-sort after parallel collection (consistent output)
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let payload = CachePayload {
        repo_root: repo_root.to_string_lossy().into_owned(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        config_hash,
        file_count: files.len(),
        changed_count,
        files,
    };

    save_cache(repo_root, &payload)?;

    Ok(RefreshResult {
        payload,
        cache_path: cp,
    })
}

fn build_entry(
    candidate: &CandidateFile,
    prev: Option<&FileEntry>,
    config: &CacheConfig,
) -> Result<(FileEntry, bool)> {
    let meta = std::fs::metadata(&candidate.absolute_path)?;
    let mtime_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let size = meta.len();

    // Fast path: mtime + size unchanged and same mode => reuse previous entry
    if let Some(prev) = prev
        && !file_changed(mtime_ms, size, prev.mtime_ms, prev.size)
        && prev.mode == config.mode
    {
        return Ok((
            FileEntry {
                path: prev.path.clone(),
                hash: prev.hash.clone(),
                mtime_ms,
                size,
                mode: prev.mode.clone(),
                content: prev.content.clone(),
                summary: prev.summary.clone(),
            },
            false,
        ));
    }

    let raw = std::fs::read(&candidate.absolute_path)?;
    let hash = hash_bytes(&raw);

    // Hash path: content hash unchanged => reuse summary, update mtime
    if let Some(prev) = prev
        && prev.hash == hash
    {
        return Ok((
            FileEntry {
                path: prev.path.clone(),
                hash,
                mtime_ms,
                size,
                mode: config.mode.clone(),
                content: if config.mode == "full" {
                    String::from_utf8_lossy(&raw).into_owned().into()
                } else {
                    None
                },
                summary: prev.summary.clone(),
            },
            false,
        ));
    }

    // Full re-index
    let content_str = String::from_utf8_lossy(&raw);
    let summary = extract_summary(
        &content_str,
        &candidate.absolute_path,
        config.max_file_chars,
    );
    let content = if config.mode == "full" {
        Some(content_str.into_owned())
    } else {
        None
    };

    Ok((
        FileEntry {
            path: candidate.relative_path.clone(),
            hash,
            mtime_ms,
            size,
            mode: config.mode.clone(),
            content,
            summary,
        },
        true,
    ))
}

#[allow(dead_code)]
pub fn format_prompt(payload: &CachePayload, max_chars: usize) -> String {
    let mut out = String::with_capacity(max_chars.min(4 * 1024 * 1024));

    out.push_str("[context-cache]\n");
    out.push_str(&format!("repo: {}\n", payload.repo_root));
    out.push_str(&format!("updatedAt: {}\n", payload.updated_at));
    out.push_str(&format!("files: {}\n\n", payload.file_count));

    for entry in &payload.files {
        let body = if entry.mode == "full" {
            entry.content.as_deref().unwrap_or("")
        } else {
            &entry.summary
        };

        let block = format!("### {}\n{}\n\n", entry.path, body);

        // check if adding this block would exceed the limit
        let remaining = max_chars.saturating_sub(out.len());
        if block.len() > remaining {
            // try to fit a truncated version instead
            let mut partial = block;
            // find safe char boundary for the remaining budget
            let mut boundary = remaining;
            while boundary > 0 && !partial.is_char_boundary(boundary) {
                boundary -= 1;
            }
            partial.truncate(boundary);
            out.push_str(&partial);
            out.push_str("...truncated...\n");
            break;
        }

        out.push_str(&block);
    }

    out
}
