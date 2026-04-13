pub mod build;
pub mod community;
pub mod detect;
pub mod embed;
mod parser;
pub mod query;
pub mod refactor;
pub mod registry;
pub mod schema;
pub mod types;
pub mod wiki;

pub use build::{build_graph, build_or_update_graph, status};
pub use community::{affected_flows, architecture_overview, get_community, get_flow, list_communities, list_flows, run_postprocess};
pub use detect::{detect_changes, minimal_context};
pub use embed::{embed_graph, semantic_search};
pub use query::{find_large_symbols, impact_radius, query, review_context};
pub use refactor::{apply_refactor, refactor_preview};
pub use registry::{cross_repo_impact, cross_repo_search, list_repos, register_repo, unregister_repo};
// Re-export all public domain types so callers can write `graph::SemanticRow` etc.
#[allow(unused_imports)]
pub use types::{
    ArchitectureOverview, ChangeRisk, CommunityDetail, CommunityRow, EmbedResult, FlowDetail,
    FlowRow, GraphStatus, LargeSymbolRow, MinimalContext, QueryRow, RefactorOccurrence,
    RefactorPreview, ReviewContext, SemanticRow, WikiResult,
};
pub use wiki::{generate_wiki, get_wiki_page};

use crate::hasher::hash_bytes;
use std::path::{Path, PathBuf};

/// Root of the persistent store directory (`~/.context-cache-store`).
/// Falls back to `/tmp` if `$HOME` is unset.
pub fn store_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
        .join(".context-cache-store")
}

/// Derive the SQLite graph file path for a given repo root.
pub fn graph_path(repo_root: &Path) -> PathBuf {
    let key = hash_bytes(repo_root.to_string_lossy().as_bytes());
    let dir = store_dir().join("graphs");
    std::fs::create_dir_all(&dir).ok();
    dir.join(format!("{}.db", key))
}

/// Extract the top-level directory segment from a POSIX file path.
pub fn top_segment(file_path: &str) -> String {
    file_path.split('/').next().unwrap_or("(root)").to_string()
}

/// Normalise path separators to forward slashes.
pub fn to_posix<S: AsRef<str>>(input: S) -> String {
    input.as_ref().replace('\\', "/")
}
