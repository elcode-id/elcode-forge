<svelte:head>
  <title>Overview, Installation & Quickstart | ELCODE Forge Documentation</title>
</svelte:head>

<script lang="ts">
  import { base } from "$app/paths";

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

<div class="space-y-12">
  <!-- Hero Section -->
  <div class="relative overflow-hidden rounded-2xl bg-gradient-to-br from-slate-900 via-[#12151f] to-slate-900 border border-[#262c3f] p-8 sm:p-12">
    <div class="max-w-3xl space-y-4">
      <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-blue-500/10 border border-blue-500/20 text-blue-400 text-xs font-mono">
        <span>Production Ready</span>
        <span>•</span>
        <span>Binary: eforge & elcode-forge</span>
        <span>•</span>
        <span>v1.0.0 GA</span>
      </div>
      <h1 class="text-3xl sm:text-5xl font-extrabold tracking-tight text-white">
        Know Your Code. Ship with Confidence.
      </h1>
      <p class="text-base sm:text-lg text-slate-300 leading-relaxed">
        <strong>ELCODE Forge (<code class="text-blue-400 font-mono">eforge</code>)</strong> is a local-first, deterministic codebase intelligence layer, polyglot architecture analyzer, security & supply-chain scanner, Policy-as-Code gate, and native Model Context Protocol (MCP) server built in Rust by <a href="https://elcode.id" target="_blank" rel="noreferrer" class="text-blue-400 underline hover:text-blue-300">ELCODE</a>.
      </p>
      <div class="flex flex-wrap gap-3 pt-4">
        <a
          href="#installation"
          class="px-5 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-medium text-sm transition-all shadow-lg shadow-blue-600/20"
        >
          Install eforge
        </a>
        <a
          href="{base}/configuration"
          class="px-5 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-medium text-sm transition-all shadow-lg shadow-indigo-600/20"
        >
          Configuration Guide
        </a>
        <a
          href="{base}/cli"
          class="px-5 py-2.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 font-medium text-sm border border-slate-700 transition-all"
        >
          CLI Reference (34 Commands)
        </a>
        <a
          href="{base}/mcp"
          class="px-5 py-2.5 rounded-xl bg-emerald-600/20 hover:bg-emerald-600/30 text-emerald-300 font-medium text-sm border border-emerald-500/30 transition-all"
        >
          Native MCP Server
        </a>
      </div>
    </div>
  </div>

  <!-- Studio Manifesto Pillars -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
      <div>
        <div class="text-xs font-mono uppercase tracking-wider text-blue-400">Engineering Philosophy</div>
        <h2 class="text-xl font-bold text-white">The ELCODE Studio Manifesto (<code class="text-sm font-mono text-slate-400">01–05</code>)</h2>
      </div>
      <span class="text-xs font-mono text-slate-400">unsafe_code = "forbid" • Zero-Panic Guarantee</span>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-5 gap-3 text-xs">
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3.5 space-y-1.5">
        <div class="text-blue-400 font-mono font-bold">01 • Instrument</div>
        <p class="text-slate-300 font-medium">"Software should feel like an instrument, not a compromise."</p>
        <p class="text-slate-500">Collision-free <code class="text-slate-300">eforge</code> binary, deterministic exit codes (<code class="text-slate-300">0..=4</code>), and single-file <code class="text-slate-300">eforge.config.yaml</code>.</p>
      </div>
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3.5 space-y-1.5">
        <div class="text-emerald-400 font-mono font-bold">02 • Speed & Durability</div>
        <p class="text-slate-300 font-medium">"Code quality is visible in behavior, speed, and durability."</p>
        <p class="text-slate-500">Zero <code class="text-slate-300">.unwrap()</code> in libraries, <code class="text-slate-300">&lt; 5s</code> doctor diagnostics, and BLAKE3 incremental caching.</p>
      </div>
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3.5 space-y-1.5">
        <div class="text-amber-400 font-mono font-bold">03 • AI as Leverage</div>
        <p class="text-slate-300 font-medium">"We use AI as leverage, not as identity."</p>
        <p class="text-slate-500">Deterministic IR before generative synthesis, <code class="text-slate-300">UntrustedEnvelope</code> isolation, and offline Ollama/LM Studio support.</p>
      </div>
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3.5 space-y-1.5">
        <div class="text-purple-400 font-mono font-bold">04 • Precision</div>
        <p class="text-slate-300 font-medium">"We prefer precision over decoration."</p>
        <p class="text-slate-500">Byte-for-byte identical JSON, SARIF v2.1.0, CycloneDX/SPDX SBOMs, and explainable 8-dimension deductions.</p>
      </div>
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3.5 space-y-1.5">
        <div class="text-sky-400 font-mono font-bold">05 • Worth Keeping</div>
        <p class="text-slate-300 font-medium">"We build things worth keeping."</p>
        <p class="text-slate-500">Offline-capable Web Dashboard, append-only commit history (<code class="text-slate-300">.eforge/history/</code>), and living docs generator.</p>
      </div>
    </div>
  </div>

  <!-- Complete Installation Section -->
  <div id="installation" class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 sm:p-8 space-y-6 scroll-mt-20">
    <div>
      <div class="text-xs font-mono uppercase tracking-wider text-emerald-400 mb-1">Step 1 • Distribution Channels</div>
      <h2 class="text-2xl font-extrabold text-white">Installation Guide</h2>
      <p class="text-sm text-slate-400 mt-1">
        Install <code class="text-blue-300 font-mono">eforge</code> (and its studio alias <code class="text-blue-300 font-mono">elcode-forge</code>) on Linux, macOS, Windows, or run it with zero installation via <code class="text-emerald-300 font-mono">npx</code>. Every installer verifies cryptographic <code class="text-slate-300 font-mono">SHA256SUMS.txt</code> digests before unpacking.
      </p>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- 1. Zero-Install npx -->
      <div class="bg-black/40 border border-slate-800 rounded-xl p-5 space-y-3">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">1. Zero-Install via Node.js (npx / npm)</h3>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-emerald-950/80 text-emerald-400 border border-emerald-800/40">Fastest Tryout</span>
        </div>
        <p class="text-xs text-slate-400 leading-relaxed">
          Downloads the matching pre-compiled native binary for your OS/CPU (<code class="text-slate-300">win32-x64</code>, <code class="text-slate-300">linux-x64</code>, <code class="text-slate-300">darwin-arm64</code>), verifies its SHA-256 checksum, and caches it in <code class="text-slate-300">~/.eforge/bin/v&lt;version&gt;/</code>:
        </p>
        <div class="bg-black/60 border border-slate-800/80 rounded-lg p-3.5 font-mono text-xs text-slate-200 overflow-x-auto">
          <pre><code>{`# Run directly without global install
npx elcode-forge doctor
npx elcode-forge scan --fail-on high

# Or install globally via npm / pnpm
npm install -g elcode-forge
eforge --version`}</code></pre>
        </div>
      </div>

      <!-- 2. Standalone OS Installers -->
      <div class="bg-black/40 border border-slate-800 rounded-xl p-5 space-y-3">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">2. Standalone OS Installers (POSIX & Windows)</h3>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-blue-950/80 text-blue-400 border border-blue-800/40">Recommended</span>
        </div>
        <p class="text-xs text-slate-400 leading-relaxed">
          Installs both <code class="text-slate-300">eforge</code> and <code class="text-slate-300">elcode-forge</code> to <code class="text-slate-300">~/.local/bin</code> (Linux/macOS) or <code class="text-slate-300">%LOCALAPPDATA%\eforge\bin</code> (Windows, auto-registered in User <code class="text-slate-300">PATH</code>):
        </p>
        <div class="bg-black/60 border border-slate-800/80 rounded-lg p-3.5 font-mono text-xs text-slate-200 overflow-x-auto">
          <pre><code>{`# Linux & macOS (POSIX Shell)
curl -fsSL https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.ps1 | iex`}</code></pre>
        </div>
      </div>

      <!-- 3. Homebrew & Cargo Binstall -->
      <div class="bg-black/40 border border-slate-800 rounded-xl p-5 space-y-3">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">3. Homebrew Tap & Cargo Binstall</h3>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-purple-950/80 text-purple-400 border border-purple-800/40">Package Managers</span>
        </div>
        <p class="text-xs text-slate-400 leading-relaxed">
          Install pre-compiled release binaries via our official Homebrew tap (macOS Apple Silicon & Linux x86_64) or Rust's <code class="text-slate-300">cargo-binstall</code>:
        </p>
        <div class="bg-black/60 border border-slate-800/80 rounded-lg p-3.5 font-mono text-xs text-slate-200 overflow-x-auto">
          <pre><code>{`# Homebrew (macOS & Linux)
brew install elcode-id/tap/eforge

# Cargo Binstall (Pre-compiled binary without compiling)
cargo binstall eforge`}</code></pre>
        </div>
      </div>

      <!-- 4. Build from Source -->
      <div class="bg-black/40 border border-slate-800 rounded-xl p-5 space-y-3">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">4. Build from Source (Rust Toolchain 1.75+)</h3>
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-amber-950/80 text-amber-400 border border-amber-800/40">Source Build</span>
        </div>
        <p class="text-xs text-slate-400 leading-relaxed">
          Compile directly from the workspace with <code class="text-slate-300">--locked</code> dependency guarantees:
        </p>
        <div class="bg-black/60 border border-slate-800/80 rounded-lg p-3.5 font-mono text-xs text-slate-200 overflow-x-auto">
          <pre><code>{`git clone https://github.com/elcode-id/elcode-forge.git
cd elcode-forge
cargo install --path crates/forge-cli --locked

# Verify binary installation
eforge --version`}</code></pre>
        </div>
      </div>
    </div>
  </div>

  <!-- Golden Path Quickstart -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 sm:p-8 space-y-6">
    <div>
      <div class="text-xs font-mono uppercase tracking-wider text-blue-400 mb-1">Step 2 • Onboarding Workflow</div>
      <h2 class="text-2xl font-extrabold text-white">The Golden Path Quickstart</h2>
      <p class="text-sm text-slate-400 mt-1">
        Follow these 6 steps to initialize ELCODE Forge in any repository, diagnose project hygiene, enforce architectural rules, and launch the Web Intelligence Dashboard.
      </p>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 font-mono text-xs">
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-blue-400 font-bold">1. Initialize Workspace & Config</div>
        <p class="text-slate-400 font-sans text-xs">Creates <code class="text-slate-200 font-mono">eforge.config.yaml</code> and initializes the <code class="text-slate-200 font-mono">.eforge/</code> BLAKE3 incremental cache directory without overwriting existing configs.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-slate-200 overflow-x-auto"><code>{`eforge init --path .
