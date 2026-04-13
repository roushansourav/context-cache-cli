use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use super::schema::init_schema;
use super::types::{ArchitectureOverview, CommunityDetail, CommunityRow, FlowDetail, FlowRow};
use super::{graph_path, to_posix, top_segment};

pub fn run_postprocess(repo_root: &Path) -> Result<(i64, i64)> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let communities = rebuild_communities(&conn)?;
    let flows = rebuild_flows(&conn)?;

    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO metadata(key, value) VALUES('postprocess_at', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![now],
    )?;
    Ok((flows, communities))
}

pub fn list_flows(repo_root: &Path, limit: i64) -> Result<Vec<FlowRow>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let lim = limit.clamp(1, 200);
    let mut stmt = conn.prepare(
        "SELECT id, name, entry, file_count, node_count, criticality
         FROM flows ORDER BY criticality DESC, node_count DESC LIMIT ?1",
    )?;
    let mut rows = stmt.query(params![lim])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(FlowRow {
            id: row.get(0)?,
            name: row.get(1)?,
            entry: row.get(2)?,
            file_count: row.get(3)?,
            node_count: row.get(4)?,
            criticality: row.get(5)?,
        });
    }
    Ok(out)
}

pub fn get_flow(
    repo_root: &Path,
    flow_id: Option<i64>,
    flow_name: Option<&str>,
) -> Result<Option<FlowDetail>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let row = if let Some(id) = flow_id {
        conn.query_row(
            "SELECT id, name, entry, file_count, node_count, criticality FROM flows WHERE id=?1 LIMIT 1",
            params![id],
            |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, f64>(5)?,
            )),
        ).optional()?
    } else if let Some(name) = flow_name {
        conn.query_row(
            "SELECT id, name, entry, file_count, node_count, criticality FROM flows WHERE name LIKE ?1 ORDER BY criticality DESC LIMIT 1",
            params![format!("%{}%", name)],
            |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, f64>(5)?,
            )),
        ).optional()?
    } else {
        None
    };

    let Some((id, name, entry, file_count, node_count, criticality)) = row else {
        return Ok(None);
    };

    let (nodes, files) = trace_flow_details(&conn, &entry, 5)?;
    Ok(Some(FlowDetail {
        id,
        name,
        entry,
        file_count,
        node_count,
        criticality,
        nodes,
        files,
    }))
}

pub fn affected_flows(
    repo_root: &Path,
    changed_files: &[String],
    limit: i64,
) -> Result<Vec<FlowRow>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let lim = limit.clamp(1, 200) as usize;

    for file in changed_files {
        let file_path = to_posix(file);
        let mut stmt = conn.prepare(
            "SELECT f.id, f.name, f.entry, f.file_count, f.node_count, f.criticality
             FROM flows f JOIN flow_memberships fm ON fm.flow_id = f.id
             WHERE fm.file_path = ?1 ORDER BY f.criticality DESC, f.node_count DESC",
        )?;
        let mut rows = stmt.query(params![file_path])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            if seen.insert(id) {
                out.push(FlowRow {
                    id,
                    name: row.get(1)?,
                    entry: row.get(2)?,
                    file_count: row.get(3)?,
                    node_count: row.get(4)?,
                    criticality: row.get(5)?,
                });
            }
            if out.len() >= lim {
                break;
            }
        }
        if out.len() >= lim {
            break;
        }
    }

    out.sort_by(|a, b| {
        b.criticality
            .partial_cmp(&a.criticality)
            .unwrap_or(Ordering::Equal)
    });
    Ok(out)
}

pub fn list_communities(repo_root: &Path, limit: i64) -> Result<Vec<CommunityRow>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let lim = limit.clamp(1, 200);
    let mut stmt = conn.prepare(
        "SELECT id, name, file_count, node_count, coupling FROM communities
         ORDER BY node_count DESC, file_count DESC LIMIT ?1",
    )?;
    let mut rows = stmt.query(params![lim])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(CommunityRow {
            id: row.get(0)?,
            name: row.get(1)?,
            file_count: row.get(2)?,
            node_count: row.get(3)?,
            coupling: row.get(4)?,
        });
    }
    Ok(out)
}

