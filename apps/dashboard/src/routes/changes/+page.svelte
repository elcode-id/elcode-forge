<script lang="ts">
  import { dashboardState } from "$lib/data/store.svelte.js";
  import type { HotspotEntry } from "@elcode-forge/schema";

  const bundle = $derived(dashboardState.bundle);
  const changeRisk = $derived(bundle?.change_risk);
  const changeImpact = $derived(bundle?.change_impact);
  const hotspots = $derived<HotspotEntry[]>(bundle?.hotspots ?? []);

  function riskLevelColor(level?: string): string {
    switch (level) {
      case "CRITICAL":
        return "bg-rose-950 text-rose-300 border-rose-800";
      case "HIGH":
        return "bg-amber-950 text-amber-300 border-amber-800";
      case "MEDIUM":
        return "bg-yellow-950 text-yellow-300 border-yellow-800";
      case "LOW":
        return "bg-blue-950 text-blue-300 border-blue-800";
      default:
        return "bg-slate-800 text-slate-400 border-slate-700";
    }
  }
</script>

<div class="space-y-6">
  <!-- Title & Intro -->
  <div class="pb-2 border-b border-[#262c3f]">
    <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
      <span>Changes & Blast Radius</span>
      {#if changeRisk}
        <span class="px-2 py-0.5 rounded text-xs font-mono uppercase border {riskLevelColor(changeRisk.level)}">
          {changeRisk.level} RISK (Score: {changeRisk.score})
        </span>
      {/if}
    </h1>
    <p class="text-xs text-slate-400 mt-1">
      Git diff impact analysis, blast-radius module propagation, missing test files, and commit churn hotspots.
    </p>
  </div>

  {#if !bundle}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center text-slate-400 text-sm">
      No report loaded.
    </div>
  {:else}
    <!-- Top Risk & Impact Breakdown -->
    {#if changeRisk || changeImpact}
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Change Risk Card -->
        <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-5 space-y-4">
          <div class="flex items-center justify-between">
            <span class="text-[10px] font-mono uppercase text-slate-400">Change Risk Score</span>
            {#if changeRisk}
              <span class="px-2 py-0.5 rounded text-[10px] font-mono uppercase border {riskLevelColor(changeRisk.level)}">
                {changeRisk.level}
              </span>
            {/if}
          </div>

          <div class="flex items-baseline gap-2">
            <span class="text-3xl font-bold font-mono text-slate-100">{changeRisk?.score ?? 0}</span>
            <span class="text-xs text-slate-400">/ 100 risk index</span>
          </div>

          {#if changeRisk && changeRisk.reasons && changeRisk.reasons.length > 0}
            <div class="space-y-1.5 pt-2 border-t border-[#262c3f]">
              <span class="text-[10px] uppercase font-mono text-slate-400">Risk Contributors:</span>
              <ul class="space-y-1 text-xs text-slate-300">
                {#each changeRisk.reasons as reason (reason)}
                  <li class="flex items-start gap-1.5">
                    <span class="text-amber-400">•</span>
                    <span>{reason}</span>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>

        <!-- Blast Radius Impact -->
        <div class="lg:col-span-2 rounded-lg border border-[#262c3f] bg-[#12151f] p-5 space-y-4">
          <div class="flex items-center justify-between">
            <span class="text-[10px] font-mono uppercase text-slate-400">Blast Radius Summary</span>
            <span class="text-xs font-mono text-slate-400">
              {(changeImpact?.changed_files ?? []).length} Changed Files
            </span>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <!-- Directly affected -->
            <div class="p-3 rounded bg-[#090a0f] border border-[#262c3f] space-y-2">
              <div class="flex items-center justify-between text-xs">
                <span class="font-semibold text-slate-200">Directly Affected Modules</span>
                <span class="font-mono text-blue-400 font-bold">
                  {(changeImpact?.directly_affected_modules ?? []).length}
                </span>
              </div>
              <div class="max-h-24 overflow-y-auto space-y-1 pr-1 font-mono text-[11px] text-slate-300">
                {#if (changeImpact?.directly_affected_modules ?? []).length === 0}
                  <span class="text-slate-500 italic">None</span>
                {:else}
                  {#each (changeImpact?.directly_affected_modules ?? []) as mod (mod)}
                    <div class="px-2 py-0.5 rounded bg-[#12151f] border border-[#262c3f] break-all">{mod}</div>
                  {/each}
                {/if}
              </div>
            </div>

            <!-- Transitively affected -->
            <div class="p-3 rounded bg-[#090a0f] border border-[#262c3f] space-y-2">
              <div class="flex items-center justify-between text-xs">
                <span class="font-semibold text-slate-200">Transitively Affected Modules</span>
                <span class="font-mono text-amber-400 font-bold">
                  {(changeImpact?.transitively_affected_modules ?? []).length}
                </span>
              </div>
              <div class="max-h-24 overflow-y-auto space-y-1 pr-1 font-mono text-[11px] text-slate-300">
                {#if (changeImpact?.transitively_affected_modules ?? []).length === 0}
                  <span class="text-slate-500 italic">None</span>
                {:else}
                  {#each (changeImpact?.transitively_affected_modules ?? []) as mod (mod)}
                    <div class="px-2 py-0.5 rounded bg-[#12151f] border border-[#262c3f] break-all">{mod}</div>
                  {/each}
                {/if}
              </div>
            </div>
          </div>

          <!-- Missing Tests Warning -->
          {#if (changeImpact?.changed_files_without_tests ?? []).length > 0}
            <div class="p-3 rounded bg-amber-950/20 border border-amber-900/40 text-xs space-y-1.5">
              <div class="font-semibold text-amber-300 flex items-center gap-1.5">
                <span>⚠️</span>
                <span>{changeImpact?.changed_files_without_tests.length} Changed Source Files Without Tests:</span>
              </div>
              <div class="max-h-20 overflow-y-auto space-y-1 font-mono text-[11px] text-slate-300">
                {#each (changeImpact?.changed_files_without_tests ?? []) as file (file)}
                  <div class="text-amber-200/90 break-all">{file}</div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-6 text-center text-slate-400 text-xs">
        No active Git diff detected in working tree. Run scan or report during an active commit / PR review to inspect blast radius.
      </div>
    {/if}

    <!-- Changed Files Table (if diff present) -->
    {#if (changeImpact?.changed_files ?? []).length > 0}
      <div class="space-y-3">
        <h2 class="text-sm font-bold text-slate-200 uppercase font-mono tracking-wider">
          Changed Files ({changeImpact?.changed_files.length})
        </h2>
        <div class="rounded-lg border border-[#262c3f] bg-[#12151f] overflow-hidden">
          <table class="w-full text-left text-xs text-slate-300" aria-label="Changed files table">
            <thead class="bg-[#1a1e2e] text-slate-400 uppercase font-mono text-[10px] tracking-wider border-b border-[#262c3f]">
              <tr>
                <th scope="col" class="py-2.5 px-4">File Path</th>
                <th scope="col" class="py-2.5 px-3">Status</th>
                <th scope="col" class="py-2.5 px-3 text-right">Added Lines</th>
                <th scope="col" class="py-2.5 px-3 text-right">Deleted Lines</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-[#262c3f]/50 font-mono text-xs">
              {#each (changeImpact?.changed_files ?? []) as file (file.path)}
                <tr class="hover:bg-[#1a1e2e]/40">
                  <td class="py-2.5 px-4 font-medium text-slate-100">{file.path}</td>
                  <td class="py-2.5 px-3 text-[11px]">
                    {#if file.is_new}
                      <span class="px-1.5 py-0.5 rounded bg-emerald-950 text-emerald-300 border border-emerald-900">NEW</span>
                    {:else if file.is_deleted}
                      <span class="px-1.5 py-0.5 rounded bg-rose-950 text-rose-300 border border-rose-900">DELETED</span>
                    {:else}
                      <span class="text-slate-400">MODIFIED</span>
                    {/if}
                  </td>
                  <td class="py-2.5 px-3 text-right text-emerald-400">+{file.added_lines}</td>
                  <td class="py-2.5 px-3 text-right text-rose-400">-{file.deleted_lines}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {/if}

    <!-- Git Churn Hotspots -->
    <div class="space-y-3">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-bold text-slate-200 uppercase font-mono tracking-wider">
          Top Churn Hotspots ({hotspots.length})
        </h2>
        <span class="text-xs text-slate-400">High-churn files have higher empirical defect probability</span>
      </div>

      {#if hotspots.length === 0}
        <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-8 text-center text-slate-500 text-xs italic">
          No Git history hotspots recorded.
        </div>
      {:else}
        <div class="rounded-lg border border-[#262c3f] bg-[#12151f] overflow-hidden">
          <table class="w-full text-left text-xs text-slate-300" aria-label="Hotspots table">
            <thead class="bg-[#1a1e2e] text-slate-400 uppercase font-mono text-[10px] tracking-wider border-b border-[#262c3f]">
              <tr>
                <th scope="col" class="py-2.5 px-4">File Path</th>
                <th scope="col" class="py-2.5 px-3 text-right">Commit Count</th>
                <th scope="col" class="py-2.5 px-3 text-right">Contributors</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-[#262c3f]/50 font-mono text-xs">
              {#each hotspots as spot (spot.path)}
                <tr class="hover:bg-[#1a1e2e]/40">
                  <td class="py-2.5 px-4 font-medium text-slate-100">{spot.path}</td>
                  <td class="py-2.5 px-3 text-right text-amber-300 font-bold">{spot.commit_count}</td>
                  <td class="py-2.5 px-3 text-right text-slate-300">{spot.contributors_count}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {/if}
</div>
