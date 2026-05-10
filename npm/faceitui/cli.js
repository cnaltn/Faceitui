#!/usr/bin/env node
const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const BIN_DIR = path.join(__dirname, 'bin');
const ext = process.platform === 'win32' ? '.exe' : '';
const binary = path.join(BIN_DIR, `faceitui${ext}`);

if (!fs.existsSync(binary)) {
  console.error('faceitui binary not found. Run: npm install -g faceitui');
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: 'inherit',
  shell: false,
});

process.exit(result.status ?? 1);
