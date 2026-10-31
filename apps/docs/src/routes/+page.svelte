<svelte:head>
  <title>Getting started — ELCODE Forge Documentation</title>
</svelte:head>

<script lang="ts">
  import { base } from "$app/paths";
  import QuickLinks from "$lib/components/QuickLinks.svelte";
  import QuickLink from "$lib/components/QuickLink.svelte";
  import Callout from "$lib/components/Callout.svelte";

  const dimensions = [
    { name: "Architecture", key: "architecture", weight: "18%", desc: "Module boundary integrity, DAG acyclicity (Tarjan SCC), and layer separation." },
    { name: "Security", key: "security", weight: "18%", desc: "Secret leaks, Shannon entropy tokens, OSV dependency CVEs, and unsafe code patterns." },
    { name: "Maintainability", key: "maintainability", weight: "16%", desc: "File size thresholds, module cohesion, naming conventions, and code complexity." },
    { name: "Reliability", key: "reliability", weight: "14%", desc: "Error handling discipline, lockfile determinism, and structural resilience." },
    { name: "Testing", key: "testing", weight: "12%", desc: "Test suite presence, service test companions, and tested diff coverage." },
    { name: "Dependency Health", key: "dependency_health", weight: "10%", desc: "Unused dependencies, wildcard/unpinned versions, duplicates, and license compliance." },
    { name: "Documentation", key: "documentation", weight: "6%", desc: "README presence, living onboarding docs (docs/forge/), and architectural clarity." },
    { name: "Release Readiness", key: "release_readiness", weight: "6%", desc: "CI/CD configuration, policy gate readiness, and zero critical blockers." },
  ];

  const ecosystems = [
    {
      lang: "TypeScript / JavaScript",
      exts: ".ts, .tsx, .js, .jsx, .mjs, .cjs, .svelte, .vue",
      manifests: "package.json, package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lockb",
      frameworks: "Next.js, Nuxt, SvelteKit, React, Vue, Svelte, Express, NestJS",
      resolution: "Relative (./, ../), index files, @/ & ~/ path aliases, Vue/Svelte SFC <script> extraction",
    },
    {
      lang: "Python",
      exts: ".py, .pyi",
      manifests: "pyproject.toml (PEP 621 & Poetry), requirements.txt, poetry.lock, uv.lock, Pipfile.lock",
      frameworks: "FastAPI, Flask, Django, pytest",
      resolution: "Absolute & intra-package relative imports (., .., ...), 200+ Python stdlib filter",
    },
    {
      lang: "Go",
      exts: ".go",
      manifests: "go.mod, go.sum",
      frameworks: "Gin, Echo, Fiber, Chi, Go testing",
      resolution: "Go module path resolution, internal package imports, 40+ Go stdlib root package filter",
    },
    {
      lang: "Rust",
      exts: ".rs",
      manifests: "Cargo.toml (single crate & multi-member workspaces), Cargo.lock",
      frameworks: "Axum, Actix-web, Tokio, Rocket, Cargo test",
      resolution: "Workspace crate resolution, mod declarations, crate::, super::, self::, nested use trees",
    },
    {
      lang: "PHP",
      exts: ".php",
      manifests: "composer.json, composer.lock",
      frameworks: "Laravel, Symfony, WordPress, PHPUnit, Pest",
      resolution: "PSR-4 namespace mappings (App\\ -> app/), single/aliased/group use statements, PHP stdlib filter",
    },
  ];
</script>

<p class="lead">
  <strong>ELCODE Forge (<code>eforge</code> v1.1.0)</strong> is a local-first, deterministic codebase intelligence layer, tactile terminal instrument (<code>eforge ui</code> &amp; <code>eforge watch</code>), active remediation engine (<code>eforge fix</code> &amp; <code>eforge simulate</code>), polyglot architecture analyzer, Policy-as-Code gate, and native Model Context Protocol (MCP) server built in Rust by <a href="https://elcode.id" target="_blank" rel="noreferrer">ELCODE</a>.
</p>

<QuickLinks>
  <QuickLink
    title="Installation & Quickstart"
    icon="installation"
    href="#installation"
    description="Step-by-step guides to installing the eforge binary via npx, POSIX shell, PowerShell, Homebrew, or Cargo."
  />
  <QuickLink
    title="Configuration & AI Providers"
    icon="presets"
    href="{base}/configuration"
    description="Configure eforge.config.yaml, rulesets, ignore globs, and local/cloud AI providers with strict credential isolation."
  />
  <QuickLink
    title="CLI & Custom Rules Engine"
    icon="plugins"
    href="{base}/cli"
    description="Explore all 41 CLI commands & subcommands, interactive TUI, surgical auto-fix, and 6 deterministic rule matchers."
  />
  <QuickLink
    title="MCP Server & Governance"
    icon="theming"
    href="{base}/mcp"
    description="Connect Claude Code, Cursor, and Windsurf to 11 eforge_* MCP tools with closed-loop pre-flight patch verification."
  />
