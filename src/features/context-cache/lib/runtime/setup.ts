import { execSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { DEFAULT_MAX_CHARS } from '../../../../constants/core/constants';

export function setupVscodeGlobal(): void {
  const tasksPath = join(homedir(), 'Library', 'Application Support', 'Code', 'User', 'tasks.json');
  mkdirSync(dirname(tasksPath), { recursive: true });

  let current: { version: string; tasks: Array<Record<string, unknown>> } = {
    version: '2.0.0',
    tasks: [],
  };

  if (existsSync(tasksPath)) {
    try {
      const parsed = JSON.parse(readFileSync(tasksPath, 'utf8'));
      if (parsed && typeof parsed === 'object') {
        current = {
          version: ((parsed as Record<string, unknown>).version as string) ?? '2.0.0',
          tasks: Array.isArray((parsed as Record<string, unknown>).tasks)
            ? ((parsed as Record<string, unknown>).tasks as Array<Record<string, unknown>>)
            : [],
        };
      }
    } catch {
      /* keep defaults */
    }
  }

  const upsert = (task: Record<string, unknown>) => {
    const idx = current.tasks.findIndex((t) => t?.label === task.label);
    if (idx >= 0) {
      current.tasks[idx] = task;
    } else {
      current.tasks.push(task);
    }
  };

  upsert({
    label: 'Context Cache: Refresh',
    type: 'shell',
    command: 'context-cache refresh',
    problemMatcher: [],
  });
  upsert({
    label: 'Context Cache: Prompt Ready',
    type: 'shell',
    command: `context-cache ready --max-chars ${DEFAULT_MAX_CHARS}`,
    problemMatcher: [],
  });
  upsert({
    label: 'Context Cache: Watch',
    type: 'shell',
    command: 'context-cache watch',
    isBackground: true,
    problemMatcher: [],
  });

  writeFileSync(tasksPath, `${JSON.stringify(current, null, 2)}\n`, 'utf8');
  console.log(`Updated global VS Code tasks: ${tasksPath}`);
}

export function copyToClipboard(content: string): void {
  const platform = process.platform;
  if (platform === 'darwin') {
    execSync('pbcopy', { input: content });
    return;
  }
  if (platform === 'win32') {
    execSync('clip', { input: content });
    return;
  }
  execSync('xclip -selection clipboard', { input: content });
}

export function check(command: string): boolean {
  try {
    execSync(`command -v ${command}`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

export function upsertJsonServerConfig(
  filePath: string,
  serverName: string,
  command: string,
  args: string[],
  dryRun = false,
): void {
  mkdirSync(dirname(filePath), { recursive: true });
  let data: Record<string, unknown> = {};
  if (existsSync(filePath)) {
    try {
      data = JSON.parse(readFileSync(filePath, 'utf8')) as Record<string, unknown>;
    } catch {
      data = {};
    }
  }
  const mcpServers = (data.mcpServers as Record<string, unknown> | undefined) ?? {};
  mcpServers[serverName] = { command, args };
  data.mcpServers = mcpServers;
  if (dryRun) {
    console.log(`[dry-run] Would write ${filePath}`);
    console.log(JSON.stringify(data, null, 2));
    return;
  }
  writeFileSync(filePath, `${JSON.stringify(data, null, 2)}\n`, 'utf8');
  console.log(`Updated ${filePath}`);
}

export function runInstall(opts: { platform: string; dryRun?: boolean }): void {
  const target = (opts.platform ?? 'all').toLowerCase();
  const dryRun = Boolean(opts.dryRun);
  const applyFor = (name: string): boolean => target === 'all' || target === name;

  if (applyFor('claude'))
    upsertJsonServerConfig(
      join(homedir(), '.claude', 'mcp.json'),
      'context-cache',
      'context-cache',
      ['mcp-serve'],
      dryRun,
    );
  if (applyFor('codex'))
    upsertJsonServerConfig(
      join(homedir(), '.codex', 'mcp.json'),
      'context-cache',
      'context-cache',
      ['mcp-serve'],
      dryRun,
    );
  if (applyFor('cursor'))
    upsertJsonServerConfig(
      join(homedir(), '.cursor', 'mcp.json'),
      'context-cache',
      'context-cache',
      ['mcp-serve'],
      dryRun,
    );

  if (applyFor('copilot')) {
    // 1. Global VS Code MCP config (~/.vscode/mcp.json) — picked up by all workspaces
    const globalMcpPath = join(homedir(), '.vscode', 'mcp.json');
    upsertVscodeMcpConfig(globalMcpPath, dryRun);

    // 2. VS Code user tasks (Refresh / Prompt Ready / Watch)
    if (!dryRun) setupVscodeGlobal();
  }
}

/**
 * Upsert context-cache into a VS Code mcp.json file.
 * VS Code uses { "servers": { ... } } (not { "mcpServers": { ... } }).
 */
function upsertVscodeMcpConfig(filePath: string, dryRun: boolean): void {
  mkdirSync(dirname(filePath), { recursive: true });
  let data: Record<string, unknown> = {};
  if (existsSync(filePath)) {
    try {
      data = JSON.parse(readFileSync(filePath, 'utf8')) as Record<string, unknown>;
    } catch {
      data = {};
    }
  }
  const servers = (data.servers as Record<string, unknown> | undefined) ?? {};
  servers['context-cache'] = {
    type: 'stdio',
    command: 'context-cache',
    args: ['mcp-serve'],
  };
  data.servers = servers;
  if (dryRun) {
    console.log(`[dry-run] Would write ${filePath}`);
    console.log(JSON.stringify(data, null, 2));
    return;
  }
  writeFileSync(filePath, `${JSON.stringify(data, null, 2)}\n`, 'utf8');
  console.log(`Updated ${filePath}`);
}
