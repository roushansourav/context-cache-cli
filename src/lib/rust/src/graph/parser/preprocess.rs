use serde_json::Value;

use super::language::{Lang, detect_language};

pub fn normalize_source(file_path: &str, content: &str) -> (String, Option<Lang>) {
    let lower = file_path.to_ascii_lowercase();

    if lower.ends_with(".vue") {
        let extracted = extract_vue_script(content);
        if extracted.is_empty() {
            return (content.to_string(), detect_language(file_path));
        }
        let lang = if content.contains("lang=\"ts\"") || content.contains("lang='ts'") {
            Some(Lang::TypeScript)
        } else {
            Some(Lang::JavaScript)
        };
        return (extracted, lang);
    }

    if lower.ends_with(".ipynb") {
        let (extracted, lang) = extract_notebook_cells(content);
        return (extracted, lang.or(Some(Lang::Notebook)));
    }

    if lower.ends_with(".py") && content.contains("# COMMAND ----------") {
        let extracted = extract_databricks_cells(content);
        return (extracted, Some(Lang::Python));
    }

    (content.to_string(), detect_language(file_path))
}

fn extract_vue_script(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while let Some(start) = rest.find("<script") {
        let after_tag = &rest[start..];
        let Some(tag_end) = after_tag.find('>') else {
            break;
        };
        let body_start = start + tag_end + 1;
        let trailing = &rest[body_start..];
        let Some(close) = trailing.find("</script>") else {
            break;
        };
        out.push_str(&trailing[..close]);
        out.push('\n');
        rest = &trailing[(close + "</script>".len())..];
    }
    out
}

fn extract_notebook_cells(input: &str) -> (String, Option<Lang>) {
    let parsed: Value = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(_) => return (input.to_string(), Some(Lang::Notebook)),
    };

    let mut out = String::new();
    let mut lang = parsed
        .get("metadata")
        .and_then(|m| m.get("language_info"))
        .and_then(|l| l.get("name"))
        .and_then(Value::as_str)
        .and_then(map_notebook_language);

    if let Some(cells) = parsed.get("cells").and_then(Value::as_array) {
        for cell in cells {
            if cell.get("cell_type").and_then(Value::as_str) != Some("code") {
                continue;
            }
            let source = cell.get("source");
            match source {
                Some(Value::Array(lines)) => {
                    for line in lines {
                        if let Some(s) = line.as_str() {
                            out.push_str(s);
                        }
                    }
                    out.push('\n');
                }
                Some(Value::String(s)) => {
                    out.push_str(s);
                    out.push('\n');
                }
                _ => {}
            }

            if lang.is_none() {
                lang = cell
                    .get("metadata")
                    .and_then(|m| m.get("language"))
                    .and_then(Value::as_str)
                    .and_then(map_notebook_language);
            }
        }
    }

    (out, lang)
}

fn extract_databricks_cells(input: &str) -> String {
    let mut out = String::new();
    for block in input.split("# COMMAND ----------") {
        out.push_str(block);
        out.push('\n');
    }
    out
}

fn map_notebook_language(name: &str) -> Option<Lang> {
    match name.to_ascii_lowercase().as_str() {
        "python" => Some(Lang::Python),
        "javascript" | "js" => Some(Lang::JavaScript),
        "typescript" | "ts" => Some(Lang::TypeScript),
        "r" => Some(Lang::R),
        "scala" => Some(Lang::Scala),
        "sql" => Some(Lang::Notebook),
        _ => None,
    }
}
