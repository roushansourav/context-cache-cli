use std::collections::HashSet;

use tree_sitter::Node;

use super::classify::{is_call_node, is_class_node, is_import_node};
use super::extract::{
    extract_bases, extract_call_name, extract_function_name, extract_import_targets, extract_symbol_name,
};
use super::super::language::Lang;
use super::super::types::{CallSite, ParsedFile, PendingRelation, SymbolDef, SymbolKind};
use super::super::utils::qualify;

pub fn walk_tree(root: Node, source: &[u8], file_path: &str, lang: Lang) -> ParsedFile {
    let mut parsed = ParsedFile::empty();
    let mut seen_imports = HashSet::new();
    walk_node(
        root,
        source,
        file_path,
        lang,
        &mut parsed,
        None,
        None,
        &mut seen_imports,
    );
    parsed
}

fn walk_node(
    node: Node,
    source: &[u8],
    file_path: &str,
    lang: Lang,
    out: &mut ParsedFile,
    enclosing_class: Option<&str>,
    enclosing_func: Option<&str>,
    seen_imports: &mut HashSet<String>,
) {
    let kind = node.kind();

    if is_class_node(kind, lang)
        && let Some(name) = extract_symbol_name(node, source, lang, true)
    {
        let class_qname = qualify(file_path, None, &name);
        out.symbols.push(SymbolDef {
            kind: SymbolKind::Class,
            name: name.clone(),
            qualified_name: class_qname.clone(),
            container: format!("file::{}", file_path),
            line_start: node.start_position().row as i64 + 1,
            line_end: node.end_position().row as i64 + 1,
            language: String::new(),
        });

        for base in extract_bases(node, source) {
            out.relations.push(PendingRelation {
                kind: "inherits".to_string(),
                source_qname: class_qname.clone(),
                target_name: base,
            });
        }

        let mut child_cursor = node.walk();
        for child in node.children(&mut child_cursor) {
            walk_node(
                child,
                source,
                file_path,
                lang,
                out,
                Some(&name),
                enclosing_func,
                seen_imports,
            );
        }
        return;
    }

    if let Some((name, line)) = extract_function_name(node, source, lang) {
        let qualified_name = qualify(file_path, enclosing_class, &name);
        let container = match enclosing_class {
            Some(cls) => qualify(file_path, None, cls),
            None => format!("file::{}", file_path),
        };

        out.symbols.push(SymbolDef {
            kind: SymbolKind::Function,
            name: name.clone(),
            qualified_name,
            container,
            line_start: line,
            line_end: node.end_position().row as i64 + 1,
            language: String::new(),
        });

        let mut child_cursor = node.walk();
        for child in node.children(&mut child_cursor) {
            walk_node(
                child,
                source,
                file_path,
                lang,
                out,
                enclosing_class,
                Some(&name),
                seen_imports,
            );
        }
        return;
    }

    if is_import_node(kind, lang) {
        for target in extract_import_targets(node, source, lang) {
            if seen_imports.insert(target.clone()) {
                out.imports.push(target);
            }
        }
    }

    if is_call_node(kind, lang)
        && let (Some(func), Some(call_name)) = (enclosing_func, extract_call_name(node, source))
    {
        out.calls.push(CallSite {
            source_qname: qualify(file_path, enclosing_class, func),
            target_name: call_name,
        });
    }

    let mut child_cursor = node.walk();
    for child in node.children(&mut child_cursor) {
        walk_node(
            child,
            source,
            file_path,
            lang,
            out,
            enclosing_class,
            enclosing_func,
            seen_imports,
        );
    }
}
