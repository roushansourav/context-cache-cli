#![deny(clippy::correctness)]

mod cache;
mod config;
pub mod graph;
pub mod graph_napi;
mod hasher;
mod summarize;
mod walker;

use napi_derive::napi;
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────────────────────
// Exported types (mirrored as plain JS objects via napi)
// ──────────────────────────────────────────────────────────────────────────────

#[napi(object)]
pub struct JsFileEntry {
    pub path: String,
    pub hash: String,
    pub mtime_ms: i64,
    pub size: i64,
    pub mode: String,
    pub content: Option<String>,
    pub summary: String,
}

#[napi(object)]
pub struct JsCachePayload {
    pub repo_root: String,
    pub updated_at: String,
    pub config_hash: String,
    pub file_count: i64,
    pub changed_count: i64,
    pub files: Vec<JsFileEntry>,
}

#[napi(object)]
pub struct JsRefreshResult {
    pub payload: JsCachePayload,
    pub cache_path: String,
}

#[napi(object)]
pub struct JsStatusResult {
    pub exists: bool,
    pub cache_path: String,
    pub repo_root: Option<String>,
    pub updated_at: Option<String>,
    pub file_count: Option<i64>,
}

#[napi(object)]
pub struct JsGraphStatusResult {
    pub exists: bool,
    pub graph_path: String,
    pub node_count: i64,
    pub edge_count: i64,
    pub updated_at: Option<String>,
}

#[napi(object)]
pub struct JsGraphQueryRow {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub file_path: Option<String>,
}

#[napi(object)]
pub struct JsChangeRiskRow {
    pub file_path: String,
    pub impacted_files: i64,
    pub callers: i64,
    pub test_hits: i64,
    pub changed_lines: i64,
    pub security_hits: i64,
    pub risk_score: f64,
    pub risk: String,
}

#[napi(object)]
pub struct JsMinimalContext {
    pub risk: String,
    pub changed_files: i64,
    pub impacted_files: i64,
    pub top_files: Vec<String>,
    pub suggested_tools: Vec<String>,
}

#[napi(object)]
pub struct JsFlowRow {
    pub id: i64,
    pub name: String,
    pub entry: String,
    pub file_count: i64,
    pub node_count: i64,
    pub criticality: f64,
}

#[napi(object)]
pub struct JsFlowDetail {
    pub id: i64,
    pub name: String,
    pub entry: String,
    pub file_count: i64,
    pub node_count: i64,
    pub criticality: f64,
    pub nodes: Vec<String>,
    pub files: Vec<String>,
}

#[napi(object)]
pub struct JsCommunityRow {
    pub id: i64,
    pub name: String,
    pub file_count: i64,
    pub node_count: i64,
    pub coupling: i64,
}

#[napi(object)]
pub struct JsCommunityDetail {
    pub id: i64,
    pub name: String,
    pub file_count: i64,
    pub node_count: i64,
    pub coupling: i64,
    pub members: Vec<String>,
}

#[napi(object)]
pub struct JsArchitectureOverview {
    pub communities: Vec<JsCommunityRow>,
    pub warnings: Vec<String>,
}

#[napi(object)]
pub struct JsEmbedResult {
    pub embedded: i64,
    pub total: i64,
    pub model: String,
}

#[napi(object)]
pub struct JsSemanticRow {
    pub qualified_name: String,
    pub kind: String,
    pub file_path: String,
    pub score: f64,
}

#[napi(object)]
pub struct JsLargeSymbolRow {
    pub kind: String,
    pub qualified_name: String,
    pub file_path: String,
    pub line_start: i64,
    pub line_end: i64,
    pub line_count: i64,
}

#[napi(object)]
pub struct JsReviewContext {
    pub changed_files: Vec<String>,
    pub impacted_files: Vec<String>,
    pub snippets: Vec<String>,
}

#[napi(object)]
pub struct JsRefactorOccurrence {
    pub file_path: String,
    pub line: i64,
    pub text: String,
}

#[napi(object)]
pub struct JsRefactorPreview {
    pub symbol: String,
    pub new_name: String,
    pub total_occurrences: i64,
    pub files_touched: i64,
    pub occurrences: Vec<JsRefactorOccurrence>,
}

