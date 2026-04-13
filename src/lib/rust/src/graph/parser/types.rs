#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Class,
}

#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub kind: SymbolKind,
    pub name: String,
    pub qualified_name: String,
    pub container: String,
    pub line_start: i64,
    pub line_end: i64,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct PendingRelation {
    pub kind: String,
    pub source_qname: String,
    pub target_name: String,
}

#[derive(Debug, Clone)]
pub struct CallSite {
    pub source_qname: String,
    pub target_name: String,
}

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub symbols: Vec<SymbolDef>,
    pub imports: Vec<String>,
    pub relations: Vec<PendingRelation>,
    pub calls: Vec<CallSite>,
}

impl ParsedFile {
    pub fn empty() -> Self {
        Self {
            symbols: Vec::new(),
            imports: Vec::new(),
            relations: Vec::new(),
            calls: Vec::new(),
        }
    }

    pub fn with_language_defaults(mut self, language: &str) -> Self {
        for symbol in &mut self.symbols {
            if symbol.language.is_empty() {
                symbol.language = language.to_string();
            }
        }
        self
    }
}
