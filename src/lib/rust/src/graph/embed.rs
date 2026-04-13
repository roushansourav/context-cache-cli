use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde::Deserialize;

use super::schema::init_schema;
use super::types::{EmbedResult, RRF_K, SemanticRow};
use super::graph_path;

pub fn embed_graph(repo_root: &Path, model: &str) -> Result<EmbedResult> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let model_name = if model.trim().is_empty() { "hash-v1".to_string() } else { model.trim().to_string() };

    let mut stmt = conn.prepare("SELECT qualified_name, kind, name, file_path FROM nodes WHERE kind != 'file'")?;
    let mut rows = stmt.query([])?;

    let tx = conn.unchecked_transaction()?;
    let mut embedded = 0i64;
    let mut total = 0i64;

    while let Some(row) = rows.next()? {
        let qname: String = row.get(0)?;
        let kind: String = row.get(1)?;
        let name: String = row.get(2)?;
        let file_path: String = row.get(3)?;
        total += 1;

        let text = format!("{} {} {} {}", kind, name, qname, file_path);
        let vec = build_embedding_vector(&text, &model_name).unwrap_or_else(|_| vectorize_text(&text, 64));
        let vec_json = serde_json::to_string(&vec)?;

        tx.execute(
            "INSERT INTO node_embeddings(qualified_name, model, vector) VALUES(?1, ?2, ?3)
             ON CONFLICT(qualified_name, model) DO UPDATE SET vector=excluded.vector",
            params![qname, model_name, vec_json],
        )?;
        embedded += 1;
    }

    tx.commit()?;
    Ok(EmbedResult { embedded, total, model: model_name })
}

pub fn semantic_search(repo_root: &Path, query: &str, kind: Option<&str>, limit: i64, model: &str) -> Result<Vec<SemanticRow>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let lim = limit.max(1).min(200) as usize;
    let model_name = if model.trim().is_empty() { "hash-v1" } else { model.trim() };

    let emb_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM node_embeddings WHERE model=?1",
        params![model_name], |r| r.get(0),
    )?;

    if emb_count > 0 {
        let qvec = build_embedding_vector(query, model_name).unwrap_or_else(|_| vectorize_text(query, 64));
        return semantic_search_hybrid(&conn, &qvec, query, kind, lim, model_name);
    }

    semantic_search_lexical(&conn, query, kind, lim)
}

fn semantic_search_hybrid(conn: &Connection, qvec: &[f64], query: &str, kind: Option<&str>, limit: usize, model: &str) -> Result<Vec<SemanticRow>> {
    let candidate_limit = (limit * 3).max(60);
    let vector_results = semantic_search_vector(conn, qvec, kind, candidate_limit, model)?;
    let lexical_results = semantic_search_lexical(conn, query, kind, candidate_limit)?;

    let mut meta: HashMap<String, (String, String)> = HashMap::new();
    for row in lexical_results.iter().chain(vector_results.iter()) {
        meta.entry(row.qualified_name.clone())
            .or_insert_with(|| (row.kind.clone(), row.file_path.clone()));
    }

    let mut rrf: HashMap<String, f64> = HashMap::new();
    for (rank, row) in vector_results.iter().enumerate() {
        *rrf.entry(row.qualified_name.clone()).or_insert(0.0) += 1.0 / (RRF_K + rank as f64 + 1.0);
    }
    for (rank, row) in lexical_results.iter().enumerate() {
        *rrf.entry(row.qualified_name.clone()).or_insert(0.0) += 1.0 / (RRF_K + rank as f64 + 1.0);
    }

    let mut scored: Vec<(String, f64)> = rrf.into_iter().collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

    let mut out = Vec::new();
    for (qname, score) in scored.into_iter().take(limit) {
        if let Some((k, fp)) = meta.remove(&qname) {
            out.push(SemanticRow { qualified_name: qname, kind: k, file_path: fp, score });
        }
    }
    Ok(out)
}

fn semantic_search_vector(conn: &Connection, qvec: &[f64], kind: Option<&str>, limit: usize, model: &str) -> Result<Vec<SemanticRow>> {
    let mut stmt = conn.prepare(
        "SELECT n.qualified_name, n.kind, n.file_path, e.vector
         FROM node_embeddings e JOIN nodes n ON n.qualified_name = e.qualified_name
         WHERE e.model=?1 AND (?2 IS NULL OR n.kind=?2)",
    )?;
    let mut rows = stmt.query(params![model, kind])?;
    let mut out = Vec::new();

    while let Some(row) = rows.next()? {
        let qname: String = row.get(0)?;
        let k: String = row.get(1)?;
        let file_path: String = row.get(2)?;
        let vec_json: String = row.get(3)?;
        let v: Vec<f64> = serde_json::from_str(&vec_json).unwrap_or_default();
        let score = cosine_similarity(qvec, &v);
        out.push(SemanticRow { qualified_name: qname, kind: k, file_path, score });
    }

    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    out.truncate(limit);
    Ok(out)
}

