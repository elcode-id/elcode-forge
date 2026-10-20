<script lang="ts">
  import { onMount } from "svelte";
  import { dashboardState } from "$lib/data/store.svelte.js";
  import type { RuleDescriptor, Finding, Severity } from "@elcode-forge/schema";

  const ALL_CATEGORIES = ["all", "architecture", "quality", "security", "dependencies"];

  let selectedCategory = $state("all");
  let selectedPack = $state("all");
  let searchQuery = $state("");
  let enabledOnly = $state(false);
  let highlightedRuleId = $state<string | null>(null);

  const bundle = $derived(dashboardState.bundle);
  const rules = $derived<RuleDescriptor[]>(bundle?.rules ?? []);
  const allFindings = $derived<Finding[]>(bundle?.repository?.findings ?? []);

  // Compute violation count per rule ID
  const violationCounts = $derived.by<Record<string, number>>(() => {
    const counts: Record<string, number> = {};
    for (const f of allFindings) {
      if (f.rule_id) {
        counts[f.rule_id] = (counts[f.rule_id] || 0) + 1;
      }
    }
    return counts;
  });

  // Extract unique packs from rules
  const availablePacks = $derived.by<string[]>(() => {
    return rules
      .map((r) => r.source)
      .filter((s, idx, self) => Boolean(s) && self.indexOf(s) === idx)
      .sort();
  });

  // Filtered rules
  const filteredRules = $derived.by<RuleDescriptor[]>(() => {
    return rules.filter((r) => {
      if (selectedCategory !== "all") {
        const cat = r.category.toLowerCase();
        if (selectedCategory === "dependencies") {
          if (!cat.includes("dep")) return false;
        } else if (!cat.includes(selectedCategory)) {
          return false;
        }
      }
      if (selectedPack !== "all" && r.source !== selectedPack) {
        return false;
      }
      if (enabledOnly && !r.enabled) {
        return false;
      }
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matchesId = r.id.toLowerCase().includes(q);
        const matchesName = r.name.toLowerCase().includes(q);
        const matchesDesc = r.description.toLowerCase().includes(q);
        if (!matchesId && !matchesName && !matchesDesc) {
          return false;
        }
      }
      return true;
    });
  });

  // Stats
  const totalRules = $derived(rules.length);
  const enabledCount = $derived(rules.filter((r) => r.enabled).length);
  const customCount = $derived(rules.filter((r) => r.source.startsWith("custom:")).length);
  const totalViolations = $derived(
    rules.reduce((sum, r) => sum + (violationCounts[r.id] || 0), 0)
  );

  onMount(() => {
    if (typeof window !== "undefined") {
      const params = new URLSearchParams(window.location.search);
      const targetId = params.get("id") || params.get("rule");
      if (targetId) {
        highlightedRuleId = targetId;
        searchQuery = targetId;
      }
      const cat = params.get("category");
      if (cat && ALL_CATEGORIES.includes(cat.toLowerCase())) {
        selectedCategory = cat.toLowerCase();
      }
      const pack = params.get("pack");
      if (pack) {
        selectedPack = pack;
      }
    }
  });

  function severityColor(sev: Severity): string {
    switch (sev) {
      case "critical":
        return "bg-rose-950/80 text-rose-300 border-rose-800";
      case "high":
        return "bg-amber-950/80 text-amber-300 border-amber-800";
      case "medium":
        return "bg-yellow-950/80 text-yellow-300 border-yellow-800";
      case "low":
        return "bg-blue-950/80 text-blue-300 border-blue-800";
      default:
        return "bg-slate-800 text-slate-400 border-slate-700";
    }
  }

  function sourceBadgeColor(source: string): string {
    if (source === "builtin") {
      return "bg-slate-800 text-slate-300 border-slate-700";
    }
    if (source.startsWith("pack:")) {
      return "bg-purple-950/70 text-purple-300 border-purple-800";
    }
    if (source.startsWith("custom:")) {
      return "bg-teal-950/70 text-teal-300 border-teal-800";
    }
    return "bg-slate-800 text-slate-400 border-slate-700";
  }

  function resetFilters() {
    selectedCategory = "all";
    selectedPack = "all";
    searchQuery = "";
    enabledOnly = false;
    highlightedRuleId = null;
  }
