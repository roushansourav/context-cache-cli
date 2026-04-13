import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  applyRefactor,
  architectureOverview,
  buildGraph,
  crossRepoImpact,
  crossRepoSearch,
  detectChanges,
  embedGraph,
  findLargeFunctions,
  generateWiki,
  getAffectedFlows,
  getCommunity,
  getFlow,
  getReviewContext,
  getWikiPage,
  graphImpactRadius,
  graphStatus,
  listCommunities,
  listFlows,
  listRepos,
  minimalContext,
  queryGraph,
  refactorPreview,
  registerRepo,
  runPostprocess,
  semanticSearch,
  unregisterRepo,
} from '../../../../index';
import { getChangedFiles } from '../../../../utils/core/repo';

export type ToolArgs = Record<string, unknown>;
export type ToolResult = { content: Array<{ type: 'text'; text: string }> };
export type ToolHandler = (args: ToolArgs) => ToolResult;

export function makeToolResult(text: string): ToolResult {
  return { content: [{ type: 'text', text }] };
}

export const MCP_TOOLS = [
  {
    name: 'get_minimal_context',
    description:
      'Return compact context: risk, changed file counts, impact estimate, suggested next tools.',
    inputSchema: { type: 'object', properties: { base: { type: 'string', default: 'HEAD~1' } } },
  },
  {
    name: 'build_or_update_graph',
    description: 'Build or update graph database from current cache.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'list_graph_stats',
    description: 'Return graph node/edge counts and update timestamp.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'query_graph',
    description:
      'Query graph relationships: callers_of, callees_of, imports_of, importers_of, tests_for, container_of, depends_on, inheritance_of, implemented_by.',
    inputSchema: {
      type: 'object',
      required: ['pattern', 'target'],
      properties: {
        pattern: { type: 'string' },
        target: { type: 'string' },
        limit: { type: 'number', default: 50 },
      },
    },
  },
  {
    name: 'get_impact_radius',
    description: 'Get impacted files from changed files.',
    inputSchema: {
      type: 'object',
      properties: {
        changed_files: { type: 'array', items: { type: 'string' } },
        max_depth: { type: 'number', default: 2 },
        base: { type: 'string', default: 'HEAD~1' },
      },
    },
  },
  {
    name: 'detect_changes',
    description: 'Risk-scored change analysis for modified files.',
    inputSchema: { type: 'object', properties: { base: { type: 'string', default: 'HEAD~1' } } },
  },
  {
    name: 'run_postprocess',
    description: 'Recompute flow and community artifacts from current graph.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'list_flows',
    description: 'List execution flows sorted by criticality.',
    inputSchema: { type: 'object', properties: { limit: { type: 'number', default: 30 } } },
  },
  {
    name: 'get_flow',
    description: 'Get details of a single flow by id or name.',
    inputSchema: {
      type: 'object',
      properties: { flow_id: { type: 'number' }, flow_name: { type: 'string' } },
    },
  },
  {
    name: 'get_affected_flows',
    description: 'Find flows affected by changed files.',
    inputSchema: {
      type: 'object',
      properties: {
        changed_files: { type: 'array', items: { type: 'string' } },
        base: { type: 'string', default: 'HEAD~1' },
        limit: { type: 'number', default: 30 },
      },
    },
  },
  {
    name: 'list_communities',
    description: 'List detected code communities.',
    inputSchema: { type: 'object', properties: { limit: { type: 'number', default: 30 } } },
  },
  {
    name: 'get_community',
    description: 'Get details of a single community by id or name.',
    inputSchema: {
      type: 'object',
      properties: {
        community_id: { type: 'number' },
        community_name: { type: 'string' },
        include_members: { type: 'boolean', default: false },
      },
    },
  },
  {
    name: 'get_architecture_overview',
    description: 'Get architecture overview with coupling warnings.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'get_review_context',
    description: 'Build focused review context from changed files and blast radius.',
    inputSchema: {
      type: 'object',
      properties: {
        changed_files: { type: 'array', items: { type: 'string' } },
        max_depth: { type: 'number', default: 2 },
        include_source: { type: 'boolean', default: true },
        max_lines_per_file: { type: 'number', default: 200 },
        base: { type: 'string', default: 'HEAD~1' },
      },
    },
  },
  {
    name: 'find_large_functions',
    description: 'Find oversized functions/classes by line count.',
    inputSchema: {
      type: 'object',
      properties: {
        min_lines: { type: 'number', default: 50 },
        kind: { type: 'string' },
        file_path_pattern: { type: 'string' },
        limit: { type: 'number', default: 50 },
      },
    },
  },
  {
    name: 'get_docs_section',
    description: 'Return selected section from local docs/readme.',
    inputSchema: {
      type: 'object',
      required: ['section_name'],
      properties: { section_name: { type: 'string' } },
    },
  },
  {
    name: 'embed_graph',
    description: 'Compute hashed embeddings for semantic search.',
    inputSchema: { type: 'object', properties: { model: { type: 'string', default: 'hash-v1' } } },
  },
  {
    name: 'semantic_search_nodes',
    description: 'Semantic (vector) search with lexical fallback.',
    inputSchema: {
      type: 'object',
      required: ['query'],
      properties: {
        query: { type: 'string' },
        kind: { type: 'string' },
        limit: { type: 'number', default: 20 },
        model: { type: 'string', default: 'hash-v1' },
      },
    },
  },
  {
    name: 'refactor_tool',
    description: 'Preview symbol rename impact.',
    inputSchema: {
      type: 'object',
      required: ['symbol', 'new_name'],
      properties: {
        symbol: { type: 'string' },
        new_name: { type: 'string' },
        limit: { type: 'number', default: 100 },
      },
    },
  },
  {
    name: 'apply_refactor_tool',
    description: 'Apply symbol rename across repository files.',
    inputSchema: {
      type: 'object',
      required: ['symbol', 'new_name'],
      properties: {
        symbol: { type: 'string' },
        new_name: { type: 'string' },
        max_files: { type: 'number', default: 200 },
      },
    },
  },
  {
    name: 'generate_wiki_tool',
    description: 'Generate markdown wiki pages from communities.',
    inputSchema: { type: 'object', properties: { force: { type: 'boolean', default: false } } },
  },
  {
    name: 'get_wiki_page_tool',
    description: 'Read a generated wiki page by name.',
    inputSchema: {
      type: 'object',
      required: ['page_name'],
      properties: { page_name: { type: 'string' } },
    },
  },
  // Stage 5.1: Multi-repo registry tools
  {
    name: 'registry_list',
    description: 'List all registered repositories.',
    inputSchema: { type: 'object', properties: {} },
  },
  {
    name: 'registry_add',
    description: 'Register a repository in the multi-repo registry.',
    inputSchema: {
      type: 'object',
      required: ['repo_path'],
      properties: { repo_path: { type: 'string' }, alias: { type: 'string' } },
    },
  },
  {
    name: 'registry_remove',
    description: 'Remove a repository from the registry by alias.',
    inputSchema: { type: 'object', required: ['alias'], properties: { alias: { type: 'string' } } },
  },
  {
    name: 'cross_repo_search',
    description: 'Semantic search across all registered repositories.',
    inputSchema: {
      type: 'object',
      required: ['query'],
      properties: {
        query: { type: 'string' },
        kind: { type: 'string' },
        limit: { type: 'number', default: 20 },
        model: { type: 'string', default: 'hash-v1' },
        aliases: { type: 'array', items: { type: 'string' } },
      },
    },
  },
  {
    name: 'cross_repo_impact',
    description: 'Compute impact radius across registered repositories.',
    inputSchema: {
      type: 'object',
      properties: {
        changed_files: { type: 'array', items: { type: 'string' } },
        base: { type: 'string', default: 'HEAD~1' },
        max_depth: { type: 'number', default: 2 },
        aliases: { type: 'array', items: { type: 'string' } },
      },
    },
  },
] as const;

