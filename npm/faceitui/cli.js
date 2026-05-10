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

const installedMarker = path.join(BIN_DIR, '.installed');

if (fs.existsSync(installedMarker)) {
  fs.unlinkSync(installedMarker);

  const C = {
    reset: '\x1b[0m',
    bold:  '\x1b[1m',
    dim:   '\x1b[2m',
    cyan:  '\x1b[36m',
  };
  function c(code, text) { return code + text + C.reset; }

  const banner = [
    '    ███████╗ █████╗  ██████╗███████╗██╗████████╗██╗   ██╗██╗',
    '    ██╔════╝██╔══██╗██╔════╝██╔════╝██║╚══██╔══╝██║   ██║██║',
    '    █████╗  ███████║██║     █████╗  ██║   ██║   ██║   ██║██║',
    '    ██╔══╝  ██╔══██║██║     ██╔══╝  ██║   ██║   ██║   ██║██║',
    '    ██║     ██║  ██║╚██████╗███████╗██║   ██║   ╚██████╔╝██║',
    '    ╚═╝     ╚═╝  ╚═╝ ╚═════╝╚══════╝╚═╝   ╚═╝    ╚═════╝ ╚═╝',
  ];

  console.error('');
  for (const line of banner) {
    let colored = '';
    for (const ch of line) {
      colored += (ch === ' ') ? ' ' : c(C.cyan, ch);
    }
    console.error(colored);
  }
  console.error('');
  console.error(c(C.bold, '     FACEIT CS2 Stats TUI  —  ' + c(C.dim, 'ready')));
  console.error('');
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: 'inherit',
  shell: false,
});

process.exit(result.status ?? 1);
