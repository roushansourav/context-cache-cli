use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::types::RRF_K;
use super::{graph_path, store_dir};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoEntry {
    pub alias: String,
    pub path: String,
    pub node_count: i64,
    pub edge_count: i64,
    pub registered_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrossRepoSearchResult {
    pub repo_alias: String,
    pub repo_path: String,
    pub qualified_name: String,
    pub kind: String,
    pub file_path: String,
    pub score: f64,
}

fn registry_path() -> PathBuf {
    store_dir().join("registry.json")
}

fn load_registry() -> Result<Vec<RepoEntry>> {
    let path = registry_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)?;
    serde_json::from_str(&raw).map_err(|e| anyhow!("Failed to parse registry: {}", e))
}

fn save_registry(entries: &[RepoEntry]) -> Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(entries)?;
    std::fs::write(&path, json + "\n")?;
    Ok(())
}

pub fn register_repo(repo_root: &Path, alias: Option<&str>) -> Result<RepoEntry> {
    let alias = alias.map(str::to_string).unwrap_or_else(|| {
        repo_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string())
    });

    let db_path = graph_path(repo_root);
    let (node_count, edge_count) = if db_path.exists() {
        let conn = Connection::open(&db_path)?;
        let nc: i64 = conn.query_row("SELECT COUNT(*) FROM nodes", [], |r| r.get(0))?;
        let ec: i64 = conn.query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?;
        (nc, ec)
    } else {
        (0, 0)
    };

    let mut registry = load_registry()?;
    registry.retain(|r| r.alias != alias);

    let entry = RepoEntry {
        alias,
        path: repo_root.to_string_lossy().into_owned(),
        node_count,
        edge_count,
        registered_at: chrono::Utc::now().to_rfc3339(),
    };
    registry.push(entry.clone());
    save_registry(&registry)?;
    Ok(entry)
}

pub fn list_repos() -> Result<Vec<RepoEntry>> {
    load_registry()
}

pub fn unregister_repo(alias: &str) -> Result<()> {
    let mut registry = load_registry()?;
    let before = registry.len();
    registry.retain(|r| r.alias != alias);
    if registry.len() == before {
        return Err(anyhow!("Alias not found in registry: {}", alias));
    }
    save_registry(&registry)
}

pub fn cross_repo_search(
    query: &str,
    kind: Option<&str>,
    limit: i64,
    model: &str,
    aliases: &[String],
) -> Result<Vec<CrossRepoSearchResult>> {
    let repos = load_registry()?;
    let filtered: Vec<&RepoEntry> = if aliases.is_empty() {
        repos.iter().collect()
    } else {
        repos
            .iter()
            .filter(|r| aliases.contains(&r.alias))
            .collect()
    };

    let mut rrf_scores: std::collections::HashMap<(String, String), f64> =
        std::collections::HashMap::new();
    let mut result_meta: std::collections::HashMap<(String, String), CrossRepoSearchResult> =
        std::collections::HashMap::new();

    let candidate_limit = (limit * 3).max(60);
    for repo in filtered {
        let repo_path = Path::new(&repo.path);
        let rows = super::embed::semantic_search(repo_path, query, kind, candidate_limit, model)
            .unwrap_or_default();
        for (rank, row) in rows.iter().enumerate() {
            let key = (repo.alias.clone(), row.qualified_name.clone());
            *rrf_scores.entry(key.clone()).or_insert(0.0) += 1.0 / (RRF_K + rank as f64 + 1.0);
            result_meta
                .entry(key)
                .or_insert_with(|| CrossRepoSearchResult {
                    repo_alias: repo.alias.clone(),
                    repo_path: repo.path.clone(),
                    qualified_name: row.qualified_name.clone(),
                    kind: row.kind.clone(),
                    file_path: row.file_path.clone(),
                    score: 0.0,
                });
        }
    }

    let mut scored: Vec<((String, String), f64)> = rrf_scores.into_iter().collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    let mut out = Vec::new();
    for (key, score) in scored.into_iter().take(limit as usize) {
        if let Some(mut r) = result_meta.remove(&key) {
            r.score = score;
            out.push(r);
        }
    }
    Ok(out)
}

pub fn cross_repo_impact(
    changed_files: &[String],
    max_depth: i64,
    aliases: &[String],
) -> Result<Vec<(String, Vec<String>)>> {
    let repos = load_registry()?;
    let filtered: Vec<&RepoEntry> = if aliases.is_empty() {
        repos.iter().collect()
    } else {
        repos
            .iter()
            .filter(|r| aliases.contains(&r.alias))
            .collect()
    };

    let mut out = Vec::new();
    for repo in filtered {
        let repo_path = Path::new(&repo.path);
        let files =
            super::query::impact_radius(repo_path, changed_files, max_depth).unwrap_or_default();
        out.push((repo.alias.clone(), files));
    }
    Ok(out)
}