fn semantic_search_lexical(conn: &Connection, query: &str, kind: Option<&str>, limit: usize) -> Result<Vec<SemanticRow>> {
    let like = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
        "SELECT qualified_name, kind, file_path, name FROM nodes
         WHERE (?1 IS NULL OR kind=?1) AND (name LIKE ?2 OR qualified_name LIKE ?2) LIMIT ?3",
    )?;
    let mut rows = stmt.query(params![kind, like, limit as i64])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let qname: String = row.get(0)?;
        let k: String = row.get(1)?;
        let file_path: String = row.get(2)?;
        let name: String = row.get(3)?;
        let score = lexical_score(&name, query);
        out.push(SemanticRow { qualified_name: qname, kind: k, file_path, score });
    }
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    Ok(out)
}

fn build_embedding_vector(text: &str, model: &str) -> Result<Vec<f64>> {
    if model.starts_with("ollama:") {
        let m = model.trim_start_matches("ollama:").trim();
        return if m.is_empty() { Ok(vectorize_text(text, 64)) } else { ollama_embed(text, m) };
    }
    if model.starts_with("openai:") {
        let m = model.trim_start_matches("openai:").trim();
        return if m.is_empty() { Ok(vectorize_text(text, 64)) } else { openai_embed(text, m) };
    }
    Ok(vectorize_text(text, 64))
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse { embedding: Vec<f64> }

fn ollama_embed(text: &str, model: &str) -> Result<Vec<f64>> {
    let base = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let url = format!("{}/api/embeddings", base.trim_end_matches('/'));
    let client = Client::builder().timeout(std::time::Duration::from_secs(20)).build()?;
    let res = client.post(url).json(&serde_json::json!({ "model": model, "prompt": text })).send()?;
    if !res.status().is_success() {
        return Err(anyhow!("Ollama embeddings request failed: {}", res.status()));
    }
    Ok(res.json::<OllamaEmbeddingResponse>()?.embedding)
}

#[derive(Deserialize)]
struct OpenAiEmbeddingItem { embedding: Vec<f64> }

#[derive(Deserialize)]
struct OpenAiEmbeddingResponse { data: Vec<OpenAiEmbeddingItem> }

fn openai_embed(text: &str, model: &str) -> Result<Vec<f64>> {
    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| anyhow!("OPENAI_API_KEY is not set"))?;
    let base = std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let url = format!("{}/embeddings", base.trim_end_matches('/'));
    let client = Client::builder().timeout(std::time::Duration::from_secs(25)).build()?;
    let res = client.post(url).bearer_auth(api_key)
        .json(&serde_json::json!({ "model": model, "input": text })).send()?;
    if !res.status().is_success() {
        return Err(anyhow!("OpenAI embeddings request failed: {}", res.status()));
    }
    let parsed: OpenAiEmbeddingResponse = res.json()?;
    parsed.data.into_iter().next()
        .map(|item| item.embedding)
        .ok_or_else(|| anyhow!("OpenAI response missing embedding data"))
}

fn vectorize_text(text: &str, dims: usize) -> Vec<f64> {
    let mut v = vec![0.0f64; dims];
    for token in text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        if token.is_empty() { continue; }
        let h = blake3::hash(token.as_bytes());
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&h.as_bytes()[0..8]);
        let idx = (u64::from_le_bytes(bytes) as usize) % dims;
        v[idx] += 1.0;
    }
    let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm > 0.0 { for x in &mut v { *x /= norm; } }
    v
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() { return 0.0; }
    let mut dot = 0.0; let mut na = 0.0; let mut nb = 0.0;
    for (x, y) in a.iter().zip(b.iter()) { dot += x * y; na += x * x; nb += y * y; }
    dot / (na.sqrt() * nb.sqrt()).max(1e-9)
}

fn lexical_score(name: &str, query: &str) -> f64 {
    let name_l = name.to_lowercase();
    let q = query.to_lowercase();
    if name_l == q { 1.0 } else if name_l.contains(&q) { 0.85 } else { 0.5 }
}
