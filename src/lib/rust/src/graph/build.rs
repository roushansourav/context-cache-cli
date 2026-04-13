use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::cache;
use super::parser::{parse_file, SymbolKind};
use super::community::run_postprocess;
use super::schema::init_schema;
use super::types::GraphStatus;
use super::{graph_path, to_posix};

pub fn build_graph(repo_root: &Path) -> Result<String> {
    build_or_update_graph(repo_root, true)
}

pub fn build_or_update_graph(repo_root: &Path, full_rebuild: bool) -> Result<String> {
    let payload = cache::load_cache(repo_root)
        .ok_or_else(|| anyhow!("No cache found. Run `context-cache refresh` first."))?;

    let db_path = graph_path(repo_root);
    let mut conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let tx = conn.transaction()?;

    if full_rebuild {
        tx.execute("DELETE FROM edges", [])?;
        tx.execute("DELETE FROM nodes", [])?;
        tx.execute("DELETE FROM file_index", [])?;
    }

    let mut known_files: HashSet<String> = HashSet::new();
    let mut payload_hashes: HashMap<String, String> = HashMap::new();
    for file in &payload.files {
        let path = to_posix(&file.path);
        known_files.insert(path.clone());
        payload_hashes.insert(path, file.hash.clone());
    }

    let mut indexed_hashes = load_indexed_hashes(&tx)?;
    let changed_files: HashSet<String> = if full_rebuild {
        known_files.clone()
    } else {
        let mut set = HashSet::new();
        for (path, hash) in &payload_hashes {
            match indexed_hashes.get(path) {
                Some(prev) if prev == hash => {}
                _ => {
                    set.insert(path.clone());
                }
            }
        }
        set
    };

    if !full_rebuild {
        let stale_files: Vec<String> = indexed_hashes
            .keys()
            .filter(|p| !known_files.contains(*p))
            .cloned()
            .collect();
        for stale in stale_files {
            delete_file_subgraph(&tx, &stale)?;
            tx.execute("DELETE FROM file_index WHERE file_path=?1", params![stale])?;
            indexed_hashes.remove(&stale);
        }
    }

    let mut global_symbols: HashMap<String, String> = HashMap::new();
    if !full_rebuild {
        load_existing_symbols(&tx, &mut global_symbols)?;
    }
    let mut pending_relations: Vec<(String, String, String, String)> = Vec::new();
    let mut pending_calls: Vec<(String, String, String)> = Vec::new();

    for file in &payload.files {
        let file_path = to_posix(&file.path);
        if !full_rebuild && !changed_files.contains(&file_path) {
            continue;
        }

        if !full_rebuild {
            delete_file_subgraph(&tx, &file_path)?;
        }

        let file_lang = detect_lang_from_path(&file_path);
        let qname = format!("file::{}", file_path);
        let line_end = file
            .content
            .as_ref()
            .map(|c| c.lines().count() as i64)
            .unwrap_or(0);
        insert_node(&tx, "file", &file_path, &qname, &file_path, 0, line_end, Some(file_lang))?;

        let content = match &file.content {
            Some(c) => c,
            None => continue,
        };

        let parsed = parse_file(&file_path, content);

        for sym in parsed.symbols {
            let node_kind = match sym.kind {
                SymbolKind::Function => "function",
                SymbolKind::Class => "class",
            };
            insert_node(
                &tx,
                node_kind,
                &sym.name,
                &sym.qualified_name,
                &file_path,
                sym.line_start,
                sym.line_end,
                Some(&sym.language),
            )?;
            insert_edge(&tx, "contains", &sym.container, &sym.qualified_name, &file_path)?;
            global_symbols.entry(sym.name.clone()).or_insert_with(|| sym.qualified_name.clone());
        }

        for rel in parsed.relations {
            pending_relations.push((rel.kind, rel.source_qname, rel.target_name, file_path.clone()));
        }

        for specifier in parsed.imports {
            if let Some(target_file) = resolve_import_target(&file_path, &specifier, &known_files) {
                let target_q = format!("file::{}", target_file);
                insert_edge(&tx, "imports_from", &qname, &target_q, &file_path)?;
            }
        }

        for call in parsed.calls {
            pending_calls.push((call.source_qname, call.target_name, file_path.clone()));
        }

        tx.execute(
            "INSERT INTO file_index(file_path, content_hash) VALUES(?1, ?2)
             ON CONFLICT(file_path) DO UPDATE SET content_hash=excluded.content_hash",
            params![file_path, file.hash],
        )?;
    }

    for (kind, source_q, target_name, file_path) in pending_relations {
        if let Some(target_q) = global_symbols.get(&target_name) {
            insert_edge(&tx, &kind, &source_q, target_q, &file_path)?;
        }
    }

    for (source_q, target_name, file_path) in pending_calls {
        if is_call_noise(&target_name) {
            continue;
        }
        if let Some(target_q) = global_symbols.get(&target_name) {
            insert_edge(&tx, "calls", &source_q, target_q, &file_path)?;
        }
    }

    let now = Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO metadata(key, value) VALUES('updated_at', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![now],
    )?;

    tx.commit()?;
    run_postprocess(repo_root)?;
    Ok(db_path.to_string_lossy().into_owned())
}

