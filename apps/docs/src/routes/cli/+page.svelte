<svelte:head>
  <title>CLI Reference & Commands Catalog | ELCODE Forge Documentation</title>
</svelte:head>

<script lang="ts">
  type CommandInfo = {
    name: string;
    category: "Diagnostics" | "Architecture" | "Security" | "AI" | "Governance" | "Compliance" | "Integration";
    summary: string;
    usage: string;
    flags: { flag: string; desc: string; default?: string }[];
    examples: string[];
    exitCodes: { code: string; desc: string }[];
  };

  const commands: CommandInfo[] = [
    {
      name: "eforge doctor",
      category: "Diagnostics",
      summary: "Rapid repository diagnostics, toolchain discovery, git status, and framework health checks.",
      usage: "eforge doctor [OPTIONS]",
      flags: [
        { flag: "--format <fmt>", desc: "Output format: terminal, json, markdown", default: "terminal" },
        { flag: "--repo <path>", desc: "Path to repository root", default: "." },
      ],
      examples: [
        "eforge doctor",
        "eforge doctor --format json",
        "eforge doctor --repo ../my-project",
      ],
      exitCodes: [
        { code: "0", desc: "Repository is fully initialized and healthy" },
        { code: "1", desc: "Configuration warning or missing dependency detected" },
      ],
    },
    {
      name: "eforge scan",
      category: "Diagnostics",
      summary: "Full AST parse, import graph resolution, rule evaluations, and health score computation.",
      usage: "eforge scan [OPTIONS]",
      flags: [
        { flag: "--fail-on <severity>", desc: "Exit with code 1 if findings reach: critical, high, medium, low", default: "none" },
        { flag: "--format <fmt>", desc: "Output format: terminal, json, sarif, markdown", default: "terminal" },
        { flag: "-o, --output <path>", desc: "Write output directly to file" },
        { flag: "--config <path>", desc: "Path to custom eforge.config.yaml" },
      ],
      examples: [
        "eforge scan",
        "eforge scan --fail-on high",
        "eforge scan --format sarif -o results.sarif",
      ],
      exitCodes: [
        { code: "0", desc: "No findings above threshold; health score within bounds" },
        { code: "1", desc: "Findings breached specified --fail-on severity" },
        { code: "2", desc: "Syntax or fatal runtime error" },
      ],
    },
    {
      name: "eforge review",
      category: "Diagnostics",
      summary: "Git-aware change inspector analyzing staged or unstaged working-tree modifications.",
      usage: "eforge review [OPTIONS]",
      flags: [
        { flag: "--fast", desc: "Skip full AST rebuild and use targeted diff heuristics" },
        { flag: "--base <ref>", desc: "Git base reference to diff against", default: "HEAD" },
        { flag: "--format <fmt>", desc: "terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge review --fast",
        "eforge review --base origin/main",
      ],
      exitCodes: [
        { code: "0", desc: "Clean diff with zero new introduced violations" },
        { code: "1", desc: "New violations or architecture regression introduced" },
      ],
    },
    {
      name: "eforge diff",
      category: "Diagnostics",
      summary: "Compare architectural metrics, health score, and findings between two commits or branches.",
      usage: "eforge diff --base <REF> --head <REF> [OPTIONS]",
      flags: [
        { flag: "--base <ref>", desc: "Baseline commit, tag, or branch" },
        { flag: "--head <ref>", desc: "Target commit, tag, or branch", default: "HEAD" },
        { flag: "--format <fmt>", desc: "terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge diff --base v1.0.0 --head main",
        "eforge diff --base origin/develop --head HEAD --format markdown",
      ],
      exitCodes: [
        { code: "0", desc: "Comparison completed successfully" },
      ],
    },
    {
      name: "eforge graph",
      category: "Architecture",
      summary: "Export or visualize module dependencies, cyclic graphs, and cross-boundary coupling.",
      usage: "eforge graph [OPTIONS]",
      flags: [
        { flag: "--format <fmt>", desc: "Output format: terminal, json, mermaid, dot", default: "terminal" },
        { flag: "--focus <module>", desc: "Filter graph to immediate upstream/downstream of module" },
        { flag: "--max-depth <N>", desc: "Maximum traversal depth for dependency resolution", default: "5" },
      ],
      examples: [
        "eforge graph --format mermaid",
        "eforge graph --focus src/auth --format dot -o auth-graph.dot",
      ],
      exitCodes: [
        { code: "0", desc: "Graph resolved and formatted" },
      ],
    },
    {
      name: "eforge dependencies",
      category: "Architecture",
      summary: "Inspect manifest dependencies across Cargo, npm, pip, composer, and Go modules.",
      usage: "eforge dependencies [OPTIONS]",
      flags: [
        { flag: "--tree", desc: "Render nested hierarchical dependency tree" },
        { flag: "--ecosystem <eco>", desc: "Filter ecosystem: cargo, npm, pip, composer, gomod" },
        { flag: "--format <fmt>", desc: "terminal, json", default: "terminal" },
      ],
      examples: [
        "eforge dependencies",
        "eforge dependencies --tree",
        "eforge dependencies --ecosystem npm --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Dependencies parsed successfully" },
      ],
    },
    {
      name: "eforge sbom",
      category: "Architecture",
      summary: "Generate an industry-standard Software Bill of Materials (SPDX or CycloneDX).",
      usage: "eforge sbom [OPTIONS]",
      flags: [
        { flag: "--format <fmt>", desc: "spdx-json, cyclonedx-json", default: "spdx-json" },
        { flag: "-o, --output <path>", desc: "Target output file path" },
      ],
      examples: [
        "eforge sbom -o sbom.spdx.json",
        "eforge sbom --format cyclonedx-json -o sbom.cyclonedx.json",
      ],
      exitCodes: [
        { code: "0", desc: "SBOM generated with cryptographic package hashes" },
      ],
    },
    {
      name: "eforge security",
      category: "Security",
      summary: "High-speed static secret scanning, unsafe code heuristics, and vulnerability checks.",
      usage: "eforge security [OPTIONS]",
      flags: [
        { flag: "--fail-on <severity>", desc: "critical, high, medium, low", default: "high" },
        { flag: "--format <fmt>", desc: "terminal, json, sarif", default: "terminal" },
        { flag: "-o, --output <path>", desc: "Write findings to file" },
      ],
      examples: [
        "eforge security",
        "eforge security --fail-on critical --format sarif -o security.sarif",
      ],
      exitCodes: [
        { code: "0", desc: "No security vulnerabilities or secrets found" },
        { code: "1", desc: "Security findings breached the fail-on threshold" },
      ],
    },
    {
      name: "eforge explain",
      category: "AI",
      summary: "Grounding-backed architecture and symbol explainer using local AST facts and optional AI.",
      usage: "eforge explain [OPTIONS]",
      flags: [
        { flag: "--symbol <name>", desc: "Explain a specific function, struct, or type" },
        { flag: "--module <path>", desc: "Explain a file or module's architectural role" },
        { flag: "--ai", desc: "Synthesize plain-language insights using configured AI provider" },
        { flag: "--dry-run", desc: "Calculate prompt tokens and estimated cost without calling API" },
      ],
      examples: [
        "eforge explain",
        "eforge explain --symbol AuthService",
        "eforge explain --ai",
        "eforge explain --ai --dry-run",
      ],
      exitCodes: [
        { code: "0", desc: "Explanation successfully retrieved and verified" },
      ],
    },
    {
      name: "eforge ai-review",
      category: "AI",
      summary: "Autonomous AI-powered PR reviewer grounding feedback strictly against static analysis facts.",
      usage: "eforge ai-review [OPTIONS]",
      flags: [
        { flag: "--base <ref>", desc: "Git reference to compare against", default: "origin/main" },
        { flag: "--ai", desc: "Enable AI reasoning engine" },
        { flag: "--dry-run", desc: "Inspect assembled context envelope and token budget without API call" },
      ],
      examples: [
        "eforge ai-review --dry-run",
        "eforge ai-review --ai --base origin/develop",
      ],
      exitCodes: [
        { code: "0", desc: "AI review generated with grounded claims" },
      ],
    },
    {
      name: "eforge ai status",
      category: "AI",
      summary: "Inspect configured AI provider, model, rate limits, and budget health.",
      usage: "eforge ai status",
      flags: [
        { flag: "--format <fmt>", desc: "terminal, json", default: "terminal" },
      ],
      examples: [
        "eforge ai status",
      ],
      exitCodes: [
        { code: "0", desc: "AI provider configuration valid and reachable" },
      ],
    },
    {
      name: "eforge ai usage",
      category: "AI",
      summary: "View cumulative token metrics, prompt/completion split, and remaining daily USD budget.",
      usage: "eforge ai usage [OPTIONS]",
      flags: [
        { flag: "--format <fmt>", desc: "terminal, json", default: "terminal" },
      ],
      examples: [
        "eforge ai usage",
        "eforge ai usage --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Usage metrics displayed" },
      ],
    },
    {
      name: "eforge policy check",
      category: "Governance",
      summary: "Evaluate enterprise Policy-as-Code rules, constraints, and time-bounded exception waivers.",
      usage: "eforge policy check [OPTIONS]",
      flags: [
        { flag: "--policy <path>", desc: "Path to specific policy file", default: "eforge.config.yaml" },
        { flag: "--format <fmt>", desc: "terminal, json, markdown", default: "terminal" },
      ],
      examples: [
        "eforge policy check",
        "eforge policy check --format json",
      ],
      exitCodes: [
        { code: "0", desc: "Compliant with all policy constraints" },
        { code: "1", desc: "Policy assertion failed (health score low, forbidden license, etc.)" },
      ],
    },
    {
      name: "eforge policy validate",
      category: "Governance",
      summary: "Validate policy syntax, waivers expiry, and override schemas without running a full scan.",
      usage: "eforge policy validate [OPTIONS]",
      flags: [
        { flag: "--policy <path>", desc: "Policy file to validate" },
      ],
      examples: [
        "eforge policy validate",
      ],
      exitCodes: [
        { code: "0", desc: "Policy file is syntactically valid" },
        { code: "1", desc: "Invalid policy schema or expired waiver detected" },
      ],
    },
    {
      name: "eforge rules",
      category: "Governance",
      summary: "List all active built-in and custom declarative rules with severities and categories.",
      usage: "eforge rules [OPTIONS]",
      flags: [
        { flag: "--category <cat>", desc: "Filter by architecture, security, style, performance, dependency" },
        { flag: "--format <fmt>", desc: "terminal, json", default: "terminal" },
      ],
      examples: [
        "eforge rules",
        "eforge rules --category security",
      ],
      exitCodes: [
        { code: "0", desc: "Rule catalog retrieved" },
      ],
    },
    {
      name: "eforge rules validate",
      category: "Governance",
      summary: "Validate custom YAML rule packs against the forge-rule specification schema.",
      usage: "eforge rules validate [OPTIONS]",
      flags: [
        { flag: "--rules-dir <path>", desc: "Directory containing custom YAML rule files", default: ".eforge/rules" },
      ],
      examples: [
        "eforge rules validate",
        "eforge rules validate --rules-dir my-rules/",
      ],
      exitCodes: [
        { code: "0", desc: "All rule files are valid" },
        { code: "1", desc: "Rule syntax or matcher validation error" },
      ],
    },
    {
      name: "eforge rules test",
      category: "Governance",
      summary: "Execute automated positive and negative test cases for custom rule definitions.",
      usage: "eforge rules test [OPTIONS]",
      flags: [
        { flag: "--fixtures-dir <path>", desc: "Directory with test fixtures", default: "fixtures/rules" },
      ],
      examples: [
        "eforge rules test",
      ],
      exitCodes: [
        { code: "0", desc: "All rule test assertions passed" },
        { code: "1", desc: "Rule failed test assertion" },
      ],
    },
    {
      name: "eforge hook install",
      category: "Governance",
      summary: "Install zero-latency git pre-commit or pre-push hooks to enforce gates locally.",
      usage: "eforge hook install [OPTIONS]",
      flags: [
        { flag: "--type <type>", desc: "pre-commit or pre-push", default: "pre-commit" },
      ],
      examples: [
        "eforge hook install",
        "eforge hook install --type pre-push",
      ],
      exitCodes: [
        { code: "0", desc: "Hook script written to .git/hooks/" },
      ],
    },
    {
      name: "eforge hook run",
      category: "Governance",
      summary: "Directly trigger hook verification on staged files without committing.",
      usage: "eforge hook run [OPTIONS]",
      flags: [
        { flag: "--type <type>", desc: "pre-commit or pre-push", default: "pre-commit" },
      ],
      examples: [
        "eforge hook run",
      ],
      exitCodes: [
        { code: "0", desc: "Hook verification passed" },
        { code: "1", desc: "Hook verification failed" },
      ],
    },
    {
      name: "eforge audit",
      category: "Compliance",
      summary: "Execute deterministic audits mapped directly to SOC 2, ISO 27001, or OWASP Top 10.",
      usage: "eforge audit --standard <soc2|iso27001|owasp> [OPTIONS]",
      flags: [
        { flag: "--standard <std>", desc: "Standard to audit: soc2, iso27001, owasp" },
        { flag: "--format <fmt>", desc: "terminal, markdown, json", default: "terminal" },
        { flag: "-o, --output <path>", desc: "Export report file path" },
      ],
      examples: [
        "eforge audit --standard soc2",
        "eforge audit --standard iso27001 --format markdown -o iso27001.md",
        "eforge audit --standard owasp --format json",
      ],
      exitCodes: [
        { code: "0", desc: "All mandatory standard clauses passed" },
        { code: "4", desc: "Audit non-compliance: one or more controls failed" },
      ],
    },
    {
      name: "eforge report",
      category: "Integration",
      summary: "Generate unified single-page HTML, Markdown, or JSON diagnostic audit reports.",
      usage: "eforge report [OPTIONS]",
      flags: [
        { flag: "--format <fmt>", desc: "html, markdown, json", default: "html" },
        { flag: "-o, --output <path>", desc: "Output report file destination", default: "forge-report.html" },
      ],
      examples: [
        "eforge report",
        "eforge report --format markdown -o audit-summary.md",
      ],
      exitCodes: [
        { code: "0", desc: "Report bundle rendered successfully" },
      ],
    },
    {
      name: "eforge schema",
      category: "Integration",
      summary: "Export official JSON Schemas for report bundles, SARIF converters, and configuration files.",
      usage: "eforge schema [OPTIONS]",
      flags: [
        { flag: "-o, --output <path>", desc: "Path to write schema file" },
        { flag: "--type <type>", desc: "bundle, config, rule, policy", default: "bundle" },
      ],
      examples: [
        "eforge schema -o report.schema.json",
        "eforge schema --type policy",
      ],
      exitCodes: [
        { code: "0", desc: "Schema output successfully" },
      ],
    },
    {
      name: "eforge mcp",
      category: "Integration",
      summary: "Launch native Model Context Protocol (MCP) server over stdio for Cursor, Claude, Windsurf, Cline.",
      usage: "eforge mcp",
      flags: [],
      examples: [
        "eforge mcp",
      ],
      exitCodes: [
        { code: "0", desc: "Server exited cleanly upon client disconnect" },
      ],
    },
    {
      name: "eforge login",
      category: "Integration",
      summary: "Authenticate local CLI with a self-hosted or managed Forge Server.",
      usage: "eforge login --server <URL> --token <TOKEN>",
      flags: [
        { flag: "--server <url>", desc: "URL of forge-server (e.g., https://forge.corp.internal)" },
        { flag: "--token <token>", desc: "Personal or CI API access token" },
      ],
      examples: [
        "eforge login --server https://forge.internal --token ef_live_...",
      ],
      exitCodes: [
        { code: "0", desc: "Credentials verified and saved to .eforge/credentials" },
      ],
    },
    {
      name: "eforge sync push",
      category: "Integration",
      summary: "Push repository diagnostic baseline, SBOM, and health metrics to Forge Server.",
      usage: "eforge sync push [OPTIONS]",
      flags: [
        { flag: "--tag <label>", desc: "Release or pipeline tag label" },
      ],
      examples: [
        "eforge sync push",
        "eforge sync push --tag v1.0.0",
      ],
      exitCodes: [
        { code: "0", desc: "Baseline accepted and indexed into server materialized rollups" },
      ],
    },
    {
      name: "eforge sync pull",
      category: "Integration",
      summary: "Download organization-wide policy rules and waivers from Forge Server.",
      usage: "eforge sync pull",
      flags: [],
      examples: [
        "eforge sync pull",
      ],
      exitCodes: [
        { code: "0", desc: "Policies updated locally in .eforge/remote-policy.yaml" },
      ],
    },
  ];

  let searchQuery = $state("");
  let selectedCategory = $state<string>("All");

  const categories = ["All", "Diagnostics", "Architecture", "Security", "AI", "Governance", "Compliance", "Integration"];

  const filteredCommands = $derived(
    commands.filter((cmd) => {
      const matchesCat = selectedCategory === "All" || cmd.category === selectedCategory;
      const q = searchQuery.toLowerCase();
      const matchesQuery =
        !q ||
        cmd.name.toLowerCase().includes(q) ||
        cmd.summary.toLowerCase().includes(q) ||
        cmd.usage.toLowerCase().includes(q);
      return matchesCat && matchesQuery;
    })
  );
