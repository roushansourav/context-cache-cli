use std::path::Path;

use anyhow::{Result, anyhow};
use rusqlite::{Connection, params};

use super::community::list_communities;
use super::schema::init_schema;
use super::types::WikiResult;
use super::{graph_path, store_dir};
use crate::hasher::hash_bytes;

pub fn generate_wiki(repo_root: &Path, force: bool) -> Result<WikiResult> {
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;

    let key = hash_bytes(repo_root.to_string_lossy().as_bytes());
    let wiki_root = store_dir().join("wiki").join(key);
    std::fs::create_dir_all(&wiki_root)?;

    let communities = list_communities(repo_root, 500)?;
    let mut pages = 0i64;

    for c in communities {
        let name = sanitize_page_name(&c.name);
        let path = wiki_root.join(format!("{}.md", name));
        if path.exists() && !force {
            continue;
        }

        let prefix = format!("{}%", c.name);
        let mut stmt = conn.prepare(
            "SELECT file_path, COUNT(*) AS cnt FROM nodes WHERE file_path LIKE ?1
             GROUP BY file_path ORDER BY cnt DESC LIMIT 20",
        )?;
        let mut rows = stmt.query(params![prefix])?;
        let mut top_files = Vec::new();
        while let Some(row) = rows.next()? {
            let file_path: String = row.get(0)?;
            let cnt: i64 = row.get(1)?;
            top_files.push(format!("- {} ({} symbols)", file_path, cnt));
        }

        let body = format!(
            "# {}\n\n- Files: {}\n- Nodes: {}\n- Coupling: {}\n\n## Top Files\n{}\n",
            c.name,
            c.file_count,
            c.node_count,
            c.coupling,
            if top_files.is_empty() {
                "- (none)".to_string()
            } else {
                top_files.join("\n")
            },
        );

        std::fs::write(path, body)?;
        pages += 1;
    }

    Ok(WikiResult {
        wiki_root: wiki_root.to_string_lossy().into_owned(),
        pages_generated: pages,
    })
}

pub fn get_wiki_page(repo_root: &Path, page_name: &str) -> Result<String> {
    let key = hash_bytes(repo_root.to_string_lossy().as_bytes());
    let wiki_root = store_dir().join("wiki").join(key);
    let file = wiki_root.join(format!("{}.md", sanitize_page_name(page_name)));
    std::fs::read_to_string(&file)
        .map_err(|_| anyhow!("Wiki page not found: {}", file.to_string_lossy()))
}

fn sanitize_page_name(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}
