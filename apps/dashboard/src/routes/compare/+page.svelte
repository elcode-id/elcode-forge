<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { dashboardState } from "$lib/data/store.svelte.js";
  import {
    fetchProjects,
    fetchProjectBranches,
    pullSnapshot,
    computeBaselineDiff,
    type BaselineDiffReport,
    type BranchSummary,
  } from "$lib/data/server.js";
  import type { Finding, HealthScore } from "@elcode-forge/schema";

  let selectedRepo = $state("");
  let baseBranch = $state("main");
  let compareBranch = $state("");
  let availableBranches = $state<BranchSummary[]>([]);
  let isLoading = $state(false);
  let diffReport = $state<BaselineDiffReport | null>(null);
  let activeTab = $state<"added" | "resolved" | "all">("added");
  let severityFilter = $state<string>("all");

  async function loadBranchesForRepo(repoId: string) {
    if (!repoId) return;
    const res = await fetchProjectBranches(
      dashboardState.serverConfig.serverUrl,
      repoId,
      dashboardState.serverConfig.token
    );
    if (res.ok) {
      availableBranches = res.branches;
      if (res.branches.length > 0) {
        if (!compareBranch && res.branches.length > 1) {
          compareBranch = res.branches[1].branch;
        } else if (!compareBranch) {
          compareBranch = res.branches[0].branch;
        }
      }
    }
  }

  async function runComparison() {
    if (!selectedRepo || !baseBranch || !compareBranch) {
      dashboardState.setError("Please select repository, base branch, and compare branch");
      return;
    }

    isLoading = true;
    dashboardState.setError(null);
    diffReport = null;

    try {
      // 1. Pull Base Branch
      const baseRes = await pullSnapshot(
        dashboardState.serverConfig.serverUrl,
        selectedRepo,
        baseBranch,
        dashboardState.serverConfig.token
      );
      if (!baseRes.ok || !baseRes.snapshot) {
        dashboardState.setError(
          baseRes.error || `Could not fetch baseline snapshot for '${baseBranch}'`
        );
        isLoading = false;
        return;
      }

      // 2. Pull Compare Branch (or use local bundle if matching)
      let compareHealth: HealthScore = { overall: 100, dimensions: [] };
      let compareFindings: Finding[] = [];

      const compRes = await pullSnapshot(
        dashboardState.serverConfig.serverUrl,
        selectedRepo,
        compareBranch,
        dashboardState.serverConfig.token
      );

      if (compRes.ok && compRes.snapshot) {
        compareHealth = compRes.snapshot.baseline_health_score || { overall: 100, dimensions: [] };
        compareFindings = compRes.snapshot.baseline_findings || [];
      } else if (
        dashboardState.bundle &&
        dashboardState.bundle.repository.name === selectedRepo
      ) {
        compareHealth = dashboardState.bundle.repository.health || { overall: 100, dimensions: [] };
        compareFindings = dashboardState.bundle.repository.findings;
      } else {
        dashboardState.setError(
          compRes.error || `Could not fetch snapshot for compare branch '${compareBranch}'`
        );
        isLoading = false;
        return;
      }

      // 3. Compute Baseline Differential
      const baseHealth = baseRes.snapshot.baseline_health_score || {
        overall: 100,
        dimensions: [],
      };
      const baseFindings = baseRes.snapshot.baseline_findings || [];

      diffReport = computeBaselineDiff(
        selectedRepo,
        { branch: baseBranch, health: baseHealth, findings: baseFindings },
        { branch: compareBranch, health: compareHealth, findings: compareFindings }
      );
    } catch (err) {
      dashboardState.setError(err instanceof Error ? err.message : "Failed to compute diff");
    } finally {
      isLoading = false;
    }
  }

  let displayedFindings = $derived.by(() => {
    if (!diffReport) return [];
    const list =
      activeTab === "added"
        ? diffReport.addedFindings
        : activeTab === "resolved"
        ? diffReport.resolvedFindings
        : [...diffReport.addedFindings, ...diffReport.resolvedFindings];

    if (severityFilter !== "all") {
      return list.filter((f) => f.severity.toLowerCase() === severityFilter);
    }
    return list;
  });

  onMount(async () => {
    if (dashboardState.serverProjects.length === 0) {
      const res = await fetchProjects(
        dashboardState.serverConfig.serverUrl,
        dashboardState.serverConfig.token
      );
      if (res.ok) {
        dashboardState.setServerProjects(res.projects);
      }
    }

    const queryRepo = page.url.searchParams.get("repo");
    const queryBranch = page.url.searchParams.get("branch");

    if (queryRepo) {
      selectedRepo = queryRepo;
      await loadBranchesForRepo(queryRepo);
      if (queryBranch) {
        compareBranch = queryBranch;
      }
      if (selectedRepo && baseBranch && compareBranch) {
        runComparison();
      }
    } else if (dashboardState.serverProjects.length > 0) {
      selectedRepo = dashboardState.serverProjects[0].repo_id;
      await loadBranchesForRepo(selectedRepo);
    }
  });
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 pb-4 border-b border-[#262c3f]">
    <div>
      <h1 class="text-2xl font-bold tracking-tight text-white font-mono">
        PR Baseline Diff Inspector
      </h1>
      <p class="text-xs text-slate-400 font-mono mt-1">
        Compare active PR branches against remote baseline (<span class="text-slate-300 font-bold">{baseBranch}</span>) to detect architectural drift, new vulnerabilities, and score shifts.
      </p>
    </div>
  </div>

  <!-- Branch Selection Toolbar -->
  <div class="p-4 rounded-xl bg-[#12151f] border border-[#262c3f] flex flex-wrap items-center gap-4 text-xs font-mono">
    <!-- Project Selector -->
    <div class="flex items-center gap-2">
      <label for="compare-repo-select" class="text-slate-400">Repository:</label>
      <select
        id="compare-repo-select"
        bind:value={selectedRepo}
        onchange={() => loadBranchesForRepo(selectedRepo)}
        class="px-3 py-1.5 bg-[#090a0f] border border-[#262c3f] rounded text-slate-200 focus:outline-none focus:border-blue-500"
      >
        <option value="" disabled>Select repository...</option>
        {#each dashboardState.serverProjects as p (p.repo_id)}
          <option value={p.repo_id}>{p.name} ({p.repo_id})</option>
        {/each}
      </select>
    </div>

    <!-- Base Branch -->
    <div class="flex items-center gap-2">
      <label for="compare-base-branch" class="text-slate-400">Base Branch:</label>
      <input
        id="compare-base-branch"
        type="text"
        bind:value={baseBranch}
        placeholder="main"
        class="w-32 px-2.5 py-1.5 bg-[#090a0f] border border-[#262c3f] rounded text-slate-200 focus:outline-none focus:border-blue-500"
      />
    </div>

    <!-- Compare Branch -->
    <div class="flex items-center gap-2">
      <label for="compare-pr-branch" class="text-slate-400">PR / Compare Branch:</label>
      {#if availableBranches.length > 0}
        <select
          id="compare-pr-branch"
          bind:value={compareBranch}
          class="px-3 py-1.5 bg-[#090a0f] border border-[#262c3f] rounded text-slate-200 focus:outline-none focus:border-blue-500"
        >
          {#each availableBranches as b (b.branch)}
            <option value={b.branch}>{b.branch} ({b.latest_commit.slice(0, 7)})</option>
          {/each}
        </select>
      {:else}
        <input
          id="compare-pr-branch"
          type="text"
          bind:value={compareBranch}
          placeholder="feature/branch"
          class="w-40 px-2.5 py-1.5 bg-[#090a0f] border border-[#262c3f] rounded text-slate-200 focus:outline-none focus:border-blue-500"
        />
      {/if}
    </div>

    <!-- Submit Button -->
    <button
      onclick={runComparison}
      disabled={isLoading || !selectedRepo}
      class="px-4 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white font-medium transition-colors disabled:opacity-50"
    >
      {isLoading ? "Comparing..." : "Inspect Diff"}
    </button>
  </div>

  <!-- Comparison Results -->
  {#if diffReport}
    <!-- Score Delta Metric Cards -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <!-- Health Score Shift -->
      <div class="p-4 rounded-xl bg-[#12151f] border border-[#262c3f]">
        <span class="text-xs text-slate-400 font-mono block mb-1">Health Score Shift</span>
        <div class="flex items-baseline gap-2 font-mono">
          <span class="text-xl font-bold text-white">{diffReport.compareHealth}/100</span>
          <span class="text-xs text-slate-400">from {diffReport.baseHealth}</span>
          <span
            class="text-xs font-bold px-2 py-0.5 rounded {diffReport.healthDelta > 0
              ? 'bg-emerald-950 text-emerald-400 border border-emerald-800'
              : diffReport.healthDelta < 0
              ? 'bg-red-950 text-red-400 border border-red-800'
              : 'bg-slate-800 text-slate-300'}"
          >
            {diffReport.healthDelta >= 0 ? `+${diffReport.healthDelta}` : diffReport.healthDelta}
          </span>
        </div>
        <span class="text-[11px] text-slate-500 font-mono mt-1 block capitalize">
          Trajectory: {diffReport.trendDirection}
        </span>
      </div>

      <!-- Added Findings -->
      <div class="p-4 rounded-xl bg-[#12151f] border border-[#262c3f]">
        <span class="text-xs text-slate-400 font-mono block mb-1">New Findings Introduced</span>
        <div class="flex items-baseline gap-2 font-mono">
          <span class="text-xl font-bold {diffReport.addedFindings.length > 0 ? 'text-red-400' : 'text-emerald-400'}">
            +{diffReport.addedFindings.length}
          </span>
          <span class="text-xs text-slate-500">issues</span>
        </div>
        <span class="text-[11px] text-slate-500 font-mono mt-1 block">
          Requires review before merge
        </span>
      </div>

      <!-- Resolved Findings -->
      <div class="p-4 rounded-xl bg-[#12151f] border border-[#262c3f]">
        <span class="text-xs text-slate-400 font-mono block mb-1">Findings Resolved</span>
        <div class="flex items-baseline gap-2 font-mono">
          <span class="text-xl font-bold text-emerald-400">
            -{diffReport.resolvedFindings.length}
          </span>
          <span class="text-xs text-slate-500">fixed</span>
        </div>
        <span class="text-[11px] text-slate-500 font-mono mt-1 block">
          Cleaned up in this PR
        </span>
      </div>

      <!-- Unchanged Findings -->
      <div class="p-4 rounded-xl bg-[#12151f] border border-[#262c3f]">
        <span class="text-xs text-slate-400 font-mono block mb-1">Persistent Findings</span>
        <div class="flex items-baseline gap-2 font-mono">
          <span class="text-xl font-bold text-slate-200">
            {diffReport.unchangedFindings.length}
          </span>
          <span class="text-xs text-slate-500">unchanged</span>
        </div>
        <span class="text-[11px] text-slate-500 font-mono mt-1 block">
          Existing baseline issues
        </span>
      </div>
    </div>

    <!-- Dimension Breakdown Grid -->
    <div class="p-5 rounded-xl bg-[#12151f] border border-[#262c3f]">
      <h3 class="text-sm font-bold text-white font-mono mb-4">Engineering Dimension Shift</h3>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-xs font-mono">
        {#each diffReport.dimensionDeltas as dim (dim.dimension)}
          <div class="p-3 rounded-lg bg-[#090a0f] border border-[#262c3f]">
            <div class="flex justify-between items-center text-slate-400 mb-1">
              <span class="capitalize">{dim.dimension}</span>
              <span
                class="font-bold {dim.delta > 0
                  ? 'text-emerald-400'
                  : dim.delta < 0
                  ? 'text-red-400'
                  : 'text-slate-400'}"
              >
                {dim.delta >= 0 ? `+${dim.delta}` : dim.delta}
              </span>
            </div>
            <div class="flex items-center justify-between text-slate-300">
              <span>{dim.baseScore} → <strong class="text-white">{dim.compareScore}</strong></span>
              <span class="text-slate-500">/100</span>
            </div>
          </div>
        {/each}
      </div>
    </div>

    <!-- Findings Delta Detail Table -->
    <div class="p-5 rounded-xl bg-[#12151f] border border-[#262c3f] space-y-4">
      <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 pb-3 border-b border-[#262c3f]">
        <div class="flex items-center gap-1 bg-[#090a0f] p-1 rounded-lg border border-[#262c3f] text-xs font-mono">
          <button
            onclick={() => (activeTab = "added")}
            class="px-3 py-1 rounded {activeTab === 'added' ? 'bg-red-950 text-red-300 font-bold border border-red-800' : 'text-slate-400 hover:text-white'}"
          >
            New Added ({diffReport.addedFindings.length})
          </button>
          <button
            onclick={() => (activeTab = "resolved")}
            class="px-3 py-1 rounded {activeTab === 'resolved' ? 'bg-emerald-950 text-emerald-300 font-bold border border-emerald-800' : 'text-slate-400 hover:text-white'}"
          >
            Resolved ({diffReport.resolvedFindings.length})
          </button>
          <button
            onclick={() => (activeTab = "all")}
            class="px-3 py-1 rounded {activeTab === 'all' ? 'bg-slate-800 text-white font-bold' : 'text-slate-400 hover:text-white'}"
          >
            All Changes ({diffReport.addedFindings.length + diffReport.resolvedFindings.length})
          </button>
        </div>

        <!-- Severity Filter -->
        <div class="flex items-center gap-2 text-xs font-mono">
          <label for="compare-severity-filter" class="text-slate-400">Severity:</label>
          <select
            id="compare-severity-filter"
            bind:value={severityFilter}
            class="px-2.5 py-1 bg-[#090a0f] border border-[#262c3f] rounded text-slate-200 focus:outline-none focus:border-blue-500"
          >
            <option value="all">All Severities</option>
            <option value="critical">Critical</option>
            <option value="high">High</option>
            <option value="medium">Medium</option>
            <option value="low">Low</option>
          </select>
        </div>
      </div>

      <!-- Findings List -->
      {#if displayedFindings.length === 0}
        <div class="text-center py-8 text-xs text-slate-500 font-mono">
          No findings in this category.
        </div>
      {:else}
        <div class="divide-y divide-[#262c3f]/60 text-xs font-mono">
          {#each displayedFindings as finding (finding.id)}
            <div class="py-3 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2">
              <div class="space-y-1">
                <div class="flex items-center gap-2">
                  <span
                    class="px-2 py-0.5 rounded font-bold uppercase text-[10px] {finding.severity === 'critical'
                      ? 'bg-red-950 text-red-300 border border-red-800'
                      : finding.severity === 'high'
                      ? 'bg-orange-950 text-orange-300 border border-orange-800'
                      : finding.severity === 'medium'
                      ? 'bg-amber-950 text-amber-300 border border-amber-800'
                      : 'bg-blue-950 text-blue-300 border border-blue-800'}"
                  >
                    {finding.severity}
                  </span>
                  <span class="text-slate-400 font-bold">{finding.rule_id}</span>
                  <span class="text-white">{finding.title}</span>
                </div>
                {#if finding.location}
                  <p class="text-slate-400 text-[11px]">
                    📍 {finding.location.file_path}
                    {#if finding.location.start_line}
                      :{finding.location.start_line}
                    {/if}
                  </p>
                {/if}
              </div>

              {#if finding.evidence}
                <div class="max-w-xs truncate bg-[#090a0f] px-2 py-1 rounded text-slate-400 text-[11px] border border-[#262c3f]">
                  <code>{finding.evidence}</code>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {:else if !isLoading}
    <div class="p-12 rounded-xl border border-dashed border-[#262c3f] bg-[#12151f]/50 text-center text-xs font-mono text-slate-400">
      Select a repository, base branch, and PR compare branch above, then click <strong class="text-slate-200">"Inspect Diff"</strong> to visualize architectural drift and score shifts.
    </div>
  {/if}
</div>
