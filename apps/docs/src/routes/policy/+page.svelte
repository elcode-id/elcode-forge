<svelte:head>
  <title>Enterprise Policy-as-Code Engine | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8 max-w-5xl">
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-blue-400 mb-1">Specification & Gate Guide</div>
    <h1 class="text-3xl font-extrabold text-white">Enterprise Policy-as-Code Engine (<code class="text-blue-300 font-mono text-2xl">eforge policy check</code>)</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Declare non-negotiable architectural boundaries, minimum 8-dimension health thresholds, secret zero-tolerance, license allow/denylists, diff blast-radius caps, and time-bounded exception waivers either inline in <code class="text-blue-300 font-mono text-sm">eforge.config.yaml</code> or in standalone YAML/JSON policy documents.
    </p>
  </div>

  <!-- 1. Policy Discovery Hierarchy & CLI Usage -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">1. Policy Discovery Hierarchy & CLI Execution</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      When <code class="text-blue-300 font-mono">eforge policy check</code> runs, it resolves the active <code class="text-slate-200 font-mono">PolicyDocument</code> in the following precedence order:
      <strong class="text-white">1. Explicit CLI flag (<code class="text-blue-300 font-mono">--policy &lt;path&gt;</code>)</strong> &rarr;
      <strong class="text-white">2. Inline <code class="text-amber-300 font-mono">policy:</code> block in <code class="text-amber-300 font-mono">eforge.config.yaml</code></strong>.
      If neither is present, it exits with <code class="text-orange-400 font-mono">ExitCode::ConfigurationError (2)</code>.
    </p>

    <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-200">
      <pre><code>{`# Evaluate inline policy: block in eforge.config.yaml
eforge policy check

# Evaluate an external YAML or JSON organization policy file
eforge policy check --policy ./policies/org-security-policy.yaml --format markdown

# Fail on Warning verdict as well when findings meet or exceed medium severity
eforge policy check --fail-on medium --format json`}</code></pre>
    </div>
  </div>

  <!-- 2. Complete Declarative Policy Schema -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">2. Complete Declarative Policy Schema</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Below is the complete schema supported by <code class="text-slate-200 font-mono">parse_policy_yaml</code> and <code class="text-slate-200 font-mono">load_policy</code> (can be placed directly under <code class="text-amber-300 font-mono">policy:</code> in <code class="text-amber-300 font-mono">eforge.config.yaml</code> or at the root of an external <code class="text-blue-300 font-mono">policy.yaml</code> / <code class="text-blue-300 font-mono">policy.json</code> file):
    </p>

    <div class="bg-black/60 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-200">
      <pre><code>{`name: enterprise-standard-gate
version: 1
organization: elcode-id

constraints:
  min_health_score: 80
  min_dimension_scores:
    architecture: 85
    security: 90
    testing: 75
    dependency_health: 80
  enforce_secret_zero_tolerance: true
  disallowed_severities:
    - critical
    - high
  forbidden_licenses:
    - GPL-3.0-only
    - AGPL-3.0-only
    - SSPL-1.0
  max_circular_dependencies: 0
  max_blast_radius_modules: 5
  max_modified_lines_without_tests: 150
  required_rule_packs:
    - recommended
    - security

waivers:
  - rule_id: security/high-entropy-secret
    path_pattern: "fixtures/**"
    expires_at: "2027-06-30T23:59:59Z"
    justification: "Synthetic test credentials used exclusively in deterministic test fixtures"

overrides:
  - rule_id: arch/no-circular-dependencies
    severity: critical
    enabled: true`}</code></pre>
    </div>
  </div>

  <!-- 3. All 9 Evaluated Constraints Reference -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">3. Evaluated Constraints Reference (<code class="text-blue-300">PolicyConstraints</code>)</h2>
    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead>
          <tr class="border-b border-slate-800 text-slate-400">
            <th class="py-2 pr-4">Constraint Field</th>
            <th class="py-2 px-3">Type</th>
            <th class="py-2 pl-3">Deterministic Gate Evaluation Rule</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 text-slate-300">
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">min_health_score</td>
            <td class="py-2.5 px-3 text-blue-400">u8 (0..=100)</td>
            <td class="py-2.5 pl-3 font-sans">Fails with a violation if the overall weighted repository <code class="font-mono text-slate-200">HealthScore.total</code> is below this threshold.</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">min_dimension_scores</td>
            <td class="py-2.5 px-3 text-blue-400">Map&lt;string, u8&gt;</td>
            <td class="py-2.5 pl-3 font-sans">Enforces per-dimension minimum scores across any of the 8 dimensions (<code class="font-mono text-slate-200">architecture</code>, <code class="font-mono text-slate-200">security</code>, <code class="font-mono text-slate-200">maintainability</code>, <code class="font-mono text-slate-200">reliability</code>, <code class="font-mono text-slate-200">testing</code>, <code class="font-mono text-slate-200">dependency_health</code>, <code class="font-mono text-slate-200">documentation</code>, <code class="font-mono text-slate-200">release_readiness</code>).</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">enforce_secret_zero_tolerance</td>
            <td class="py-2.5 px-3 text-blue-400">boolean</td>
            <td class="py-2.5 pl-3 font-sans">When <code class="font-mono text-slate-200">true</code>, immediately fails if any un-waived secret or credential finding (<code class="font-mono text-slate-200">security/hardcoded-secret</code>, <code class="font-mono text-slate-200">security/private-key</code>, <code class="font-mono text-slate-200">security/high-entropy-secret</code>) is present.</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">disallowed_severities</td>
            <td class="py-2.5 px-3 text-blue-400">Severity[]</td>
            <td class="py-2.5 pl-3 font-sans">Fails if any un-waived finding matches a listed severity (<code class="font-mono text-rose-400">critical</code>, <code class="font-mono text-amber-400">high</code>, <code class="font-mono text-yellow-300">medium</code>, <code class="font-mono text-blue-400">low</code>, <code class="font-mono text-slate-400">info</code>).</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">forbidden_licenses</td>
            <td class="py-2.5 px-3 text-blue-400">string[]</td>
            <td class="py-2.5 pl-3 font-sans">Fails if any declared dependency uses a prohibited SPDX license identifier (e.g. <code class="font-mono text-slate-200">GPL-3.0-only</code>, <code class="font-mono text-slate-200">AGPL-3.0-only</code>).</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">max_circular_dependencies</td>
            <td class="py-2.5 px-3 text-blue-400">usize</td>
            <td class="py-2.5 pl-3 font-sans">Caps the maximum allowable Tarjan SCC circular dependency cycles in the architecture graph (set to <code class="font-mono text-slate-200">0</code> for strict DAG enforcement).</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">max_blast_radius_modules</td>
            <td class="py-2.5 px-3 text-blue-400">usize</td>
            <td class="py-2.5 pl-3 font-sans">Caps the maximum number of directly + transitively affected modules in the current Git working-tree diff.</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">max_modified_lines_without_tests</td>
            <td class="py-2.5 px-3 text-blue-400">usize</td>
            <td class="py-2.5 pl-3 font-sans">Caps how many modified source lines can be introduced in a diff without accompanying test file modifications.</td>
          </tr>
          <tr>
            <td class="py-2.5 pr-4 font-bold text-white">required_rule_packs</td>
            <td class="py-2.5 px-3 text-blue-400">string[]</td>
            <td class="py-2.5 pl-3 font-sans">Verifies that the repository's <code class="font-mono text-slate-200">rules.extends</code> configuration includes all mandated rule packs.</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>

  <!-- 4. Time-Bounded Waivers & Exit Code Verdict Contract -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-3">
      <h2 class="text-lg font-bold text-white font-mono">4. Time-Bounded Exception Waivers</h2>
      <p class="text-xs text-slate-300 leading-relaxed">
        Each <code class="text-blue-300 font-mono">PolicyWaiver</code> supports:
      </p>
      <ul class="text-xs text-slate-400 space-y-1.5 font-mono">
        <li>• <strong class="text-white">rule_id:</strong> Exact rule ID or <code class="text-slate-200">"*"</code> wildcard.</li>
        <li>• <strong class="text-white">path_pattern:</strong> Optional glob (<code class="text-slate-200">*</code>, <code class="text-slate-200">**</code>, <code class="text-slate-200">prefix/**</code>, <code class="text-slate-200">**/suffix</code>).</li>
        <li>• <strong class="text-white">expires_at:</strong> RFC 3339 UTC timestamp (<code class="text-slate-200">2027-01-01T00:00:00Z</code>) or date (<code class="text-slate-200">2027-01-01</code>). Invalid or past dates are automatically treated as expired!</li>
        <li>• <strong class="text-white">justification:</strong> Mandatory audit trail explanation.</li>
      </ul>
    </div>

    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-3">
      <h2 class="text-lg font-bold text-white font-mono">5. Gate Verdict & Exit Codes</h2>
      <div class="space-y-2 text-xs font-mono">
        <div class="bg-black/40 border border-slate-800 rounded p-2.5 flex items-center justify-between">
          <span class="text-emerald-400 font-bold">Exit 0 • Passed</span>
          <span class="text-slate-400 font-sans">All policy constraints satisfied</span>
        </div>
        <div class="bg-black/40 border border-slate-800 rounded p-2.5 flex items-center justify-between">
          <span class="text-amber-400 font-bold">Exit 1 • Warning</span>
          <span class="text-slate-400 font-sans">Policy warnings met --fail-on threshold</span>
        </div>
        <div class="bg-black/40 border border-slate-800 rounded p-2.5 flex items-center justify-between">
          <span class="text-orange-400 font-bold">Exit 2 • Config Error</span>
          <span class="text-slate-400 font-sans">Missing or malformed policy YAML/JSON</span>
        </div>
        <div class="bg-black/40 border border-slate-800 rounded p-2.5 flex items-center justify-between">
          <span class="text-purple-400 font-bold">Exit 4 • Violation</span>
          <span class="text-slate-400 font-sans">SecurityPolicyFailure: constraint breached</span>
        </div>
      </div>
    </div>
  </div>
</div>
