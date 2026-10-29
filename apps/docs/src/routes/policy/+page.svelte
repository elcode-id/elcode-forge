<svelte:head>
  <title>Enterprise Policy-as-Code | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8 max-w-4xl">
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-blue-400 mb-1">Specification & Guide</div>
    <h1 class="text-3xl font-extrabold text-white">Enterprise Policy-as-Code Engine</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Declare architectural constraints, security thresholds, and time-bounded exception waivers directly in <code class="text-blue-300 font-mono text-sm">eforge.config.yaml</code>.
    </p>
  </div>

  <!-- Policy Schema Overview -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">1. Declarative Policy Schema</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Policies are defined under the <code class="text-amber-300 font-mono">policy:</code> block in <code class="text-amber-300 font-mono">eforge.config.yaml</code>. The engine evaluates these rules deterministically during <code class="text-blue-300 font-mono">eforge policy check</code> and in CI workflows.
    </p>

    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs overflow-x-auto text-slate-300">
      <pre><code>name: enterprise-standard-gate
version: 1.0.0
description: Organization security, license compliance, and architectural boundary constraints.

constraints:
  min_health_score: 80
  enforce_secret_zero_tolerance: true
  disallowed_severities:
    - critical
    - high
  forbidden_licenses:
    - GPL-3.0-only
    - AGPL-3.0-only
  max_circular_dependencies: 0
  max_blast_radius_modules: 5

waivers:
  - rule_id: security/secret-generic-token
    path_pattern: "**/fixtures/**"
    expires_at: "2027-01-01T00:00:00Z"
    justification: "Synthetic test credentials used exclusively in fixture test suites"

overrides:
  - rule_id: arch/no-circular-dependencies
    severity: high
    enabled: true</code></pre>
    </div>
  </div>

  <!-- Constraint Reference Table -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">2. Evaluated Constraints Reference</h2>
    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs font-mono">
        <thead>
          <tr class="border-b border-slate-800 text-slate-400">
            <th class="pb-2">Constraint</th>
            <th class="pb-2">Type</th>
            <th class="pb-2">Description</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-800/60 text-slate-300">
          <tr>
            <td class="py-2.5 font-bold text-white">min_health_score</td>
            <td class="py-2.5 text-blue-400">number (0-100)</td>
            <td class="py-2.5">Fails the gate if the overall codebase health score is below threshold.</td>
          </tr>
          <tr>
            <td class="py-2.5 font-bold text-white">disallowed_severities</td>
            <td class="py-2.5 text-blue-400">string[]</td>
            <td class="py-2.5">Lists severities (e.g. <code class="text-rose-400">critical</code>, <code class="text-amber-400">high</code>) that unconditionally block merge if un-waived.</td>
          </tr>
          <tr>
            <td class="py-2.5 font-bold text-white">enforce_secret_zero_tolerance</td>
            <td class="py-2.5 text-blue-400">boolean</td>
            <td class="py-2.5">Blocks build immediately if any unmasked API key, private key, or token is detected.</td>
          </tr>
          <tr>
            <td class="py-2.5 font-bold text-white">forbidden_licenses</td>
            <td class="py-2.5 text-blue-400">string[]</td>
            <td class="py-2.5">Rejects copyleft or restrictive licenses (e.g. GPL-3.0-only, AGPL-3.0-only).</td>
          </tr>
          <tr>
            <td class="py-2.5 font-bold text-white">max_circular_dependencies</td>
            <td class="py-2.5 text-blue-400">number</td>
            <td class="py-2.5">Sets allowable threshold for Tarjan Strongly Connected Component module cycles.</td>
          </tr>
          <tr>
            <td class="py-2.5 font-bold text-white">max_blast_radius_modules</td>
            <td class="py-2.5 text-blue-400">number</td>
            <td class="py-2.5">Limits maximum number of modules transitively impacted by a pull request.</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>

  <!-- Time-Bounded Waivers -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">3. Time-Bounded Exception Waivers</h2>
    <p class="text-sm text-slate-300 leading-relaxed">
      Waivers allow pragmatic software engineering without compromising visibility. Each waiver requires an explicit ISO timestamp <code class="text-blue-300 font-mono">expires_at</code> and mandatory <code class="text-blue-300 font-mono">justification</code>. Once expired, the waiver automatically invalidates and the gate fails, preventing forgotten exemptions.
    </p>
  </div>
</div>
