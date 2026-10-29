<svelte:head>
  <title>Declarative Custom Rules Manual | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8 max-w-4xl">
  <!-- Title -->
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-amber-400 mb-1">Architecture & Governance</div>
    <h1 class="text-3xl font-extrabold text-white">Declarative Custom Rules Manual</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Author custom architectural constraints, security boundaries, and codebase conventions using pure declarative YAML evaluated in milliseconds.
    </p>
  </div>

  <!-- Introduction -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-amber-400">01.</span>
      <span>Rule File Organization</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Custom rules can be authored directly within <code class="text-amber-300 font-mono">eforge.config.yaml</code> under the <code class="text-amber-300 font-mono">rules:</code> block, or modularized into individual files in the <code class="text-blue-300 font-mono">.eforge/rules/</code> directory:
    </p>

    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
      <pre><code>.eforge/
└── rules/
    ├── architecture-boundaries.yaml
    ├── security-forbidden-patterns.yaml
    └── naming-conventions.yaml</code></pre>
    </div>
  </div>

  <!-- Anatomy of a Rule -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-amber-400">02.</span>
      <span>Anatomy of a Rule Definition</span>
    </h2>

    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
      <pre><code>id: arch/no-direct-db-in-ui
name: Prohibit Direct Database Imports in UI Components
severity: high
category: architecture
message: "UI components must not import database clients or repositories directly. Use an application service or API layer."
remediation: "Refactor the component to call a hook or API client from '@/lib/api' instead of importing from '@/server/db'."

matcher:
  type: forbidden_imports
  files:
    - "src/components/**/*.svelte"
    - "src/components/**/*.tsx"
  forbidden:
    - "@/server/db"
    - "@prisma/client"
    - "drizzle-orm"</code></pre>
    </div>

    <!-- Fields Table -->
    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead>
          <tr class="border-b border-slate-800 text-slate-400">
            <th class="pb-2">Field</th>
            <th class="pb-2">Type</th>
            <th class="pb-2">Description</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/40 text-slate-300">
          <tr>
            <td class="py-2 text-blue-300">id</td>
            <td class="py-2 text-slate-500">string</td>
            <td class="py-2">Unique namespaced rule identifier (e.g. <code class="text-slate-200">arch/layer-boundary</code>)</td>
          </tr>
          <tr>
            <td class="py-2 text-blue-300">severity</td>
            <td class="py-2 text-slate-500">enum</td>
            <td class="py-2"><code class="text-emerald-400">critical</code>, <code class="text-red-400">high</code>, <code class="text-amber-400">medium</code>, <code class="text-blue-400">low</code>, or <code class="text-slate-400">info</code></td>
          </tr>
          <tr>
            <td class="py-2 text-blue-300">category</td>
            <td class="py-2 text-slate-500">enum</td>
            <td class="py-2"><code class="text-slate-200">architecture</code>, <code class="text-slate-200">security</code>, <code class="text-slate-200">style</code>, <code class="text-slate-200">performance</code>, or <code class="text-slate-200">dependency</code></td>
          </tr>
          <tr>
            <td class="py-2 text-blue-300">message</td>
            <td class="py-2 text-slate-500">string</td>
            <td class="py-2">Concise diagnostic summary shown when the rule matches</td>
          </tr>
          <tr>
            <td class="py-2 text-blue-300">remediation</td>
            <td class="py-2 text-slate-500">string</td>
            <td class="py-2">Explicit, actionable instructions for resolving the violation</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>

  <!-- Matcher Types Reference -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-6">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-amber-400">03.</span>
      <span>Matcher Types & Examples</span>
    </h2>

    <!-- Matcher 1: Forbidden Imports -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-white font-mono">1. forbidden_imports</h3>
        <span class="text-[10px] font-mono text-blue-400 uppercase">Architecture</span>
      </div>
      <p class="text-xs text-slate-400">
        Prevents specified glob patterns from importing restricted modules or internal packages.
      </p>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-3 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>matcher:
  type: forbidden_imports
  files: ["crates/forge-core/**"]
  forbidden: ["crates/forge-cli/**", "crates/forge-analyzer/**"]</code></pre>
      </div>
    </div>

    <!-- Matcher 2: Required Companion -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-white font-mono">2. required_companion</h3>
        <span class="text-[10px] font-mono text-emerald-400 uppercase">Quality</span>
      </div>
      <p class="text-xs text-slate-400">
        Enforces companion file pairing, such as requiring a unit test or documentation companion.
      </p>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-3 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>matcher:
  type: required_companion
  files: ["src/services/**/*.service.ts"]
  companion_pattern: "&lbrace;stem&rbrace;.service.test.ts"</code></pre>
      </div>
    </div>

    <!-- Matcher 3: Regex Pattern -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-white font-mono">3. regex_pattern</h3>
        <span class="text-[10px] font-mono text-red-400 uppercase">Security & Style</span>
      </div>
      <p class="text-xs text-slate-400">
        Scans file contents for banned regex patterns, such as unsafe string evaluations or DOM injections.
      </p>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-3 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>matcher:
  type: regex_pattern
  files: ["**/*.&lbrace;js,ts,jsx,tsx&rbrace;"]
  pattern: "eval\\s*\\(|dangerouslySetInnerHTML"</code></pre>
      </div>
    </div>

    <!-- Matcher 4: File Naming -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-white font-mono">4. file_naming</h3>
        <span class="text-[10px] font-mono text-purple-400 uppercase">Conventions</span>
      </div>
      <p class="text-xs text-slate-400">
        Enforces naming casing conventions (<code class="text-slate-300 font-mono">kebab-case</code>, <code class="text-slate-300 font-mono">PascalCase</code>, <code class="text-slate-300 font-mono">camelCase</code>).
      </p>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-3 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>matcher:
  type: file_naming
  files: ["src/components/**/*.svelte"]
  casing: "PascalCase"</code></pre>
      </div>
    </div>

    <!-- Matcher 5: File Size Limits -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-white font-mono">5. file_size</h3>
        <span class="text-[10px] font-mono text-amber-400 uppercase">Maintainability</span>
      </div>
      <p class="text-xs text-slate-400">
        Flags God-objects by capping maximum lines of code per file.
      </p>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-3 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>matcher:
  type: file_size
  files: ["**/*.rs"]
  max_lines: 800</code></pre>
      </div>
    </div>
  </div>

  <!-- Testing & Validation CLI -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-amber-400">04.</span>
      <span>Testing & Validating Rules in CI</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Verify rule pack validity before committing to ensure zero runtime or syntax surprises:
    </p>

    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs space-y-2 text-slate-300">
      <div class="text-slate-400"># Validate all YAML rule files against the official schema</div>
      <div class="text-amber-400">eforge rules validate</div>
      <div class="text-slate-400 pt-2"># Run automated positive/negative test assertions</div>
      <div class="text-amber-400">eforge rules test --fixtures-dir fixtures/rules</div>
    </div>
  </div>
</div>
