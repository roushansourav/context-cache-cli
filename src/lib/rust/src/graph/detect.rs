use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use rusqlite::{params, Connection};

use super::query::impact_radius;
use super::types::{
    ChangeRisk, MinimalContext, RISK_PENALTY_NO_TESTS, RISK_THRESHOLD_HIGH,
    RISK_THRESHOLD_MEDIUM, RISK_WEIGHT_CALLERS, RISK_WEIGHT_FUNCTIONS,
    RISK_WEIGHT_IMPACTED, RISK_WEIGHT_LINES, RISK_WEIGHT_SECURITY,
};
use super::{graph_path, to_posix};

pub fn detect_changes(repo_root: &Path, base: &str) -> Result<Vec<ChangeRisk>> {
    let changed = git_changed_files(repo_root, base);
    if changed.is_empty() { return Ok(Vec::new()); }

    let line_stats = git_changed_line_stats(repo_root, base);
    let security_stats = git_changed_security_hits(repo_root, base);
    let impacted = impact_radius(repo_root, &changed, 2)?;
    let db_path = graph_path(repo_root);
    let conn = Connection::open(&db_path)?;

    let mut out = Vec::new();

    for file in changed {
        let file_key = to_posix(&file);

        let callers: i64 = conn.query_row(
            "SELECT COUNT(*) FROM edges WHERE kind='calls' AND target LIKE ?1",
            params![format!("{}::%", to_posix(&file))],
            |r| r.get(0),
        )?;

        let test_hits: i64 = conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE kind='function'
               AND (file_path LIKE '%test%' OR file_path LIKE '%.spec.%')
               AND (name LIKE ?1 OR qualified_name LIKE ?2)",
            params![
                format!("%{}%", stem_for_test_matching(&file_key)),
                format!("%{}%", stem_for_test_matching(&file_key))
            ],
            |r| r.get(0),
        )?;

        let changed_functions: i64 = conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE kind='function' AND file_path=?1",
            params![file_key],
            |r| r.get(0),
        )?;

        let test_symbol_links = count_symbol_linked_tests(&conn, &file_key)?;
        let lines_changed = *line_stats.get(&file_key).unwrap_or(&0);
        let security_hits = *security_stats.get(&file_key).unwrap_or(&0);
        let impacted_files = impacted.iter().filter(|p| *p != &file).count() as i64;

        let risk_score = (callers as f64 * RISK_WEIGHT_CALLERS)
            + (impacted_files as f64 * RISK_WEIGHT_IMPACTED)
            + (lines_changed as f64 * RISK_WEIGHT_LINES)
            + (changed_functions as f64 * RISK_WEIGHT_FUNCTIONS)
            + (security_hits as f64 * RISK_WEIGHT_SECURITY)
            + if test_hits == 0 && test_symbol_links == 0 { RISK_PENALTY_NO_TESTS } else { 0.0 };

        let risk = if risk_score >= RISK_THRESHOLD_HIGH { "high" }
            else if risk_score >= RISK_THRESHOLD_MEDIUM { "medium" }
            else { "low" };

        out.push(ChangeRisk {
            file_path: file,
            impacted_files,
            callers,
            test_hits,
            changed_lines: lines_changed,
            security_hits,
            risk_score,
            risk: risk.to_string(),
        });
    }

    out.sort_by(|a, b| b.impacted_files.cmp(&a.impacted_files).then_with(|| a.file_path.cmp(&b.file_path)));
    Ok(out)
}

pub fn minimal_context(repo_root: &Path, base: &str) -> Result<MinimalContext> {
    let changes = detect_changes(repo_root, base)?;
    let changed_count = changes.len() as i64;
    let impacted_total: i64 = changes.iter().map(|c| c.impacted_files).sum();

    let risk = if changes.iter().any(|c| c.risk == "high") { "high" }
        else if changes.iter().any(|c| c.risk == "medium") { "medium" }
        else { "low" };

    let mut top_files: Vec<String> = changes.iter().take(5)
        .map(|c| format!("{} ({})", c.file_path, c.risk))
        .collect();

    if top_files.is_empty() {
        top_files.push("No changed files detected".to_string());
    }

    Ok(MinimalContext {
        risk: risk.to_string(),
        changed_files: changed_count,
        impacted_files: impacted_total,
        top_files,
        suggested_tools: vec![
            "query_graph callers_of <symbol>".to_string(),
            "query_graph tests_for <symbol>".to_string(),
            "detect_changes".to_string(),
            "list_flows".to_string(),
            "architecture_overview".to_string(),
            "semantic_search".to_string(),
        ],
    })
}

fn git_changed_files(repo_root: &Path, base: &str) -> Vec<String> {
    let out = Command::new("git").arg("diff").arg("--name-only").arg(base)
        .current_dir(repo_root).output();
    let bytes = match out { Ok(o) if o.status.success() => o.stdout, _ => return Vec::new() };
    String::from_utf8_lossy(&bytes).lines()
        .map(str::trim).filter(|s| !s.is_empty()).map(to_posix).collect()
}

fn git_changed_line_stats(repo_root: &Path, base: &str) -> HashMap<String, i64> {
    let out = Command::new("git").arg("diff").arg("--numstat").arg(base)
        .current_dir(repo_root).output();
    let bytes = match out { Ok(o) if o.status.success() => o.stdout, _ => return HashMap::new() };
    let mut stats = HashMap::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 3 { continue; }
        let added = parts[0].parse::<i64>().unwrap_or(0);
        let deleted = parts[1].parse::<i64>().unwrap_or(0);
        stats.insert(to_posix(parts[2].trim()), added + deleted);
    }
    stats
}

fn git_changed_security_hits(repo_root: &Path, base: &str) -> HashMap<String, i64> {
    let out = Command::new("git").arg("diff").arg("--unified=0").arg(base)
        .current_dir(repo_root).output();
    let bytes = match out { Ok(o) if o.status.success() => o.stdout, _ => return HashMap::new() };
    let keywords = ["auth", "token", "password", "secret", "sql", "permission", "credential", "jwt"];
    let mut file = String::new();
    let mut hits: HashMap<String, i64> = HashMap::new();
    for line in String::from_utf8_lossy(&bytes).lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") { file = to_posix(rest.trim()); continue; }
        if file.is_empty() { continue; }
        if (line.starts_with('+') || line.starts_with('-')) && !line.starts_with("+++") && !line.starts_with("---") {
            let lower = line.to_lowercase();
            let count = keywords.iter().filter(|k| lower.contains(*k)).count() as i64;
            if count > 0 { *hits.entry(file.clone()).or_insert(0) += count; }
        }
    }
    hits
}

fn count_symbol_linked_tests(conn: &Connection, file_path: &str) -> Result<i64> {
    let mut names_stmt = conn.prepare(
        "SELECT name FROM nodes WHERE kind='function' AND file_path=?1 LIMIT 100",
    )?;
    let mut name_rows = names_stmt.query(params![file_path])?;
    let mut names = Vec::new();
    while let Some(row) = name_rows.next()? { names.push(row.get::<_, String>(0)?); }

    let mut total = 0i64;
    for name in names {
        let c: i64 = conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE (file_path LIKE '%test%' OR file_path LIKE '%.spec.%') AND (name LIKE ?1 OR qualified_name LIKE ?1)",
            params![format!("%{}%", name)],
            |r| r.get(0),
        )?;
        total += c;
    }
    Ok(total)
}

fn stem_for_test_matching(file_path: &str) -> String {
    let name = file_path.split('/').last().unwrap_or(file_path)
        .split('.').next().unwrap_or(file_path);
    name.replace('-', "").replace('_', "")
}
