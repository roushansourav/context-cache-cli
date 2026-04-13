const test = require('node:test');
const assert = require('node:assert/strict');
const { execSync } = require('node:child_process');

test('cli exposes new parity commands', () => {
  const help = execSync('node dist/bin/cli.js --help', { encoding: 'utf8' });
  assert.match(help, /get-flow/);
  assert.match(help, /get-community/);
  assert.match(help, /review-context/);
  assert.match(help, /find-large-functions/);
  assert.match(help, /docs-section/);
});
