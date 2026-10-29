<script lang="ts">
  import { dashboardState } from "$lib/data/store.svelte.js";
  import {
    STARTER_POLICY_YAML,
    parsePolicyYaml,
    evaluatePolicyAgainstBundle,
    isWaiverExpired,
    type PolicyEvaluationResult,
  } from "$lib/data/org.js";

  let policyYaml = $state(STARTER_POLICY_YAML);
  let parseResult = $derived(parsePolicyYaml(policyYaml));
  let policy = $derived(parseResult.ok ? parseResult.policy : null);

  let currentBundle = $derived(dashboardState.bundle);

  let evaluation = $derived.by<PolicyEvaluationResult | null>(() => {
    if (!policy) return null;
    return evaluatePolicyAgainstBundle(policy, currentBundle);
  });

  function resetTemplate() {
    policyYaml = STARTER_POLICY_YAML;
  }
</script>

<svelte:head>
  <title>ELCODE Forge | Enterprise Governance & Policy</title>
</svelte:head>

<div class="max-w-7xl mx-auto px-6 py-8 space-y-8">
  <!-- Page Header -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-[#262c3f] pb-6">
    <div>
      <div class="flex items-center gap-2 mb-1">
        <span class="w-2.5 h-2.5 rounded-full bg-purple-500"></span>
        <h1 class="text-2xl font-bold tracking-tight text-white font-mono">Enterprise Policy & Governance</h1>
      </div>
      <p class="text-sm text-slate-400">
        Declarative Policy-as-Code gate enforcement, architectural constraint meters, and time-bounded exception waivers.
      </p>
    </div>

    <div class="flex items-center gap-3">
      {#if currentBundle}
        <div class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-800 text-xs text-slate-300">
          <span class="text-slate-500">Target Repo:</span>
          <span class="font-mono font-medium text-white">{currentBundle.repository?.name || "Active Project"}</span>
          <span class="text-xs px-2 py-0.5 rounded bg-blue-950/80 text-blue-300 font-mono">
            Health: {currentBundle.repository?.health?.overall ?? 0}/100
          </span>
        </div>
      {:else}
        <div class="px-3 py-1.5 rounded-lg bg-slate-900/60 border border-slate-800 text-xs text-slate-500">
          No active repository bundle loaded
        </div>
      {/if}

      <button
        onclick={resetTemplate}
        class="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium transition-colors"
      >
        Reset Template
      </button>
    </div>
  </div>

  <!-- Live Gate Verdict Banner (if bundle loaded) -->
  {#if evaluation}
    <div
      class="rounded-xl p-5 border flex flex-col md:flex-row md:items-center justify-between gap-4 transition-all {evaluation.verdict === 'passed' ? 'bg-emerald-950/20 border-emerald-800/40' : evaluation.verdict === 'warning' ? 'bg-amber-950/20 border-amber-800/40' : 'bg-rose-950/20 border-rose-800/40'}"
    >
      <div class="flex items-center gap-4">
        <div
          class="w-12 h-12 rounded-xl flex items-center justify-center font-bold text-lg font-mono {evaluation.verdict === 'passed' ? 'bg-emerald-500/20 text-emerald-400' : evaluation.verdict === 'warning' ? 'bg-amber-500/20 text-amber-400' : 'bg-rose-500/20 text-rose-400'}"
        >
          {#if evaluation.verdict === "passed"}✓{:else if evaluation.verdict === "warning"}!{:else}✕{/if}
        </div>
        <div>
          <div class="flex items-center gap-2">
            <span class="text-xs uppercase font-mono tracking-wider text-slate-400">Policy Gate Assertion</span>
            <span
              class="px-2 py-0.5 rounded text-[11px] font-bold uppercase tracking-wider font-mono {evaluation.verdict === 'passed' ? 'bg-emerald-900/60 text-emerald-300' : evaluation.verdict === 'warning' ? 'bg-amber-900/60 text-amber-300' : 'bg-rose-900/60 text-rose-300'}"
            >
              {evaluation.verdict}
            </span>
          </div>
          <p class="text-sm text-slate-200 font-medium mt-0.5">
            {#if evaluation.verdict === "passed"}
              All enterprise security, license, and architectural constraints verified successfully.
            {:else if evaluation.verdict === "warning"}
              Policy passed with warnings ({evaluation.warnings.length} warning item(s) flagged).
            {:else}
              Policy gate failed with {evaluation.violations.length} critical constraint violation(s).
            {/if}
          </p>
        </div>
      </div>

      <div class="flex items-center gap-4 text-xs font-mono">
        <div class="text-center px-3 py-1 rounded bg-black/30 border border-slate-800">
          <div class="text-emerald-400 font-bold">{evaluation.passed_checks.length}</div>
          <div class="text-slate-500 text-[10px]">Passed</div>
        </div>
        <div class="text-center px-3 py-1 rounded bg-black/30 border border-slate-800">
          <div class="text-amber-400 font-bold">{evaluation.warnings.length}</div>
          <div class="text-slate-500 text-[10px]">Warnings</div>
        </div>
        <div class="text-center px-3 py-1 rounded bg-black/30 border border-slate-800">
          <div class="text-rose-400 font-bold">{evaluation.violations.length}</div>
          <div class="text-slate-500 text-[10px]">Violations</div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Constraints & Overview Grid -->
  {#if policy}
    <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4">
      <!-- Min Health Score Gauge -->
      <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-4 flex flex-col justify-between">
        <div class="text-xs text-slate-400 font-medium">Min Health Score</div>
        <div class="flex items-baseline gap-2 my-2">
          <span class="text-2xl font-bold font-mono text-white">
            {policy.constraints.min_health_score ?? "N/A"}
          </span>
          <span class="text-xs text-slate-500 font-mono">/ 100</span>
        </div>
        <div class="w-full bg-slate-800 rounded-full h-1.5 overflow-hidden">
          <div
            class="bg-blue-500 h-1.5 rounded-full"
            style="width: {policy.constraints.min_health_score ?? 0}%"
          ></div>
        </div>
      </div>

      <!-- Secret Zero Tolerance -->
      <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-4 flex flex-col justify-between">
        <div class="text-xs text-slate-400 font-medium">Secret Zero Tolerance</div>
        <div class="my-2">
          {#if policy.constraints.enforce_secret_zero_tolerance}
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-rose-950/60 border border-rose-800/40 text-rose-300 font-mono text-xs font-semibold">
              <span class="w-1.5 h-1.5 rounded-full bg-rose-400 animate-pulse"></span>
              Enforced
            </span>
          {:else}
            <span class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md bg-slate-800 text-slate-400 font-mono text-xs">
              Disabled
            </span>
          {/if}
        </div>
        <span class="text-[11px] text-slate-500">Blocks unmasked API keys & tokens</span>
      </div>

      <!-- Disallowed Severities -->
      <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-4 flex flex-col justify-between">
        <div class="text-xs text-slate-400 font-medium">Blocked Severities</div>
        <div class="flex flex-wrap gap-1 my-2">
          {#each policy.constraints.disallowed_severities || [] as sev (sev)}
            <span class="px-2 py-0.5 rounded text-[10px] uppercase font-mono font-bold bg-rose-950/70 border border-rose-800/50 text-rose-300">
              {sev}
            </span>
          {/each}
        </div>
        <span class="text-[11px] text-slate-500">Un-waived findings fail gate</span>
      </div>

      <!-- Max Circular Dependencies -->
      <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-4 flex flex-col justify-between">
        <div class="text-xs text-slate-400 font-medium">Max Circular Cycles</div>
        <div class="text-2xl font-bold font-mono text-white my-2">
          {policy.constraints.max_circular_dependencies ?? "∞"}
        </div>
        <span class="text-[11px] text-slate-500">Tarjan SCC module boundaries</span>
      </div>

      <!-- Max Blast Radius -->
      <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-4 flex flex-col justify-between">
        <div class="text-xs text-slate-400 font-medium">Max Blast Radius</div>
        <div class="text-2xl font-bold font-mono text-white my-2">
          {policy.constraints.max_blast_radius_modules ? `${policy.constraints.max_blast_radius_modules} mods` : "∞"}
        </div>
        <span class="text-[11px] text-slate-500">Git commit impact limit</span>
      </div>

      <!-- Forbidden Licenses -->
      <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-4 flex flex-col justify-between">
        <div class="text-xs text-slate-400 font-medium">Forbidden Licenses</div>
        <div class="flex flex-wrap gap-1 my-2">
          {#each policy.constraints.forbidden_licenses || [] as lic (lic)}
            <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-amber-950/70 border border-amber-800/40 text-amber-300">
              {lic}
            </span>
          {/each}
        </div>
        <span class="text-[11px] text-slate-500">SPDX denylist enforcement</span>
      </div>
    </div>
  {/if}

  <!-- Split View: YAML Editor vs Active Waivers -->
  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- Policy Editor Section -->
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl overflow-hidden flex flex-col">
      <div class="px-5 py-3 border-b border-[#262c3f] flex items-center justify-between bg-[#171a27]">
        <div class="flex items-center gap-2">
          <span class="text-xs font-mono font-semibold text-white">Policy Document Specification</span>
          <span class="text-[11px] px-2 py-0.5 rounded bg-slate-800 text-slate-400 font-mono">YAML</span>
        </div>

        {#if parseResult.ok}
          <span class="inline-flex items-center gap-1.5 text-xs text-emerald-400 font-mono">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
            Valid Syntax
          </span>
        {:else}
          <span class="inline-flex items-center gap-1.5 text-xs text-rose-400 font-mono">
            <span class="w-1.5 h-1.5 rounded-full bg-rose-400"></span>
            Syntax Error
          </span>
        {/if}
      </div>

      <div class="p-4 flex-1 flex flex-col">
        <textarea
          bind:value={policyYaml}
          class="w-full h-96 p-4 rounded-lg bg-[#090a0f] border border-[#262c3f] font-mono text-xs text-slate-200 leading-relaxed focus:outline-none focus:border-purple-500 resize-none font-mono"
          spellcheck="false"
        ></textarea>

        {#if !parseResult.ok}
          <div class="mt-3 p-3 rounded-lg bg-rose-950/40 border border-rose-800/40 text-rose-300 text-xs font-mono">
            {parseResult.error}
          </div>
        {/if}
      </div>
    </div>

    <!-- Active Waivers Table Section -->
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl overflow-hidden flex flex-col">
      <div class="px-5 py-3 border-b border-[#262c3f] flex items-center justify-between bg-[#171a27]">
        <div class="flex items-center gap-2">
          <span class="text-xs font-mono font-semibold text-white">Configured Waivers & Exceptions</span>
          <span class="text-[11px] px-2 py-0.5 rounded bg-slate-800 text-slate-400 font-mono">
            {policy?.waivers.length || 0} active
          </span>
        </div>
      </div>

      <div class="p-4 flex-1 overflow-x-auto">
        {#if policy && policy.waivers.length > 0}
          <table class="w-full text-left text-xs font-mono">
            <thead>
              <tr class="border-b border-slate-800 text-slate-400 text-[11px]">
                <th class="pb-2">Rule ID</th>
                <th class="pb-2">Path Pattern</th>
                <th class="pb-2">Expires</th>
                <th class="pb-2">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-800/60">
              {#each policy.waivers as waiver, idx (waiver.rule_id + (waiver.path_pattern || "") + idx)}
                {@const expired = isWaiverExpired(waiver.expires_at)}
                <tr class="text-slate-300 hover:bg-slate-900/40 transition-colors">
                  <td class="py-2.5 font-medium text-white">{waiver.rule_id}</td>
                  <td class="py-2.5 text-slate-400">{waiver.path_pattern || "All files"}</td>
                  <td class="py-2.5 text-slate-400">{waiver.expires_at ? waiver.expires_at.split("T")[0] : "Permanent"}</td>
                  <td class="py-2.5">
                    {#if expired}
                      <span class="px-2 py-0.5 rounded text-[10px] bg-rose-950/70 border border-rose-800 text-rose-300">
                        Expired
                      </span>
                    {:else}
                      <span class="px-2 py-0.5 rounded text-[10px] bg-emerald-950/70 border border-emerald-800 text-emerald-300">
                        Active
                      </span>
                    {/if}
                  </td>
                </tr>
                <tr class="bg-black/20 text-[11px] text-slate-400">
                  <td colspan="4" class="py-1 px-2 italic border-b border-slate-800/40">
                    Justification: {waiver.justification}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <div class="h-64 flex flex-col items-center justify-center text-center text-slate-500">
            <span class="text-2xl mb-2">🛡️</span>
            <p class="text-sm">No waivers declared in this policy.</p>
            <p class="text-xs text-slate-600 mt-1">All standard rule assertions are strictly enforced without bypasses.</p>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Detailed Gate Checks Breakdown -->
  {#if evaluation && (evaluation.passed_checks.length > 0 || evaluation.violations.length > 0 || evaluation.warnings.length > 0)}
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-5 space-y-4">
      <h3 class="text-sm font-semibold text-white font-mono flex items-center gap-2">
        <span>Evaluated Constraint Assertions</span>
        <span class="text-xs text-slate-500">({evaluation.passed_checks.length + evaluation.violations.length + evaluation.warnings.length} checks)</span>
      </h3>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs font-mono">
        {#each evaluation.violations as violation, idx (violation + idx)}
          <div class="p-3 rounded-lg bg-rose-950/30 border border-rose-800/40 text-rose-300 flex items-start gap-2.5">
            <span class="text-rose-400 font-bold">✕</span>
            <span>{violation}</span>
          </div>
        {/each}

        {#each evaluation.warnings as warning, idx (warning + idx)}
          <div class="p-3 rounded-lg bg-amber-950/30 border border-amber-800/40 text-amber-300 flex items-start gap-2.5">
            <span class="text-amber-400 font-bold">!</span>
            <span>{warning}</span>
          </div>
        {/each}

        {#each evaluation.passed_checks as passed, idx (passed + idx)}
          <div class="p-3 rounded-lg bg-emerald-950/20 border border-emerald-800/30 text-emerald-300 flex items-start gap-2.5">
            <span class="text-emerald-400 font-bold">✓</span>
            <span>{passed}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
