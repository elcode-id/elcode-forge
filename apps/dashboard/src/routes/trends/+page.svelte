<script lang="ts">
  import { onMount } from "svelte";
  import { dashboardState } from "$lib/data/store.svelte.js";
  import { loadHistory, type HistorySnapshot } from "$lib/data/history.js";
  import Sparkline from "$lib/charts/Sparkline.svelte";

  let snapshots = $state<HistorySnapshot[]>([]);
  let isLoading = $state(false);
  let errorMsg = $state<string | null>(null);

  const source = $derived(dashboardState.source);
  const isLive = $derived(source?.kind === "live");
  const baseUrl = $derived(source?.kind === "live" ? source.baseUrl : null);

  async function fetchHistory() {
    if (!baseUrl) return;
    isLoading = true;
    errorMsg = null;
    try {
      const data = await loadHistory(baseUrl, 50);
      snapshots = data;
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : "Failed to load history snapshots";
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    if (isLive && baseUrl) {
      fetchHistory();
    }
  });

  $effect(() => {
    if (isLive && baseUrl) {
      fetchHistory();
    }
  });

  const overallScores = $derived<number[]>(snapshots.map((s) => s.overall));
  const criticalFindings = $derived<number[]>(
    snapshots.map((s) => s.findings_by_severity?.critical ?? 0)
  );
  const totalFindings = $derived<number[]>(
    snapshots.map((s) =>
      Object.values(s.findings_by_severity ?? {}).reduce((a, b) => a + b, 0)
    )
  );
  const latestSnapshot = $derived<HistorySnapshot | null>(
    snapshots.length > 0 ? snapshots[snapshots.length - 1] : null
  );

  function formatDate(ts: number): string {
    if (!ts) return "—";
    const d = new Date(ts * 1000);
    return d.toISOString().replace("T", " ").substring(0, 19);
  }
</script>

