import { join } from 'node:path';
import { createRequire } from 'node:module';

const localRequire = createRequire(__filename);

function loadNativeBinding(): NativeBinding {
  const prebuiltPackages = packageCandidates();
  for (const pkg of prebuiltPackages) {
    try {
      return localRequire(pkg) as NativeBinding;
    } catch {
      // Keep trying other optional prebuilt packages.
    }
  }

  const packageRoot = join(__dirname, '..');
  const candidates = [
    join(packageRoot, 'binding.node'),
    join(packageRoot, 'src', 'lib', 'rust', 'target', 'release', 'libcontext_cache_core.dylib'),
    join(packageRoot, 'src', 'lib', 'rust', 'target', 'release', 'libcontext_cache_core.so'),
    join(packageRoot, 'src', 'lib', 'rust', 'target', 'release', 'context_cache_core.dll'),
  ];
  for (const p of candidates) {
    try {
      return localRequire(p) as NativeBinding;
    } catch {
      continue;
    }
  }

  throw new Error(
    [
      'context-cache: failed to load native binding.',
      'Install a matching prebuilt package for your platform, or build locally with `npm run build` (requires Rust/Cargo).',
      `Detected platform=${process.platform} arch=${process.arch}`,
    ].join('\n'),
  );
}

function packageCandidates(): string[] {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin' && arch === 'arm64') return ['context-cache-darwin-arm64'];
  if (platform === 'darwin' && arch === 'x64') return ['context-cache-darwin-x64'];

  if (platform === 'linux' && arch === 'arm64') {
    return ['context-cache-linux-arm64-gnu'];
  }
  if (platform === 'linux' && arch === 'x64') {
    return isMusl()
      ? ['context-cache-linux-x64-musl', 'context-cache-linux-x64-gnu']
      : ['context-cache-linux-x64-gnu', 'context-cache-linux-x64-musl'];
  }

  if (platform === 'win32' && arch === 'arm64') return ['context-cache-win32-arm64-msvc'];
  if (platform === 'win32' && arch === 'x64') return ['context-cache-win32-x64-msvc'];

  return [];
}

function isMusl(): boolean {
  if (process.platform !== 'linux') return false;
  try {
    const report = (
      process as {
        report?: { getReport?: () => unknown };
      }
    ).report;
    const runtime = report?.getReport?.() as
      | { header?: { glibcVersionRuntime?: string } }
      | undefined;
    const glibc = runtime?.header?.glibcVersionRuntime;
    return !glibc;
  } catch {
    return false;
  }
}

export interface FileEntry {
  path: string;
  hash: string;
  mtimeMs: number;
  size: number;
  mode: string;
  content?: string;
  summary: string;
}
export interface CachePayload {
  repoRoot: string;
  updatedAt: string;
  configHash: string;
  fileCount: number;
  changedCount: number;
  files: FileEntry[];
}
export interface RefreshResult {
  payload: CachePayload;
  cachePath: string;
}
export interface StatusResult {
  exists: boolean;
  cachePath: string;
  repoRoot?: string;
  updatedAt?: string;
  fileCount?: number;
}
export interface GraphStatusResult {
  exists: boolean;
  graphPath: string;
  nodeCount: number;
  edgeCount: number;
  updatedAt?: string;
}
export interface GraphQueryRow {
  source: string;
  target: string;
  kind: string;
  filePath?: string;
}
export interface ChangeRiskRow {
  filePath: string;
  impactedFiles: number;
  callers: number;
  testHits: number;
  changedLines: number;
  securityHits: number;
  riskScore: number;
  risk: string;
}
export interface MinimalContextResult {
  risk: string;
  changedFiles: number;
  impactedFiles: number;
  topFiles: string[];
  suggestedTools: string[];
}
export interface FlowRow {
  id: number;
  name: string;
  entry: string;
  fileCount: number;
  nodeCount: number;
  criticality: number;
}
export interface FlowDetail extends FlowRow {
  nodes: string[];
  files: string[];
}
export interface CommunityRow {
  id: number;
  name: string;
  fileCount: number;
  nodeCount: number;
  coupling: number;
}
export interface CommunityDetail extends CommunityRow {
  members: string[];
}
export interface ArchitectureOverviewResult {
  communities: CommunityRow[];
  warnings: string[];
}
export interface EmbedResult {
  embedded: number;
  total: number;
  model: string;
}
export interface SemanticRow {
  qualifiedName: string;
  kind: string;
  filePath: string;
  score: number;
}
export interface LargeSymbolRow {
  kind: string;
  qualifiedName: string;
  filePath: string;
  lineStart: number;
  lineEnd: number;
  lineCount: number;
}
export interface ReviewContextResult {
  changedFiles: string[];
  impactedFiles: string[];
  snippets: string[];
}
export interface RefactorOccurrence {
  filePath: string;
  line: number;
  text: string;
}
export interface RefactorPreviewResult {
  symbol: string;
  newName: string;
  totalOccurrences: number;
  filesTouched: number;
  occurrences: RefactorOccurrence[];
}
export interface WikiResult {
  wikiRoot: string;
  pagesGenerated: number;
}