pub fn get_community(
    repo_root: &Path,
    community_id: Option<i64>,
    community_name: Option<&str>,
    include_members: bool,
) -> Result<Option<CommunityDetail>> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let row = if let Some(id) = community_id {
        conn.query_row(
            "SELECT id, name, file_count, node_count, coupling FROM communities WHERE id=?1 LIMIT 1",
            params![id],
            |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
            )),
        ).optional()?
    } else if let Some(name) = community_name {
        conn.query_row(
            "SELECT id, name, file_count, node_count, coupling FROM communities WHERE name LIKE ?1 ORDER BY node_count DESC LIMIT 1",
            params![format!("%{}%", name)],
            |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
            )),
        ).optional()?
    } else {
        None
    };

    let Some((id, name, file_count, node_count, coupling)) = row else {
        return Ok(None);
    };

    let members = if include_members {
        let mut stmt = conn.prepare(
            "SELECT qualified_name FROM nodes WHERE file_path=?1 OR file_path LIKE ?2 ORDER BY qualified_name LIMIT 500",
        )?;
        let mut rows = stmt.query(params![name, format!("{}/%", name)])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row.get::<_, String>(0)?);
        }
        out
    } else {
        Vec::new()
    };

    Ok(Some(CommunityDetail {
        id,
        name,
        file_count,
        node_count,
        coupling,
        members,
    }))
}

pub fn architecture_overview(repo_root: &Path) -> Result<ArchitectureOverview> {
    let communities = list_communities(repo_root, 50)?;
    let mut warnings = Vec::new();
    for c in &communities {
        if c.coupling > 200 {
            warnings.push(format!(
                "High coupling in {} ({} cross-community edges)",
                c.name, c.coupling
            ));
        }
        if c.file_count == 1 && c.node_count > 80 {
            warnings.push(format!(
                "Large singleton community {} ({} nodes)",
                c.name, c.node_count
            ));
        }
    }
    if warnings.is_empty() {
        warnings.push("No major architecture warnings detected".to_string());
    }
    Ok(ArchitectureOverview {
        communities,
        warnings,
    })
}

fn rebuild_communities(conn: &Connection) -> Result<i64> {
    conn.execute("DELETE FROM communities", [])?;

    let mut comm_files: HashMap<String, HashSet<String>> = HashMap::new();
    let mut comm_nodes: HashMap<String, i64> = HashMap::new();

    let mut stmt = conn.prepare("SELECT file_path, COUNT(*) FROM nodes GROUP BY file_path")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let file_path: String = row.get(0)?;
        let count: i64 = row.get(1)?;
        let community = top_segment(&file_path);
        // perf-entry-api: use .entry() to avoid double lookup
        comm_files
            .entry(community.clone())
            .or_default()
            .insert(file_path);
        *comm_nodes.entry(community).or_insert(0) += count;
    }

    let mut ids: HashMap<String, i64> = HashMap::new();
    for (name, files) in &comm_files {
        let node_count = *comm_nodes.get(name).unwrap_or(&0);
        conn.execute(
            "INSERT INTO communities(name, file_count, node_count, coupling) VALUES(?1, ?2, ?3, 0)",
            params![name, files.len() as i64, node_count],
        )?;
        ids.insert(name.clone(), conn.last_insert_rowid());
    }

    let mut coupling: HashMap<String, i64> = HashMap::new();
    let mut es = conn.prepare("SELECT source, target FROM edges WHERE kind='imports_from'")?;
    let mut erows = es.query([])?;
    while let Some(row) = erows.next()? {
        let source: String = row.get(0)?;
        let target: String = row.get(1)?;
        let sf = source.strip_prefix("file::").unwrap_or(&source);
        let tf = target.strip_prefix("file::").unwrap_or(&target);
        let sc = top_segment(sf);
        let tc = top_segment(tf);
        if sc != tc {
            // perf-entry-api: use .entry() for coupling HashMap
            *coupling.entry(sc).or_insert(0) += 1;
        }
    }

    for (name, cpl) in coupling {
        if let Some(id) = ids.get(&name) {
            conn.execute(
                "UPDATE communities SET coupling=?1 WHERE id=?2",
                params![cpl, id],
            )?;
        }
    }

    Ok(comm_files.len() as i64)
}

