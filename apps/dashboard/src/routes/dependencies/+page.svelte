<script lang="ts">
  import { dashboardState } from "$lib/data/store.svelte.js";
  import { categorizeLicense, getLicenseBadgeClass, type LicenseCategory } from "$lib/data/license.js";
  import { downloadFile, generateClientCycloneDx, generateClientSpdx } from "$lib/data/sbom.js";
  import type { Dependency, Finding } from "@elcode-forge/schema";

  const bundle = $derived(dashboardState.bundle);
  const dependencies = $derived<Dependency[]>(bundle?.repository?.dependencies ?? []);
  const allFindings = $derived<Finding[]>(bundle?.repository?.findings ?? []);

  // Filter dependency-related findings (deps/* and license/*)
  const depFindings = $derived<Finding[]>(
    allFindings.filter((f) => f.rule_id.startsWith("deps/") || f.rule_id.startsWith("license/"))
  );

  // Map packages to their warning finding if present
  const depFindingMap = $derived.by<Record<string, Finding[]>>(() => {
    const map: Record<string, Finding[]> = {};
    for (const f of depFindings) {
      for (const d of dependencies) {
        if (
          f.evidence?.includes(d.name) ||
          f.description?.includes(d.name) ||
          f.title?.includes(d.name)
        ) {
          const list = map[d.name] || [];
          list.push(f);
          map[d.name] = list;
        }
      }
    }
    return map;
  });

  // Infer dependency license from findings evidence or fallbacks
  function getDependencyLicense(dep: Dependency): string {
    const findings = depFindingMap[dep.name] || [];
    for (const f of findings) {
      const match = f.evidence?.match(/License: ([^\s|()]+)/);
      if (match) return match[1];
    }
    // Catalog heuristics for UI display
    switch (dep.name.toLowerCase()) {
      case "lodash":
      case "minimist":
      case "react":
      case "react-dom":
      case "vue":
      case "axios":
      case "chalk":
      case "express":
      case "gin":
      case "guzzle":
      case "guzzlehttp/guzzle":
      case "github.com/gin-gonic/gin":
        return "MIT";
      case "requests":
      case "urllib3":
      case "typescript":
      case "cryptography":
        return "Apache-2.0";
      case "flask":
      case "django":
      case "github.com/gorilla/mux":
        return "BSD-3-Clause";
      case "serde":
      case "tokio":
        return "MIT OR Apache-2.0";
      default:
        return "Unknown";
    }
  }

  let searchQuery = $state("");
  let filterType: "all" | "direct" | "dev" | "flagged" = $state("all");
  let selectedEcosystem: string = $state("all");
  let selectedLicenseCategory: "all" | LicenseCategory = $state("all");

  const presentEcosystems = $derived<string[]>(
    Array.from(
      new Set(
        dependencies
          .map((d) => d.ecosystem?.toLowerCase())
          .filter((e): e is string => Boolean(e))
      )
    ).sort()
  );

  function getEcosystemCount(eco: string): number {
    return dependencies.filter((d) => d.ecosystem?.toLowerCase() === eco.toLowerCase()).length;
  }

  function formatEcosystem(eco: string): string {
    switch (eco.toLowerCase()) {
      case "npm":
        return "npm";
      case "pypi":
        return "PyPI";
      case "go":
        return "Go";
      case "cargo":
        return "Cargo";
      case "composer":
        return "Composer";
      default:
        return eco;
    }
  }

  function ecosystemBadgeClass(eco?: string): string {
    switch (eco?.toLowerCase()) {
      case "npm":
        return "bg-rose-950/70 text-rose-300 border-rose-800/80";
      case "pypi":
        return "bg-sky-950/70 text-sky-300 border-sky-800/80";
      case "go":
        return "bg-teal-950/70 text-teal-300 border-teal-800/80";
      case "cargo":
        return "bg-amber-950/70 text-amber-300 border-amber-800/80";
      case "composer":
        return "bg-indigo-950/70 text-indigo-300 border-indigo-800/80";
      default:
        return "bg-slate-800 text-slate-300 border-slate-700";
    }
  }

  // License category metrics
  const licenseStats = $derived.by(() => {
    let permissive = 0;
    let weakCopyleft = 0;
    let strongCopyleft = 0;
    let proprietary = 0;
    let unknown = 0;

    for (const d of dependencies) {
      const lic = getDependencyLicense(d);
      const cat = categorizeLicense(lic);
      if (cat === "Permissive") permissive++;
      else if (cat === "WeakCopyleft") weakCopyleft++;
      else if (cat === "StrongCopyleft") strongCopyleft++;
      else if (cat === "Proprietary") proprietary++;
      else unknown++;
    }

    const total = dependencies.length || 1;
    return {
      permissive,
      weakCopyleft,
      strongCopyleft,
      proprietary,
      unknown,
      permissivePct: Math.round((permissive / total) * 100),
      copyleftPct: Math.round(((weakCopyleft + strongCopyleft) / total) * 100),
      proprietaryPct: Math.round((proprietary / total) * 100),
      unknownPct: Math.round((unknown / total) * 100),
    };
  });

  const directCount = $derived<number>(dependencies.filter((d) => d.is_direct && !d.is_dev).length);
  const devCount = $derived<number>(dependencies.filter((d) => d.is_dev).length);
  const flaggedCount = $derived<number>(
    dependencies.filter((d) => (depFindingMap[d.name]?.length ?? 0) > 0).length
  );

  const filteredDependencies = $derived<Dependency[]>(
    dependencies.filter((d) => {
      if (selectedEcosystem !== "all" && d.ecosystem?.toLowerCase() !== selectedEcosystem.toLowerCase()) {
        return false;
      }
      if (searchQuery && !d.name.toLowerCase().includes(searchQuery.toLowerCase())) {
        return false;
      }
      if (filterType === "direct" && (!d.is_direct || d.is_dev)) {
        return false;
      }
      if (filterType === "dev" && !d.is_dev) {
        return false;
      }
      if (filterType === "flagged" && (depFindingMap[d.name]?.length ?? 0) === 0) {
        return false;
      }
      if (selectedLicenseCategory !== "all") {
        const lic = getDependencyLicense(d);
        const cat = categorizeLicense(lic);
        if (cat !== selectedLicenseCategory) return false;
      }
      return true;
    })
  );

  // SBOM Export Handlers
  function exportCycloneDx() {
    if (!bundle) return;
    const json = generateClientCycloneDx(bundle);
    const projName = bundle.repository?.name || "project";
    downloadFile(`${projName}-cyclonedx.json`, json);
  }

  function exportSpdx() {
    if (!bundle) return;
    const json = generateClientSpdx(bundle);
    const projName = bundle.repository?.name || "project";
    downloadFile(`${projName}-spdx.json`, json);
  }
