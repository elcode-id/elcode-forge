<svelte:head>
  <title>Self-Hosting & Multi-Repo Aggregator (forge-server) | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8">
  <p class="lead">
    Deploy <code>forge-server</code> (<code>crates/forge-server</code>) on your own infrastructure with SQLite or PostgreSQL 16 to centralize multi-repository health rollups, cross-repo vulnerability matrices, branch trend retention, HMAC-SHA256 webhooks, and append-only RBAC audit logs.
  </p>

  <!-- 1. Architecture, RBAC & Auth Modes -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">1. Server Architecture, RBAC Roles & Authentication Modes</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Built on Rust, Axum, and SQLx with zero-panic guarantees and non-root container isolation (<code class="text-slate-200 font-mono">UID 10001 forge:forge</code>), <code class="text-sky-300 font-mono">forge-server</code> maintains $O(1)$ materialized organization rollups (<code class="text-slate-200 font-mono">organization_rollups</code>) updated automatically whenever a repository pushes a snapshot.
    </p>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-sky-400 font-bold uppercase text-[11px]">Enterprise RBAC Privilege Tiers</div>
        <div class="text-slate-300">• <strong class="text-white">Viewer:</strong> Read project health, findings, and rollups (<code class="text-slate-400">can_read_findings</code>)</div>
        <div class="text-slate-300">• <strong class="text-white">Auditor:</strong> Viewer + query append-only <code class="text-slate-200">/api/v1/audit-events</code> (<code class="text-slate-400">can_read_audit</code>)</div>
        <div class="text-slate-300">• <strong class="text-white">Maintainer:</strong> Push snapshots & link projects (<code class="text-slate-400">can_write_snapshots</code>)</div>
        <div class="text-slate-300">• <strong class="text-white">Admin:</strong> Full org, webhook, and API key management (<code class="text-slate-400">can_manage_orgs</code>)</div>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-emerald-400 font-bold uppercase text-[11px]">3 Supported Authentication Modes</div>
        <div class="text-slate-300">• <strong class="text-white">JWT / Bearer API Keys:</strong> Issued via <code class="text-slate-200">/api/v1/auth/token</code> or <code class="text-slate-200">EFORGE_ADMIN_API_KEY</code></div>
        <div class="text-slate-300">• <strong class="text-white">Enterprise JWKS OIDC:</strong> External IdP verification via <code class="text-slate-200">EFORGE_JWKS_URL</code></div>
        <div class="text-slate-300">• <strong class="text-white">Identity-Aware Reverse Proxy:</strong> <code class="text-slate-200">X-Forwarded-User</code>, <code class="text-slate-200">X-Forwarded-Role</code>, <code class="text-slate-200">X-Forwarded-Email</code> (gated by <code class="text-slate-200">EFORGE_TRUST_PROXY_HEADERS=true</code>)</div>
      </div>
    </div>
  </div>

  <!-- 2. Deployment Options (Docker, Docker Compose, Kubernetes) -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-6">
    <h2 class="text-lg font-bold text-white font-mono">2. Production Deployment Guides</h2>

    <div class="space-y-4">
      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">Option A — Standalone Docker (Embedded SQLite)</h3>
          <span class="text-xs font-mono text-slate-500">1–15 Contributors • &lt; 30 MB Alpine Image</span>
        </div>
        <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-200">
          <pre><code>{`mkdir -p /opt/eforge/data

docker run -d \\
  --name forge-server \\
  --restart unless-stopped \\
  -p 8080:8080 \\
  -v /opt/eforge/data:/var/lib/eforge \\
  -e EFORGE_SERVER_HOST="0.0.0.0" \\
  -e EFORGE_SERVER_PORT="8080" \\
  -e EFORGE_DATABASE_URL="sqlite:///var/lib/eforge/forge.db?mode=rwc" \\
  -e EFORGE_JWT_SECRET="replace-with-at-least-32-char-random-secret" \\
  -e EFORGE_ADMIN_API_KEY="ef_admin_bootstrap_secret_key" \\
  elcode/forge-server:v1.0.0

curl -sS http://localhost:8080/api/v1/health`}</code></pre>
        </div>
      </div>

      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">Option B — Production Docker Compose (PostgreSQL 16 + MinIO + Dashboard)</h3>
          <span class="text-xs font-mono text-slate-500">docker-compose.yml</span>
        </div>
        <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-200">
          <pre><code>{`cp .env.example .env
# Edit .env to configure POSTGRES_PASSWORD, EFORGE_JWT_SECRET, and EFORGE_ADMIN_API_KEY
docker compose up -d
docker compose ps
# Dashboard: http://localhost:3000 | API: http://localhost:8080 | MinIO: http://localhost:9001`}</code></pre>
        </div>
      </div>

      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-bold text-white font-mono">Option C — Cloud-Native Kubernetes Cluster (Kustomize)</h3>
          <span class="text-xs font-mono text-slate-500">deploy/k8s/</span>
        </div>
        <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-200">
          <pre><code>{`kubectl apply -k deploy/k8s/
kubectl rollout status deployment/forge-server -n forge
kubectl get pods,svc,ingress -n forge`}</code></pre>
        </div>
      </div>
    </div>
  </div>

  <!-- 3. Server Environment Variables Reference -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">3. <code class="text-sky-300">forge-server</code> Environment Variables (<code class="text-slate-300">ServerConfig::from_env</code>)</h2>
    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead>
          <tr class="border-b border-slate-800 text-slate-400">
            <th class="py-2 pr-4">Environment Variable</th>
            <th class="py-2 px-3">Default</th>
            <th class="py-2 pl-3">Description</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 text-slate-300">
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">EFORGE_SERVER_HOST</td>
            <td class="py-2 px-3 text-slate-400">127.0.0.1</td>
            <td class="py-2 pl-3 font-sans">Bind address host (use <code class="font-mono text-slate-200">0.0.0.0</code> inside Docker/Kubernetes containers).</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">EFORGE_SERVER_PORT</td>
            <td class="py-2 px-3 text-slate-400">8080</td>
            <td class="py-2 pl-3 font-sans">TCP port for the Axum HTTP API server.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">EFORGE_DATABASE_URL</td>
            <td class="py-2 px-3 text-slate-400">sqlite://forge.db?mode=rwc</td>
            <td class="py-2 pl-3 font-sans">SQLite (<code class="font-mono text-slate-200">sqlite://...</code>) or PostgreSQL 16 (<code class="font-mono text-slate-200">postgres://user:pass@host:5432/db</code>) connection URL (also reads fallback <code class="font-mono text-slate-200">DATABASE_URL</code>).</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">EFORGE_JWT_SECRET</td>
            <td class="py-2 px-3 text-slate-400">(Dev secret)</td>
            <td class="py-2 pl-3 font-sans">32+ character HMAC secret used to sign and verify issued JWT bearer tokens.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">EFORGE_ADMIN_API_KEY</td>
            <td class="py-2 px-3 text-slate-400">None</td>
            <td class="py-2 pl-3 font-sans">Optional bootstrap Admin API key provisioned automatically on server startup.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">EFORGE_JWT_EXPIRATION_SECS</td>
            <td class="py-2 px-3 text-slate-400">604800 (7d)</td>
            <td class="py-2 pl-3 font-sans">Expiration lifetime in seconds for issued JWT access tokens.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">EFORGE_TRUST_PROXY_HEADERS</td>
            <td class="py-2 px-3 text-slate-400">false</td>
            <td class="py-2 pl-3 font-sans">When <code class="font-mono text-slate-200">true</code> or <code class="font-mono text-slate-200">1</code>, accepts <code class="font-mono text-slate-200">X-Forwarded-User</code>, <code class="font-mono text-slate-200">X-Forwarded-Role</code>, and <code class="font-mono text-slate-200">X-Forwarded-Email</code> from an authenticated reverse proxy. Rejected with 401 when <code class="font-mono text-slate-200">false</code>.</td>
          </tr>
          <tr>
            <td class="py-2 pr-4 text-sky-300 font-bold">EFORGE_JWKS_URL</td>
            <td class="py-2 px-3 text-slate-400">None</td>
            <td class="py-2 pl-3 font-sans">Optional OIDC JWKS endpoint URL for validating external identity provider JWTs.</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>

  <!-- 4. Team CLI Synchronization & REST API Endpoints -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-3">
      <h2 class="text-lg font-bold text-white font-mono">4. CLI Team Sync Workflow</h2>
      <div class="bg-black/60 border border-slate-800 rounded-lg p-3.5 font-mono text-xs text-slate-200 overflow-x-auto">
        <pre><code>{`# 1. Authenticate & store credentials in ~/.eforge/credentials.yaml
eforge login --server http://localhost:8080 --token "$EFORGE_ADMIN_API_KEY"
eforge auth status

# 2. Push snapshot or pull remote baseline (.eforge/baseline.json)
eforge push --branch main
eforge pull --branch main

# 3. Compare local working tree against remote baseline
eforge diff --remote

# 4. Run full CI sync pipeline (pull -> delta -> push -> gate)
eforge sync --branch main --fail-on high`}</code></pre>
      </div>
    </div>

    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-3">
      <h2 class="text-lg font-bold text-white font-mono">5. Core REST API & Webhooks</h2>
      <ul class="text-xs text-slate-300 space-y-1.5 font-mono">
        <li>• <code class="text-emerald-400">GET /api/v1/health</code> — Liveness & DB backend probe</li>
        <li>• <code class="text-blue-400">POST /api/v1/sync/push</code> & <code class="text-emerald-400">GET /api/v1/sync/pull</code> — Content-addressed BLAKE3 snapshot sync</li>
        <li>• <code class="text-emerald-400">GET /api/v1/projects</code> & <code class="text-emerald-400">/api/v1/projects/:id/overview|branches|history</code></li>
        <li>• <code class="text-blue-400">POST /api/v1/orgs</code> & <code class="text-emerald-400">GET /api/v1/orgs/:id/summary|vulnerabilities</code></li>
        <li>• <code class="text-blue-400">POST /api/v1/webhooks</code> — Slack, Discord & HTTP webhooks signed with <code class="text-amber-300">X-Forge-Signature: sha256=&lt;hex&gt;</code></li>
        <li>• <code class="text-emerald-400">GET /api/v1/audit-events</code> — Append-only RBAC audit log stream</li>
      </ul>
    </div>
  </div>
</div>
