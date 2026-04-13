use std::path::Path;
use std::path::PathBuf;
use glob::Pattern;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    pub preset: String,
    pub mode: String, // "full" | "summary"
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub text_extensions: Vec<String>,
    pub max_files: usize,
    pub max_file_chars: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            preset: "generic".into(),
            mode: "full".into(),
            include: vec!["**/*".into()],
            exclude: vec![
                "**/node_modules/**".into(),
                "**/.git/**".into(),
                "**/.nx/**".into(),
                "**/dist/**".into(),
                "**/coverage/**".into(),
                "**/.next/**".into(),
                "**/.turbo/**".into(),
                "**/build/**".into(),
                "**/.cache/**".into(),
                "**/*.min.*".into(),
                "**/*.lock".into(),
                "**/pnpm-lock.yaml".into(),
                "**/package-lock.json".into(),
                "**/yarn.lock".into(),
            ],
            text_extensions: default_text_extensions(),
            max_files: 0,
            max_file_chars: 4000,
        }
    }
}

pub fn default_text_extensions() -> Vec<String> {
    vec![
        ".md", ".mdx", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs",
        ".json", ".yaml", ".yml", ".txt", ".css", ".scss", ".sass",
        ".less", ".html", ".graphql", ".gql", ".py", ".java", ".kt",
        ".kts", ".go", ".rs", ".rb", ".php", ".cs", ".sh", ".zsh",
        ".bash", ".toml", ".ini", ".conf", ".sql", ".xml",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

impl CacheConfig {
    pub fn compiled_include(&self) -> Vec<Pattern> {
        self.include
            .iter()
            .filter_map(|p| Pattern::new(p).ok())
            .collect()
    }

    pub fn compiled_exclude(&self) -> Vec<Pattern> {
        self.exclude
            .iter()
            .filter_map(|p| Pattern::new(p).ok())
            .collect()
    }

    /// Global config path in user space:
    /// ~/.context-cache-store/configs/<blake3(repo_root)>.json
    pub fn global_config_path(repo_root: &Path) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let key = blake3::hash(repo_root.to_string_lossy().as_bytes()).to_hex().to_string();
        let dir = PathBuf::from(home).join(".context-cache-store").join("configs");
        std::fs::create_dir_all(&dir).ok();
        dir.join(format!("{}.json", key))
    }

    /// Load config from global user store, with fallback to legacy repo config.
    pub fn load(repo_root: &Path) -> CacheConfig {
        let global_path = Self::global_config_path(repo_root);
        let repo_path = repo_root.join(".context-cache.json");

        let config_path = if global_path.exists() {
            global_path
        } else if repo_path.exists() {
            repo_path
        } else {
            return CacheConfig::default();
        };

        let raw = match std::fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(_) => return CacheConfig::default(),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return CacheConfig::default(),
        };

        let defaults = CacheConfig::default();

        CacheConfig {
            preset: parsed["preset"].as_str().unwrap_or(&defaults.preset).to_string(),
            mode: {
                let m = parsed["mode"].as_str().unwrap_or("full");
                if m == "summary" { "summary".into() } else { "full".into() }
            },
            include: parsed["include"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or(defaults.include),
            exclude: parsed["exclude"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or(defaults.exclude),
            text_extensions: parsed["textExtensions"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .filter(|v: &Vec<String>| !v.is_empty())
                .unwrap_or(defaults.text_extensions),
            max_files: parsed["maxFiles"].as_u64().unwrap_or(0) as usize,
            max_file_chars: parsed["maxFileChars"].as_u64().unwrap_or(4000) as usize,
        }
    }
}