// Stage 5.1: Multi-repo registry
export interface RepoEntry {
  alias: string;
  path: string;
  nodeCount: number;
  edgeCount: number;
  registeredAt: string;
}
export interface CrossRepoSearchResult {
  repoAlias: string;
  repoPath: string;
  qualifiedName: string;
  kind: string;
  filePath: string;
  score: number;
}

interface NativeBinding {
  refresh(repoRoot: string): RefreshResult;
  status(repoRoot: string): StatusResult;
  formatPrompt(repoRoot: string, maxChars: number): string;
  detectPreset(repoRoot: string): string;
  getCachePath(repoRoot: string): string;
  getConfigPath(repoRoot: string): string;
  buildGraph(repoRoot: string): string;
  buildOrUpdateGraph(repoRoot: string, fullRebuild?: boolean): string;
  graphStatus(repoRoot: string): GraphStatusResult;
  queryGraph(repoRoot: string, pattern: string, target: string, limit: number): GraphQueryRow[];
  graphImpactRadius(repoRoot: string, changedFiles: string[], maxDepth: number): string[];
  detectChanges(repoRoot: string, base: string): ChangeRiskRow[];
  minimalContext(repoRoot: string, base: string): MinimalContextResult;
  getGraphPath(repoRoot: string): string;
  runPostprocess(repoRoot: string): number[];
  listFlows(repoRoot: string, limit: number): FlowRow[];
  getFlow(
    repoRoot: string,
    flowId: number | undefined,
    flowName: string | undefined,
  ): FlowDetail | undefined;
  getAffectedFlows(repoRoot: string, changedFiles: string[], limit: number): FlowRow[];
  listCommunities(repoRoot: string, limit: number): CommunityRow[];
  getCommunity(
    repoRoot: string,
    communityId: number | undefined,
    communityName: string | undefined,
    includeMembers?: boolean,
  ): CommunityDetail | undefined;
  architectureOverview(repoRoot: string): ArchitectureOverviewResult;
  getReviewContext(
    repoRoot: string,
    changedFiles: string[] | undefined,
    maxDepth?: number,
    includeSource?: boolean,
    maxLinesPerFile?: number,
    base?: string,
  ): ReviewContextResult;
  findLargeFunctions(
    repoRoot: string,
    minLines: number,
    kind: string | undefined,
    filePathPattern: string | undefined,
    limit: number,
  ): LargeSymbolRow[];
  embedGraph(repoRoot: string, model: string): EmbedResult;
  semanticSearch(
    repoRoot: string,
    query: string,
    kind: string | undefined,
    limit: number,
    model: string,
  ): SemanticRow[];
  refactorPreview(
    repoRoot: string,
    symbol: string,
    newName: string,
    limit: number,
  ): RefactorPreviewResult;
  applyRefactor(repoRoot: string, symbol: string, newName: string, maxFiles: number): number;
  generateWiki(repoRoot: string, force: boolean): WikiResult;
  getWikiPage(repoRoot: string, pageName: string): string;
  registerRepo(repoRoot: string, alias: string | undefined): RepoEntry;
  listRepos(): RepoEntry[];
  unregisterRepo(alias: string): void;
  crossRepoSearch(
    query: string,
    kind: string | undefined,
    limit: number,
    model: string,
    aliases: string[],
  ): CrossRepoSearchResult[];
  crossRepoImpact(changedFiles: string[], maxDepth: number, aliases: string[]): string[][];
}

const native: NativeBinding = loadNativeBinding();

export const refresh = (repoRoot: string): RefreshResult => native.refresh(repoRoot);
export const status = (repoRoot: string): StatusResult => native.status(repoRoot);
export const formatPrompt = (repoRoot: string, maxChars: number): string =>
  native.formatPrompt(repoRoot, maxChars);