fn rebuild_flows(conn: &Connection) -> Result<i64> {
    conn.execute("DELETE FROM flow_memberships", [])?;
    conn.execute("DELETE FROM flows", [])?;

    let mut entries = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT n.qualified_name, n.name, n.file_path FROM nodes n WHERE n.kind='function'
           AND n.qualified_name NOT IN (SELECT target FROM edges WHERE kind='calls')",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        entries.push((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ));
    }

    let mut hinted = conn.prepare(
        "SELECT qualified_name, name, file_path FROM nodes WHERE kind='function' AND (
           LOWER(name) LIKE '%main%' OR LOWER(name) LIKE '%handler%' OR
           LOWER(name) LIKE '%route%' OR LOWER(name) LIKE '%page%')",
    )?;
    let mut hrows = hinted.query([])?;
    while let Some(row) = hrows.next()? {
        entries.push((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ));
    }

    let mut seen = HashSet::new();
    let mut count = 0i64;

    for (entry_q, entry_name, _file) in entries {
        if !seen.insert(entry_q.clone()) {
            continue;
        }
        let (node_count, files) = trace_call_subgraph(conn, &entry_q, 4)?;
        if node_count == 0 {
            continue;
        }

        let file_count = files.len() as i64;
        let criticality = (node_count as f64) + (file_count as f64 * 1.5);
        let flow_name = format!("flow:{}", entry_name);

        conn.execute(
            "INSERT INTO flows(name, entry, file_count, node_count, criticality) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![flow_name, entry_q, file_count, node_count as i64, criticality],
        )?;
        let flow_id = conn.last_insert_rowid();

        for file_path in files {
            conn.execute(
                "INSERT OR IGNORE INTO flow_memberships(flow_id, file_path) VALUES(?1, ?2)",
                params![flow_id, file_path],
            )?;
        }
        count += 1;
        if count >= 300 {
            break;
        }
    }

    Ok(count)
}

fn trace_call_subgraph(
    conn: &Connection,
    entry_q: &str,
    max_depth: i64,
) -> Result<(usize, HashSet<String>)> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut files: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, i64)> = VecDeque::new();

    visited.insert(entry_q.to_string());
    queue.push_back((entry_q.to_string(), 0));

    let mut stmt = conn.prepare("SELECT target FROM edges WHERE kind='calls' AND source=?1")?;

    while let Some((node, depth)) = queue.pop_front() {
        if let Some((f, _)) = node.split_once("::") {
            files.insert(f.to_string());
        }
        if depth >= max_depth {
            continue;
        }

        let mut rows = stmt.query(params![node])?;
        while let Some(row) = rows.next()? {
            let target: String = row.get(0)?;
            if visited.insert(target.clone()) {
                queue.push_back((target, depth + 1));
            }
        }
    }
    Ok((visited.len(), files))
}

fn trace_flow_details(
    conn: &Connection,
    entry_q: &str,
    max_depth: i64,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut files: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, i64)> = VecDeque::new();

    visited.insert(entry_q.to_string());
    queue.push_back((entry_q.to_string(), 0));

    let mut stmt = conn.prepare("SELECT target FROM edges WHERE kind='calls' AND source=?1")?;

    while let Some((node, depth)) = queue.pop_front() {
        if let Some((f, _)) = node.split_once("::") {
            files.insert(f.to_string());
        }
        if depth >= max_depth {
            continue;
        }
        let mut rows = stmt.query(params![node])?;
        while let Some(row) = rows.next()? {
            let target: String = row.get(0)?;
            if visited.insert(target.clone()) {
                queue.push_back((target, depth + 1));
            }
        }
    }

    let mut nodes_out: Vec<String> = visited.into_iter().collect();
    nodes_out.sort();
    let mut files_out: Vec<String> = files.into_iter().collect();
    files_out.sort();
    Ok((nodes_out, files_out))
}
