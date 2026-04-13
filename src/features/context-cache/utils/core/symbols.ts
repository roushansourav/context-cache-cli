export type SymbolDef = {
  file: string;
  line: number;
  kind: string;
};

export type SymbolRef = {
  file: string;
  line: number;
  text: string;
};

export function escapeRegExp(input: string): string {
  return input.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export function findSymbolDefinitions(
  filePath: string,
  content: string,
  symbol: string,
): SymbolDef[] {
  const defs: SymbolDef[] = [];
  const lines = content.split('\n');
  const escaped = escapeRegExp(symbol);
  const patterns: Array<{ kind: string; re: RegExp }> = [
    { kind: 'function', re: new RegExp(`\\bfunction\\s+${escaped}\\b`) },
    { kind: 'class', re: new RegExp(`\\bclass\\s+${escaped}\\b`) },
    { kind: 'type', re: new RegExp(`\\b(type|interface|enum)\\s+${escaped}\\b`) },
    { kind: 'variable', re: new RegExp(`\\b(const|let|var)\\s+${escaped}\\b`) },
    { kind: 'python-function', re: new RegExp(`^\\s*def\\s+${escaped}\\b`) },
    { kind: 'python-class', re: new RegExp(`^\\s*class\\s+${escaped}\\b`) },
  ];
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i] ?? '';
    for (const p of patterns) {
      if (p.re.test(line)) {
        defs.push({ file: filePath, line: i + 1, kind: p.kind });
        break;
      }
    }
  }
  return defs;
}

export function findSymbolReferences(
  filePath: string,
  content: string,
  symbol: string,
): SymbolRef[] {
  const refs: SymbolRef[] = [];
  const lines = content.split('\n');
  const re = new RegExp(`\\b${escapeRegExp(symbol)}\\b`);
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i] ?? '';
    if (re.test(line)) refs.push({ file: filePath, line: i + 1, text: line.trim() });
  }
  return refs;
}

// ── Command-level helper ──────────────────────────────────────────────────────

import type { CachePayload } from '../../../../utils/core/files';

export function runQuerySymbol(
  repoRoot: string,
  symbol: string,
  opts: { refresh?: boolean; limit: string },
  payload: CachePayload,
): void {
  const limit = Number.parseInt(opts.limit, 10) || 120;
  const defs: SymbolDef[] = [];
  const refs: SymbolRef[] = [];

  for (const file of payload.files) {
    const path = file.path.replace(/\\/g, '/');
    const content = file.content ?? '';
    if (!content) continue;
    defs.push(...findSymbolDefinitions(path, content, symbol));
    refs.push(...findSymbolReferences(path, content, symbol));
  }

  const defFiles = new Set(defs.map((d) => d.file));
  const externalRefs = refs.filter((r) => !defFiles.has(r.file));
  const probableTests = externalRefs
    .filter((r) => /(^|\/)(test|tests|__tests__)\/|\.(spec|test)\./i.test(r.file))
    .map((r) => r.file);
  const uniqueTests = [...new Set(probableTests)].sort();

  console.log(`Symbol: ${symbol}`);
  console.log(`Definitions: ${defs.length}`);
  console.log(`References: ${refs.length}`);
  console.log(`References outside definition files: ${externalRefs.length}`);
  console.log(`Probable tests: ${uniqueTests.length}\n`);

  if (defs.length > 0) {
    console.log('Definitions:');
    for (const d of defs.slice(0, 40)) console.log(`  - ${d.file}:${d.line} (${d.kind})`);
    console.log('');
  }

  console.log('References:');
  for (const r of refs.slice(0, limit)) {
    console.log(`  - ${r.file}:${r.line} | ${r.text.slice(0, 160)}`);
  }
  if (refs.length > limit) console.log(`  ...and ${refs.length - limit} more`);

  if (uniqueTests.length > 0) {
    console.log('\nProbable tests:');
    for (const t of uniqueTests.slice(0, 60)) console.log(`  - ${t}`);
  }
}
