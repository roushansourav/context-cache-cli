use anyhow::Result;
use rusqlite::Connection;

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS nodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            qualified_name TEXT NOT NULL UNIQUE,
            file_path TEXT NOT NULL,
            line_start INTEGER NOT NULL DEFAULT 0,
            line_end INTEGER NOT NULL DEFAULT 0,
            language TEXT
        );

        CREATE TABLE IF NOT EXISTS edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            source TEXT NOT NULL,
            target TEXT NOT NULL,
            file_path TEXT
        );

        CREATE TABLE IF NOT EXISTS metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS flows (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            entry TEXT NOT NULL,
            file_count INTEGER NOT NULL,
            node_count INTEGER NOT NULL,
            criticality REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS flow_memberships (
            flow_id INTEGER NOT NULL,
            file_path TEXT NOT NULL,
            PRIMARY KEY(flow_id, file_path)
        );

        CREATE TABLE IF NOT EXISTS communities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            file_count INTEGER NOT NULL,
            node_count INTEGER NOT NULL,
            coupling INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS node_embeddings (
            qualified_name TEXT NOT NULL,
            model TEXT NOT NULL,
            vector TEXT NOT NULL,
            PRIMARY KEY(qualified_name, model)
        );

        CREATE TABLE IF NOT EXISTS file_index (
            file_path TEXT PRIMARY KEY,
            content_hash TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_nodes_file ON nodes(file_path);
        CREATE INDEX IF NOT EXISTS idx_nodes_qname ON nodes(qualified_name);
        CREATE INDEX IF NOT EXISTS idx_edges_kind ON edges(kind);
        CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source);
        CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target);
        CREATE INDEX IF NOT EXISTS idx_flow_memberships_file ON flow_memberships(file_path);
        CREATE INDEX IF NOT EXISTS idx_embeddings_model ON node_embeddings(model);
        CREATE INDEX IF NOT EXISTS idx_file_index_hash ON file_index(content_hash);
        ",
    )?;

    // Backward-compatible migration for older DBs created before language metadata.
    ensure_column(conn, "nodes", "line_end", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "nodes", "language", "TEXT")?;

    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma)?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(());
        }
    }
    let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition);
    conn.execute(&sql, [])?;
    Ok(())
}