</QuickLinks>

<Callout title="Why eforge instead of forge?" type="note">
  To prevent <code>PATH</code> collisions with Foundry (<code>forge</code>), Laravel Forge, and Atlassian Forge, the primary binary command is <code>eforge</code> with secondary alias <code>elcode-forge</code>. The default configuration file is <code>eforge.config.yaml</code> (with automatic fallback to <code>forge.config.yaml</code>).
</Callout>

<hr />

<h2 id="manifesto">The ELCODE Studio Manifesto</h2>

<p>
  Every subsystem in <code>eforge</code> is engineered under the five non-negotiable principles of <a href="https://elcode.id" target="_blank" rel="noreferrer">ELCODE Studio</a>:
</p>

<div class="not-prose not-toc my-6 grid grid-cols-1 sm:grid-cols-5 gap-3">
  <div class="rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-800/40 p-4 space-y-1.5">
    <span class="text-xs font-mono font-bold text-sky-500">01</span>
    <p class="font-display text-sm font-medium text-slate-900 dark:text-white">"Software should feel like an instrument, not a compromise."</p>
  </div>
  <div class="rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-800/40 p-4 space-y-1.5">
    <span class="text-xs font-mono font-bold text-sky-500">02</span>
    <p class="font-display text-sm font-medium text-slate-900 dark:text-white">"Code quality is visible in product behavior, speed, and durability."</p>
  </div>
  <div class="rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-800/40 p-4 space-y-1.5">
    <span class="text-xs font-mono font-bold text-sky-500">03</span>
    <p class="font-display text-sm font-medium text-slate-900 dark:text-white">"We use AI as leverage, not as identity. Deterministic before generative."</p>
  </div>
  <div class="rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-800/40 p-4 space-y-1.5">
    <span class="text-xs font-mono font-bold text-sky-500">04</span>
    <p class="font-display text-sm font-medium text-slate-900 dark:text-white">"We prefer precision over decoration."</p>
  </div>
  <div class="rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-800/40 p-4 space-y-1.5">
    <span class="text-xs font-mono font-bold text-sky-500">05</span>
    <p class="font-display text-sm font-medium text-slate-900 dark:text-white">"We build things worth keeping."</p>
  </div>
</div>

<hr />

<h2 id="installation">Installation Guide</h2>

<p>
  Install the single-binary <code>eforge</code> CLI across Linux, macOS, and Windows using your preferred package manager or zero-install runner.
</p>

<h3 id="install-npx">1. Zero-Install via Node / npx / pnpm / bun</h3>

<pre><code># Run directly without global installation
npx elcode-forge doctor --codeframe
pnpm dlx elcode-forge scan --codeframe
bunx elcode-forge review --fast --impacted-tests

# Or install globally via npm/pnpm
npm install -g elcode-forge</code></pre>

<h3 id="install-shell">2. Linux &amp; macOS Shell Installer (POSIX sh)</h3>

<pre><code># Installs eforge and elcode-forge to ~/.local/bin with SHA-256 verification
curl -fsSL https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.sh | sh

# Pin a specific release version or custom directory:
EFORGE_VERSION=v1.1.0 EFORGE_INSTALL_DIR=/usr/local/bin \
  curl -fsSL https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.sh | sh</code></pre>

<h3 id="install-windows">3. Windows PowerShell Installer</h3>

<pre><code># Installs eforge.exe to %LOCALAPPDATA%\elcode-forge\bin and updates User PATH
irm https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.ps1 | iex</code></pre>

<h3 id="install-brew-cargo">4. Homebrew (macOS / Linux) &amp; Cargo</h3>

<pre><code># Homebrew tap
brew tap elcode-id/tap
brew install elcode-id/tap/eforge

# Cargo binstall (pre-compiled binary) or Cargo install from source
cargo binstall eforge
cargo install --git https://github.com/elcode-id/elcode-forge forge-cli --locked</code></pre>

<h3 id="install-source">5. Build from Source (Rust 1.85+ Edition 2024)</h3>

<pre><code>git clone https://github.com/elcode-id/elcode-forge.git
cd elcode-forge
cargo build --release --locked --bin eforge
./target/release/eforge --version</code></pre>

<hr />

<h2 id="quickstart">Golden Path Quickstart (Under 60 Seconds)</h2>

<p>
  Follow these steps in any Git repository to initialize configuration with automatic boundary inference, explore the interactive terminal UI, run surgical auto-fixes, and audit your working diff.
</p>

<h3 id="step-1-init">Step 1 — Initialize Configuration &amp; Infer Architecture Boundaries</h3>

<pre><code>eforge init --infer
# Creates eforge.config.yaml, initializes .eforge/, and infers custom layer boundary rules</code></pre>

<h3 id="step-2-doctor">Step 2 — Run 5-Second Repository Diagnosis with Rich Code-Frames</h3>