</script>

<div class="space-y-6">
  <!-- Title & Top Summary Cards -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#262c3f]">
    <div>
      <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
        <span>Architectural & Custom Rules Catalog</span>
        <span class="px-2 py-0.5 rounded text-xs bg-slate-800 text-slate-300 font-mono">
          {filteredRules.length} / {totalRules}
        </span>
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        Declarative custom rules, extended rule packs, and deterministic built-in constraints.
      </p>
    </div>

    <!-- Quick Stats Summary -->
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-2 text-xs">
      <div class="px-3 py-2 rounded bg-[#12151f] border border-[#262c3f]">
        <div class="text-[10px] uppercase font-mono text-slate-500">Active Rules</div>
        <div class="text-base font-bold text-white font-mono">{totalRules}</div>
      </div>
      <div class="px-3 py-2 rounded bg-[#12151f] border border-[#262c3f]">
        <div class="text-[10px] uppercase font-mono text-slate-500">Enabled</div>
        <div class="text-base font-bold text-emerald-400 font-mono">{enabledCount}</div>
      </div>
      <div class="px-3 py-2 rounded bg-[#12151f] border border-[#262c3f]">
        <div class="text-[10px] uppercase font-mono text-slate-500">Custom Rules</div>
        <div class="text-base font-bold text-teal-300 font-mono">{customCount}</div>
      </div>
      <div class="px-3 py-2 rounded bg-[#12151f] border border-[#262c3f]">
        <div class="text-[10px] uppercase font-mono text-slate-500">Violations</div>
        <div class="text-base font-bold {totalViolations > 0 ? 'text-amber-400' : 'text-emerald-400'} font-mono">
          {totalViolations}
        </div>
      </div>
    </div>
  </div>

  <!-- Filter Controls Bar -->
  <div class="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-3 p-3 rounded-lg bg-[#12151f] border border-[#262c3f] text-xs">
    <div class="flex flex-wrap items-center gap-2">
      <!-- Category Tabs -->
      <span class="text-[11px] font-mono text-slate-500 uppercase mr-1">Category:</span>
      {#each ALL_CATEGORIES as cat (cat)}
        <button
          onclick={() => (selectedCategory = cat)}
          class="px-2.5 py-1 rounded capitalize font-mono text-[11px] transition-colors border {selectedCategory === cat
            ? 'bg-blue-600 border-blue-500 text-white font-semibold'
            : 'bg-[#1a1e2e] border-[#262c3f] text-slate-400 hover:text-slate-200 hover:bg-slate-800'}"
        >
          {cat}
        </button>
      {/each}
    </div>

    <!-- Pack Filter & Enabled Filter & Search -->
    <div class="flex flex-wrap items-center gap-2">
      <!-- Source Pack Select -->
      {#if availablePacks.length > 1}
        <select
          bind:value={selectedPack}
          class="px-2.5 py-1 rounded bg-[#090a0f] border border-[#262c3f] text-slate-300 font-mono text-xs focus:outline-none focus:border-blue-500"
        >
          <option value="all">All Sources / Packs</option>
          {#each availablePacks as pack (pack)}
            <option value={pack}>{pack}</option>
          {/each}
        </select>
      {/if}

      <!-- Enabled Only Toggle -->
      <label class="flex items-center gap-1.5 px-2.5 py-1 rounded bg-[#1a1e2e] border border-[#262c3f] cursor-pointer text-slate-300 font-mono text-xs hover:bg-slate-800">
        <input type="checkbox" bind:checked={enabledOnly} class="rounded bg-[#090a0f] border-[#262c3f]" />
        <span>Enabled only</span>
      </label>

      <!-- Search Input -->
      <div class="relative">
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Filter rules..."
          class="px-2.5 py-1 bg-[#090a0f] border border-[#262c3f] rounded font-mono text-xs text-slate-200 placeholder-slate-600 focus:outline-none focus:border-blue-500 w-44"
        />
        {#if searchQuery}
          <button
            onclick={() => (searchQuery = "")}
            class="absolute right-2 top-1 text-slate-500 hover:text-slate-300 text-xs"
          >
            ×
          </button>
        {/if}
      </div>

      {#if selectedCategory !== "all" || selectedPack !== "all" || searchQuery || enabledOnly}
        <button
          onclick={resetFilters}
          class="px-2 py-1 rounded text-slate-400 hover:text-white hover:bg-slate-800 font-mono text-[11px]"
        >
          Reset
        </button>
      {/if}
    </div>
  </div>

  <!-- Rules List -->
  {#if filteredRules.length === 0}
    <div class="p-8 text-center rounded-lg border border-[#262c3f] bg-[#12151f] text-slate-400 font-mono text-xs">
      {#if rules.length === 0}
        No rules loaded in this ReportBundle. Run <code class="text-blue-400">eforge report</code> on a repository to inspect active rules.
      {:else}
        No rules match the current filter criteria.
      {/if}
    </div>
  {:else}
    <div class="space-y-3">
      {#each filteredRules as rule (rule.id)}
        {@const count = violationCounts[rule.id] || 0}
        {@const isHighlighted = highlightedRuleId === rule.id}
        <div
          class="rounded-lg border {isHighlighted
            ? 'border-blue-500 bg-blue-950/20'
            : 'border-[#262c3f] bg-[#12151f]'} p-4 text-xs space-y-3 transition-colors hover:border-slate-700"
        >
          <!-- Rule Header -->
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span class="px-2 py-0.5 rounded text-[10px] font-mono uppercase font-bold border {severityColor(rule.severity)}">
                {rule.severity}
              </span>
              <span class="font-mono text-slate-200 font-semibold text-xs">{rule.id}</span>
              <span class="text-slate-600">•</span>
              <span class="px-2 py-0.5 rounded text-[10px] font-mono border {sourceBadgeColor(rule.source)}">
                {rule.source}
              </span>
              <span class="px-2 py-0.5 rounded text-[10px] font-mono border border-slate-700 text-slate-400 uppercase">
                {rule.category}
              </span>
              {#if !rule.enabled}
                <span class="px-2 py-0.5 rounded text-[10px] font-mono bg-slate-800 text-slate-500 border border-slate-700">
                  DISABLED
                </span>
              {/if}
            </div>

            <!-- Violations Link Badge -->
            <div class="flex items-center gap-2">
              {#if count > 0}
                <a
                  href="/findings?rule_prefix={encodeURIComponent(rule.id)}"
                  class="flex items-center gap-1.5 px-2.5 py-1 rounded bg-amber-950/50 hover:bg-amber-900/60 border border-amber-800/80 text-amber-300 font-mono text-[11px] transition-colors"
                >
                  <span class="w-1.5 h-1.5 rounded-full bg-amber-400"></span>
                  <span>{count} {count === 1 ? 'violation' : 'violations'}</span>
                  <span>→</span>
                </a>
              {:else}
                <span class="flex items-center gap-1.5 px-2.5 py-1 rounded bg-emerald-950/30 border border-emerald-900/50 text-emerald-400 font-mono text-[11px]">
                  <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
                  <span>Clean (0 violations)</span>
                </span>
              {/if}
            </div>
          </div>

          <!-- Rule Name & Description -->
          <div>
            <h2 class="text-sm font-semibold text-slate-100">{rule.name}</h2>
            <p class="text-slate-300 mt-1 leading-relaxed">{rule.description}</p>
          </div>

          <!-- Remediation Guidance -->
          {#if rule.remediation}
            <div class="p-2.5 rounded bg-blue-950/20 border border-blue-900/30 text-blue-200">
              <span class="font-semibold text-blue-300">Remediation:</span>
              <span class="ml-1.5">{rule.remediation}</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
