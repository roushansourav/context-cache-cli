import { existsSync, readFileSync } from 'node:fs';
import { getCachePath } from '../../index';

export type CacheFile = {
  path: string;
  content?: string;
  summary?: string;
};

export type CachePayload = {
  repoRoot: string;
  updatedAt: string;
  fileCount: number;
  files: CacheFile[];
};

export function loadCachePayload(repoRoot: string): CachePayload {
  const cachePath = getCachePath(repoRoot);
  if (!existsSync(cachePath)) {
    throw new Error('Cache does not exist yet. Run `context-cache refresh` first.');
  }
  return JSON.parse(readFileSync(cachePath, 'utf8')) as CachePayload;
}

export function toPosixPath(input: string): string {
  return input.replace(/\\/g, '/');
}

export function stripQuotes(input: string): string {
  return input.replace(/^['"]|['"]$/g, '');
}

export function extensionCandidates(basePath: string): string[] {
  return [
    basePath,
    `${basePath}.ts`,
    `${basePath}.tsx`,
    `${basePath}.js`,
    `${basePath}.jsx`,
    `${basePath}.mjs`,
    `${basePath}.cjs`,
    `${basePath}.py`,
    `${basePath}/index.ts`,
    `${basePath}/index.tsx`,
    `${basePath}/index.js`,
    `${basePath}/index.jsx`,
    `${basePath}/index.mjs`,
    `${basePath}/index.cjs`,
    `${basePath}/__init__.py`,
  ];
}

export function resolveRelativeImport(
  fromFile: string,
  specifier: string,
  knownFiles: Set<string>,
): string | null {
  if (!specifier.startsWith('.')) return null;
  const fromParts = toPosixPath(fromFile).split('/');
  fromParts.pop();
  for (const part of specifier.split('/')) {
    if (!part || part === '.') continue;
    if (part === '..') {
      fromParts.pop();
    } else {
      fromParts.push(part);
    }
  }
  const base = fromParts.join('/');
  return extensionCandidates(base).find((c) => knownFiles.has(c)) ?? null;
}

export function extractImportSpecifiers(content: string): string[] {
  const specs: string[] = [];
  const patterns = [
    /import\s+(?:[^'"]+\s+from\s+)?['"]([^'"]+)['"]/g,
    /require\(\s*['"]([^'"]+)['"]\s*\)/g,
    /from\s+['"]([^'"]+)['"]/g,
  ];
  for (const pattern of patterns) {
    let m: RegExpExecArray | null = pattern.exec(content);
    while (m !== null) {
      if (m[1]) specs.push(m[1]);
      m = pattern.exec(content);
    }
  }
  return specs;
}

export function buildReverseDependencyMap(payload: CachePayload): Map<string, Set<string>> {
  const knownFiles = new Set(payload.files.map((f) => toPosixPath(f.path)));
  const reverse = new Map<string, Set<string>>();
  for (const file of payload.files) {
    const from = toPosixPath(file.path);
    for (const spec of extractImportSpecifiers(file.content ?? '')) {
      const target = resolveRelativeImport(from, spec, knownFiles);
      if (!target) continue;
      let set = reverse.get(target);
      if (!set) {
        set = new Set<string>();
        reverse.set(target, set);
      }
      set.add(from);
    }
  }
  return reverse;
}

export function bfsImpact(
  seeds: string[],
  reverse: Map<string, Set<string>>,
  maxDepth: number,
): Map<string, number> {
  const depthMap = new Map<string, number>();
  let frontier = new Set(seeds);
  for (const seed of frontier) depthMap.set(seed, 0);
  for (let depth = 1; depth <= maxDepth; depth += 1) {
    const next = new Set<string>();
    for (const current of frontier) {
      const dependents = reverse.get(current);
      if (!dependents) continue;
      for (const dep of dependents) {
        if (!depthMap.has(dep)) {
          depthMap.set(dep, depth);
          next.add(dep);
        }
      }
    }
    if (next.size === 0) break;
    frontier = next;
  }
  return depthMap;
}

export function runImpactRadius(
  repoRoot: string,
  opts: { base: string; changed?: string; depth: string; refresh?: boolean },
  refreshFn: (root: string) => void,
  getChangedFilesFn: (root: string, base: string) => string[],
): void {
  if (opts.refresh) refreshFn(repoRoot);
  const payload = loadCachePayload(repoRoot);
  const maxDepth = Number.parseInt(opts.depth, 10) || 3;
  const changed = opts.changed
    ? opts.changed
        .split(',')
        .map((p) => toPosixPath(stripQuotes(p.trim())))
        .filter(Boolean)
    : getChangedFilesFn(repoRoot, opts.base);

  if (changed.length === 0) {
    console.log('No changed files detected. Provide --changed or commit changes first.');
    return;
  }

  const reverse = buildReverseDependencyMap(payload);
  const impacted = bfsImpact(changed, reverse, Math.max(1, maxDepth));
  const rows = [...impacted.entries()].sort((a, b) => a[1] - b[1] || a[0].localeCompare(b[0]));

  console.log(`Changed files: ${changed.length}`);
  console.log(`Impacted files (including changed): ${rows.length}`);
  console.log(`Depth: ${maxDepth}\n`);

  for (const [file, depth] of rows.slice(0, 200)) {
    console.log(`[d${depth}] ${file}`);
  }
  if (rows.length > 200) console.log(`...and ${rows.length - 200} more`);
}
