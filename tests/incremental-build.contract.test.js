const test = require('node:test');
const assert = require('node:assert/strict');

const api = require('../dist/index');

test('api exports buildOrUpdateGraph', () => {
  assert.equal(typeof api.buildOrUpdateGraph, 'function');
});
