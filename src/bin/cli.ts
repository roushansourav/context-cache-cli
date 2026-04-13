#!/usr/bin/env node
import { existsSync, writeFileSync, mkdirSync, readFileSync } from 'node:fs';
import { join, isAbsolute, dirname } from 'node:path';
import { Command } from 'commander';
import {
  refresh, status, formatPrompt, detectPreset, getCachePath, getConfigPath,
  buildGraph, buildOrUpdateGraph, graphStatus, queryGraph, graphImpactRadius, detectChanges,
  minimalContext, getGraphPath, runPostprocess, listFlows, getAffectedFlows,
  listCommunities, getFlow, getCommunity, getReviewContext, findLargeFunctions,
  architectureOverview, embedGraph, semanticSearch,
  refactorPreview, applyRefactor, generateWiki, getWikiPage,
  registerRepo, listRepos, unregisterRepo, crossRepoSearch, crossRepoImpact,
} from '../index';
import { DEFAULT_MAX_CHARS, PRESETS, DEFAULT_TEXT_EXTENSIONS, MCP_PROMPTS } from '../constants/core/constants';
import { getRepoRoot, getChangedFiles, getDefaultPromptPath } from '../utils/core/repo';
import { loadCachePayload, toPosixPath, stripQuotes, runImpactRadius } from '../utils/core/files';
import { runQuerySymbol } from '../features/context-cache/utils/core/symbols';
import { setupVscodeGlobal, copyToClipboard, check, upsertJsonServerConfig, runInstall } from '../features/context-cache/lib/runtime/setup';
import { watchRepo } from '../features/context-cache/lib/runtime/watch';
import { startMcpServer } from '../features/context-cache/components/mcp/server';
import { MCP_TOOLS } from '../features/context-cache/components/mcp/tools';
import { evaluateParity } from '../features/context-cache/utils/core/parity';

const program = new Command();
program.name('context-cache').description('Blazing-fast global context cache for AI/LLM prompts').version('0.2.0');

program.command('init').description('Create global config for current repo')
  .option('--preset <preset>', 'framework preset: generic|nx|nextjs|python')
  .option('--force', 'overwrite existing config')
  .action((opts: { preset?: string; force?: boolean }) => {
    const repoRoot = getRepoRoot();
    const configPath = getConfigPath(repoRoot);
    if (existsSync(configPath) && !opts.force) { console.log(`Config exists: ${configPath}\nUse --force to overwrite.`); return; }
    mkdirSync(dirname(configPath), { recursive: true });
    const presetName = opts.preset && PRESETS[opts.preset] ? opts.preset : detectPreset(repoRoot);
    const { include, exclude } = PRESETS[presetName]!;
    writeFileSync(configPath, JSON.stringify({ preset: presetName, mode: 'full', include, exclude, textExtensions: DEFAULT_TEXT_EXTENSIONS, maxFiles: 0, maxFileChars: 4000 }, null, 2) + '\n', 'utf8');
    console.log(`Created ${configPath}\nUsing preset: ${presetName}`);
  });

program.command('detect').description('Print detected framework preset').action(() => console.log(detectPreset(getRepoRoot())));

program.command('graph-build').description('Build or rebuild SQLite graph from cache')
  .option('--refresh', 'Refresh cache first')
  .option('--full', 'Force full rebuild instead of incremental update')
  .action((opts: { refresh?: boolean; full?: boolean }) => {
    const repoRoot = getRepoRoot();
    if (opts.refresh) refresh(repoRoot);
    const dbPath = opts.full ? buildGraph(repoRoot) : buildOrUpdateGraph(repoRoot, false);
    const s = graphStatus(repoRoot);
    console.log(`Graph DB: ${dbPath}\nNodes:    ${s.nodeCount}\nEdges:    ${s.edgeCount}\nUpdated:  ${s.updatedAt || '-'}`);
  });

program.command('get-flow').description('Get flow details by id or name')
  .option('--id <n>').option('--name <name>')
  .action((opts: { id?: string; name?: string }) => {
    const flow = getFlow(getRepoRoot(), opts.id ? Number.parseInt(opts.id, 10) : undefined, opts.name);
    if (!flow) { console.log('Flow not found.'); return; }
    console.log(JSON.stringify(flow, null, 2));
  });

