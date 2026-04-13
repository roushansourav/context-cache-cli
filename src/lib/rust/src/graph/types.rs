// ── Risk-scoring weights (named constants — no magic numbers) ─────────────────
pub const RISK_WEIGHT_CALLERS: f64 = 1.5;
pub const RISK_WEIGHT_IMPACTED: f64 = 0.8;
pub const RISK_WEIGHT_LINES: f64 = 0.3;
pub const RISK_WEIGHT_FUNCTIONS: f64 = 0.7;
pub const RISK_WEIGHT_SECURITY: f64 = 1.3;
pub const RISK_PENALTY_NO_TESTS: f64 = 4.0;
pub const RISK_THRESHOLD_HIGH: f64 = 16.0;
pub const RISK_THRESHOLD_MEDIUM: f64 = 8.0;

/// Reciprocal Rank Fusion constant (standard value).
pub const RRF_K: f64 = 60.0;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphStatus {
    pub exists: bool,
    pub graph_path: String,
    pub node_count: i64,
    pub edge_count: i64,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryRow {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChangeRisk {
    pub file_path: String,
    pub impacted_files: i64,
    pub callers: i64,
    pub test_hits: i64,
    pub changed_lines: i64,
    pub security_hits: i64,
    pub risk_score: f64,
    pub risk: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinimalContext {
    pub risk: String,
    pub changed_files: i64,
    pub impacted_files: i64,
    pub top_files: Vec<String>,
    pub suggested_tools: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowRow {
    pub id: i64,
    pub name: String,
    pub entry: String,
    pub file_count: i64,
    pub node_count: i64,
    pub criticality: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowDetail {
    pub id: i64,
    pub name: String,
    pub entry: String,
    pub file_count: i64,
    pub node_count: i64,
    pub criticality: f64,
    pub nodes: Vec<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommunityRow {
    pub id: i64,
    pub name: String,
    pub file_count: i64,
    pub node_count: i64,
    pub coupling: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommunityDetail {
    pub id: i64,
    pub name: String,
    pub file_count: i64,
    pub node_count: i64,
    pub coupling: i64,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArchitectureOverview {
    pub communities: Vec<CommunityRow>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmbedResult {
    pub embedded: i64,
    pub total: i64,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticRow {
    pub qualified_name: String,
    pub kind: String,
    pub file_path: String,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LargeSymbolRow {
    pub kind: String,
    pub qualified_name: String,
    pub file_path: String,
    pub line_start: i64,
    pub line_end: i64,
    pub line_count: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewContext {
    pub changed_files: Vec<String>,
    pub impacted_files: Vec<String>,
    pub snippets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefactorOccurrence {
    pub file_path: String,
    pub line: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefactorPreview {
    pub symbol: String,
    pub new_name: String,
    pub total_occurrences: i64,
    pub files_touched: i64,
    pub occurrences: Vec<RefactorOccurrence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WikiResult {
    pub wiki_root: String,
    pub pages_generated: i64,
}
