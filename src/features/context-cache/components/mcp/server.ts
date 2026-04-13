import { MCP_PROMPTS } from '../../../../constants/core/constants';
import { MCP_TOOLS, buildToolHandlers, makeToolResult } from './tools';

export type JsonRpcRequest = {
  jsonrpc: '2.0';
  id?: string | number | null;
  method: string;
  params?: Record<string, unknown>;
};

export function writeMcpMessage(message: unknown): void {
  const body = JSON.stringify(message);
  const header = `Content-Length: ${Buffer.byteLength(body, 'utf8')}\r\n\r\n`;
  process.stdout.write(header + body);
}

export function startMcpServer(repoRoot: string): void {
  let buffer = Buffer.alloc(0);
  const toolHandlers = buildToolHandlers(repoRoot);

  const handle = (req: JsonRpcRequest): void => {
    try {
      if (req.method === 'initialize') {
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

      if (req.method === 'notifications/initialized') return;

      if (req.method === 'tools/list') {
        writeMcpMessage({ jsonrpc: '2.0', id: req.id ?? null, result: { tools: MCP_TOOLS } });
        return;
      }

      if (req.method === 'prompts/list') {
        const prompts = Object.keys(MCP_PROMPTS).map((name) => ({
          name,
          description: `context-cache prompt: ${name}`,
        }));
        writeMcpMessage({ jsonrpc: '2.0', id: req.id ?? null, result: { prompts } });
        return;
      }

      if (req.method === 'prompts/get') {
        const name = String(req.params?.name || '');
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
        const handler = toolHandlers[name];
        if (!handler) throw new Error(`Unknown tool: ${name}`);
        writeMcpMessage({ jsonrpc: '2.0', id: req.id ?? null, result: handler(args) });
        return;
      }

      writeMcpMessage({
        jsonrpc: '2.0',
        id: req.id ?? null,
        error: { code: -32601, message: `Method not found: ${req.method}` },
      });
    } catch (err) {
      writeMcpMessage({
        jsonrpc: '2.0',
        id: req.id ?? null,
        error: { code: -32000, message: err instanceof Error ? err.message : String(err) },
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
        writeMcpMessage({
          jsonrpc: '2.0',
          id: null,
          error: { code: -32700, message: 'Parse error' },
        });
      }
      if (req) handle(req);
    }
  });
}

// Re-export for consumers that only need the result helper
export { makeToolResult };
