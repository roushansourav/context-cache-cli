use std::collections::HashSet;

use regex::Regex;

use super::language::{detect_language, lang_to_name};
use super::types::{ParsedFile, SymbolDef, SymbolKind};
use super::utils::qualify;

pub fn fallback_parse(file_path: &str, content: &str) -> ParsedFile {
    let mut parsed = ParsedFile::empty();
    let fallback_lang = detect_language(file_path)
        .map(lang_to_name)
        .unwrap_or("unknown")
        .to_string();

    let fn_re = Regex::new(
        r"(?m)^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)|^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>|^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)|^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)"
    ).expect("valid fallback function regex");
    let class_re = Regex::new(
        r"(?m)^\s*(?:export\s+)?class\s+([A-Za-z_][A-Za-z0-9_]*)|^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)|^\s*struct\s+([A-Za-z_][A-Za-z0-9_]*)"
    ).expect("valid fallback class regex");
    let import_re = Regex::new(
        r#"(?m)import\s+(?:[^'"]+\s+from\s+)?['"]([^'"]+)['"]|require\(\s*['"]([^'"]+)['"]\s*\)|^\s*use\s+([^;\n]+);"#
    ).expect("valid fallback import regex");

    for caps in fn_re.captures_iter(content) {
        let name = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .or_else(|| caps.get(4))
            .map(|m| m.as_str().to_string());

        if let Some(name) = name {
            parsed.symbols.push(SymbolDef {
                kind: SymbolKind::Function,
                qualified_name: qualify(file_path, None, &name),
                container: format!("file::{}", file_path),
                name,
                line_start: 1,
                line_end: 1,
                language: fallback_lang.clone(),
            });
        }
    }

    for caps in class_re.captures_iter(content) {
        let name = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .map(|m| m.as_str().to_string());

        if let Some(name) = name {
            parsed.symbols.push(SymbolDef {
                kind: SymbolKind::Class,
                qualified_name: qualify(file_path, None, &name),
                container: format!("file::{}", file_path),
                name,
                line_start: 1,
                line_end: 1,
                language: fallback_lang.clone(),
            });
        }
    }

    let mut seen_imports = HashSet::new();
    for caps in import_re.captures_iter(content) {
        let target = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .map(|m| m.as_str().trim().to_string());

        if let Some(target) = target
            && seen_imports.insert(target.clone())
        {
            parsed.imports.push(target);
        }
    }

    parsed
}
