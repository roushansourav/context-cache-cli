import { MCP_PROMPTS } from '../../../../constants/core/constants';
import { graphStatus } from '../../../../index';
import { MCP_TOOLS, buildToolHandlers, makeToolResult } from './tools';

export type JsonRpcRequest = {
  jsonrpc: '2.0';
  id?: string | number | null;
  method: string;
  params?: Record<string, unknown>;
};

// ── ANSI colours (stderr only — stdout is the MCP wire protocol) ─────────────
const C = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  cyan: '\x1b[36m',
  magenta: '\x1b[35m',
  gray: '\x1b[90m',
};

function ts(): string {
  return `${C.gray}${new Date().toISOString()}${C.reset}`;
}

function log(icon: string, color: string, ...parts: string[]): void {
  process.stderr.write(`${ts()} ${color}${icon}${C.reset} ${parts.join(' ')}\n`);
}

const info = (...p: string[]) => log('●', C.cyan, ...p);
const ok = (...p: string[]) => log('✔', C.green, ...p);
const warn = (...p: string[]) => log('⚠', C.yellow, ...p);
const fail = (...p: string[]) => log('✖', C.red, ...p);
const trace = (...p: string[]) => log('→', C.magenta, ...p);

export function writeMcpMessage(message: unknown): void {
  const body = JSON.stringify(message);
  const header = `Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n`;
  process.stdout.write(header + body);
}

export function startMcpServer(repoRoot: string): void {
  let buffer = Buffer.alloc(0);
  const toolHandlers = buildToolHandlers(repoRoot);

  // ── Startup banner ────────────────────────────────────────────────────────
  const gs = graphStatus(repoRoot);
  process.stderr.write(
    `\n${C.bold}${C.cyan}context-cache MCP server${C.reset}\n` +
      `${C.dim}${'─'.repeat(40)}${C.reset}\n`,
  );
  info(`Repo:      ${C.bold}${repoRoot}${C.reset}`);
  info(`Graph DB:  ${gs.graphPath}`);
  if (gs.exists) {
    ok(
      `Graph:     ${C.green}${gs.nodeCount} nodes, ${gs.edgeCount} edges${C.reset}  (updated: ${gs.updatedAt ?? 'unknown'})`,
    );
  } else {
    warn(`Graph:     not found — run ${C.bold}context-cache graph-build${C.reset}`);
  }
  info(`Tools:     ${MCP_TOOLS.length} registered`);
  info('Transport: stdio (JSON-RPC 2.0)');
  process.stderr.write(`${C.dim}${'─'.repeat(40)}${C.reset}\n`);
  ok(`${C.bold}${C.green}Listening on stdin — waiting for client…${C.reset}\n`);

  const handle = (req: JsonRpcRequest): void => {
    try {
      if (req.method === 'initialize') {
        const clientName = (req.params as Record<string, unknown> | undefined)?.clientInfo;
        info(
          `Client connected${clientName ? `: ${C.bold}${JSON.stringify(clientName)}${C.reset}` : ''}`,
        );
        writeMcpMessage({
          jsonrpc: '2.0',
          id: req.id ?? null,
          result: {
            protocolVersion: '2024-11-05',
            serverInfo: { name: 'context-cache', version: '0.2.0' },
            capabilities: { tools: {}, prompts: {} },
          },
        });
        return;
      }

      if (req.method === 'notifications/initialized') {
        ok(`Handshake complete — server is ${C.bold}${C.green}ready${C.reset}`);
        return;
      }

      if (req.method === 'tools/list') {
        trace(`tools/list  (${MCP_TOOLS.length} tools)`);
        writeMcpMessage({ jsonrpc: '2.0', id: req.id ?? null, result: { tools: MCP_TOOLS } });
        return;
      }

      if (req.method === 'prompts/list') {
        const keys = Object.keys(MCP_PROMPTS);
        trace(`prompts/list  (${keys.length} prompts)`);
        const prompts = keys.map((name) => ({
          name,
          description: `context-cache prompt: ${name}`,
        }));
        writeMcpMessage({ jsonrpc: '2.0', id: req.id ?? null, result: { prompts } });
        return;
      }

      if (req.method === 'prompts/get') {
        const name = String(req.params?.name || '');
        trace(`prompts/get  name=${C.bold}${name}${C.reset}`);
        const body = MCP_PROMPTS[name];
        if (!body) throw new Error(`Unknown prompt: ${name}`);
        writeMcpMessage({
          jsonrpc: '2.0',
          id: req.id ?? null,
          result: {
            description: `Prompt template for ${name}`,
            messages: [{ role: 'user', content: { type: 'text', text: body } }],
          },
        });
        return;
      }

      if (req.method === 'tools/call') {
        const name = String(req.params?.name || '');
        const args = (req.params?.arguments as Record<string, unknown> | undefined) ?? {};
        const argSummary = Object.entries(args)
          .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
          .join(', ');
        trace(
          `tools/call   ${C.bold}${name}${C.reset}${argSummary ? `  ${C.dim}(${argSummary})${C.reset}` : ''}`,
        );
        const handler = toolHandlers[name];
        if (!handler) {
          warn(`Unknown tool: ${C.bold}${name}${C.reset}`);
          throw new Error(`Unknown tool: ${name}`);
        }
        const t0 = Date.now();
        const result = handler(args);
        ok(`tools/call   ${C.bold}${name}${C.reset}  ${C.dim}${Date.now() - t0}ms${C.reset}`);
        writeMcpMessage({ jsonrpc: '2.0', id: req.id ?? null, result });
        return;
      }

      warn(`Method not found: ${C.bold}${req.method}${C.reset}`);
      writeMcpMessage({
        jsonrpc: '2.0',
        id: req.id ?? null,
        error: { code: -32601, message: `Method not found: ${req.method}` },
      });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      fail(`Error [${req.method}]: ${msg}`);
      writeMcpMessage({
        jsonrpc: '2.0',
        id: req.id ?? null,
        error: { code: -32000, message: msg },
      });
    }
  };

  process.stdin.on('data', (chunk: Buffer) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (true) {
      const headerEnd = buffer.indexOf('\r\n\r\n');
      if (headerEnd < 0) return;
      const header = buffer.subarray(0, headerEnd).toString('utf8');
      const m = header.match(/Content-Length:\s*(\d+)/i);
      if (!m) {
        warn('Malformed header — no Content-Length, dropping frame');
        buffer = buffer.subarray(headerEnd + 4);
        continue;
      }
      const length = Number.parseInt(m[1], 10);
      const total = headerEnd + 4 + length;
      if (buffer.length < total) return;
      const body = buffer.subarray(headerEnd + 4, total).toString('utf8');
      buffer = buffer.subarray(total);
      let req: JsonRpcRequest | null = null;
      try {
        req = JSON.parse(body) as JsonRpcRequest;
      } catch {
        fail(`JSON parse error — body: ${body.slice(0, 120)}`);
        writeMcpMessage({
          jsonrpc: '2.0',
          id: null,
          error: { code: -32700, message: 'Parse error' },
        });
      }
      if (req) handle(req);
    }
  });

  process.stdin.on('end', () => {
    info('stdin closed — client disconnected, shutting down');
    process.exit(0);
  });

  process.stdin.on('error', (err) => {
    fail(`stdin error: ${err.message}`);
    process.exit(1);
  });

  process.on('SIGINT', () => {
    info('SIGINT received — shutting down');
    process.exit(0);
  });

  process.on('SIGTERM', () => {
    info('SIGTERM received — shutting down');
    process.exit(0);
  });
}

// Re-export for consumers that only need the result helper
export { makeToolResult };