eforge rules init`}</code></pre>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-emerald-400 font-bold">2. Run Fast Repository Diagnostics (&lt; 5s)</div>
        <p class="text-slate-400 font-sans text-xs">Checks file inventory, detected languages/frameworks, manifest & lockfile hygiene, test suite presence, secret leaks, and active rules.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-slate-200 overflow-x-auto"><code>{`eforge doctor --path .
eforge doctor --format json`}</code></pre>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-amber-400 font-bold">3. Execute Full or Incremental Scan</div>
        <p class="text-slate-400 font-sans text-xs">Builds the polyglot AST import graph, detects Tarjan SCC cycles, evaluates declarative rules, audits OSV CVEs, and computes the 8-dimension Health Score.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-slate-200 overflow-x-auto"><code>{`eforge scan --path . --incremental
eforge scan --fail-on high --format sarif -o eforge.sarif`}</code></pre>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-purple-400 font-bold">4. Review Diff Blast Radius & Install Git Hooks</div>
        <p class="text-slate-400 font-sans text-xs">Computes direct & transitive module impact for working-tree changes, flags untested modified files, and installs sub-200ms pre-commit hooks.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-slate-200 overflow-x-auto"><code>{`eforge review --fast
eforge diff --format markdown
eforge hook install`}</code></pre>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-sky-400 font-bold">5. Generate Living Docs & Explain Modules</div>
        <p class="text-slate-400 font-sans text-xs">Generates 5 deterministic Markdown files in <code class="text-slate-200 font-mono">docs/forge/</code> and explains module dependencies with optional grounded AI synthesis.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-slate-200 overflow-x-auto"><code>{`eforge onboard --path .
