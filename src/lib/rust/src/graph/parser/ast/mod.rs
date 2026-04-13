mod classify;
mod extract;
mod traversal;

use tree_sitter::Parser;

use super::language::{language_for, Lang};
use super::types::ParsedFile;

pub fn parse_with_tree_sitter(file_path: &str, content: &str, lang: Lang) -> Option<ParsedFile> {
    let language = language_for(lang)?;

    let mut parser = Parser::new();
    if parser.set_language(language).is_err() {
        return None;
    }

    let tree = parser.parse(content, None)?;
    Some(traversal::walk_tree(tree.root_node(), content.as_bytes(), file_path, lang))
}
