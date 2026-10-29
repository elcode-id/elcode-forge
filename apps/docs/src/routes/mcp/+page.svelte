<svelte:head>
  <title>Model Context Protocol (MCP) Server Guide | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8 max-w-4xl">
  <!-- Title -->
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-emerald-400 mb-1">AI Toolchain Integration</div>
    <h1 class="text-3xl font-extrabold text-white">Model Context Protocol (MCP) Server</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Connect Cursor, Claude Desktop, Windsurf, and VS Code Cline directly to ELCODE Forge's deterministic AST graph, blast-radius calculator, and security scanners via native <code class="text-emerald-300 font-mono text-sm">eforge mcp</code>.
    </p>
  </div>

  <!-- Architecture Overview -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-emerald-400">01.</span>
      <span>Architecture & Principles</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      The ELCODE Forge MCP server operates over standard input/output (<code class="text-blue-300 font-mono">stdio</code>) using JSON-RPC 2.0. Rather than letting AI assistants guess or perform slow grep searches, Forge exposes structured, zero-hallucination code intelligence directly into the agent's context window.
    </p>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs font-mono">
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3 space-y-1.5">
        <div class="text-emerald-400 font-bold uppercase text-[11px]">Deterministic Facts</div>
        <p class="text-slate-400">AST parsing and import graphs computed before any LLM prompt is assembled.</p>
      </div>
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3 space-y-1.5">
        <div class="text-blue-400 font-bold uppercase text-[11px]">Strict Security Boundary</div>
        <p class="text-slate-400">Enforced read-only mode with strict canonical path traversal protection.</p>
      </div>
      <div class="bg-black/40 border border-slate-800 rounded-lg p-3 space-y-1.5">
        <div class="text-purple-400 font-bold uppercase text-[11px]">Injection Containment</div>
        <p class="text-slate-400">Untrusted repository files wrapped inside isolation envelopes.</p>
      </div>
    </div>
  </div>

  <!-- Client Setup Configurations -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-6">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-emerald-400">02.</span>
      <span>IDE & Client Configuration</span>
    </h2>

    <!-- Cursor -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <div class="text-sm font-bold text-white font-mono">Cursor IDE</div>
        <div class="text-xs font-mono text-slate-500">.cursor/mcp.json</div>
      </div>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>{`{
  "mcpServers": {
    "elcode-forge": {
      "command": "eforge",
      "args": ["mcp"]
    }
  }
}`}</code></pre>
      </div>
    </div>

    <!-- Claude Desktop -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <div class="text-sm font-bold text-white font-mono">Claude Desktop</div>
        <div class="text-xs font-mono text-slate-500">claude_desktop_config.json</div>
      </div>
      <p class="text-xs text-slate-400">
        Located at <code class="text-slate-300 font-mono">~/Library/Application Support/Claude/claude_desktop_config.json</code> (macOS) or <code class="text-slate-300 font-mono">%APPDATA%\Claude\claude_desktop_config.json</code> (Windows):
      </p>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>{`{
  "mcpServers": {
    "elcode-forge": {
      "command": "eforge",
      "args": ["mcp"],
      "env": {
        "FORGE_LOG": "info"
      }
    }
  }
}`}</code></pre>
      </div>
    </div>

    <!-- Windsurf -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <div class="text-sm font-bold text-white font-mono">Windsurf IDE</div>
        <div class="text-xs font-mono text-slate-500">~/.codeium/windsurf/mcp_config.json</div>
      </div>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>{`{
  "mcpServers": {
    "eforge": {
      "command": "eforge",
      "args": ["mcp"]
    }
  }
}`}</code></pre>
      </div>
    </div>

    <!-- VS Code Cline -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <div class="text-sm font-bold text-white font-mono">VS Code (Cline Extension)</div>
        <div class="text-xs font-mono text-slate-500">cline_mcp_settings.json</div>
      </div>
      <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
        <pre><code>{`{
  "mcpServers": {
    "elcode-forge": {
      "command": "eforge",
      "args": ["mcp"],
      "disabled": false,
      "autoApprove": [
        "eforge_project_info",
        "eforge_get_architecture",
        "eforge_get_dependencies"
      ]
    }
  }
}`}</code></pre>
      </div>
    </div>
  </div>

  <!-- MCP Tools Catalog -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-emerald-400">03.</span>
      <span>Native MCP Tool Catalog</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Every tool is prefixed with <code class="text-emerald-300 font-mono">eforge_*</code> to prevent collision with other MCP servers in your workspace:
    </p>

    <div class="space-y-3 pt-2">
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-1.5">
        <div class="flex items-center justify-between">
          <code class="text-emerald-300 font-mono font-bold text-sm">eforge_project_info</code>
          <span class="text-[10px] font-mono text-slate-500 uppercase">Read-Only</span>
        </div>
        <p class="text-xs text-slate-300 leading-relaxed">
          Returns project name, root path, detected languages, frameworks, package managers, and overall health score.
        </p>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-1.5">
        <div class="flex items-center justify-between">
          <code class="text-emerald-300 font-mono font-bold text-sm">eforge_get_architecture</code>
          <span class="text-[10px] font-mono text-slate-500 uppercase">Read-Only</span>
        </div>
        <p class="text-xs text-slate-300 leading-relaxed">
          Returns modular architecture graph, module boundaries, cyclic dependencies count, and cross-boundary coupling stats.
        </p>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-1.5">
        <div class="flex items-center justify-between">
          <code class="text-emerald-300 font-mono font-bold text-sm">eforge_get_dependencies</code>
          <span class="text-[10px] font-mono text-slate-500 uppercase">Read-Only</span>
        </div>
        <p class="text-xs text-slate-300 leading-relaxed">
          Enumerate direct and transitive dependencies across Cargo, npm, pip, composer, and Go modules with licenses and versions.
        </p>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-1.5">
        <div class="flex items-center justify-between">
          <code class="text-emerald-300 font-mono font-bold text-sm">eforge_get_security_hotspots</code>
          <span class="text-[10px] font-mono text-slate-500 uppercase">Read-Only</span>
        </div>
        <p class="text-xs text-slate-300 leading-relaxed">
          Returns active security findings, secret leaks, dangerous code constructs, and CVE advisory matches with file/line evidence.
        </p>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-1.5">
        <div class="flex items-center justify-between">
          <code class="text-emerald-300 font-mono font-bold text-sm">eforge_get_change_impact</code>
          <span class="text-[10px] font-mono text-slate-500 uppercase">Read-Only</span>
        </div>
        <p class="text-xs text-slate-300 leading-relaxed">
          Given a list of changed files, computes the reverse-dependency blast radius, downstream affected modules, and risk score.
        </p>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-1.5">
        <div class="flex items-center justify-between">
          <code class="text-emerald-300 font-mono font-bold text-sm">eforge_explain_symbol</code>
          <span class="text-[10px] font-mono text-slate-500 uppercase">Read-Only</span>
        </div>
        <p class="text-xs text-slate-300 leading-relaxed">
          Retrieves deterministic AST symbol declarations, inbound callers, exported types, and associated test companion files.
        </p>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-1.5">
        <div class="flex items-center justify-between">
          <code class="text-emerald-300 font-mono font-bold text-sm">eforge_evaluate_policy</code>
          <span class="text-[10px] font-mono text-slate-500 uppercase">Read-Only</span>
        </div>
        <p class="text-xs text-slate-300 leading-relaxed">
          Evaluates working tree state against the declarative Policy-as-Code constraints in <code class="text-slate-300 font-mono">eforge.config.yaml</code> and reports compliance status.
        </p>
      </div>
    </div>
  </div>

  <!-- Security Model -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span class="text-emerald-400">04.</span>
      <span>Security Model & Prompt Isolation</span>
    </h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      In accordance with ELCODE Studio Manifesto standard <code class="text-amber-300 font-mono">03</code> (<em>"We use AI as leverage, not as identity"</em>), all repository content delivered to LLMs via MCP is treated as untrusted input.
    </p>

    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
      <pre><code>&lt;untrusted_repository_content source="src/auth/token.ts"&gt;
// File contents safely escaped and isolated
&lt;/untrusted_repository_content&gt;</code></pre>
    </div>
    <p class="text-xs text-slate-400">
      This prevents prompt injection vectors embedded in comments, pull request titles, or untrusted third-party dependencies from hijacking model instructions.
    </p>
  </div>
</div>
