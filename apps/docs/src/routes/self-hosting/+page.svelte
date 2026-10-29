<svelte:head>
  <title>Self-Hosting & Multi-Repo Aggregator | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8 max-w-4xl">
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-amber-400 mb-1">Architecture & Deployment</div>
    <h1 class="text-3xl font-extrabold text-white">Multi-Repo Aggregator & Self-Hosting</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Deploy <code class="text-amber-300 font-mono text-sm">forge-server</code> on your own infrastructure to aggregate organization health rollups, manage cross-repo vulnerability matrices, and enforce RBAC.
    </p>
  </div>

  <!-- Architecture Overview -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">1. Forge Server Architecture</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      <code class="text-blue-300 font-mono">forge-server</code> is a lightweight, zero-panic Rust service built on Axum and SQLx with support for PostgreSQL and SQLite. It provides fast $O(1)$ portfolio rollups through pre-aggregated materialized views.
    </p>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-slate-400 font-bold uppercase text-[11px]">Enterprise RBAC Roles</div>
        <div class="text-slate-300">• <strong class="text-white">Viewer:</strong> Read-only access to project health</div>
        <div class="text-slate-300">• <strong class="text-white">Auditor:</strong> Read access to full audit streams</div>
        <div class="text-slate-300">• <strong class="text-white">Maintainer:</strong> Push baselines & link projects</div>
        <div class="text-slate-300">• <strong class="text-white">Admin:</strong> Manage orgs, webhooks, & policies</div>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-slate-400 font-bold uppercase text-[11px]">Authentication Modes</div>
        <div class="text-slate-300">• <strong class="text-white">Bearer API Tokens:</strong> Simple CI service tokens</div>
        <div class="text-slate-300">• <strong class="text-white">JWKS OIDC:</strong> Public key verification</div>
        <div class="text-slate-300">• <strong class="text-white">Reverse Proxy:</strong> Identity headers (<code class="text-slate-400">X-Forwarded-*</code>)</div>
      </div>
    </div>
  </div>

  <!-- Docker Run Example -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">2. Quickstart with Docker</h2>
    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
      <pre><code>docker run -d \
  --name forge-server \
  -p 8080:8080 \
  -e FORGE_DATABASE_URL=postgres://user:pass@postgres:5432/forge \
  -e FORGE_API_KEY=your-secret-api-key \
  -e FORGE_TRUST_PROXY_HEADERS=true \
  elcode/forge-server:v1.0.0</code></pre>
    </div>
  </div>
</div>