pub fn status(repo_root: &Path) -> Result<GraphStatus> {
    let db_path = graph_path(repo_root);
    if !db_path.exists() {
        return Ok(GraphStatus {
            exists: false,
            graph_path: db_path.to_string_lossy().into_owned(),
            node_count: 0,
            edge_count: 0,
            updated_at: None,
        });
    }

    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;
    let node_count: i64 = conn.query_row("SELECT COUNT(*) FROM nodes", [], |r| r.get(0))?;
    let edge_count: i64 = conn.query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?;
    let updated_at: Option<String> = conn
        .query_row("SELECT value FROM metadata WHERE key='updated_at'", [], |r| r.get(0))
        .optional()?;

    Ok(GraphStatus {
        exists: true,
        graph_path: db_path.to_string_lossy().into_owned(),
        node_count,
        edge_count,
        updated_at,
    })
}

fn insert_node(
    conn: &Connection,
    kind: &str,
    name: &str,
    qname: &str,
    file_path: &str,
    line_start: i64,
    line_end: i64,
    language: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO nodes(kind, name, qualified_name, file_path, line_start, line_end, language) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![kind, name, qname, file_path, line_start, line_end, language],
    )?;
    Ok(())
}

fn insert_edge(conn: &Connection, kind: &str, source: &str, target: &str, file_path: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO edges(kind, source, target, file_path) VALUES(?1, ?2, ?3, ?4)",
        params![kind, source, target, file_path],
    )?;
    Ok(())
}

fn resolve_import_target(from_file: &str, specifier: &str, known_files: &HashSet<String>) -> Option<String> {
    if specifier.starts_with('.') {
        return resolve_relative_import(from_file, specifier, known_files);
    }

    let lang = detect_lang_from_path(from_file);
    match lang {
        "python" => resolve_python_module(specifier, known_files),
        "rust" => resolve_rust_module(specifier, known_files),
        "java" => resolve_java_module(specifier, known_files),
        "go" => resolve_go_module(specifier, known_files),
        _ => None,
    }
}

fn resolve_relative_import(from_file: &str, specifier: &str, known_files: &HashSet<String>) -> Option<String> {
    let mut parts: Vec<&str> = from_file.split('/').collect();
    parts.pop();
    for part in specifier.split('/') {
        if part.is_empty() || part == "." { continue; }
        if part == ".." { let _ = parts.pop(); } else { parts.push(part); }
    }
    let base = parts.join("/");
    let candidates = [
        base.clone(), format!("{}.ts", base), format!("{}.tsx", base),
        format!("{}.js", base), format!("{}.jsx", base), format!("{}.mjs", base),
        format!("{}.cjs", base), format!("{}.py", base), format!("{}.rs", base),
        format!("{}.go", base), format!("{}.java", base), format!("{}.c", base),
        format!("{}.h", base), format!("{}.cpp", base), format!("{}.hpp", base),
        format!("{}/index.ts", base), format!("{}/index.tsx", base),
        format!("{}/index.js", base), format!("{}/index.py", base), format!("{}/index.rs", base),
    ];
    candidates.into_iter().find(|c| known_files.contains(c))
}

