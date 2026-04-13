use std::collections::{HashSet, VecDeque};
use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, params};

use super::schema::init_schema;
use super::types::{LargeSymbolRow, QueryRow, ReviewContext};
use super::{graph_path, to_posix};
use crate::cache;

pub fn query(repo_root: &Path, pattern: &str, target: &str, limit: i64) -> Result<Vec<QueryRow>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let norm = normalize_pattern(pattern);
    let lim = limit.clamp(1, 500);

    let sql = match norm {
        "imports_of" => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE e.kind='imports_from' AND (e.source=?1 OR e.source LIKE ?2) LIMIT ?3"
        }
        "importers_of" => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE e.kind='imports_from' AND (e.target=?1 OR e.target LIKE ?2) LIMIT ?3"
        }
        "callees_of" => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE e.kind='calls' AND (e.source=?1 OR e.source LIKE ?2) LIMIT ?3"
        }
        "callers_of" => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE e.kind='calls' AND (e.target=?1 OR e.target LIKE ?2) LIMIT ?3"
        }
        "tests_for" => {
            "SELECT n.qualified_name AS source, ?1 AS target, 'tests_for' AS kind, n.file_path
             FROM nodes n WHERE n.kind='function'
               AND (n.file_path LIKE '%test%' OR n.file_path LIKE '%.spec.%')
               AND n.name LIKE ?2
             LIMIT ?3"
        }
        "container_of" => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE e.kind='contains' AND (e.source=?1 OR e.source LIKE ?2) LIMIT ?3"
        }
        "depends_on" => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE (e.kind='imports_from' OR e.kind='calls') AND (e.source=?1 OR e.source LIKE ?2) LIMIT ?3"
        }
        "inheritance_of" => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE e.kind='inherits' AND (e.source=?1 OR e.target=?1 OR e.source LIKE ?2 OR e.target LIKE ?2) LIMIT ?3"
        }
        "implemented_by" => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE e.kind='implements' AND (e.target=?1 OR e.target LIKE ?2) LIMIT ?3"
        }
        _ => {
            "SELECT e.source, e.target, e.kind, e.file_path FROM edges e WHERE (e.source=?1 OR e.target=?1 OR e.source LIKE ?2 OR e.target LIKE ?2) LIMIT ?3"
        }
    };

    let target_q = to_target_qname(target);
    let like = format!("%{}%", target);
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query(params![target_q, like, lim])?;

    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(QueryRow {
            source: row.get(0)?,
            target: row.get(1)?,
            kind: row.get(2)?,
            file_path: row.get(3)?,
        });
    }
    Ok(out)
}

pub fn impact_radius(
    repo_root: &Path,
    changed_files: &[String],
    max_depth: i64,
) -> Result<Vec<String>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let mut impacted: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, i64)> = VecDeque::new();

    for f in changed_files {
        let q = format!("file::{}", to_posix(f));
        impacted.insert(q.clone());
        queue.push_back((q, 0));
    }

    let depth_limit = max_depth.clamp(1, 10);

    while let Some((node, depth)) = queue.pop_front() {
        if depth >= depth_limit {
            continue;
        }

        let mut stmt = conn.prepare(
            "SELECT source, target FROM edges WHERE (kind='imports_from' OR kind='calls') AND (source=?1 OR target=?1)",
        )?;
        let mut rows = stmt.query(params![node])?;
        while let Some(row) = rows.next()? {
            let src: String = row.get(0)?;
            let dst: String = row.get(1)?;
            for n in [src, dst] {
                if impacted.insert(n.clone()) {
                    queue.push_back((n, depth + 1));
                }
            }
        }
    }

    let mut files: HashSet<String> = HashSet::new();
    for n in impacted {
        if let Some(rest) = n.strip_prefix("file::") {
            files.insert(rest.to_string());
        } else if let Some((file, _)) = n.split_once("::") {
            files.insert(file.to_string());
        }
    }

    let mut out: Vec<String> = files.into_iter().collect();
    out.sort();
    Ok(out)
}

