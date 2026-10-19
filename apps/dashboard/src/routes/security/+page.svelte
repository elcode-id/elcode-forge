<script lang="ts">
  import { dashboardState } from "$lib/data/store.svelte.js";
  import type { Finding } from "@elcode-forge/schema";

  const bundle = $derived(dashboardState.bundle);
  const allFindings = $derived<Finding[]>(bundle?.repository?.findings ?? []);

  // Filter security-specific findings: category 'security' or rule_id starting with 'security/'
  const securityFindings = $derived<Finding[]>(
    allFindings.filter(
      (f) =>
        f.category?.toLowerCase() === "security" ||
        f.rule_id.toLowerCase().startsWith("security/")
    )
  );

  const secretFindings = $derived<Finding[]>(
    securityFindings.filter((f) => f.rule_id.includes("secret") || f.rule_id.includes("private-key"))
  );

  const unsafePatternFindings = $derived<Finding[]>(
    securityFindings.filter(
      (f) =>
        f.rule_id.includes("eval") ||
        f.rule_id.includes("shell") ||
        f.rule_id.includes("dangerous") ||
        f.rule_id.includes("cors")
    )
  );

  const criticalAndHighCount = $derived<number>(
    securityFindings.filter((f) => f.severity === "critical" || f.severity === "high").length
  );
</script>

<div class="space-y-6">
  <!-- Title & Context -->
  <div class="pb-2 border-b border-[#262c3f]">
    <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
      <span>Security Intelligence</span>
      {#if securityFindings.length > 0}
        <span class="px-2 py-0.5 rounded text-xs font-mono {criticalAndHighCount > 0 ? 'bg-rose-950 text-rose-300 border border-rose-800' : 'bg-amber-950 text-amber-300 border border-amber-800'}">
          {securityFindings.length} Security {securityFindings.length === 1 ? 'Finding' : 'Findings'}
        </span>
      {:else}
        <span class="px-2 py-0.5 rounded text-xs font-mono bg-emerald-950 text-emerald-300 border border-emerald-800">
          Clean
        </span>
      {/if}
    </h1>
    <p class="text-xs text-slate-400 mt-1">
      Deterministic secret leakage prevention, unsafe pattern detection, and supply-chain advisory analysis.
    </p>
  </div>

  <!-- Metric KPI Cards -->
  <div class="grid grid-cols-1 sm:grid-cols-4 gap-4 text-xs">
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4">
      <div class="text-[11px] font-mono uppercase text-slate-400">Total Security Findings</div>
      <div class="text-2xl font-bold font-mono text-slate-100 mt-1">{securityFindings.length}</div>
    </div>
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4">
      <div class="text-[11px] font-mono uppercase text-rose-400">Critical & High Risks</div>
      <div class="text-2xl font-bold font-mono text-rose-300 mt-1">{criticalAndHighCount}</div>
    </div>
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4">
      <div class="text-[11px] font-mono uppercase text-amber-400">Credential / Key Leaks</div>
      <div class="text-2xl font-bold font-mono text-amber-300 mt-1">{secretFindings.length}</div>
    </div>
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4">
      <div class="text-[11px] font-mono uppercase text-blue-400">Unsafe Pattern Violations</div>
      <div class="text-2xl font-bold font-mono text-blue-300 mt-1">{unsafePatternFindings.length}</div>
    </div>
  </div>

  {#if !bundle}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center text-slate-400 text-sm">
      No report loaded.
    </div>
  {:else if securityFindings.length === 0}
    <div class="rounded-lg border border-emerald-900/40 bg-emerald-950/10 p-12 text-center space-y-2">
      <p class="text-emerald-400 font-semibold text-base">Zero Security Vulnerabilities Detected</p>
      <p class="text-xs text-slate-400 max-w-md mx-auto">
        No hardcoded secrets, private keys, eval calls, shell injection patterns, or dangerous innerHTML usages were found.
      </p>
    </div>
  {:else}
    <!-- Security Findings Stack -->
    <div class="space-y-4">
      {#each securityFindings as finding (finding.id)}
        <div class="rounded-lg border border-rose-950/70 bg-[#12151f] p-5 text-xs space-y-3">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div class="flex items-center gap-2">
              <span class="px-2 py-0.5 rounded text-[10px] font-mono uppercase font-bold bg-rose-950 text-rose-300 border border-rose-800">
                {finding.severity}
              </span>
              <span class="font-mono text-slate-300 font-semibold">{finding.rule_id}</span>
              <span class="text-slate-600">•</span>
              <span class="text-slate-400 font-mono text-[10px]">Confidence: {finding.confidence}</span>
            </div>

            {#if finding.location?.file_path}
              <div class="font-mono text-[11px] text-slate-400">
                <span>{finding.location.file_path}</span>
                {#if finding.location.start_line}
                  <span class="text-slate-500">:{finding.location.start_line}</span>
                {/if}
              </div>
            {/if}
          </div>

          <div>
            <h2 class="text-sm font-semibold text-slate-100">{finding.title}</h2>
            <p class="text-slate-300 mt-1 leading-relaxed">{finding.description}</p>
          </div>

          <!-- Plain-Text Redacted Evidence (Zero Injection, Strict Hostile Envelope) -->
          {#if finding.evidence}
            <div class="space-y-1">
              <div class="text-[10px] uppercase font-mono text-slate-400">Redacted Secret / Pattern Evidence:</div>
              <pre class="p-2.5 rounded bg-[#090a0f] border border-[#262c3f] text-rose-300/90 font-mono text-[11px] overflow-x-auto whitespace-pre-wrap break-all">{finding.evidence}</pre>
            </div>
          {/if}

          <!-- Actionable Remediation -->
          {#if finding.recommendation}
            <div class="p-2.5 rounded bg-[#1a1e2e] border border-[#262c3f] text-slate-300">
              <span class="font-semibold text-slate-200">Mitigation:</span>
              <span class="ml-1.5">{finding.recommendation}</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
