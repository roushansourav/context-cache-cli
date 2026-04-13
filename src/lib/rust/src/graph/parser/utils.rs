use tree_sitter::Node;

pub fn text_of(node: Node, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(|s| s.to_string())
}

pub fn qualify(file_path: &str, enclosing_class: Option<&str>, symbol: &str) -> String {
    match enclosing_class {
        Some(cls) => format!("{}::{}.{}", file_path, cls, symbol),
        None => format!("{}::{}", file_path, symbol),
    }
}

pub fn extract_quoted(input: &str) -> Option<String> {
    let start_single = input.find('\'');
    let start_double = input.find('"');
    let (start, quote) = match (start_single, start_double) {
        (Some(a), Some(b)) => {
            if a < b {
                (a, '\'')
            } else {
                (b, '"')
            }
        }
        (Some(a), None) => (a, '\''),
        (None, Some(b)) => (b, '"'),
        (None, None) => return None,
    };

    let rest = &input[(start + 1)..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}
