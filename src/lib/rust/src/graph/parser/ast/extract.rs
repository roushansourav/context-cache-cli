use tree_sitter::Node;

use super::super::language::Lang;
use super::super::utils::{extract_quoted, text_of};

pub fn extract_symbol_name(
    node: Node,
    source: &[u8],
    lang: Lang,
    is_class: bool,
) -> Option<String> {
    if matches!(lang, Lang::Rust)
        && node.kind() == "impl_item"
        && let Some(ty) = node.child_by_field_name("type")
    {
        return text_of(ty, source);
    }

    if matches!(lang, Lang::Go)
        && node.kind() == "type_spec"
        && let Some(n) = node.child_by_field_name("name")
    {
        return text_of(n, source);
    }

    if matches!(lang, Lang::C | Lang::Cpp) && is_class {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if matches!(child.kind(), "type_identifier" | "identifier") {
                return text_of(child, source);
            }
        }
    }

    if let Some(name_node) = node.child_by_field_name("name") {
        return text_of(name_node, source);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "identifier" | "type_identifier" | "property_identifier"
        ) && let Some(name) = text_of(child, source)
            && !name.is_empty()
        {
            return Some(name);
        }
    }
    None
}

pub fn extract_function_name(node: Node, source: &[u8], lang: Lang) -> Option<(String, i64)> {
    let kind = node.kind();

    if matches!(
        kind,
        "function_declaration"
            | "method_definition"
            | "function_definition"
            | "method_declaration"
            | "constructor_declaration"
    ) && let Some(name) = extract_symbol_name(node, source, lang, false)
    {
        return Some((name, node.start_position().row as i64 + 1));
    }

    if matches!(lang, Lang::Rust)
        && kind == "function_item"
        && let Some(name) = extract_symbol_name(node, source, lang, false)
    {
        return Some((name, node.start_position().row as i64 + 1));
    }

    if matches!(lang, Lang::Ruby)
        && kind == "method"
        && let Some(name) = extract_symbol_name(node, source, lang, false)
    {
        return Some((name, node.start_position().row as i64 + 1));
    }

    if matches!(lang, Lang::Php)
        && (kind == "function_definition" || kind == "method_declaration")
        && let Some(name) = extract_symbol_name(node, source, lang, false)
    {
        return Some((name, node.start_position().row as i64 + 1));
    }

    if matches!(lang, Lang::CSharp)
        && (kind == "method_declaration" || kind == "constructor_declaration")
        && let Some(name) = extract_symbol_name(node, source, lang, false)
    {
        return Some((name, node.start_position().row as i64 + 1));
    }

    if matches!(lang, Lang::Lua)
        && kind == "function_declaration"
        && let Some(name) = extract_symbol_name(node, source, lang, false)
    {
        return Some((name, node.start_position().row as i64 + 1));
    }

    if matches!(lang, Lang::JavaScript | Lang::TypeScript | Lang::Tsx)
        && kind == "variable_declarator"
        && let (Some(name_node), Some(value_node)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("value"),
        )
        && matches!(
            value_node.kind(),
            "arrow_function" | "function" | "function_expression"
        )
        && let Some(name) = text_of(name_node, source)
    {
        return Some((name, node.start_position().row as i64 + 1));
    }

    None
}

pub fn extract_import_targets(node: Node, source: &[u8], lang: Lang) -> Vec<String> {
    let mut out = Vec::new();
    match lang {
        Lang::JavaScript | Lang::TypeScript | Lang::Tsx => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "string"
                    && let Some(v) = text_of(child, source)
                {
                    out.push(v.trim_matches('\'').trim_matches('"').to_string());
                }
            }
        }
        Lang::Python => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if (child.kind() == "dotted_name" || child.kind() == "identifier")
                    && let Some(v) = text_of(child, source)
                {
                    out.push(v);
                    break;
                }
            }
        }
        Lang::Rust => {
            if let Some(v) = text_of(node, source) {
                out.push(v.replace("use ", "").replace(';', "").trim().to_string());
            }
        }
        Lang::Go => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "interpreted_string_literal"
                    && let Some(v) = text_of(child, source)
                {
                    out.push(v.trim_matches('"').to_string());
                }
            }
        }
        Lang::Java => {
            if let Some(v) = text_of(node, source) {
                let cleaned = v.replace("import", "").replace(';', "").trim().to_string();
                if !cleaned.is_empty() {
                    out.push(cleaned);
                }
            }
        }
        Lang::C | Lang::Cpp => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if matches!(child.kind(), "system_lib_string" | "string_literal")
                    && let Some(v) = text_of(child, source)
                {
                    out.push(
                        v.trim_matches('<')
                            .trim_matches('>')
                            .trim_matches('"')
                            .to_string(),
                    );
                }
            }
        }
        Lang::Ruby => {
            if let Some(v) = text_of(node, source)
                && (v.contains("require") || v.contains("require_relative"))
                && let Some(target) = extract_quoted(&v)
            {
                out.push(target);
            }
        }
        Lang::Php => {
            if let Some(v) = text_of(node, source) {
                let cleaned = v.replace("use", "").replace(';', "").trim().to_string();
                if !cleaned.is_empty() {
                    out.push(cleaned);
                }
            }
        }
        Lang::CSharp => {
            if let Some(v) = text_of(node, source) {
                let cleaned = v.replace("using", "").replace(';', "").trim().to_string();
                if !cleaned.is_empty() {
                    out.push(cleaned);
                }
            }
        }
        Lang::Lua => {
            if let Some(v) = text_of(node, source)
                && v.contains("require")
                && let Some(target) = extract_quoted(&v)
            {
                out.push(target);
            }
        }
        Lang::Kotlin
        | Lang::Scala
        | Lang::Swift
        | Lang::Solidity
        | Lang::Dart
        | Lang::R
        | Lang::Perl
        | Lang::Vue
        | Lang::Notebook
        | Lang::Bash => {}
    }
    out
}

pub fn extract_bases(node: Node, source: &[u8]) -> Vec<String> {
    let mut bases = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "extends_clause"
                | "implements_clause"
                | "superclass"
                | "base_class_clause"
                | "inheritance_specifier"
        ) {
            let mut sub_cursor = child.walk();
            for sub in child.children(&mut sub_cursor) {
                if matches!(
                    sub.kind(),
                    "identifier" | "type_identifier" | "scoped_type_identifier"
                ) && let Some(v) = text_of(sub, source)
                {
                    bases.push(v);
                }
            }
        }
    }
    bases
}

pub fn extract_call_name(node: Node, source: &[u8]) -> Option<String> {
    if let Some(function_node) = node.child_by_field_name("function") {
        return extract_last_identifier(function_node, source);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "identifier" | "field_identifier" | "property_identifier"
        ) {
            return text_of(child, source);
        }
    }

    extract_last_identifier(node, source)
}

fn extract_last_identifier(node: Node, source: &[u8]) -> Option<String> {
    let mut found = None;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if matches!(
            child.kind(),
            "identifier" | "field_identifier" | "property_identifier" | "type_identifier"
        ) {
            found = text_of(child, source);
        }
        if let Some(deep) = extract_last_identifier(child, source) {
            found = Some(deep);
        }
    }
    found
}
