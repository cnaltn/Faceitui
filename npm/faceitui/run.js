#!/usr/bin/env node
// Shim: clean exit on Ctrl+C without extra error output
process.on('SIGINT', () => process.exit(0));
require('./cli.js');