</script>

<div class="space-y-6">
  <!-- Title & Controls Header -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#262c3f]">
    <div>
      <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
        <span>Package Dependencies & License Compliance</span>
        <span class="px-2 py-0.5 rounded text-xs bg-slate-800 text-slate-300 font-mono">
          {dependencies.length} Total
        </span>
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        Multi-ecosystem supply chain inventory across npm, PyPI, Go, Cargo, and Composer with one-click SBOM export.
      </p>
    </div>

    <!-- Right Actions: SBOM Exporters & Type Filter -->
    <div class="flex flex-wrap items-center gap-2">
      <div class="inline-flex rounded-md border border-[#262c3f] bg-[#12151f] p-0.5 text-xs">
        <button
          onclick={exportCycloneDx}
          class="px-2.5 py-1 rounded text-slate-300 hover:text-white hover:bg-[#1a1e2e] transition-colors font-mono text-[11px] flex items-center gap-1"
          title="Download CycloneDX v1.5 JSON SBOM"
        >
          <span>CycloneDX</span>
          <span class="text-[9px] text-slate-500">JSON</span>
        </button>
        <span class="w-[1px] bg-[#262c3f] my-1"></span>
        <button
          onclick={exportSpdx}
          class="px-2.5 py-1 rounded text-slate-300 hover:text-white hover:bg-[#1a1e2e] transition-colors font-mono text-[11px] flex items-center gap-1"
          title="Download SPDX v2.3 JSON SBOM"
        >
          <span>SPDX 2.3</span>
          <span class="text-[9px] text-slate-500">JSON</span>
        </button>
      </div>

      <!-- Filter Buttons: Type (Direct/Dev/Flagged) -->
      <div class="inline-flex rounded-md border border-[#262c3f] bg-[#12151f] p-0.5 text-xs">
        <button
          onclick={() => (filterType = "all")}
          class="px-2.5 py-1 rounded transition-colors {filterType === 'all' ? 'bg-blue-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
        >
          All ({dependencies.length})
        </button>
        <button
          onclick={() => (filterType = "direct")}
          class="px-2.5 py-1 rounded transition-colors {filterType === 'direct' ? 'bg-blue-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
        >
          Direct ({directCount})
        </button>
        <button
          onclick={() => (filterType = "dev")}
          class="px-2.5 py-1 rounded transition-colors {filterType === 'dev' ? 'bg-blue-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
        >
          Dev ({devCount})
        </button>
        <button
          onclick={() => (filterType = "flagged")}
          class="px-2.5 py-1 rounded transition-colors {filterType === 'flagged' ? 'bg-amber-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
        >
          Flagged ({flaggedCount})
        </button>
      </div>
    </div>
  </div>

  <!-- Open-Source License Distribution Bar & Summary -->
  <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4 text-xs space-y-3">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
      <div class="text-[11px] font-mono uppercase text-slate-400 font-bold">Open-Source License Distribution</div>
      <div class="flex items-center gap-3 text-[11px] font-mono">
        <span class="text-emerald-400">Permissive: {licenseStats.permissive} ({licenseStats.permissivePct}%)</span>
        <span class="text-rose-400">Copyleft: {licenseStats.weakCopyleft + licenseStats.strongCopyleft} ({licenseStats.copyleftPct}%)</span>
        <span class="text-purple-400">Proprietary: {licenseStats.proprietary}</span>
        <span class="text-slate-500">Unknown: {licenseStats.unknown}</span>
      </div>
    </div>

    <!-- Progress Stack Bar -->
    <div class="w-full h-2.5 rounded-full bg-[#1a1e2e] flex overflow-hidden">
      {#if licenseStats.permissive > 0}
        <div style="width: {licenseStats.permissivePct}%" class="h-full bg-emerald-500" title="Permissive"></div>
      {/if}
      {#if licenseStats.weakCopyleft > 0}
        <div style="width: {Math.round((licenseStats.weakCopyleft / (dependencies.length || 1)) * 100)}%" class="h-full bg-amber-500" title="Weak Copyleft"></div>
      {/if}
      {#if licenseStats.strongCopyleft > 0}
        <div style="width: {Math.round((licenseStats.strongCopyleft / (dependencies.length || 1)) * 100)}%" class="h-full bg-rose-500" title="Strong Copyleft"></div>
      {/if}
      {#if licenseStats.proprietary > 0}
        <div style="width: {licenseStats.proprietaryPct}%" class="h-full bg-purple-500" title="Proprietary"></div>
      {/if}
      {#if licenseStats.unknown > 0}
        <div style="width: {licenseStats.unknownPct}%" class="h-full bg-slate-700" title="Unknown"></div>
      {/if}
    </div>
  </div>

  <!-- Search & Filter Toolbar -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <!-- Search Box -->
    <div class="w-full sm:max-w-xs">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search package name..."
        class="w-full px-3 py-1.5 bg-[#12151f] border border-[#262c3f] rounded text-slate-200 font-mono text-xs focus:outline-none focus:border-blue-500"
      />
    </div>

    <!-- License Category Filter -->
    <div class="flex flex-wrap items-center gap-1.5 text-xs">
      <span class="text-slate-400 font-mono text-[11px] mr-1">License:</span>
      <button
        onclick={() => (selectedLicenseCategory = "all")}
        class="px-2 py-0.5 rounded font-mono text-[11px] border transition-colors {selectedLicenseCategory === 'all'
          ? 'bg-blue-600 text-white border-blue-500 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        All
      </button>
      <button
        onclick={() => (selectedLicenseCategory = "Permissive")}
        class="px-2 py-0.5 rounded font-mono text-[11px] border transition-colors {selectedLicenseCategory === 'Permissive'
          ? 'bg-emerald-950 text-emerald-300 border-emerald-800 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        Permissive
      </button>
      <button
        onclick={() => (selectedLicenseCategory = "StrongCopyleft")}
        class="px-2 py-0.5 rounded font-mono text-[11px] border transition-colors {selectedLicenseCategory === 'StrongCopyleft'
          ? 'bg-rose-950 text-rose-300 border-rose-800 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        Strong Copyleft
      </button>
      <button
        onclick={() => (selectedLicenseCategory = "WeakCopyleft")}
        class="px-2 py-0.5 rounded font-mono text-[11px] border transition-colors {selectedLicenseCategory === 'WeakCopyleft'
          ? 'bg-amber-950 text-amber-300 border-amber-800 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        Weak Copyleft
      </button>
    </div>

    <!-- Ecosystem Filter Badges -->
    {#if presentEcosystems.length > 1}
      <div class="flex flex-wrap items-center gap-1.5 text-xs">
        <span class="text-slate-400 font-mono text-[11px] mr-1">Ecosystem:</span>
        <button
          onclick={() => (selectedEcosystem = "all")}
          class="px-2 py-0.5 rounded font-mono text-[11px] border transition-colors {selectedEcosystem === 'all'
            ? 'bg-blue-600 text-white border-blue-500 font-medium'
            : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
        >
          All ({dependencies.length})
        </button>
        {#each presentEcosystems as eco (eco)}
          <button
            onclick={() => (selectedEcosystem = eco)}
            class="px-2 py-0.5 rounded font-mono text-[11px] border transition-colors {selectedEcosystem === eco
              ? ecosystemBadgeClass(eco) + ' font-medium'
              : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
          >
            {formatEcosystem(eco)} ({getEcosystemCount(eco)})
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if !bundle}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center text-slate-400 text-sm">
      No report loaded.
    </div>
  {:else if filteredDependencies.length === 0}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center text-slate-400 text-sm">
      No dependencies found matching current criteria.
    </div>
  {:else}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] overflow-hidden">
      <table class="w-full text-left text-xs text-slate-300" aria-label="Dependencies table">
        <thead class="bg-[#1a1e2e] text-slate-400 uppercase font-mono text-[10px] tracking-wider border-b border-[#262c3f]">
          <tr>
            <th scope="col" class="py-2.5 px-4">Package</th>
            <th scope="col" class="py-2.5 px-3">Ecosystem</th>
            <th scope="col" class="py-2.5 px-3">Requirement</th>
            <th scope="col" class="py-2.5 px-3">Resolved</th>
            <th scope="col" class="py-2.5 px-3">License</th>
            <th scope="col" class="py-2.5 px-3">Type</th>
            <th scope="col" class="py-2.5 px-3">Health Status</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[#262c3f]/50">
          {#each filteredDependencies as dep (dep.name + dep.ecosystem + String(dep.is_dev))}
            {@const findings = depFindingMap[dep.name] || []}
            {@const lic = getDependencyLicense(dep)}
            {@const cat = categorizeLicense(lic)}
            <tr class="hover:bg-[#1a1e2e]/40">
              <td class="py-2.5 px-4 font-mono font-medium text-slate-100">{dep.name}</td>
              <td class="py-2.5 px-3">
                <span class="px-2 py-0.5 rounded text-[10px] font-mono border {ecosystemBadgeClass(dep.ecosystem)}">
                  {formatEcosystem(dep.ecosystem)}
                </span>
              </td>
              <td class="py-2.5 px-3 font-mono text-slate-300">{dep.version_req}</td>
              <td class="py-2.5 px-3 font-mono text-slate-400">{dep.resolved_version || "—"}</td>
              <td class="py-2.5 px-3">
                <span class="px-2 py-0.5 rounded text-[10px] font-mono border {getLicenseBadgeClass(cat)}">
                  {lic}
                </span>
              </td>
              <td class="py-2.5 px-3">
                {#if dep.is_dev}
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-purple-950 text-purple-300 border border-purple-900">DEV</span>
                {:else if dep.is_direct}
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-blue-950 text-blue-300 border border-blue-900">DIRECT</span>
                {:else}
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-slate-800 text-slate-400 border border-slate-700">INDIRECT</span>
                {/if}
              </td>
              <td class="py-2.5 px-3">
                {#if findings.length === 0}
                  <span class="text-emerald-400 font-mono text-[10px]">HEALTHY</span>
                {:else}
                  <div class="flex flex-wrap items-center gap-1">
                    {#each findings as f (f.id)}
                      <span
                        class="px-1.5 py-0.5 rounded text-[10px] font-mono {f.rule_id.startsWith('license/') ? 'bg-purple-950 text-purple-300 border border-purple-800' : 'bg-amber-950 text-amber-300 border border-amber-800'}"
                        title={f.description}
                      >
                        {f.rule_id.replace("deps/", "").replace("license/", "")}
                      </span>
                    {/each}
                  </div>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