<pre><code>eforge doctor --codeframe
# Outputs Health Score (0-100), 8 dimension scores, and multi-line box-drawn diagnostic spans</code></pre>

<h3 id="step-3-ui-watch">Step 3 — Explore in Interactive TUI or Live Architectural Radar</h3>

<pre><code>eforge ui
# Launches the 4-tab ratatui TUI (DAG Explorer, Tarjan SCC Cycles, Blast Radius, Code-Frames)

eforge watch
# Starts the &lt;50ms incremental file-save architectural radar backed by .eforge/cache/</code></pre>

<h3 id="step-4-scan-fix">Step 4 — Full Scan, Surgical Auto-Fix &amp; Refactor Simulation</h3>

<pre><code>eforge scan --codeframe --fail-on high
eforge fix --dry-run
eforge simulate --move src/db/client.ts:src/infra/db/client.ts
# Evaluates rules, previews transactional codemods, and simulates module moves in memory</code></pre>

<h3 id="step-5-review">Step 5 — Review Uncommitted Changes &amp; Smart Impacted Tests</h3>

<pre><code>eforge review --base origin/main --impacted-tests --fail-on high
# Computes transitive blast radius, risk score, and exact test files/commands to execute</code></pre>

<h3 id="step-6-mcp">Step 6 — Start the Native Closed-Loop MCP Server for AI Agents</h3>

<pre><code>eforge mcp
# Starts JSON-RPC 2.0 stdio server exposing 11 eforge_* tools to Claude Code, Cursor, or Windsurf</code></pre>

<Callout title="Deterministic Execution & Incremental AST Caching" type="warning">
  Identical repository state + identical <code>eforge.config.yaml</code> is guaranteed to produce byte-for-byte identical JSON/SARIF output and identical Health/Risk scores. Incremental file-level <code>blake3</code> AST entries are persisted in <code>.eforge/cache/file-ast-v1.json</code> for sub-50ms watch cycles.
</Callout>

<hr />

<h2 id="ecosystems">Multi-Language &amp; Ecosystem Support Matrix</h2>

<p>
  <code>forge-analyzer</code> provides native AST/import graph extraction, manifest parsing, lockfile dependency resolution, and framework detection across 5 major ecosystems:
</p>

<div class="not-prose my-6 overflow-x-auto rounded-2xl border border-slate-200 dark:border-slate-800">
  <table class="w-full text-left text-xs">
    <thead class="bg-slate-50 dark:bg-slate-800/80 text-slate-600 dark:text-slate-300 uppercase font-mono border-b border-slate-200 dark:border-slate-800">
      <tr>
        <th class="py-3.5 px-4">Language</th>
        <th class="py-3.5 px-4">Extensions</th>
        <th class="py-3.5 px-4">Manifests &amp; Lockfiles</th>
        <th class="py-3.5 px-4">Frameworks Detected</th>
        <th class="py-3.5 px-4">Import Graph Resolution</th>
      </tr>
    </thead>
    <tbody class="divide-y divide-slate-200 dark:divide-slate-800 bg-white dark:bg-slate-900/40 text-slate-700 dark:text-slate-300">
      {#each ecosystems as eco}
        <tr>
          <td class="py-3 px-4 font-semibold text-slate-900 dark:text-white whitespace-nowrap">{eco.lang}</td>
          <td class="py-3 px-4 font-mono text-sky-600 dark:text-sky-400">{eco.exts}</td>
          <td class="py-3 px-4 font-mono text-slate-600 dark:text-slate-300">{eco.manifests}</td>
          <td class="py-3 px-4">{eco.frameworks}</td>
          <td class="py-3 px-4 text-slate-500 dark:text-slate-400">{eco.resolution}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<hr />

<h2 id="health-score">8-Dimension Health Score &amp; Deterministic Risk Model</h2>

<p>
  Every <code>eforge doctor</code> and <code>eforge scan</code> run computes a deterministic <strong>Overall Health Score (0–100)</strong> and <strong>Letter Grade (<code>A+</code> through <code>F</code>)</strong> across 8 weighted engineering dimensions:
</p>

<div class="not-prose not-toc my-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
  {#each dimensions as dim}
    <div class="rounded-2xl border border-slate-200 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-800/40 p-5 flex flex-col justify-between gap-2">
      <div class="flex items-center justify-between">
        <span class="font-display font-medium text-slate-900 dark:text-white">{dim.name}</span>
        <span class="text-xs font-mono px-2.5 py-0.5 rounded-full bg-sky-500/10 text-sky-600 dark:text-sky-400 ring-1 ring-inset ring-sky-500/20">Weight: {dim.weight}</span>
      </div>
      <p class="text-xs text-slate-600 dark:text-slate-400 leading-relaxed">{dim.desc}</p>
      <code class="text-[11px] font-mono text-slate-500">dimension.{dim.key}</code>
    </div>
  {/each}
</div>
