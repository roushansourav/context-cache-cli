use std::path::Path;

/// For full mode we store raw content as-is.
/// For summary mode we extract the most signal-rich lines.
/// Safely truncate a String at a UTF-8 char boundary.
fn safe_truncate(s: &mut String, max: usize) {
    if s.len() <= max {
        return;
    }
    // walk back from max until we land on a char boundary
    let mut boundary = max;
    while boundary > 0 && !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    s.truncate(boundary);
}

pub fn extract_summary(content: &str, file_path: &Path, max_chars: usize) -> String {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "md" | "mdx" => summarize_markdown(content, max_chars),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => summarize_code(content, max_chars),
        "py" => summarize_python(content, max_chars),
        "rs" => summarize_rust(content, max_chars),
        _ => summarize_generic(content, max_chars),
    }
}

fn summarize_markdown(content: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(max_chars.min(content.len()));
    let mut in_fence = false;

    for line in content.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let norm = normalize_line(trimmed);
        if norm.is_empty() {
            continue;
        }
        if norm.starts_with('#')
            || norm.starts_with("- ")
            || norm.starts_with("* ")
            || norm.starts_with("1.")
            || norm.contains("Rule")
            || norm.contains("Violation")
            || norm.contains("MUST")
            || norm.contains("must")
        {
            out.push_str(&norm);
            out.push('\n');
            if out.len() >= max_chars {
                break;
            }
        }
    }

    safe_truncate(&mut out, max_chars);
    out
}

fn summarize_code(content: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(max_chars.min(content.len()));
    for line in content.lines() {
        let norm = normalize_line(line.trim_end());
        if norm.is_empty() {
            continue;
        }
        if norm.starts_with("import ")
            || norm.starts_with("export ")
            || norm.starts_with("// ")
            || norm.starts_with("/*")
            || norm.contains("function ")
            || norm.contains("class ")
            || norm.contains("interface ")
            || norm.starts_with("type ")
            || norm.starts_with("const ")
            || norm.starts_with("async ")
        {
            out.push_str(&norm);
            out.push('\n');
            if out.len() >= max_chars {
                break;
            }
        }
    }
    safe_truncate(&mut out, max_chars);
    out
}

fn summarize_python(content: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(max_chars.min(content.len()));
    for line in content.lines() {
        let norm = normalize_line(line.trim_end());
        if norm.is_empty() {
            continue;
        }
        if norm.starts_with("def ")
            || norm.starts_with("class ")
            || norm.starts_with("import ")
            || norm.starts_with("from ")
            || norm.starts_with('#')
        {
            out.push_str(&norm);
            out.push('\n');
            if out.len() >= max_chars {
                break;
            }
        }
    }
    safe_truncate(&mut out, max_chars);
    out
}

fn summarize_rust(content: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(max_chars.min(content.len()));
    for line in content.lines() {
        let norm = normalize_line(line.trim_end());
        if norm.is_empty() {
            continue;
        }
        if norm.starts_with("pub ")
            || norm.starts_with("fn ")
            || norm.starts_with("struct ")
            || norm.starts_with("enum ")
            || norm.starts_with("impl ")
            || norm.starts_with("trait ")
            || norm.starts_with("use ")
            || norm.starts_with("//")
        {
            out.push_str(&norm);
            out.push('\n');
            if out.len() >= max_chars {
                break;
            }
        }
    }
    safe_truncate(&mut out, max_chars);
    out
}

fn summarize_generic(content: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(max_chars.min(content.len()));
    for (i, line) in content.lines().enumerate() {
        if i >= 200 {
            break;
        }
        let norm = normalize_line(line.trim_end());
        if norm.is_empty() {
            continue;
        }
        out.push_str(&norm);
        out.push('\n');
        if out.len() >= max_chars {
            break;
        }
    }
    safe_truncate(&mut out, max_chars);
    out
}

fn normalize_line(line: &str) -> String {
    // collapse internal whitespace to single space
    let mut result = String::with_capacity(line.len());
    let mut prev_space = false;
    for ch in line.chars() {
        if ch.is_whitespace() {
            if !prev_space && !result.is_empty() {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result
}