</script>

<div class="space-y-8 max-w-5xl">
  <!-- Header -->
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-blue-400 mb-1">Command-Line Interface</div>
    <h1 class="text-3xl font-extrabold text-white">CLI Reference & Command Catalog</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Complete reference for all <code class="text-blue-300 font-mono text-sm">eforge</code> CLI commands, arguments, exit codes, and CI automation flags.
    </p>
  </div>

  <!-- Search & Filter Controls -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-4 flex flex-col md:flex-row gap-4 items-center justify-between">
    <div class="relative w-full md:w-80">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search commands, flags, or keywords..."
        class="w-full bg-black/60 border border-slate-800 rounded-lg px-3 py-2 text-xs font-mono text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500 transition-colors"
      />
    </div>

    <div class="flex flex-wrap gap-1.5 w-full md:w-auto">
      {#each categories as cat}
        <button
          onclick={() => (selectedCategory = cat)}
          class="px-2.5 py-1 rounded-md text-xs font-mono transition-colors {selectedCategory === cat ? 'bg-blue-600 text-white font-semibold' : 'bg-slate-900 border border-slate-800 text-slate-400 hover:text-slate-200'}"
        >
          {cat}
        </button>
      {/each}
    </div>
  </div>

  <!-- Command Cards -->
  <div class="space-y-6">
    {#if filteredCommands.length === 0}
      <div class="text-center py-12 text-slate-500 font-mono text-sm">
        No commands match "{searchQuery}" in category "{selectedCategory}".
      </div>
    {/if}

    {#each filteredCommands as cmd}
      <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4 hover:border-slate-700 transition-colors">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-slate-800/80 pb-3">
          <div class="flex items-center gap-3">
            <h2 class="text-lg font-bold font-mono text-white">{cmd.name}</h2>
            <span class="text-[10px] font-mono uppercase px-2 py-0.5 rounded bg-blue-950/80 text-blue-400 border border-blue-800/40">
              {cmd.category}
            </span>
          </div>
          <code class="text-xs font-mono text-slate-400 bg-black/40 px-2 py-1 rounded border border-slate-800 self-start sm:self-auto">
            {cmd.usage}
          </code>
        </div>

        <p class="text-sm text-slate-300 leading-relaxed">{cmd.summary}</p>

        <!-- Flags Table -->
        {#if cmd.flags.length > 0}
          <div class="space-y-1.5">
            <div class="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400">Flags & Options</div>
            <div class="overflow-x-auto bg-black/30 border border-slate-800/60 rounded-lg p-2">
              <table class="w-full text-left text-xs font-mono">
                <thead>
                  <tr class="text-slate-500 border-b border-slate-800/60 pb-1">
                    <th class="py-1 px-2 font-medium">Option</th>
                    <th class="py-1 px-2 font-medium">Description</th>
                    <th class="py-1 px-2 font-medium">Default</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-800/40">
                  {#each cmd.flags as flag}
                    <tr>
                      <td class="py-1.5 px-2 text-blue-300 whitespace-nowrap">{flag.flag}</td>
                      <td class="py-1.5 px-2 text-slate-300">{flag.desc}</td>
                      <td class="py-1.5 px-2 text-slate-500">{flag.default ?? "—"}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/if}

        <!-- Examples -->
        <div class="space-y-1.5">
          <div class="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400">Practical Examples</div>
          <div class="bg-black/50 border border-slate-800 rounded-lg p-3 font-mono text-xs space-y-1 text-slate-300">
            {#each cmd.examples as ex}
              <div class="flex items-center gap-2">
                <span class="text-blue-500 select-none">$</span>
                <span class="text-slate-100">{ex}</span>
              </div>
            {/each}
          </div>
        </div>

        <!-- Exit Codes -->
        {#if cmd.exitCodes.length > 0}
          <div class="space-y-1.5">
            <div class="text-xs font-mono font-semibold uppercase tracking-wider text-slate-400">Exit Codes Contract</div>
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs font-mono">
              {#each cmd.exitCodes as ec}
                <div class="flex items-start gap-2 bg-slate-900/40 border border-slate-800/60 rounded px-2.5 py-1.5">
                  <span class="font-bold {ec.code === '0' ? 'text-emerald-400' : ec.code === '4' ? 'text-purple-400' : 'text-amber-400'}">
                    [{ec.code}]
                  </span>
                  <span class="text-slate-300">{ec.desc}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>
