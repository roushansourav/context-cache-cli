use napi_derive::napi;
use std::path::PathBuf;

use crate::graph;
use crate::{
    JsArchitectureOverview, JsChangeRiskRow, JsCommunityDetail, JsCommunityRow,
    JsCrossRepoSearchResult, JsEmbedResult, JsFlowDetail, JsFlowRow, JsGraphQueryRow,
    JsGraphStatusResult, JsLargeSymbolRow, JsMinimalContext, JsRefactorOccurrence,
    JsRefactorPreview, JsRepoEntry, JsReviewContext, JsSemanticRow, JsWikiResult,
};

/// Build or rebuild SQLite graph from cached full content.
#[napi]
pub fn build_graph(repo_root: String) -> napi::Result<String> {
    let path = PathBuf::from(&repo_root);
    graph::build_graph(&path).map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn build_or_update_graph(
    repo_root: String,
    full_rebuild: Option<bool>,
) -> napi::Result<String> {
    let path = PathBuf::from(&repo_root);
    graph::build_or_update_graph(&path, full_rebuild.unwrap_or(false))
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Return graph status.
#[napi]
pub fn graph_status(repo_root: String) -> napi::Result<JsGraphStatusResult> {
    let path = PathBuf::from(&repo_root);
    let s = graph::status(&path).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(JsGraphStatusResult {
        exists: s.exists,
        graph_path: s.graph_path,
        node_count: s.node_count,
        edge_count: s.edge_count,
        updated_at: s.updated_at,
    })
}

/// Query graph relations by pattern and target.
#[napi]
pub fn query_graph(
    repo_root: String,
    pattern: String,
    target: String,
    limit: i64,
) -> napi::Result<Vec<JsGraphQueryRow>> {
    let path = PathBuf::from(&repo_root);
    let rows = graph::query(&path, &pattern, &target, limit)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| JsGraphQueryRow {
            source: r.source,
            target: r.target,
            kind: r.kind,
            file_path: r.file_path,
        })
        .collect())
}

/// Compute impacted files from changed files.
#[napi]
pub fn graph_impact_radius(
    repo_root: String,
    changed_files: Vec<String>,
    max_depth: i64,
) -> napi::Result<Vec<String>> {
    let path = PathBuf::from(&repo_root);
    graph::impact_radius(&path, &changed_files, max_depth)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Risk-scored change analysis by base ref.
#[napi]
pub fn detect_changes(repo_root: String, base: String) -> napi::Result<Vec<JsChangeRiskRow>> {
    let path = PathBuf::from(&repo_root);
    let rows =
        graph::detect_changes(&path, &base).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| JsChangeRiskRow {
            file_path: r.file_path,
            impacted_files: r.impacted_files,
            callers: r.callers,
            test_hits: r.test_hits,
            changed_lines: r.changed_lines,
            security_hits: r.security_hits,
            risk_score: r.risk_score,
            risk: r.risk,
        })
        .collect())
}

/// Compact context for AI entry-point calls.
#[napi]
pub fn minimal_context(repo_root: String, base: String) -> napi::Result<JsMinimalContext> {
    let path = PathBuf::from(&repo_root);
    let mc = graph::minimal_context(&path, &base)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(JsMinimalContext {
        risk: mc.risk,
        changed_files: mc.changed_files,
        impacted_files: mc.impacted_files,
        top_files: mc.top_files,
        suggested_tools: mc.suggested_tools,
    })
}

/// Return the absolute path where this repo's SQLite graph is stored.
#[napi]
pub fn get_graph_path(repo_root: String) -> String {
    let path = PathBuf::from(&repo_root);
    graph::graph_path(&path).to_string_lossy().into_owned()
}

/// Recompute flow/community postprocess artifacts.
#[napi]
pub fn run_postprocess(repo_root: String) -> napi::Result<Vec<i64>> {
    let path = PathBuf::from(&repo_root);
    let (flows, communities) =
        graph::run_postprocess(&path).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(vec![flows, communities])
}

#[napi]
pub fn list_flows(repo_root: String, limit: i64) -> napi::Result<Vec<JsFlowRow>> {
    let path = PathBuf::from(&repo_root);
    graph::list_flows(&path, limit)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
        .map(|rows| {
            rows.into_iter()
                .map(|r| JsFlowRow {
                    id: r.id,
                    name: r.name,
                    entry: r.entry,
                    file_count: r.file_count,
                    node_count: r.node_count,
                    criticality: r.criticality,
                })
                .collect()
        })
}

