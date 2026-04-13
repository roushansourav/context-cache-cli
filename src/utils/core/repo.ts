import { execSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { basename, join } from 'node:path';
import { getCachePath } from '../../index';

export function getRepoRoot(): string {
  try {
    const root = execSync('git rev-parse --show-toplevel', {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
    if (!root) throw new Error('empty');
    return root;
  } catch {
    throw new Error('Not inside a git repository.');
  }
}

export function getChangedFiles(repoRoot: string, base: string): string[] {
  try {
    const out = execSync(`git diff --name-only ${base}`, {
      cwd: repoRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
    if (!out) return [];
    return out
      .split('\n')
      .map((s) => s.replace(/\\/g, '/').trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

export function getDefaultPromptPath(repoRoot: string): string {
  const cache = getCachePath(repoRoot);
  const repoHash = basename(cache, '.json');
  const promptDir = join(homedir(), '.context-cache-store', 'prompts');
  mkdirSync(promptDir, { recursive: true });
  return join(promptDir, `${repoHash}.txt`);
}
