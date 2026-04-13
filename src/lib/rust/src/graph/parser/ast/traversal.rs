use std::collections::HashSet;

use tree_sitter::Node;

use super::super::language::Lang;
use super::super::types::{CallSite, ParsedFile, PendingRelation, SymbolDef, SymbolKind};
use super::super::utils::qualify;
use super::classify::{is_call_node, is_class_node, is_import_node};
use super::extract::{
    extract_bases, extract_call_name, extract_function_name, extract_import_targets,
    extract_symbol_name,
};

pub fn walk_tree(root: Node, source: &[u8], file_path: &str, lang: Lang) -> ParsedFile {
    let mut parsed = ParsedFile::empty();
    let mut seen_imports = HashSet::new();
    let mut ctx = WalkCtx {
        file_path,
        lang,
        out: &mut parsed,
        seen_imports: &mut seen_imports,
    };
    walk_node(root, source, &mut ctx, None, None);
    parsed
}

struct WalkCtx<'a> {
    file_path: &'a str,
    lang: Lang,
    out: &'a mut ParsedFile,
    seen_imports: &'a mut HashSet<String>,
}

fn walk_node(
    node: Node,
    source: &[u8],
    ctx: &mut WalkCtx<'_>,
    enclosing_class: Option<&str>,
    enclosing_func: Option<&str>,
) {
    let kind = node.kind();

    if is_class_node(kind, ctx.lang)
        && let Some(name) = extract_symbol_name(node, source, ctx.lang, true)
    {
        let class_qname = qualify(ctx.file_path, None, &name);
        ctx.out.symbols.push(SymbolDef {
            kind: SymbolKind::Class,
            name: name.clone(),
            qualified_name: class_qname.clone(),
            container: format!("file::{}", ctx.file_path),
            line_start: node.start_position().row as i64 + 1,
            line_end: node.end_position().row as i64 + 1,
            language: String::new(),
        });

        for base in extract_bases(node, source) {
            ctx.out.relations.push(PendingRelation {
                kind: "inherits".to_string(),
                source_qname: class_qname.clone(),
                target_name: base,
            });
        }

        let mut child_cursor = node.walk();
        for child in node.children(&mut child_cursor) {
            walk_node(child, source, ctx, Some(&name), enclosing_func);
        }
        return;
    }

    if let Some((name, line)) = extract_function_name(node, source, ctx.lang) {
        let qualified_name = qualify(ctx.file_path, enclosing_class, &name);
        let container = match enclosing_class {
            Some(cls) => qualify(ctx.file_path, None, cls),
            None => format!("file::{}", ctx.file_path),
        };

        ctx.out.symbols.push(SymbolDef {
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
            walk_node(child, source, ctx, enclosing_class, Some(&name));
        }
        return;
    }

    if is_import_node(kind, ctx.lang) {
        for target in extract_import_targets(node, source, ctx.lang) {
            if ctx.seen_imports.insert(target.clone()) {
                ctx.out.imports.push(target);
            }
        }
    }

    if is_call_node(kind, ctx.lang)
        && let (Some(func), Some(call_name)) = (enclosing_func, extract_call_name(node, source))
    {
        ctx.out.calls.push(CallSite {
            source_qname: qualify(ctx.file_path, enclosing_class, func),
            target_name: call_name,
        });
    }

    let mut child_cursor = node.walk();
    for child in node.children(&mut child_cursor) {
        walk_node(child, source, ctx, enclosing_class, enclosing_func);
    }
}
