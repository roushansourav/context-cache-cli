// ──────────────────────────────────────────────────────────────────────────────
// Scalar defaults
// ──────────────────────────────────────────────────────────────────────────────

export const DEFAULT_MAX_CHARS = 64_000;

// ──────────────────────────────────────────────────────────────────────────────
// Framework presets
// ──────────────────────────────────────────────────────────────────────────────

export const PRESETS: Record<string, { include: string[]; exclude: string[] }> = {
  generic: {
    include: ['**/*'],
    exclude: [
      '**/node_modules/**', '**/.git/**', '**/dist/**', '**/coverage/**',
      '**/.cache/**', '**/*.min.*', '**/pnpm-lock.yaml', '**/package-lock.json', '**/yarn.lock',
    ],
  },
  nx: {
    include: ['**/*'],
    exclude: [
      '**/node_modules/**', '**/.git/**', '**/.nx/**', '**/dist/**', '**/coverage/**',
      '**/.next/**', '**/.turbo/**', '**/build/**', '**/.cache/**', '**/*.min.*',
      '**/pnpm-lock.yaml', '**/package-lock.json', '**/yarn.lock',
    ],
  },
  nextjs: {
    include: ['**/*'],
    exclude: [
      '**/node_modules/**', '**/.git/**', '**/.next/**', '**/out/**', '**/dist/**',
      '**/coverage/**', '**/.turbo/**', '**/*.min.*',
      '**/pnpm-lock.yaml', '**/package-lock.json', '**/yarn.lock',
    ],
  },
  python: {
    include: ['**/*'],
    exclude: [
      '**/.git/**', '**/.venv/**', '**/venv/**', '**/__pycache__/**',
      '**/.pytest_cache/**', '**/.mypy_cache/**', '**/dist/**', '**/build/**', '**/*.pyc',
    ],
  },
};

// ──────────────────────────────────────────────────────────────────────────────
// File extension allowlist for the file watcher
// ──────────────────────────────────────────────────────────────────────────────

export const DEFAULT_TEXT_EXTENSIONS = [
  '.md', '.mdx', '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs',
  '.json', '.yaml', '.yml', '.txt', '.css', '.scss', '.sass', '.less',
  '.html', '.graphql', '.gql', '.py', '.java', '.kt', '.kts', '.go',
  '.rs', '.rb', '.php', '.cs', '.sh', '.zsh', '.bash', '.toml',
  '.ini', '.conf', '.sql', '.xml',
];

// ──────────────────────────────────────────────────────────────────────────────
// MCP prompt templates
// ──────────────────────────────────────────────────────────────────────────────

export const MCP_PROMPTS: Record<string, string> = {
  review_changes: [
    'Use graph tools to review changed code efficiently.',
    '1) Call get_minimal_context first.',
    '2) Call detect_changes and summarize high-risk files.',
    '3) Call get_impact_radius for blast radius.',
    '4) Call query_graph tests_for on high-risk symbols.',
  ].join('\n'),
  architecture_map: [
    'Generate architecture overview using graph structure.',
    '1) list_communities',
    '2) get_architecture_overview',
    '3) list_flows for runtime-critical paths',
  ].join('\n'),
  debug_issue: [
    'Debug using semantic + structural graph traversal.',
    '1) semantic_search_nodes with issue terms',
    '2) query_graph callers_of and callees_of',
    '3) get_affected_flows if changes are involved',
  ].join('\n'),
  onboard_developer: [
    'Onboard quickly with minimal token usage.',
    '1) list_graph_stats',
    '2) list_communities',
    '3) get_architecture_overview',
    '4) list_flows',
  ].join('\n'),
  pre_merge_check: [
    'Pre-merge quality pass.',
    '1) get_minimal_context',
    '2) detect_changes',
    '3) query_graph tests_for',
    '4) summarize risk and merge recommendation',
  ].join('\n'),
};
