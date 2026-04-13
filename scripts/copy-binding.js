#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');

const candidates = [
  path.join(
    __dirname,
    '..',
    'src',
    'lib',
    'rust',
    'target',
    'release',
    'libcontext_cache_core.dylib',
  ),
  path.join(__dirname, '..', 'src', 'lib', 'rust', 'target', 'release', 'libcontext_cache_core.so'),
  path.join(__dirname, '..', 'src', 'lib', 'rust', 'target', 'release', 'context_cache_core.dll'),
];

const dest = path.join(__dirname, '..', 'binding.node');

let copied = false;
for (const src of candidates) {
  if (fs.existsSync(src)) {
    fs.copyFileSync(src, dest);
    console.log(`Copied ${src} -> ${dest}`);
    copied = true;
    break;
  }
}

if (!copied) {
  console.error('Could not find compiled Rust binary. Run `cargo build --release` first.');
  process.exit(1);
}