fn resolve_python_module(specifier: &str, known_files: &HashSet<String>) -> Option<String> {
    let base = specifier.replace('.', "/");
    [format!("{}.py", base), format!("{}/__init__.py", base)]
        .into_iter()
        .find(|c| known_files.contains(c))
}

fn resolve_rust_module(specifier: &str, known_files: &HashSet<String>) -> Option<String> {
    let tail = specifier.trim_start_matches("crate::").replace("::", "/");
    [
        format!("src/{}.rs", tail),
        format!("src/{}/mod.rs", tail),
        format!("{}.rs", tail),
    ]
    .into_iter()
    .find(|c| known_files.contains(c))
}

fn resolve_java_module(specifier: &str, known_files: &HashSet<String>) -> Option<String> {
    let base = specifier.replace('.', "/");
    [format!("{}.java", base), format!("src/main/java/{}.java", base)]
        .into_iter()
        .find(|c| known_files.contains(c))
}

fn resolve_go_module(specifier: &str, known_files: &HashSet<String>) -> Option<String> {
    [
        format!("{}.go", specifier),
        format!("{}/{}.go", specifier, "main"),
        format!("{}/{}.go", specifier, "index"),
    ]
    .into_iter()
    .find(|c| known_files.contains(c))
}

fn detect_lang_from_path(path: &str) -> &'static str {
    let p = path.to_ascii_lowercase();
    if p.ends_with(".py") {
        "python"
    } else if p.ends_with(".rs") {
        "rust"
    } else if p.ends_with(".go") {
        "go"
    } else if p.ends_with(".java") {
        "java"
    } else if p.ends_with(".ts") {
        "typescript"
    } else if p.ends_with(".tsx") {
        "tsx"
    } else if p.ends_with(".js") || p.ends_with(".jsx") || p.ends_with(".mjs") || p.ends_with(".cjs") {
        "javascript"
    } else if p.ends_with(".c") || p.ends_with(".h") {
        "c"
    } else if p.ends_with(".cpp") || p.ends_with(".cc") || p.ends_with(".cxx") || p.ends_with(".hpp") {
        "cpp"
    } else if p.ends_with(".cs") {
        "csharp"
    } else if p.ends_with(".rb") {
        "ruby"
    } else if p.ends_with(".php") {
        "php"
    } else if p.ends_with(".lua") {
        "lua"
    } else {
        "unknown"
    }
}

fn is_call_noise(name: &str) -> bool {
    matches!(name, "if" | "for" | "while" | "switch" | "catch" | "return" | "new" | "typeof" | "await" | "console")
}

fn load_indexed_hashes(conn: &Connection) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    let mut stmt = conn.prepare("SELECT file_path, content_hash FROM file_index")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        out.insert(row.get::<_, String>(0)?, row.get::<_, String>(1)?);
    }
    Ok(out)
}

fn load_existing_symbols(conn: &Connection, out: &mut HashMap<String, String>) -> Result<()> {
    let mut stmt = conn.prepare("SELECT name, qualified_name FROM nodes WHERE kind IN ('function', 'class')")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let qname: String = row.get(1)?;
        out.entry(name).or_insert(qname);
    }
    Ok(())
}

fn delete_file_subgraph(conn: &Connection, file_path: &str) -> Result<()> {
    let file_q = format!("file::{}", file_path);
    conn.execute("DELETE FROM edges WHERE file_path=?1 OR source=?2 OR target=?2", params![file_path, file_q])?;
    conn.execute("DELETE FROM nodes WHERE file_path=?1", params![file_path])?;
    Ok(())
}