<div class="space-y-6">
  <!-- Title & Refresh -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#262c3f]">
    <div>
      <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
        <span>Trends & History Tracking</span>
        {#if snapshots.length > 0}
          <span class="px-2 py-0.5 rounded text-xs bg-slate-800 text-slate-300 font-mono">
            {snapshots.length} Commits
          </span>
        {/if}
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        Deterministic repository evolution, health score trajectory, and finding severity trends across commits.
      </p>
    </div>

    {#if isLive}
      <button
        onclick={fetchHistory}
        disabled={isLoading}
        class="px-3 py-1.5 rounded bg-[#1a1e2e] hover:bg-slate-800 border border-[#262c3f] text-xs font-medium text-slate-200 transition-colors disabled:opacity-50"
      >
        {isLoading ? "Refreshing..." : "Refresh History"}
      </button>
    {/if}
  </div>

  {#if !isLive}
    <!-- Empty State for Static File Mode -->
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center space-y-3">
      <div class="w-10 h-10 rounded-full bg-blue-950 border border-blue-800 text-blue-400 flex items-center justify-center mx-auto text-lg">
        📈
      </div>
      <h2 class="text-base font-semibold text-slate-200">Historical Trends Requires Live Mode</h2>
      <p class="text-xs text-slate-400 max-w-lg mx-auto leading-relaxed">
        You are currently viewing a static standalone report bundle file. Historical commit trends are recorded by running
        <code class="px-1.5 py-0.5 rounded bg-[#090a0f] border border-[#262c3f] font-mono text-slate-300">eforge report --save-history</code>
        and accessed live through the local server:
        <code class="px-1.5 py-0.5 rounded bg-[#090a0f] border border-[#262c3f] font-mono text-slate-300">eforge serve</code>.
      </p>
      <div class="pt-2">
        <span class="text-xs text-blue-400">Connect to http://127.0.0.1:7878 using the header bar to view trends.</span>
      </div>
    </div>
  {:else if errorMsg}
    <div class="rounded-lg border border-red-900/40 bg-red-950/20 p-6 text-center text-xs text-red-300 font-mono">
      {errorMsg}
    </div>
  {:else if snapshots.length === 0}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center space-y-2">
      <h2 class="text-sm font-semibold text-slate-300">No History Snapshots Recorded Yet</h2>
      <p class="text-xs text-slate-400 max-w-md mx-auto">
        Run <code class="px-1 py-0.5 rounded bg-[#090a0f] font-mono text-slate-300">eforge report --save-history</code> across your Git commits to build up the historical trajectory.
      </p>
    </div>
  {:else}
    <!-- Trend Sparkline Cards -->
    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4 text-xs">
      <!-- Health Score Trend -->
      <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4 space-y-3">
        <div class="flex items-center justify-between">
          <span class="text-[10px] font-mono uppercase text-slate-400">Health Score Trend</span>
          <span class="font-mono text-sm font-bold text-blue-400">{latestSnapshot?.overall ?? "—"}</span>
        </div>
        <div>
          <Sparkline values={overallScores} width={220} height={40} color="#38bdf8" label="Health Score" />
        </div>
      </div>

      <!-- Critical / High Findings Trend -->
      <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4 space-y-3">
        <div class="flex items-center justify-between">
          <span class="text-[10px] font-mono uppercase text-rose-400">Critical Findings</span>
          <span class="font-mono text-sm font-bold text-rose-300">{criticalFindings[criticalFindings.length - 1] ?? 0}</span>
        </div>
        <div>
          <Sparkline values={criticalFindings} width={220} height={40} color="#f43f5e" label="Critical Findings" />
        </div>
      </div>

      <!-- Total Findings Trend -->
      <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4 space-y-3">
        <div class="flex items-center justify-between">
          <span class="text-[10px] font-mono uppercase text-amber-400">Total Findings</span>
          <span class="font-mono text-sm font-bold text-amber-300">{totalFindings[totalFindings.length - 1] ?? 0}</span>
        </div>
        <div>
          <Sparkline values={totalFindings} width={220} height={40} color="#fbbf24" label="Total Findings" />
        </div>
      </div>
    </div>

    <!-- History Snapshots Table -->
    <div class="space-y-3">
      <h2 class="text-sm font-bold text-slate-200 uppercase font-mono tracking-wider">
        Commit Snapshot History
      </h2>
      <div class="rounded-lg border border-[#262c3f] bg-[#12151f] overflow-hidden">
        <table class="w-full text-left text-xs text-slate-300" aria-label="Commit snapshots history table">
          <thead class="bg-[#1a1e2e] text-slate-400 uppercase font-mono text-[10px] tracking-wider border-b border-[#262c3f]">
            <tr>
              <th scope="col" class="py-2.5 px-4">Commit SHA</th>
              <th scope="col" class="py-2.5 px-3">Date</th>
              <th scope="col" class="py-2.5 px-3 text-right">Health Score</th>
              <th scope="col" class="py-2.5 px-3 text-right">Critical</th>
              <th scope="col" class="py-2.5 px-3 text-right">High</th>
              <th scope="col" class="py-2.5 px-3 text-right">Medium</th>
              <th scope="col" class="py-2.5 px-3 text-right">Low</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[#262c3f]/50 font-mono text-xs">
            {#each snapshots as snap (snap.head_sha + String(snap.commit_unix_ts))}
              <tr class="hover:bg-[#1a1e2e]/40">
                <td class="py-2.5 px-4 font-bold text-blue-400">{snap.head_sha}</td>
                <td class="py-2.5 px-3 text-slate-400">{formatDate(snap.commit_unix_ts)}</td>
                <td class="py-2.5 px-3 text-right text-slate-100 font-bold">{snap.overall}</td>
                <td class="py-2.5 px-3 text-right text-rose-400">{snap.findings_by_severity?.critical ?? 0}</td>
                <td class="py-2.5 px-3 text-right text-amber-400">{snap.findings_by_severity?.high ?? 0}</td>
                <td class="py-2.5 px-3 text-right text-yellow-400">{snap.findings_by_severity?.medium ?? 0}</td>
                <td class="py-2.5 px-3 text-right text-blue-400">{snap.findings_by_severity?.low ?? 0}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>