program.command('get-community').description('Get community details by id or name')
  .option('--id <n>').option('--name <name>').option('--members', 'Include members')
  .action((opts: { id?: string; name?: string; members?: boolean }) => {
    const community = getCommunity(getRepoRoot(), opts.id ? Number.parseInt(opts.id, 10) : undefined, opts.name, Boolean(opts.members));
    if (!community) { console.log('Community not found.'); return; }
    console.log(JSON.stringify(community, null, 2));
  });

program.command('review-context').description('Build focused review context for changed files')
  .option('--base <ref>', '', 'HEAD~1')
  .option('--changed <paths>', 'Comma-separated paths')
  .option('--depth <n>', '', '2')
  .option('--max-lines <n>', '', '200')
  .option('--no-source', 'Skip source snippets')
  .action((opts: { base: string; changed?: string; depth: string; maxLines: string; source?: boolean }) => {
    const repoRoot = getRepoRoot();
    const changed = opts.changed ? opts.changed.split(',').map((p) => toPosixPath(stripQuotes(p.trim()))).filter(Boolean) : getChangedFiles(repoRoot, opts.base);
    const context = getReviewContext(repoRoot, changed, Number.parseInt(opts.depth, 10) || 2, opts.source !== false, Number.parseInt(opts.maxLines, 10) || 200, opts.base);
    console.log(JSON.stringify(context, null, 2));
  });

program.command('find-large-functions').description('Find oversized functions/classes')
  .option('--min-lines <n>', '', '50')
  .option('--kind <kind>')
  .option('--file <pattern>')
  .option('--limit <n>', '', '50')
  .action((opts: { minLines: string; kind?: string; file?: string; limit: string }) => {
    const rows = findLargeFunctions(getRepoRoot(), Number.parseInt(opts.minLines, 10) || 50, opts.kind, opts.file, Number.parseInt(opts.limit, 10) || 50);
    console.log(JSON.stringify(rows, null, 2));
  });

