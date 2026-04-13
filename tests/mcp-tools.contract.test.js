const test = require('node:test');
const assert = require('node:assert/strict');

const { MCP_TOOLS } = require('../dist/features/context-cache/components/mcp/tools');

test('mcp tool parity contracts include newly added tools', () => {
  const names = new Set(MCP_TOOLS.map((t) => t.name));
  assert.ok(names.has('get_flow'));
  assert.ok(names.has('get_community'));
  assert.ok(names.has('get_review_context'));
  assert.ok(names.has('find_large_functions'));
  assert.ok(names.has('get_docs_section'));
});
