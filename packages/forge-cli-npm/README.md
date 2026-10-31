# elcode-forge

Zero-install npm wrapper, binary runner, and Node.js programmatic API for **ELCODE Forge (`eforge`)**.

Executes 100% of ELCODE Forge features—all **41 CLI commands and subcommands**, interactive TUI, live architectural watch radar, surgical auto-fix, in-memory refactor simulation, and **11 Model Context Protocol (MCP) tools**—with zero native Rust toolchain setup required.

```bash
# Instant zero-install execution via npx (all 41 commands supported)
npx elcode-forge doctor
npx elcode-forge scan --codeframe
npx elcode-forge ui
npx elcode-forge watch
npx elcode-forge fix --dry-run
npx elcode-forge simulate --move src/db/client.ts:src/infra/db/client.ts
npx elcode-forge rules infer
npx elcode-forge mcp

# Also fully compatible with pnpm dlx and bunx
pnpm dlx elcode-forge review --fast --impacted-tests
bunx elcode-forge audit --standard soc2
```

---

## 41 Commands & Subcommands Matrix via `npx`

Every single `eforge <command>` maps 1:1 to `npx elcode-forge <command>`:

| Category | Example Zero-Install Commands | Description |
| :--- | :--- | :--- |
| **Diagnostics** | `npx elcode-forge doctor`<br>`npx elcode-forge scan --incremental`<br>`npx elcode-forge trend --commits 20`<br>`npx elcode-forge blame-arch` | Evaluates 8-dimension health score, runs incremental BLAKE3 scans, inspects temporal health drift, and attributes architectural violations. |
| **Tactile Instrument** | `npx elcode-forge ui`<br>`npx elcode-forge watch --debounce-ms 150` | Launches keyboard-driven `ratatui` TUI (DAG, Cycles, Blast Radius, Code-Frames) or starts `< 50ms` live file-save radar. |
| **Active Remediation** | `npx elcode-forge fix --dry-run`<br>`npx elcode-forge fix --apply`<br>`npx elcode-forge fix --rollback`<br>`npx elcode-forge simulate --move <from:to>` | Generates deterministic codemods, performs atomic transaction backups in `.eforge/tx/`, and simulates module moves in memory. |
| **Architecture & Rules** | `npx elcode-forge rules infer --write`<br>`npx elcode-forge rules list`<br>`npx elcode-forge rules check`<br>`npx elcode-forge graph --format markdown` | Infers architecture archetype and layer boundary rules from import graph, validates rules, and exports Mermaid diagrams. |
| **Change Blast Radius** | `npx elcode-forge review --fast --impacted-tests`<br>`npx elcode-forge diff --remote` | Traverses reverse import BFS to compute change risk and selects exact test files impacted by a diff. |
| **Security & Compliance** | `npx elcode-forge security --audit`<br>`npx elcode-forge sbom --spec cyclonedx`<br>`npx elcode-forge policy check`<br>`npx elcode-forge audit --standard soc2` | Redacts secrets, audits polyglot OSV CVEs, generates CycloneDX/SPDX SBOMs, and validates SOC 2 / OWASP / ISO 27001 clauses. |
| **AI & MCP Agent Gate** | `npx elcode-forge mcp`<br>`npx elcode-forge explain src/services --ai`<br>`npx elcode-forge ai-review --fast` | Starts stdio JSON-RPC 2.0 server with 11 `eforge_*` tools (including pre-flight patch verification) and runs grounded AI reviews. |
| **Git Hooks & Sync** | `npx elcode-forge hook install`<br>`npx elcode-forge push`<br>`npx elcode-forge pull`<br>`npx elcode-forge sync` | Installs sub-200ms pre-commit hooks and synchronizes analysis baselines with remote `forge-server`. |

---

## Programmatic Node.js / TypeScript API

You can also import `elcode-forge` directly into your JavaScript or TypeScript build scripts and test pipelines:

```javascript
const { execute, getBinary, runCli } = require('elcode-forge');

// 1. Programmatic execution with structured result
async function runHealthCheck() {
  const result = await execute(['doctor', '--format', 'json']);
  
  if (result.status === 0) {
    const report = JSON.parse(result.stdout);
    console.log(`Health Score: ${report.health_score.overall_score}`);
  } else {
    console.error('Doctor diagnostics failed:', result.stderr);
  }
}

// 2. Resolve verified native binary path
async function locateBinary() {
  const binaryPath = await getBinary();
  console.log(`Native eforge binary: ${binaryPath}`);
}
```

---

## Integration in `package.json`

Add `elcode-forge` to your project's `devDependencies`:

```bash
npm install -D elcode-forge
# or: pnpm add -D elcode-forge
# or: yarn add -D elcode-forge
```

Then define standard project scripts in `package.json`:

```json
{
  "scripts": {
    "doctor": "eforge doctor",
    "scan": "eforge scan --codeframe --fail-on high",
    "review": "eforge review --fast --impacted-tests",
    "fix": "eforge fix --apply",
    "watch": "eforge watch",
    "mcp": "eforge mcp",
    "precommit": "eforge hook run pre-commit"
  }
}
```

---

## How It Works

1. Automatically resolves your OS and architecture (`win32-x64`, `linux-x64`, `darwin-arm64`).
2. Downloads official native release binaries directly from GitHub Releases (`v1.1.0`+).
3. Verifies binary integrity using official cryptographic `SHA256SUMS.txt`.
4. Caches the verified native executable locally (`~/.eforge/bin/` or `%LOCALAPPDATA%\eforge\bin\`).
5. Directly executes the native Rust binary with zero wrapper latency on subsequent runs.

---

## Environment Variables

- `EFORGE_BINARY_PATH`: Override native binary path with a local build or custom binary location.
- `EFORGE_CACHE_DIR`: Override destination cache directory for downloaded binaries.
- `EFORGE_DOWNLOAD_URL`: Override release assets download base URL (useful for air-gapped or internal enterprise mirrors).

---

## License

MIT © [ELCODE](https://elcode.id/) (Bandung, Indonesia).
