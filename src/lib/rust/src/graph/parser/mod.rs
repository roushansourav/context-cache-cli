mod ast;
mod fallback;
mod language;
mod preprocess;
mod types;
mod utils;

#[cfg(test)]
mod tests;

pub use types::{ParsedFile, SymbolKind};

use language::lang_to_name;
use preprocess::normalize_source;

pub fn parse_file(file_path: &str, content: &str) -> ParsedFile {
    let (normalized_content, detected_lang) = normalize_source(file_path, content);

    let Some(lang) = detected_lang else {
        return fallback::fallback_parse(file_path, content);
    };

    let Some(parsed) = ast::parse_with_tree_sitter(file_path, &normalized_content, lang) else {
        return fallback::fallback_parse(file_path, &normalized_content);
    };

    if parsed.symbols.is_empty() && parsed.imports.is_empty() {
        return fallback::fallback_parse(file_path, &normalized_content);
    }

    parsed.with_language_defaults(lang_to_name(lang))
}
