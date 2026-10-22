<script lang="ts">
  import { onMount } from "svelte";
  import { dashboardState } from "$lib/data/store.svelte.js";
  import {
    parseFilter,
    serializeFilter,
    applyFilter,
    detectFileLanguage,
    type FindingFilter,
  } from "$lib/filters.js";
  import type { Finding, Severity } from "@elcode-forge/schema";

  const ALL_SEVERITIES: Severity[] = ["critical", "high", "medium", "low", "info"];

  const bundle = $derived(dashboardState.bundle);
  const allFindings = $derived<Finding[]>(bundle?.repository?.findings ?? []);

  let selectedSeverities = $state<Severity[]>([]);
  let rulePrefix = $state("");
  let pathPrefix = $state("");
  let selectedLanguage = $state<string>("all");

  const presentLanguages = $derived<string[]>(
    Array.from(
      new Set(
        allFindings
          .map((f) => detectFileLanguage(f.location?.file_path))
          .filter((l): l is string => Boolean(l))
      )
    ).sort()
  );

  function getLanguageCount(lang: string): number {
    return allFindings.filter((f) => detectFileLanguage(f.location?.file_path) === lang).length;
  }

  function languageBadgeClass(lang?: string | null): string {
    switch (lang?.toLowerCase()) {
      case "python":
        return "bg-sky-950/80 text-sky-300 border-sky-800";
      case "go":
        return "bg-teal-950/80 text-teal-300 border-teal-800";
      case "rust":
        return "bg-amber-950/80 text-amber-300 border-amber-800";
      case "php":
        return "bg-purple-950/80 text-purple-300 border-purple-800";
      case "typescript":
      case "javascript":
        return "bg-blue-950/80 text-blue-300 border-blue-800";
      case "svelte":
      case "vue":
        return "bg-emerald-950/80 text-emerald-300 border-emerald-800";
      default:
        return "bg-slate-800 text-slate-400 border-slate-700";
    }
  }

  const activeFilter = $derived<FindingFilter>({
    severity: selectedSeverities,
    confidence: [],
    rulePrefix: rulePrefix || undefined,
    pathPrefix: pathPrefix || undefined,
    language: selectedLanguage !== "all" ? selectedLanguage : undefined,
  });

  const filteredFindings = $derived<Finding[]>(applyFilter(allFindings, activeFilter));

  onMount(() => {
    if (typeof window !== "undefined") {
      const params = new URLSearchParams(window.location.search);
      const parsed = parseFilter(params);
      selectedSeverities = parsed.severity;
      rulePrefix = parsed.rulePrefix ?? "";
      pathPrefix = parsed.pathPrefix ?? "";
      selectedLanguage = parsed.language ?? "all";
    }
  });

  $effect(() => {
    if (typeof window !== "undefined") {
      const params = serializeFilter(activeFilter);
      const queryString = params.toString();
      const newUrl = queryString ? `${window.location.pathname}?${queryString}` : window.location.pathname;
      window.history.replaceState(null, "", newUrl);
    }
  });

  function toggleSeverity(sev: Severity) {
    if (selectedSeverities.includes(sev)) {
      selectedSeverities = selectedSeverities.filter((s) => s !== sev);
    } else {
      selectedSeverities = [...selectedSeverities, sev];
    }
  }

  function resetFilters() {
    selectedSeverities = [];
    rulePrefix = "";
    pathPrefix = "";
    selectedLanguage = "all";
  }

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
</script>