program.command('docs-section').description('Print a section from local README')
  .argument('<name>').action((name: string) => {
    const readme = readFileSync(join(getRepoRoot(), 'README.md'), 'utf8');
    const marker = `## ${name}`;
    const start = readme.toLowerCase().indexOf(marker.toLowerCase());
    if (start < 0) { console.log(`Section not found: ${name}`); return; }
    const tail = readme.slice(start);
    const next = tail.slice(marker.length).search(/\n##\s+/);
    console.log(next > 0 ? tail.slice(0, marker.length + next).trim() : tail.trim());
  });

program.command('graph-status').description('Show graph DB status and counts').action(() => {
  const repoRoot = getRepoRoot();
  const s = graphStatus(repoRoot);
  if (!s.exists) { console.log(`No graph DB found at ${s.graphPath}\nRun: context-cache graph-build --refresh`); return; }
  console.log(`Graph:   ${s.graphPath}\nNodes:   ${s.nodeCount}\nEdges:   ${s.edgeCount}\nUpdated: ${s.updatedAt || '-'}`);
});

program.command('query-graph').description('Query graph relationships')
  .argument('<pattern>', 'callers_of|callees_of|imports_of|importers_of|tests_for|container_of|depends_on|inheritance_of|implemented_by')
  .argument('<target>', 'symbol or path target').option('--limit <n>', 'Result limit', '80')
  .action((pattern: string, target: string, opts: { limit: string }) => {
    const rows = queryGraph(getRepoRoot(), pattern, target, Number.parseInt(opts.limit, 10) || 80);
    console.log(`Pattern: ${pattern}\nTarget:  ${target}\nRows:    ${rows.length}\n`);
    for (const row of rows) console.log(`- [${row.kind}] ${row.source} -> ${row.target}${row.filePath ? ` (${row.filePath})` : ''}`);
  });

program.command('mcp-prompts').description('List built-in MCP prompt templates').option('--show <name>', 'Print full prompt body')
  .action((opts: { show?: string }) => {
    if (opts.show) { const body = MCP_PROMPTS[opts.show.trim()]; if (!body) { console.error(`Unknown: ${opts.show}`); process.exit(1); } console.log(`# ${opts.show}\n${body}`); return; }
    console.log('Built-in prompts:'); for (const key of Object.keys(MCP_PROMPTS)) console.log(`- ${key}`);
  });

program.command('install').description('Install MCP server config for AI tools')
  .option('--platform <name>', 'all|claude|codex|cursor|copilot', 'all').option('--dry-run', 'Preview without writing')
  .action((opts: { platform: string; dryRun?: boolean }) => runInstall(opts));

program.command('detect-changes').description('Risk-scored change analysis from graph').option('--base <ref>', 'Git base ref', 'HEAD~1')
  .action((opts: { base: string }) => {
    const rows = detectChanges(getRepoRoot(), opts.base);
    if (rows.length === 0) { console.log('No changed files detected.'); return; }
    for (const row of rows) console.log(`[${row.risk}] ${row.filePath} | impacted=${row.impactedFiles} callers=${row.callers} tests=${row.testHits}`);
  });

program.command('minimal-context').description('Compact graph-aware context for AI entry-point calls').option('--base <ref>', 'Git base ref', 'HEAD~1')
  .action((opts: { base: string }) => {
    const mc = minimalContext(getRepoRoot(), opts.base);
    console.log(`Risk: ${mc.risk}\nChanged files: ${mc.changedFiles}\nImpacted files: ${mc.impactedFiles}\nTop files:`);
    for (const f of mc.topFiles) console.log(`  - ${f}`);
    console.log('Suggested tools:'); for (const t of mc.suggestedTools) console.log(`  - ${t}`);
  });

program.command('graph-postprocess').description('Recompute flow and community artifacts').action(() => {
  const [flows, communities] = runPostprocess(getRepoRoot());
  console.log(`Flows:       ${flows}\nCommunities: ${communities}`);
});

program.command('list-flows').description('List execution flows sorted by criticality').option('--limit <n>', 'Max rows', '30')
  .action((opts: { limit: string }) => {
    for (const r of listFlows(getRepoRoot(), Number.parseInt(opts.limit, 10) || 30))
      console.log(`#${r.id} ${r.name} | entry=${r.entry} files=${r.fileCount} criticality=${r.criticality.toFixed(2)}`);
  });

program.command('affected-flows').description('Find flows affected by changed files')
  .option('--base <ref>', '', 'HEAD~1').option('--changed <paths>', 'Comma-separated paths').option('--limit <n>', '', '30')
  .action((opts: { base: string; changed?: string; limit: string }) => {
    const repoRoot = getRepoRoot();
    const changed = opts.changed ? opts.changed.split(',').map((p) => toPosixPath(stripQuotes(p.trim()))).filter(Boolean) : getChangedFiles(repoRoot, opts.base);
    const rows = getAffectedFlows(repoRoot, changed, Number.parseInt(opts.limit, 10) || 30);
    console.log(`Changed: ${changed.length}  Affected flows: ${rows.length}`);
    for (const r of rows) console.log(`#${r.id} ${r.name} | entry=${r.entry} files=${r.fileCount} criticality=${r.criticality.toFixed(2)}`);
  });

program.command('list-communities').description('List detected communities').option('--limit <n>', '', '30')
  .action((opts: { limit: string }) => {
    for (const r of listCommunities(getRepoRoot(), Number.parseInt(opts.limit, 10) || 30))
      console.log(`#${r.id} ${r.name} | files=${r.fileCount} nodes=${r.nodeCount} coupling=${r.coupling}`);
  });

program.command('architecture-overview').description('Show architecture overview and coupling warnings').action(() => {
  const overview = architectureOverview(getRepoRoot());
  console.log(`Communities: ${overview.communities.length}`);
  for (const c of overview.communities.slice(0, 20)) console.log(`- ${c.name} | files=${c.fileCount} nodes=${c.nodeCount} coupling=${c.coupling}`);
  console.log('Warnings:'); for (const w of overview.warnings) console.log(`- ${w}`);
});

program.command('embed-graph').description('Compute embeddings for semantic search').option('--model <name>', 'hash-v1 | ollama:<model> | openai:<model>', 'hash-v1')
  .action((opts: { model: string }) => {
    const r = embedGraph(getRepoRoot(), opts.model);
    console.log(`Model: ${r.model}\nEmbedded: ${r.embedded}\nTotal: ${r.total}`);
  });

program.command('semantic-search').description('Semantic search').argument('<query>').option('--kind <kind>').option('--limit <n>', '', '20').option('--model <name>', '', 'hash-v1')
  .action((query: string, opts: { kind?: string; limit: string; model: string }) => {
    const rows = semanticSearch(getRepoRoot(), query, opts.kind, Number.parseInt(opts.limit, 10) || 20, opts.model);
    console.log(`Query: ${query}  Kind: ${opts.kind || '(any)'}  Rows: ${rows.length}\n`);
    for (const r of rows) console.log(`${r.score.toFixed(4)} | [${r.kind}] ${r.qualifiedName} (${r.filePath})`);
  });

program.command('refactor-preview').description('Preview symbol rename impact').argument('<symbol>').argument('<newName>').option('--limit <n>', '', '120')
  .action((symbol: string, newName: string, opts: { limit: string }) => {
    const p = refactorPreview(getRepoRoot(), symbol, newName, Number.parseInt(opts.limit, 10) || 120);
    console.log(`Symbol: ${p.symbol}\nNew:    ${p.newName}\nFiles:  ${p.filesTouched}\nHits:   ${p.totalOccurrences}\n`);
    for (const occ of p.occurrences) console.log(`- ${occ.filePath}:${occ.line} | ${occ.text}`);
  });

program.command('refactor-apply').description('Apply symbol rename across files').argument('<symbol>').argument('<newName>').option('--max-files <n>', '', '200').option('--yes', 'Confirm write')
  .action((symbol: string, newName: string, opts: { maxFiles: string; yes?: boolean }) => {
    const repoRoot = getRepoRoot();
    if (!opts.yes) { const p = refactorPreview(repoRoot, symbol, newName, 20); console.log(`Preview only. Re-run with --yes to apply.\nWould touch ~${p.filesTouched} files.`); return; }
    console.log(`Updated files: ${applyRefactor(repoRoot, symbol, newName, Number.parseInt(opts.maxFiles, 10) || 200)}`);
  });

program.command('generate-wiki').description('Generate markdown wiki from communities').option('--force', 'Overwrite existing pages')
  .action((opts: { force?: boolean }) => {
    const r = generateWiki(getRepoRoot(), Boolean(opts.force));
    console.log(`Wiki root: ${r.wikiRoot}\nPages generated: ${r.pagesGenerated}`);
  });

program.command('get-wiki-page').description('Print generated wiki page content').argument('<pageName>').action((pn: string) => console.log(getWikiPage(getRepoRoot(), pn)));

program.command('mcp-serve').description('Run MCP stdio server for AI integrations').action(() => {
  const repoRoot = getRepoRoot();
  if (!existsSync(getGraphPath(repoRoot))) { console.error('Graph DB not found. Run `context-cache graph-build --refresh` first.'); process.exit(1); }
  startMcpServer(repoRoot);
});

program.command('refresh').description('Refresh cache for current repo').action(() => {
  const r = refresh(getRepoRoot());
  console.log(`Cache: ${r.cachePath}\nIndexed: ${r.payload.fileCount} files\nChanged: ${r.payload.changedCount} files`);
});

program.command('status').description('Show cache status').action(() => {
  const repoRoot = getRepoRoot(); const s = status(repoRoot);
  if (!s.exists) { console.log(`No cache found at ${s.cachePath}\nRun: context-cache refresh`); return; }
  console.log(`Cache: ${s.cachePath}\nRepo: ${s.repoRoot}\nUpdated: ${s.updatedAt}\nFiles: ${s.fileCount}`);
});

program.command('prompt').description('Print cached context to stdout').option('--max-chars <n>', 'Max characters', String(DEFAULT_MAX_CHARS))
  .action((opts: { maxChars: string }) => {
    const max = Number.parseInt(opts.maxChars, 10);
    console.log(formatPrompt(getRepoRoot(), Number.isFinite(max) ? max : DEFAULT_MAX_CHARS));
  });

program.command('prompt-ready').description('Refresh then write prompt file').option('--out <file>').option('--max-chars <n>', '', String(DEFAULT_MAX_CHARS))
  .action((opts: { out?: string; maxChars: string }) => {
    const repoRoot = getRepoRoot(); const max = Number.parseInt(opts.maxChars, 10);
    const result = refresh(repoRoot); const output = formatPrompt(repoRoot, Number.isFinite(max) ? max : DEFAULT_MAX_CHARS);
    const outPath = opts.out ? (isAbsolute(opts.out) ? opts.out : join(repoRoot, opts.out)) : getDefaultPromptPath(repoRoot);
    writeFileSync(outPath, output + '\n', 'utf8');
    console.log(`Refreshed: ${result.cachePath}\nPrompt written: ${outPath} (${output.length.toLocaleString()} chars)`);
  });

program.command('ready').description('One-step: refresh + write prompt + print next step').option('--max-chars <n>', '', String(DEFAULT_MAX_CHARS)).option('--out <file>')
  .action((opts: { maxChars: string; out?: string }) => {
    const repoRoot = getRepoRoot(); const max = Number.parseInt(opts.maxChars, 10);
    const result = refresh(repoRoot); const output = formatPrompt(repoRoot, Number.isFinite(max) ? max : DEFAULT_MAX_CHARS);
    const outPath = opts.out ? (isAbsolute(opts.out) ? opts.out : join(repoRoot, opts.out)) : getDefaultPromptPath(repoRoot);
    writeFileSync(outPath, output + '\n', 'utf8');
    console.log(`Context cache is ready.\n1) Cache: ${result.cachePath}\n2) Prompt: ${outPath}\n3) Attach this prompt file in Copilot Chat before asking your task.`);
  });

program.command('prompt-copy').description('Copy prompt text to clipboard').option('--max-chars <n>', '', String(DEFAULT_MAX_CHARS)).option('--no-refresh', 'Skip refresh')
  .action((opts: { maxChars: string; refresh?: boolean }) => {
    const repoRoot = getRepoRoot(); const max = Number.parseInt(opts.maxChars, 10);
    if (opts.refresh !== false) refresh(repoRoot);
    const output = formatPrompt(repoRoot, Number.isFinite(max) ? max : DEFAULT_MAX_CHARS);
    copyToClipboard(output); console.log(`Copied ${output.length.toLocaleString()} characters to clipboard.`);
  });

program.command('doctor').description('Run environment and setup checks').action(() => {
  const repoRoot = getRepoRoot(); const s = status(repoRoot); const configPath = getConfigPath(repoRoot);
  const checks = [
    ['context-cache on PATH', check('context-cache')], ['git available', check('git')],
    ['global config exists', existsSync(configPath)], ['global cache exists', s.exists],
    ['cache directory writable', existsSync(dirname(s.cachePath))],
    ['clipboard tool available', process.platform === 'darwin' ? check('pbcopy') : true],
  ] as const;
  let failed = 0;
  for (const [name, ok] of checks) { console.log(`${ok ? 'OK' : 'FAIL'} ${name}`); if (!ok) failed += 1; }
  if (failed > 0) { if (!existsSync(configPath)) console.log('  - Run: context-cache init'); if (!s.exists) console.log('  - Run: context-cache refresh'); process.exitCode = 1; }
  else console.log('All checks passed.');
});

program.command('impact-radius').description('Estimate impacted files using import dependency graph')
  .option('--base <ref>', '', 'HEAD~1').option('--changed <paths>', 'Comma-separated paths').option('--depth <n>', '', '3').option('--refresh', 'Refresh cache before analysis')
  .action((opts: { base: string; changed?: string; depth: string; refresh?: boolean }) => {
    runImpactRadius(getRepoRoot(), opts, (r: string) => { refresh(r); }, getChangedFiles);
  });

program.command('query-symbol').description('Find symbol definitions, references, and probable test files')
  .argument('<symbol>').option('--refresh', 'Refresh cache before query').option('--limit <n>', '', '120')
  .action((symbol: string, opts: { refresh?: boolean; limit: string }) => {
    const repoRoot = getRepoRoot();
    if (opts.refresh) refresh(repoRoot);
    runQuerySymbol(repoRoot, symbol, opts, loadCachePayload(repoRoot));
  });

program.command('watch').description('Watch repo and auto-refresh on changes').action(() => watchRepo(getRepoRoot()));
program.command('cache-path').description('Print cache store file path').action(() => console.log(getCachePath(getRepoRoot())));
program.command('vscode-setup').description('Add context-cache tasks to user-level VS Code tasks.json').action(() => setupVscodeGlobal());

// ── Stage 5.1: Multi-repo registry ───────────────────────────────────────────

program.command('registry-list').description('List registered repositories').action(() => {
  const repos = listRepos();
  if (repos.length === 0) { console.log('No repositories registered.'); return; }
  for (const r of repos) console.log(`${r.alias} | ${r.path} | nodes=${r.nodeCount} edges=${r.edgeCount}`);
});

program.command('registry-add').description('Register a repository').argument('<path>').option('--alias <name>', 'Alias for this repo')
  .action((repoPath: string, opts: { alias?: string }) => {
    const entry = registerRepo(repoPath, opts.alias);
    console.log(`Registered: ${entry.alias} (${entry.path})\nNodes: ${entry.nodeCount}  Edges: ${entry.edgeCount}`);
  });

program.command('registry-remove').description('Remove a repository from the registry by alias').argument('<alias>')
  .action((alias: string) => { unregisterRepo(alias); console.log(`Removed: ${alias}`); });

program.command('cross-repo-search').description('Search across registered repositories').argument('<query>')
  .option('--kind <kind>').option('--limit <n>', '', '20').option('--model <name>', '', 'hash-v1').option('--aliases <csv>', 'Comma-separated aliases to filter')
  .action((query: string, opts: { kind?: string; limit: string; model: string; aliases?: string }) => {
    const aliases = opts.aliases ? opts.aliases.split(',').map((a) => a.trim()).filter(Boolean) : [];
    const rows = crossRepoSearch(query, opts.kind, Number.parseInt(opts.limit, 10) || 20, opts.model, aliases);
    console.log(`Query: ${query}  Results: ${rows.length}\n`);
    for (const r of rows) console.log(`${r.score.toFixed(4)} | [${r.repoAlias}] ${r.qualifiedName} (${r.filePath})`);
  });

program.command('cross-repo-impact').description('Compute impact across registered repositories')
  .option('--base <ref>', '', 'HEAD~1').option('--changed <paths>').option('--depth <n>', '', '2').option('--aliases <csv>')
  .action((opts: { base: string; changed?: string; depth: string; aliases?: string }) => {
    const repoRoot = getRepoRoot();
    const changed = opts.changed ? opts.changed.split(',').map((p) => toPosixPath(stripQuotes(p.trim()))).filter(Boolean) : getChangedFiles(repoRoot, opts.base);
    const aliases = opts.aliases ? opts.aliases.split(',').map((a) => a.trim()).filter(Boolean) : [];
    const results = crossRepoImpact(changed, Number.parseInt(opts.depth, 10) || 2, aliases);
    for (const row of results) {
      const [alias, ...files] = row;
      console.log(`[${alias}] ${files.length} impacted`);
      for (const f of (files ?? []).slice(0, 20)) console.log(`  - ${f}`);
    }
  });

// ── Stage 5.2: Parity evaluation ─────────────────────────────────────────────

program.command('parity').description('Report CLI ↔ MCP tool parity').action(() => {
  const cliCommands = program.commands.map((c) => c.name()).filter((n) => n !== 'parity');
  const mcpToolNames = (MCP_TOOLS as ReadonlyArray<{ name: string }>).map((t) => t.name);
  const report = evaluateParity(cliCommands, mcpToolNames);
  console.log(`\nMatched (${report.matched.length}):`);
  for (const m of report.matched) console.log(`  ${m.cliCommand} ↔ ${m.mcpTool}`);
  console.log(`\nCLI-only (${report.cliOnly.length}):`);
  for (const c of report.cliOnly) console.log(`  ${c}`);
  console.log(`\nMCP-only (${report.mcpOnly.length}):`);
  for (const t of report.mcpOnly) console.log(`  ${t}`);
});

// Re-export for completeness — upsertJsonServerConfig used in install tests
export { upsertJsonServerConfig };

program.parse(process.argv);