pub fn find_large_symbols(
    repo_root: &Path,
    min_lines: i64,
    kind: Option<&str>,
    file_path_pattern: Option<&str>,
    limit: i64,
) -> Result<Vec<LargeSymbolRow>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let lim = limit.clamp(1, 500);
    let min_l = min_lines.max(1);
    let mut stmt = conn.prepare(
        "SELECT kind, qualified_name, file_path, line_start, line_end,
                (CASE WHEN line_end >= line_start THEN line_end - line_start + 1 ELSE 0 END) AS line_count
         FROM nodes
         WHERE kind != 'file'
           AND (?1 IS NULL OR kind = ?1)
           AND (?2 IS NULL OR file_path LIKE ?2)
           AND (CASE WHEN line_end >= line_start THEN line_end - line_start + 1 ELSE 0 END) >= ?3
         ORDER BY line_count DESC, qualified_name ASC
         LIMIT ?4",
    )?;

    let like_path = file_path_pattern.map(|p| format!("%{}%", p));
    let mut rows = stmt.query(params![kind, like_path, min_l, lim])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(LargeSymbolRow {
            kind: row.get(0)?,
            qualified_name: row.get(1)?,
            file_path: row.get(2)?,
            line_start: row.get(3)?,
            line_end: row.get(4)?,
            line_count: row.get(5)?,
        });
    }
    Ok(out)
}

pub fn review_context(
    repo_root: &Path,
    changed_files: Option<&[String]>,
    max_depth: i64,
    include_source: bool,
    max_lines_per_file: i64,
    base: &str,
) -> Result<ReviewContext> {
    let changed = match changed_files {
        Some(files) if !files.is_empty() => files.to_vec(),
        _ => git_changed_files(repo_root, base),
    };

    let impacted_files = impact_radius(repo_root, &changed, max_depth)?;
    if !include_source {
        return Ok(ReviewContext {
            changed_files: changed,
            impacted_files,
            snippets: Vec::new(),
        });
    }

    let payload = match cache::load_cache(repo_root) {
        Some(p) => p,
        None => {
            return Ok(ReviewContext {
                changed_files: changed,
                impacted_files,
                snippets: Vec::new(),
            });
        }
    };

    let mut snippets = Vec::new();
    let max_lines = max_lines_per_file.max(10) as usize;
    for file in &impacted_files {
        if snippets.len() >= 30 {
            break;
        }
        let Some(entry) = payload.files.iter().find(|f| to_posix(&f.path) == *file) else {
            continue;
        };
        let Some(content) = &entry.content else {
            continue;
        };
        let sample = content
            .lines()
            .take(max_lines)
            .collect::<Vec<_>>()
            .join("\n");
        snippets.push(format!("### {}\n{}", file, sample));
    }

    Ok(ReviewContext {
        changed_files: changed,
        impacted_files,
        snippets,
    })
}

fn normalize_pattern(pattern: &str) -> &'static str {
    match pattern.trim() {
        "imports_of" => "imports_of",
        "importers_of" => "importers_of",
        "callees_of" => "callees_of",
        "callers_of" => "callers_of",
        "tests_for" => "tests_for",
        "container_of" => "container_of",
        "depends_on" => "depends_on",
        "inheritance_of" => "inheritance_of",
        "implemented_by" => "implemented_by",
        _ => "related",
    }
}

fn to_target_qname(target: &str) -> String {
    if target.contains("::") {
        return target.to_string();
    }
    format!("%::{}", target)
}

fn git_changed_files(repo_root: &Path, base: &str) -> Vec<String> {
    use std::process::Command;

    let out = Command::new("git")
        .arg("diff")
        .arg("--name-only")
        .arg(base)
        .current_dir(repo_root)
        .output();
    let bytes = match out {
        Ok(o) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    String::from_utf8_lossy(&bytes)
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(to_posix)
        .collect()
}