export const detectPreset = (repoRoot: string): string => native.detectPreset(repoRoot);
export const getCachePath = (repoRoot: string): string => native.getCachePath(repoRoot);
export const getConfigPath = (repoRoot: string): string => native.getConfigPath(repoRoot);
export const buildGraph = (repoRoot: string): string => native.buildGraph(repoRoot);
export const buildOrUpdateGraph = (repoRoot: string, fullRebuild?: boolean): string =>
  native.buildOrUpdateGraph(repoRoot, fullRebuild);
export const graphStatus = (repoRoot: string): GraphStatusResult => native.graphStatus(repoRoot);
export const queryGraph = (
  repoRoot: string,
  pattern: string,
  target: string,
  limit: number,
): GraphQueryRow[] => native.queryGraph(repoRoot, pattern, target, limit);
export const graphImpactRadius = (
  repoRoot: string,
  changedFiles: string[],
  maxDepth: number,
): string[] => native.graphImpactRadius(repoRoot, changedFiles, maxDepth);
export const detectChanges = (repoRoot: string, base: string): ChangeRiskRow[] =>
  native.detectChanges(repoRoot, base);
export const minimalContext = (repoRoot: string, base: string): MinimalContextResult =>
  native.minimalContext(repoRoot, base);
export const getGraphPath = (repoRoot: string): string => native.getGraphPath(repoRoot);
export const runPostprocess = (repoRoot: string): number[] => native.runPostprocess(repoRoot);
export const listFlows = (repoRoot: string, limit: number): FlowRow[] =>
  native.listFlows(repoRoot, limit);
export const getFlow = (
  repoRoot: string,
  flowId?: number,
  flowName?: string,
): FlowDetail | undefined => native.getFlow(repoRoot, flowId, flowName);
export const getAffectedFlows = (
  repoRoot: string,
  changedFiles: string[],
  limit: number,
): FlowRow[] => native.getAffectedFlows(repoRoot, changedFiles, limit);
export const listCommunities = (repoRoot: string, limit: number): CommunityRow[] =>
  native.listCommunities(repoRoot, limit);
export const getCommunity = (
  repoRoot: string,
  communityId?: number,
  communityName?: string,
  includeMembers = false,
): CommunityDetail | undefined =>
  native.getCommunity(repoRoot, communityId, communityName, includeMembers);
export const architectureOverview = (repoRoot: string): ArchitectureOverviewResult =>
  native.architectureOverview(repoRoot);
export const getReviewContext = (
  repoRoot: string,
  changedFiles: string[] | undefined,
  maxDepth = 2,
  includeSource = true,
  maxLinesPerFile = 200,
  base = 'HEAD~1',
): ReviewContextResult =>
  native.getReviewContext(repoRoot, changedFiles, maxDepth, includeSource, maxLinesPerFile, base);
export const findLargeFunctions = (
  repoRoot: string,
  minLines: number,
  kind: string | undefined,
  filePathPattern: string | undefined,
  limit: number,
): LargeSymbolRow[] => native.findLargeFunctions(repoRoot, minLines, kind, filePathPattern, limit);
export const embedGraph = (repoRoot: string, model: string): EmbedResult =>
  native.embedGraph(repoRoot, model);
export const semanticSearch = (
  repoRoot: string,
  query: string,
  kind: string | undefined,
  limit: number,
  model: string,
): SemanticRow[] => native.semanticSearch(repoRoot, query, kind, limit, model);
export const refactorPreview = (
  repoRoot: string,
  symbol: string,
  newName: string,
  limit: number,
): RefactorPreviewResult => native.refactorPreview(repoRoot, symbol, newName, limit);
export const applyRefactor = (
  repoRoot: string,
  symbol: string,
  newName: string,
  maxFiles: number,
): number => native.applyRefactor(repoRoot, symbol, newName, maxFiles);
export const generateWiki = (repoRoot: string, force: boolean): WikiResult =>
  native.generateWiki(repoRoot, force);
export const getWikiPage = (repoRoot: string, pageName: string): string =>
  native.getWikiPage(repoRoot, pageName);

// Stage 5.1: Multi-repo registry
export const registerRepo = (repoRoot: string, alias: string | undefined): RepoEntry =>
  native.registerRepo(repoRoot, alias);
export const listRepos = (): RepoEntry[] => native.listRepos();
export const unregisterRepo = (alias: string): void => native.unregisterRepo(alias);
export const crossRepoSearch = (
  query: string,
  kind: string | undefined,
  limit: number,
  model: string,
  aliases: string[],
): CrossRepoSearchResult[] => native.crossRepoSearch(query, kind, limit, model, aliases);
export const crossRepoImpact = (
  changedFiles: string[],
  maxDepth: number,
  aliases: string[],
): string[][] => native.crossRepoImpact(changedFiles, maxDepth, aliases);
