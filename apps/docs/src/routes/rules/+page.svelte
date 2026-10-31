<svelte:head>
  <title>Declarative Custom Rules & Rule Packs Manual | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8">
  <p class="lead">
    Enforce custom architectural boundaries, companion test coverage, naming conventions, file size caps, prohibited regex tokens, and dependency restrictions in pure declarative YAML or JSON with zero arbitrary code execution.
  </p>

  <!-- 1. Rule File Organization & Inheritance Hierarchy -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 id="rule-packs" class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-sky-400">01.</span>
      <span>Rule Packs & 4-Tier Inheritance Hierarchy</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      ELCODE Forge evaluates rules deterministically across a 4-tier precedence hierarchy:
      <code class="text-amber-300 font-mono text-xs">1. Built-in Defaults &lt; 2. Extended Packs &lt; 3. Custom Local Rules (.eforge/rules/*) &lt; 4. Per-Rule Overrides (eforge.config.yaml)</code>.
      Projects without an explicit <code class="text-blue-300 font-mono">extends:</code> list automatically extend the <code class="text-emerald-300 font-mono">recommended</code> pack.
    </p>

    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead>
          <tr class="border-b border-slate-800 text-slate-400">
            <th class="py-2 pr-4">Built-in Pack Name</th>
            <th class="py-2 px-3">Engineering Focus</th>
            <th class="py-2 pl-3">Included Rules & Thresholds</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 text-slate-300">
          <tr>
            <td class="py-2.5 pr-4 font-bold text-emerald-400">recommended</td>
            <td class="py-2.5 px-3">Standard Baseline</td>
            <td class="py-2.5 pl-3 font-sans"><code class="font-mono text-slate-200">arch/no-direct-db-in-ui</code>, <code class="font-mono text-slate-200">arch/no-circular-dependencies</code>, <code class="font-mono text-slate-200">quality/require-service-tests</code>, <code class="font-mono text-slate-200">quality/max-file-lines</code> (300 lines)</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-blue-400">strict</td>
            <td class="py-2.5 px-3">Elevated Discipline</td>
            <td class="py-2.5 pl-3 font-sans">Critical architecture boundaries, 250-line file limit, strict test companions, and unpinned dependency version prevention</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-rose-400">security</td>
            <td class="py-2.5 px-3">Code Hardening</td>
            <td class="py-2.5 pl-3 font-sans">Prohibits <code class="font-mono text-slate-200">eval()</code>, <code class="font-mono text-slate-200">child_process.exec()</code>, <code class="font-mono text-slate-200">debugger</code> statements, <code class="font-mono text-slate-200">dangerouslySetInnerHTML</code>, and deprecated packages</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-purple-400">clean-architecture</td>
            <td class="py-2.5 px-3">Hexagonal / Onion Layers</td>
            <td class="py-2.5 pl-3 font-sans">Enforces 4-tier hexagonal boundaries (<code class="font-mono text-slate-200">ui-no-infra</code>, <code class="font-mono text-slate-200">domain-pure</code>, <code class="font-mono text-slate-200">infra-no-ui</code>, <code class="font-mono text-slate-200">ui-no-db</code>)</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>

  <!-- 2. Declarative Rule File Schema -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-amber-400">02.</span>
      <span>Declarative Rule File Schema (<code class="text-amber-300">.eforge/rules/*.yaml</code>)</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Initialize a starter rule file via <code class="text-amber-300 font-mono">eforge rules init</code>. All rule files are path-guarded via <code class="text-slate-200 font-mono">resolve_safe_path</code> to prevent directory traversal outside the repository root:
    </p>

    <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-200">
      <pre><code>{`version: 1
pack: team-custom-rules
description: Organization-specific architectural and supply-chain rules

rules:
  - id: arch/no-direct-db-in-ui
    name: No Direct DB Imports in UI Layer
    category: architecture          # architecture (arch) | quality | security (sec) | dependencies (deps)
    severity: critical              # critical | high | medium | low | info
    confidence: certain             # certain | high | medium | low (default: high)
    enabled: true                   # true | false (default: true)
    description: "UI components must not import database clients or ORM drivers directly."
    remediation: "Route data access through application services in src/services/."
    matcher:                        # (also accepts alias 'match:')
      type: forbidden_imports
      files:
        - "src/ui/**"
        - "src/components/**"
      forbidden:
        - "src/db/**"
        - "@prisma/client"
        - "pg"`}</code></pre>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead>
          <tr class="border-b border-slate-800 text-slate-400">
            <th class="py-2 pr-3">Field</th>
            <th class="py-2 px-3">Required</th>
            <th class="py-2 pl-3">Validation Rule</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/40 text-slate-300">
          <tr>
            <td class="py-2 pr-3 text-blue-300 font-bold">id</td>
            <td class="py-2 px-3 text-emerald-400">Yes</td>
            <td class="py-2 pl-3 font-sans">Must strictly match <code class="font-mono text-slate-200">&lt;category&gt;/&lt;slug&gt;</code> where category is <code class="font-mono text-slate-200">arch</code>, <code class="font-mono text-slate-200">quality</code>, <code class="font-mono text-slate-200">security</code>, or <code class="font-mono text-slate-200">deps</code>.</td>
          </tr>
          <tr>
            <td class="py-2 pr-3 text-blue-300 font-bold">name</td>
            <td class="py-2 px-3 text-emerald-400">Yes</td>
            <td class="py-2 pl-3 font-sans">Non-empty human-readable rule title.</td>
          </tr>
          <tr>
            <td class="py-2 pr-3 text-blue-300 font-bold">category</td>
            <td class="py-2 px-3 text-emerald-400">Yes</td>
            <td class="py-2 pl-3 font-sans"><code class="font-mono text-slate-200">architecture</code> (alias <code class="font-mono">arch</code>), <code class="font-mono text-slate-200">quality</code>, <code class="font-mono text-slate-200">security</code> (alias <code class="font-mono">sec</code>), or <code class="font-mono text-slate-200">dependencies</code> (alias <code class="font-mono">deps</code>).</td>
          </tr>
          <tr>
            <td class="py-2 pr-3 text-blue-300 font-bold">severity</td>
            <td class="py-2 px-3 text-emerald-400">Yes</td>
            <td class="py-2 pl-3 font-sans"><code class="font-mono text-rose-400">critical</code>, <code class="font-mono text-amber-400">high</code>, <code class="font-mono text-yellow-300">medium</code>, <code class="font-mono text-blue-400">low</code>, or <code class="font-mono text-slate-400">info</code>.</td>
          </tr>
          <tr>
            <td class="py-2 pr-3 text-blue-300 font-bold">confidence</td>
            <td class="py-2 px-3 text-slate-500">Optional</td>
            <td class="py-2 pl-3 font-sans"><code class="font-mono text-slate-200">certain</code>, <code class="font-mono text-slate-200">high</code> (default), <code class="font-mono text-slate-200">medium</code>, or <code class="font-mono text-slate-200">low</code>.</td>
          </tr>
          <tr>
            <td class="py-2 pr-3 text-blue-300 font-bold">description</td>
            <td class="py-2 px-3 text-emerald-400">Yes</td>
            <td class="py-2 pl-3 font-sans">Non-empty diagnostic explanation included in every emitted <code class="font-mono text-slate-200">Finding</code>.</td>
          </tr>
          <tr>
            <td class="py-2 pr-3 text-blue-300 font-bold">remediation</td>
            <td class="py-2 px-3 text-slate-500">Optional</td>
            <td class="py-2 pl-3 font-sans">Actionable remediation steps shown in terminal cards, SARIF help text, and the dashboard.</td>
          </tr>
          <tr>
            <td class="py-2 pr-3 text-blue-300 font-bold">matcher / match</td>
            <td class="py-2 px-3 text-emerald-400">Yes</td>
            <td class="py-2 pl-3 font-sans">One of the 6 deterministic matcher objects documented below.</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>

  <!-- 3. All 6 Deterministic Matchers -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-6">
    <h2 id="matchers" class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-sky-400">03.</span>
      <span>The 6 Deterministic Rule Matchers</span>
    </h2>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <!-- 1. forbidden_imports -->
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">1. forbidden_imports</h3>
          <span class="text-[10px] font-mono text-blue-400 uppercase">Layer Boundaries</span>
        </div>
        <p class="text-xs text-slate-400">Requires <code class="text-slate-200 font-mono">forbidden</code> (non-empty list of restricted module paths or package names) and optional <code class="text-slate-200 font-mono">files</code> globs.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-xs font-mono text-slate-200 overflow-x-auto"><code>{`matcher:
  type: forbidden_imports
  files: ["src/domain/**"]
  forbidden: ["src/infra/**", "src/ui/**", "axios"]`}</code></pre>
      </div>

      <!-- 2. required_companion -->
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">2. required_companion</h3>
          <span class="text-[10px] font-mono text-emerald-400 uppercase">Test Coverage</span>
        </div>
        <p class="text-xs text-slate-400">Requires <code class="text-slate-200 font-mono">companion</code> pattern. Supports <code class="text-slate-200 font-mono">{`{name}`}</code> interpolation of the source file stem.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-xs font-mono text-slate-200 overflow-x-auto"><code>{`matcher:
  type: required_companion
  files: ["src/services/**"]
  companion: "tests/{name}.test.ts"`}</code></pre>
      </div>

      <!-- 3. file_naming -->
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">3. file_naming</h3>
          <span class="text-[10px] font-mono text-purple-400 uppercase">Naming Discipline</span>
        </div>
        <p class="text-xs text-slate-400">Requires <code class="text-slate-200 font-mono">convention</code> (<code class="text-slate-200 font-mono">kebab-case</code>, <code class="text-slate-200 font-mono">snake_case</code>, <code class="text-slate-200 font-mono">camelCase</code>, <code class="text-slate-200 font-mono">PascalCase</code>, or wildcard <code class="text-slate-200 font-mono">*Controller.ts</code>).</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-xs font-mono text-slate-200 overflow-x-auto"><code>{`matcher:
  type: file_naming
  files: ["src/controllers/**"]
  convention: "*Controller.ts"`}</code></pre>
      </div>

      <!-- 4. file_size -->
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">4. file_size</h3>
          <span class="text-[10px] font-mono text-amber-400 uppercase">Cohesion & Size</span>
        </div>
        <p class="text-xs text-slate-400">Requires at least one of <code class="text-slate-200 font-mono">max_lines</code> (line count limit) or <code class="text-slate-200 font-mono">max_bytes</code> (byte size limit).</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-xs font-mono text-slate-200 overflow-x-auto"><code>{`matcher:
  type: file_size
  files: ["src/**"]
  max_lines: 300
  max_bytes: 50000`}</code></pre>
      </div>

      <!-- 5. regex_pattern -->
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">5. regex_pattern</h3>
          <span class="text-[10px] font-mono text-rose-400 uppercase">Banned Tokens</span>
        </div>
        <p class="text-xs text-slate-400">Requires <code class="text-slate-200 font-mono">pattern</code>. Scans path-guarded source files for prohibited tokens or unsafe API calls.</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-xs font-mono text-slate-200 overflow-x-auto"><code>{`matcher:
  type: regex_pattern
  files: ["**/*.ts", "**/*.js"]
  pattern: "eval\\s*\\(|debugger"`}</code></pre>
      </div>

      <!-- 6. dependency_restriction -->
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">6. dependency_restriction</h3>
          <span class="text-[10px] font-mono text-sky-400 uppercase">Supply-Chain</span>
        </div>
        <p class="text-xs text-slate-400">Requires <code class="text-slate-200 font-mono">disallowed_packages</code> (non-empty list of prohibited packages or <code class="text-slate-200 font-mono">wildcard-version</code>).</p>
        <pre class="bg-black/60 p-3 rounded border border-slate-800/80 text-xs font-mono text-slate-200 overflow-x-auto"><code>{`matcher:
  type: dependency_restriction
  disallowed_packages:
    - "left-pad"
    - "moment"
    - "wildcard-version"`}</code></pre>
      </div>
    </div>
  </div>

  <!-- 4. CLI & MCP Validation Workflow -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-amber-400">04.</span>
      <span>CLI Rules Workflow & MCP Querying</span>
    </h2>
    <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs space-y-1.5 text-slate-200 overflow-x-auto">
      <pre><code>{`# 1. Generate starter .eforge/rules/custom-rules.yaml with all 6 matchers
eforge rules init

# 2. Validate rule YAML/JSON syntax and schema without scanning (exits 2 on error)
eforge rules check
eforge rules check .eforge/rules/custom-rules.yaml

# 3. Inspect active merged rule catalog (built-in + packs + custom + overrides)
eforge rules list
eforge rules list --category architecture --enabled-only --format markdown

# 4. Run full repository scan evaluating all active rules
eforge scan --fail-on high`}</code></pre>
    </div>
  </div>

  <!-- 5. Topology & Boundary Inference -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 id="inference" class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-sky-400">05.</span>
      <span>Topology & Boundary Inference (<code class="text-sky-300">eforge rules infer</code>)</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      ELCODE Forge analyzes the polyglot import graph using graph-theoretic metrics to automatically discover your project's architectural structure, layer instability, and boundary rules:
    </p>
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs font-mono">
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3.5 space-y-1.5">
        <div class="text-emerald-400 font-bold uppercase text-[11px]">Layer Classification</div>
        <p class="text-slate-400 font-sans">Classifies modules into UI, Service, Domain, Persistence, Infrastructure, and Shared layers using path semantics and coupling metrics.</p>
      </div>
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3.5 space-y-1.5">
        <div class="text-blue-400 font-bold uppercase text-[11px]">Instability Metric (I)</div>
        <p class="text-slate-400 font-sans">Computes Martin's metric <code>I = FanOut / (FanIn + FanOut)</code> to verify that stable abstractions do not depend on unstable modules.</p>
      </div>
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3.5 space-y-1.5">
        <div class="text-purple-400 font-bold uppercase text-[11px]">Archetype Detection</div>
        <p class="text-slate-400 font-sans">Identifies Clean / Hexagonal, Layered N-Tier, or Feature-Sliced architectures and recommends tailored <code>forbidden_imports</code> rules.</p>
      </div>
    </div>
    <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs text-slate-200 overflow-x-auto">
      <pre><code>{`# Inspect inferred archetype and layer instability
eforge rules infer

# Export inferred boundary rules directly to rules pack
eforge rules infer --write

# Or initialize a new project with automatic inference
eforge init --infer`}</code></pre>
    </div>
  </div>

  <!-- 6. Surgical Auto-Fix & Transactional Rollback -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 id="auto-fix" class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-emerald-400">06.</span>
      <span>Surgical Auto-Fix & Transactional Rollback (<code class="text-emerald-300">eforge fix</code>)</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Fix violations deterministically with byte-for-byte post-write BLAKE3 verification and atomic transaction records in <code class="text-slate-200 font-mono">.eforge/tx/&lt;tx_id&gt;/</code>:
    </p>
    <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs space-y-1.5 text-slate-200 overflow-x-auto">
      <pre><code>{`# 1. Preview codemods with unified diff previews without touching disk
eforge fix --dry-run

# 2. Apply fixes atomically with pre-write file backups in .eforge/tx/
eforge fix --apply

# 3. Scope fixes strictly to a specific rule ID
eforge fix --apply --rule quality/max-file-lines

# 4. Rollback transaction cleanly if needed
eforge fix --rollback`}</code></pre>
    </div>
  </div>
</div>