export function buildToolHandlers(repoRoot: string): Record<string, ToolHandler> {
  const safeLimit = (v: unknown, fallback: number): number => {
    const n = Number(v ?? fallback);
    return Number.isFinite(n) ? n : fallback;
  };
  const safeBase = (v: unknown): string => String(v || 'HEAD~1');
  const safeChangedFiles = (v: unknown, base: string): string[] =>
    Array.isArray(v) ? v.map(String) : getChangedFiles(repoRoot, base);
  const safeAliases = (v: unknown): string[] => (Array.isArray(v) ? v.map(String) : []);

  return {
    build_or_update_graph: () => {
      const dbPath = buildGraph(repoRoot);
      const stats = graphStatus(repoRoot);
      return makeToolResult(JSON.stringify({ dbPath, stats }, null, 2));
    },
    list_graph_stats: () => makeToolResult(JSON.stringify(graphStatus(repoRoot), null, 2)),
    get_minimal_context: (args) =>
      makeToolResult(JSON.stringify(minimalContext(repoRoot, safeBase(args.base)), null, 2)),
    query_graph: (args) => {
      const pattern = String(args.pattern || 'related');
      const target = String(args.target || '');
      const rows = queryGraph(repoRoot, pattern, target, safeLimit(args.limit, 50));
      return makeToolResult(JSON.stringify({ pattern, target, rows }, null, 2));
    },
    get_impact_radius: (args) => {
      const base = safeBase(args.base);
      const changed = safeChangedFiles(args.changed_files, base);
      const files = graphImpactRadius(repoRoot, changed, safeLimit(args.max_depth, 2));
      return makeToolResult(
        JSON.stringify({ changed, impacted_files: files, count: files.length }, null, 2),
      );
    },
    detect_changes: (args) => {
      const base = safeBase(args.base);
      return makeToolResult(
        JSON.stringify({ base, changes: detectChanges(repoRoot, base) }, null, 2),
      );
    },
    run_postprocess: () => {
      const [flows, communities] = runPostprocess(repoRoot);
      return makeToolResult(JSON.stringify({ flows, communities }, null, 2));
    },
    list_flows: (args) => {
      const rows = listFlows(repoRoot, safeLimit(args.limit, 30));
      return makeToolResult(JSON.stringify({ count: rows.length, flows: rows }, null, 2));
    },
    get_flow: (args) => {
      const flow = getFlow(
        repoRoot,
        args.flow_id ? Number(args.flow_id) : undefined,
        args.flow_name ? String(args.flow_name) : undefined,
      );
      return makeToolResult(JSON.stringify(flow ?? { status: 'not_found' }, null, 2));
    },
    get_affected_flows: (args) => {
      const base = safeBase(args.base);
      const changed = safeChangedFiles(args.changed_files, base);
      const rows = getAffectedFlows(repoRoot, changed, safeLimit(args.limit, 30));
      return makeToolResult(JSON.stringify({ changed, count: rows.length, flows: rows }, null, 2));
    },
    list_communities: (args) => {
      const rows = listCommunities(repoRoot, safeLimit(args.limit, 30));
      return makeToolResult(JSON.stringify({ count: rows.length, communities: rows }, null, 2));
    },
    get_community: (args) => {
      const community = getCommunity(
        repoRoot,
        args.community_id ? Number(args.community_id) : undefined,
        args.community_name ? String(args.community_name) : undefined,
        Boolean(args.include_members),
      );
      return makeToolResult(JSON.stringify(community ?? { status: 'not_found' }, null, 2));
    },
    get_architecture_overview: () =>
      makeToolResult(JSON.stringify(architectureOverview(repoRoot), null, 2)),
    get_review_context: (args) => {
      const base = safeBase(args.base);
      const changed = safeChangedFiles(args.changed_files, base);
      const context = getReviewContext(
        repoRoot,
        changed,
        safeLimit(args.max_depth, 2),
        args.include_source !== false,
        safeLimit(args.max_lines_per_file, 200),
        base,
      );
      return makeToolResult(JSON.stringify(context, null, 2));
    },
    find_large_functions: (args) => {
      const rows = findLargeFunctions(
        repoRoot,
        safeLimit(args.min_lines, 50),
        args.kind ? String(args.kind) : undefined,
        args.file_path_pattern ? String(args.file_path_pattern) : undefined,
        safeLimit(args.limit, 50),
      );
      return makeToolResult(JSON.stringify({ count: rows.length, rows }, null, 2));
    },
    get_docs_section: (args) => {
      const sectionName = String(args.section_name || '').toLowerCase();
      const readme = readFileSync(join(repoRoot, 'README.md'), 'utf8');
      return makeToolResult(extractDocsSection(readme, sectionName));
    },
    embed_graph: (args) =>
      makeToolResult(
        JSON.stringify(embedGraph(repoRoot, String(args.model || 'hash-v1')), null, 2),
      ),
    semantic_search_nodes: (args) => {
      const query = String(args.query || '');
      const kind = args.kind ? String(args.kind) : undefined;
      const rows = semanticSearch(
        repoRoot,
        query,
        kind,
        safeLimit(args.limit, 20),
        String(args.model || 'hash-v1'),
      );
      return makeToolResult(JSON.stringify({ query, kind, count: rows.length, rows }, null, 2));
    },
    refactor_tool: (args) =>
      makeToolResult(
        JSON.stringify(
          refactorPreview(
            repoRoot,
            String(args.symbol || ''),
            String(args.new_name || ''),
            safeLimit(args.limit, 100),
          ),
          null,
          2,
        ),
      ),
    apply_refactor_tool: (args) => {
      const changed = applyRefactor(
        repoRoot,
        String(args.symbol || ''),
        String(args.new_name || ''),
        safeLimit(args.max_files, 200),
      );
      return makeToolResult(JSON.stringify({ changed_files: changed }, null, 2));
    },
    generate_wiki_tool: (args) =>
      makeToolResult(JSON.stringify(generateWiki(repoRoot, Boolean(args.force)), null, 2)),
    get_wiki_page_tool: (args) =>
      makeToolResult(getWikiPage(repoRoot, String(args.page_name || ''))),
    // Stage 5.1 registry handlers
    registry_list: () => makeToolResult(JSON.stringify(listRepos(), null, 2)),
    registry_add: (args) => {
      const entry = registerRepo(
        String(args.repo_path || ''),
        args.alias ? String(args.alias) : undefined,
      );
      return makeToolResult(JSON.stringify(entry, null, 2));
    },
    registry_remove: (args) => {
      unregisterRepo(String(args.alias || ''));
      return makeToolResult('Removed');
    },
    cross_repo_search: (args) => {
      const rows = crossRepoSearch(
        String(args.query || ''),
        args.kind ? String(args.kind) : undefined,
        safeLimit(args.limit, 20),
        String(args.model || 'hash-v1'),
        safeAliases(args.aliases),
      );
      return makeToolResult(JSON.stringify({ count: rows.length, rows }, null, 2));
    },
    cross_repo_impact: (args) => {
      const base = safeBase(args.base);
      const changed = safeChangedFiles(args.changed_files, base);
      const results = crossRepoImpact(
        changed,
        safeLimit(args.max_depth, 2),
        safeAliases(args.aliases),
      );
      return makeToolResult(JSON.stringify(results, null, 2));
    },
  };
}

function extractDocsSection(readme: string, sectionName: string): string {
  const aliases: Record<string, string> = {
    usage: '## Usage',
    commands: '## Usage',
    legal: '## Licence',
    watch: 'Watch mode',
    languages: '## Parser Coverage',
    troubleshooting: '## Troubleshooting',
  };
  const heading = aliases[sectionName] ?? `## ${sectionName}`;
  const lines = readme.split('\n');
  const start = lines.findIndex((l) => l.trim().toLowerCase() === heading.trim().toLowerCase());
  if (start < 0) {
    return `Section not found: ${sectionName}`;
  }
  let end = lines.length;
  for (let i = start + 1; i < lines.length; i += 1) {
    if (/^##\s+/.test(lines[i])) {
      end = i;
      break;
    }
  }
  return lines.slice(start, end).join('\n').trim();
}