#[napi(object)]
pub struct JsWikiResult {
    pub wiki_root: String,
    pub pages_generated: i64,
}

// ── Stage 5.1: Multi-repo registry types ─────────────────────────────────────

#[napi(object)]
pub struct JsRepoEntry {
    pub alias: String,
    pub path: String,
    pub node_count: i64,
    pub edge_count: i64,
    pub registered_at: String,
}

#[napi(object)]
pub struct JsCrossRepoSearchResult {
    pub repo_alias: String,
    pub repo_path: String,
    pub qualified_name: String,
    pub kind: String,
    pub file_path: String,
    pub score: f64,
}

// ──────────────────────────────────────────────────────────────────────────────
// Cache-related exported functions
// ──────────────────────────────────────────────────────────────────────────────

/// Refresh (or initially build) the cache for the given repo root.
/// Runs parallel file I/O via rayon — safe to call from JS without blocking
/// because this is exposed as a blocking napi function (runs on libuv threadpool).
#[napi]
pub fn refresh(repo_root: String) -> napi::Result<JsRefreshResult> {
    let path = PathBuf::from(&repo_root);
    let result = cache::refresh(&path).map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(map_result(result))
}

/// Return cache status without refreshing.
#[napi]
pub fn status(repo_root: String) -> JsStatusResult {
    let path = PathBuf::from(&repo_root);
    let cp = cache::cache_path(&path);
    match cache::load_cache(&path) {
        None => JsStatusResult {
            exists: false,
            cache_path: cp.to_string_lossy().into_owned(),
            repo_root: None,
            updated_at: None,
            file_count: None,
        },
        Some(d) => JsStatusResult {
            exists: true,
            cache_path: cp.to_string_lossy().into_owned(),
            repo_root: Some(d.repo_root),
            updated_at: Some(d.updated_at),
            file_count: Some(d.file_count as i64),
        },
    }
}

/// Return the path where the cache file is stored for this repo.
#[napi]
pub fn get_cache_path(repo_root: String) -> String {
    let path = PathBuf::from(&repo_root);
    cache::cache_path(&path).to_string_lossy().into_owned()
}

/// Return the global config path for this repo.
#[napi]
pub fn get_config_path(repo_root: String) -> String {
    let path = PathBuf::from(&repo_root);
    config::CacheConfig::global_config_path(&path)
        .to_string_lossy()
        .into_owned()
}

/// Format the cached prompt for a repo, truncating to max_chars.
#[napi]
pub fn format_prompt(repo_root: String, max_chars: i64) -> String {
    let path = PathBuf::from(&repo_root);
    match cache::load_cache(&path) {
        None => String::new(),
        Some(payload) => cache::format_prompt(&payload, max_chars as usize),
    }
}

/// Detect the framework preset for this repo based on config files present.
#[napi]
pub fn detect_preset(repo_root: String) -> String {
    let path = PathBuf::from(&repo_root);
    if path.join("nx.json").exists() || path.join(".nx").exists() {
        return "nx".to_string();
    }
    if path.join("next.config.js").exists()
        || path.join("next.config.ts").exists()
        || path.join("next.config.mjs").exists()
    {
        return "nextjs".to_string();
    }
    if path.join("requirements.txt").exists()
        || path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
    {
        return "python".to_string();
    }
    "generic".to_string()
}

// ──────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────────────────────────────────

fn map_result(r: cache::RefreshResult) -> JsRefreshResult {
    let p = r.payload;
    let files = p
        .files
        .into_iter()
        .map(|f| JsFileEntry {
            path: f.path,
            hash: f.hash,
            mtime_ms: f.mtime_ms,
            size: f.size as i64,
            mode: f.mode,
            content: f.content,
            summary: f.summary,
        })
        .collect();

    JsRefreshResult {
        cache_path: r.cache_path.to_string_lossy().into_owned(),
        payload: JsCachePayload {
            repo_root: p.repo_root,
            updated_at: p.updated_at,
            config_hash: p.config_hash,
            file_count: p.file_count as i64,
            changed_count: p.changed_count as i64,
            files,
        },
    }
}