#[napi]
pub fn get_flow(
    repo_root: String,
    flow_id: Option<i64>,
    flow_name: Option<String>,
) -> napi::Result<Option<JsFlowDetail>> {
    let path = PathBuf::from(&repo_root);
    let detail = graph::get_flow(&path, flow_id, flow_name.as_deref())
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(detail.map(|d| JsFlowDetail {
        id: d.id,
        name: d.name,
        entry: d.entry,
        file_count: d.file_count,
        node_count: d.node_count,
        criticality: d.criticality,
        nodes: d.nodes,
        files: d.files,
    }))
}

#[napi]
pub fn get_affected_flows(
    repo_root: String,
    changed_files: Vec<String>,
    limit: i64,
) -> napi::Result<Vec<JsFlowRow>> {
    let path = PathBuf::from(&repo_root);
    graph::affected_flows(&path, &changed_files, limit)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
        .map(|rows| {
            rows.into_iter()
                .map(|r| JsFlowRow {
                    id: r.id,
                    name: r.name,
                    entry: r.entry,
                    file_count: r.file_count,
                    node_count: r.node_count,
                    criticality: r.criticality,
                })
                .collect()
        })
}

#[napi]
pub fn list_communities(repo_root: String, limit: i64) -> napi::Result<Vec<JsCommunityRow>> {
    let path = PathBuf::from(&repo_root);
    graph::list_communities(&path, limit)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
        .map(|rows| {
            rows.into_iter()
                .map(|r| JsCommunityRow {
                    id: r.id,
                    name: r.name,
                    file_count: r.file_count,
                    node_count: r.node_count,
                    coupling: r.coupling,
                })
                .collect()
        })
}

#[napi]
pub fn get_community(
    repo_root: String,
    community_id: Option<i64>,
    community_name: Option<String>,
    include_members: Option<bool>,
) -> napi::Result<Option<JsCommunityDetail>> {
    let path = PathBuf::from(&repo_root);
    let detail = graph::get_community(
        &path,
        community_id,
        community_name.as_deref(),
        include_members.unwrap_or(false),
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(detail.map(|d| JsCommunityDetail {
        id: d.id,
        name: d.name,
        file_count: d.file_count,
        node_count: d.node_count,
        coupling: d.coupling,
        members: d.members,
    }))
}

#[napi]
pub fn architecture_overview(repo_root: String) -> napi::Result<JsArchitectureOverview> {
    let path = PathBuf::from(&repo_root);
    let overview =
        graph::architecture_overview(&path).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(JsArchitectureOverview {
        communities: overview
            .communities
            .into_iter()
            .map(|r| JsCommunityRow {
                id: r.id,
                name: r.name,
                file_count: r.file_count,
                node_count: r.node_count,
                coupling: r.coupling,
            })
            .collect(),
        warnings: overview.warnings,
    })
}

#[napi]
pub fn embed_graph(repo_root: String, model: String) -> napi::Result<JsEmbedResult> {
    let path = PathBuf::from(&repo_root);
    let result =
        graph::embed_graph(&path, &model).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(JsEmbedResult {
        embedded: result.embedded,
        total: result.total,
        model: result.model,
    })
}

#[napi]
pub fn semantic_search(
    repo_root: String,
    query: String,
    kind: Option<String>,
    limit: i64,
    model: String,
) -> napi::Result<Vec<JsSemanticRow>> {
    let path = PathBuf::from(&repo_root);
    graph::semantic_search(&path, &query, kind.as_deref(), limit, &model)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
        .map(|rows| {
            rows.into_iter()
                .map(|r| JsSemanticRow {
                    qualified_name: r.qualified_name,
                    kind: r.kind,
                    file_path: r.file_path,
                    score: r.score,
                })
                .collect()
        })
}

#[napi]
pub fn find_large_functions(
    repo_root: String,
    min_lines: i64,
    kind: Option<String>,
    file_path_pattern: Option<String>,
    limit: i64,
) -> napi::Result<Vec<JsLargeSymbolRow>> {
    let path = PathBuf::from(&repo_root);
    graph::find_large_symbols(
        &path,
        min_lines,
        kind.as_deref(),
        file_path_pattern.as_deref(),
        limit,
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))
    .map(|rows| {
        rows.into_iter()
            .map(|r| JsLargeSymbolRow {
                kind: r.kind,
                qualified_name: r.qualified_name,
                file_path: r.file_path,
                line_start: r.line_start,
                line_end: r.line_end,
                line_count: r.line_count,
            })
            .collect()
    })
}