eforge explain src/services --ai --dry-run`}</code></pre>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-rose-400 font-bold">6. Launch Local Web Intelligence Dashboard</div>
        <p class="text-slate-400 font-sans text-xs">Export a deterministic <code class="text-slate-200 font-mono">ReportBundle</code> JSON or start the read-only loopback server (<code class="text-slate-200 font-mono">127.0.0.1:7878</code>) with commit history trends.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-slate-200 overflow-x-auto"><code>{`eforge report --save-history -o report.json
eforge serve --port 7878`}</code></pre>
      </div>
    </div>
  </div>

  <!-- Multi-Language & Ecosystem Support Matrix -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 sm:p-8 space-y-4">
    <div>
      <div class="text-xs font-mono uppercase tracking-wider text-purple-400 mb-1">Polyglot Static Analysis Engine</div>
      <h2 class="text-2xl font-extrabold text-white">Multi-Language & Ecosystem Support Matrix</h2>
      <p class="text-sm text-slate-400 mt-1">
        ELCODE Forge analyzes monorepos and polyglot services in a single pass, isolating module namespaces by relative directory paths and detecting circular dependencies via Tarjan's Strongly Connected Components (SCC) algorithm.
      </p>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead>
          <tr class="border-b border-slate-800 text-slate-400">
            <th class="py-2.5 pr-4">Language / Ecosystem</th>
            <th class="py-2.5 px-3">File Extensions</th>
            <th class="py-2.5 px-3">Manifests & Lockfiles</th>
            <th class="py-2.5 px-3">Framework Detectors</th>
            <th class="py-2.5 pl-3">Import & Architecture Resolution</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 text-slate-300">
          {#each ecosystems as eco}
            <tr>
              <td class="py-3 pr-4 font-bold text-white whitespace-nowrap">{eco.lang}</td>
              <td class="py-3 px-3 text-blue-300">{eco.exts}</td>
              <td class="py-3 px-3 text-emerald-300">{eco.manifests}</td>
              <td class="py-3 px-3 text-amber-300">{eco.frameworks}</td>
              <td class="py-3 pl-3 text-slate-400 font-sans">{eco.resolution}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>

  <!-- 8-Dimension Health Score & Risk Engine -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 sm:p-8 space-y-6">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
      <div>
        <div class="text-xs font-mono uppercase tracking-wider text-amber-400 mb-1">Mathematical Scoring Engine</div>
        <h2 class="text-2xl font-extrabold text-white">Explainable 8-Dimension Health Score</h2>
        <p class="text-sm text-slate-400 mt-1">
          Every scan computes a deterministic <code class="text-slate-200 font-mono">0..=100</code> score across 8 weighted engineering dimensions stored in a canonical <code class="text-slate-200 font-mono">BTreeMap</code> with explicit penalty reasons.
        </p>
      </div>
      <div class="text-xs font-mono bg-black/50 border border-slate-800 px-3 py-2 rounded-lg text-emerald-400 shrink-0">
        Total Weight = 100% (1.00)
      </div>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
      {#each dimensions as dim}
        <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-sm font-bold text-white font-mono">{dim.name}</span>
            <span class="text-xs font-mono px-2 py-0.5 rounded bg-blue-950/80 text-blue-400 border border-blue-800/40">{dim.weight}</span>
          </div>
          <div class="text-[11px] font-mono text-slate-500">Key: {dim.key}</div>
          <p class="text-xs text-slate-400 leading-relaxed">{dim.desc}</p>
        </div>
      {/each}
    </div>
  </div>

  <!-- Deep-Dive Documentation Directory -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
    <a href="{base}/configuration" class="bg-[#12151f] border border-[#262c3f] hover:border-blue-500/50 rounded-xl p-6 space-y-2 transition-all group">
      <div class="text-xs font-mono text-blue-400 uppercase">eforge.config.yaml</div>
      <h3 class="text-lg font-bold text-white group-hover:text-blue-400 transition-colors">Configuration & CI Action →</h3>
      <p class="text-xs text-slate-400 leading-relaxed">
        Complete schema reference for project settings, rule packs, AI providers (Ollama/OpenAI/Claude/Gemini), entropy tuning, license policies, Git hooks, and GitHub Actions.
      </p>
    </a>

    <a href="{base}/cli" class="bg-[#12151f] border border-[#262c3f] hover:border-emerald-500/50 rounded-xl p-6 space-y-2 transition-all group">
      <div class="text-xs font-mono text-emerald-400 uppercase">34 Commands & Subcommands</div>
      <h3 class="text-lg font-bold text-white group-hover:text-emerald-400 transition-colors">Complete CLI Reference →</h3>
      <p class="text-xs text-slate-400 leading-relaxed">
        Interactive catalog of every CLI command, flag, default value, output format (terminal, json, markdown, sarif, github), and exit code contract (0..=4).
      </p>
    </a>

    <a href="{base}/rules" class="bg-[#12151f] border border-[#262c3f] hover:border-amber-500/50 rounded-xl p-6 space-y-2 transition-all group">
      <div class="text-xs font-mono text-amber-400 uppercase">6 Matchers & 4 Packs</div>
      <h3 class="text-lg font-bold text-white group-hover:text-amber-400 transition-colors">Declarative Custom Rules →</h3>
      <p class="text-xs text-slate-400 leading-relaxed">
        Author YAML/JSON rules using forbidden_imports, required_companion, file_naming, file_size, regex_pattern, and dependency_restriction matchers.
      </p>
    </a>
  </div>
</div>
