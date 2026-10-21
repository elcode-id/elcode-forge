#!/usr/bin/env node

/**
 * @elcode-forge/cli — Executable binary wrapper for `eforge`.
 */

const { runCli } = require('../src/index.js');

runCli()
  .then((code) => {
    process.exit(code);
  })
  .catch((err) => {
    process.stderr.write(`[eforge error] ${err.message}\n`);
    process.exit(1);
  });