#[napi]
pub fn get_review_context(
    repo_root: String,
    changed_files: Option<Vec<String>>,
    max_depth: Option<i64>,
    include_source: Option<bool>,
    max_lines_per_file: Option<i64>,
    base: Option<String>,
) -> napi::Result<JsReviewContext> {
    let path = PathBuf::from(&repo_root);
    let ctx = graph::review_context(
        &path,
        changed_files.as_deref(),
        max_depth.unwrap_or(2),
        include_source.unwrap_or(true),
        max_lines_per_file.unwrap_or(200),
        base.as_deref().unwrap_or("HEAD~1"),
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(JsReviewContext {
        changed_files: ctx.changed_files,
        impacted_files: ctx.impacted_files,
        snippets: ctx.snippets,
    })
}

#[napi]
pub fn refactor_preview(
    repo_root: String,
    symbol: String,
    new_name: String,
    limit: i64,
) -> napi::Result<JsRefactorPreview> {
    let path = PathBuf::from(&repo_root);
    let preview = graph::refactor_preview(&path, &symbol, &new_name, limit)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(JsRefactorPreview {
        symbol: preview.symbol,
        new_name: preview.new_name,
        total_occurrences: preview.total_occurrences,
        files_touched: preview.files_touched,
        occurrences: preview
            .occurrences
            .into_iter()
            .map(|o| JsRefactorOccurrence {
                file_path: o.file_path,
                line: o.line,
                text: o.text,
            })
            .collect(),
    })
}

#[napi]
pub fn apply_refactor(
    repo_root: String,
    symbol: String,
    new_name: String,
    max_files: i64,
) -> napi::Result<i64> {
    let path = PathBuf::from(&repo_root);
    graph::apply_refactor(&path, &symbol, &new_name, max_files)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn generate_wiki(repo_root: String, force: bool) -> napi::Result<JsWikiResult> {
    let path = PathBuf::from(&repo_root);
    let result =
        graph::generate_wiki(&path, force).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(JsWikiResult {
        wiki_root: result.wiki_root,
        pages_generated: result.pages_generated,
    })
}

#[napi]
pub fn get_wiki_page(repo_root: String, page_name: String) -> napi::Result<String> {
    let path = PathBuf::from(&repo_root);
    graph::get_wiki_page(&path, &page_name).map_err(|e| napi::Error::from_reason(e.to_string()))
}

// ── Stage 5.1: Multi-repo registry ───────────────────────────────────────────

#[napi]
pub fn register_repo(repo_root: String, alias: Option<String>) -> napi::Result<JsRepoEntry> {
    let path = PathBuf::from(&repo_root);
    let entry = graph::register_repo(&path, alias.as_deref())
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(JsRepoEntry {
        alias: entry.alias,
        path: entry.path,
        node_count: entry.node_count,
        edge_count: entry.edge_count,
        registered_at: entry.registered_at,
    })
}

#[napi]
pub fn list_repos() -> napi::Result<Vec<JsRepoEntry>> {
    graph::list_repos()
        .map_err(|e| napi::Error::from_reason(e.to_string()))
        .map(|entries| {
            entries
                .into_iter()
                .map(|e| JsRepoEntry {
                    alias: e.alias,
                    path: e.path,
                    node_count: e.node_count,
                    edge_count: e.edge_count,
                    registered_at: e.registered_at,
                })
                .collect()
        })
}

#[napi]
pub fn unregister_repo(alias: String) -> napi::Result<()> {
    graph::unregister_repo(&alias).map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn cross_repo_search(
    query: String,
    kind: Option<String>,
    limit: i64,
    model: String,
    aliases: Vec<String>,
) -> napi::Result<Vec<JsCrossRepoSearchResult>> {
    graph::cross_repo_search(&query, kind.as_deref(), limit, &model, &aliases)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
        .map(|rows| {
            rows.into_iter()
                .map(|r| JsCrossRepoSearchResult {
                    repo_alias: r.repo_alias,
                    repo_path: r.repo_path,
                    qualified_name: r.qualified_name,
                    kind: r.kind,
                    file_path: r.file_path,
                    score: r.score,
                })
                .collect()
        })
}

#[napi]
pub fn cross_repo_impact(
    changed_files: Vec<String>,
    max_depth: i64,
    aliases: Vec<String>,
) -> napi::Result<Vec<Vec<String>>> {
    graph::cross_repo_impact(&changed_files, max_depth, &aliases)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
        .map(|pairs| {
            pairs
                .into_iter()
                .map(|(alias, files)| {
                    let mut row = vec![alias];
                    row.extend(files);
                    row
                })
                .collect()
        })
}