<div class="space-y-6">
  <!-- Title & Counts -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#262c3f]">
    <div>
      <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
        <span>Findings & Diagnostics</span>
        <span class="px-2 py-0.5 rounded text-xs bg-slate-800 text-slate-300 font-mono">
          {filteredFindings.length} / {allFindings.length}
        </span>
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        Deterministic architectural violations, code quality findings, and security issues.
      </p>
    </div>

    <!-- Severity Filter Buttons -->
    <div class="flex flex-wrap items-center gap-1.5 text-xs">
      {#each ALL_SEVERITIES as sev (sev)}
        <button
          onclick={() => toggleSeverity(sev)}
          class="px-2.5 py-1 rounded font-mono uppercase text-[11px] border transition-colors {selectedSeverities.includes(sev)
            ? severityColor(sev) + ' font-bold'
            : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
        >
          {sev}
        </button>
      {/each}

      {#if selectedSeverities.length > 0 || rulePrefix || pathPrefix}
        <button
          onclick={resetFilters}
          class="px-2.5 py-1 rounded bg-[#1a1e2e] hover:bg-slate-800 border border-[#262c3f] text-[11px] text-slate-300 transition-colors ml-1"
        >
          Reset
        </button>
      {/if}
    </div>
  </div>

  <!-- Search Filter Inputs -->
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
    <div>
      <label for="rule-prefix-filter" class="block text-slate-400 mb-1 font-mono text-[11px]">Filter by Rule ID prefix (e.g. arch/, security/, deps/)</label>
      <input
        id="rule-prefix-filter"
        type="text"
        bind:value={rulePrefix}
        placeholder="arch/ or security/"
        class="w-full px-3 py-1.5 bg-[#12151f] border border-[#262c3f] rounded text-slate-200 font-mono text-xs focus:outline-none focus:border-blue-500"
      />
    </div>
    <div>
      <label for="path-prefix-filter" class="block text-slate-400 mb-1 font-mono text-[11px]">Filter by File Path</label>
      <input
        id="path-prefix-filter"
        type="text"
        bind:value={pathPrefix}
        placeholder="src/components/ or /db/"
        class="w-full px-3 py-1.5 bg-[#12151f] border border-[#262c3f] rounded text-slate-200 font-mono text-xs focus:outline-none focus:border-blue-500"
      />
    </div>
  </div>

  <!-- Language Filter Pills -->
  {#if presentLanguages.length > 1}
    <div class="flex flex-wrap items-center gap-1.5 text-xs">
      <span class="text-slate-400 font-mono text-[11px] mr-1">Language:</span>
      <button
        onclick={() => (selectedLanguage = "all")}
        class="px-2 py-0.5 rounded font-mono text-[11px] border transition-colors {selectedLanguage === 'all'
          ? 'bg-blue-600 text-white border-blue-500 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        All ({allFindings.length})
      </button>
      {#each presentLanguages as lang (lang)}
        <button
          onclick={() => (selectedLanguage = lang)}
          class="px-2 py-0.5 rounded font-mono text-[11px] uppercase border transition-colors {selectedLanguage === lang
            ? languageBadgeClass(lang) + ' font-medium'
            : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
        >
          {lang} ({getLanguageCount(lang)})
        </button>
      {/each}
    </div>
  {/if}

  <!-- Findings List -->
  {#if !bundle}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center text-slate-400 text-sm">
      No report loaded.
    </div>
  {:else if filteredFindings.length === 0}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center text-slate-400 text-sm">
      {#if allFindings.length === 0}
        Zero findings! Repository is completely clean across all rules.
      {:else}
        No findings match the current filter criteria.
      {/if}
    </div>
  {:else}
    <div class="space-y-3">
      {#each filteredFindings as finding (finding.id)}
        <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4 text-xs space-y-3 transition-colors hover:border-slate-700">
          <!-- Finding Header -->
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div class="flex items-center gap-2">
              <span class="px-2 py-0.5 rounded text-[10px] font-mono uppercase font-bold border {severityColor(finding.severity)}">
                {finding.severity}
              </span>
              <a
                href="/rules?id={encodeURIComponent(finding.rule_id)}"
                class="font-mono text-blue-400 hover:text-blue-300 hover:underline text-[11px]"
                title="View rule definition in catalog"
              >
                {finding.rule_id}
              </a>
              <span class="text-slate-600">•</span>
              <span class="text-slate-400 font-mono text-[10px] uppercase">Confidence: {finding.confidence}</span>
            </div>

            {#if finding.location?.file_path}
              {@const fileLang = detectFileLanguage(finding.location.file_path)}
              <div class="flex items-center gap-1.5 font-mono text-[11px] text-slate-400">
                {#if fileLang}
                  <span class="px-1.5 py-0.2 rounded text-[10px] font-mono uppercase border {languageBadgeClass(fileLang)}">
                    {fileLang}
                  </span>
                {/if}
                <span>{finding.location.file_path}</span>
                {#if finding.location.start_line}
                  <span class="text-slate-500">:{finding.location.start_line}{finding.location.end_line ? `-${finding.location.end_line}` : ''}</span>
                {/if}
              </div>
            {/if}
          </div>

          <!-- Title & Description -->
          <div>
            <h2 class="text-sm font-semibold text-slate-100">{finding.title}</h2>
            <p class="text-slate-300 mt-1 leading-relaxed">{finding.description}</p>
          </div>

          <!-- Plain-Text Redacted Evidence Envelope -->
          {#if finding.evidence}
            <div class="space-y-1">
              <div class="text-[10px] uppercase font-mono text-slate-400">Deterministic Evidence (Hostile Envelope):</div>
              <pre class="p-2.5 rounded bg-[#090a0f] border border-[#262c3f] text-slate-300 font-mono text-[11px] overflow-x-auto whitespace-pre-wrap break-all">{finding.evidence}</pre>
            </div>
          {/if}

          <!-- Actionable Recommendation -->
          {#if finding.recommendation}
            <div class="p-2.5 rounded bg-blue-950/20 border border-blue-900/30 text-blue-200">
              <span class="font-semibold text-blue-300">Recommendation:</span>
              <span class="ml-1.5">{finding.recommendation}</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
