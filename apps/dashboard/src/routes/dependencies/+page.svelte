<script lang="ts">
  import { dashboardState } from "$lib/data/store.svelte.js";
  import type { Dependency, Finding } from "@elcode-forge/schema";

  const bundle = $derived(dashboardState.bundle);
  const dependencies = $derived<Dependency[]>(bundle?.repository?.dependencies ?? []);
  const allFindings = $derived<Finding[]>(bundle?.repository?.findings ?? []);

  // Filter dependency-related findings
  const depFindings = $derived<Finding[]>(
    allFindings.filter((f) => f.rule_id.startsWith("deps/"))
  );

  // Map packages to their warning finding if present
  const depFindingMap = $derived.by<Record<string, Finding[]>>(() => {
    const map: Record<string, Finding[]> = {};
    for (const f of depFindings) {
      // Finding titles/descriptions or evidence usually mention the dependency name
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

  let searchQuery = $state("");
  let filterType: "all" | "direct" | "dev" | "flagged" = $state("all");

  const directCount = $derived<number>(dependencies.filter((d) => d.is_direct && !d.is_dev).length);
  const devCount = $derived<number>(dependencies.filter((d) => d.is_dev).length);
  const flaggedCount = $derived<number>(
    dependencies.filter((d) => (depFindingMap[d.name]?.length ?? 0) > 0).length
  );

  const filteredDependencies = $derived<Dependency[]>(
    dependencies.filter((d) => {
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
      return true;
    })
  );
</script>

<div class="space-y-6">
  <!-- Title & Controls Header -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#262c3f]">
    <div>
      <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
        <span>Package Dependencies & Health</span>
        <span class="px-2 py-0.5 rounded text-xs bg-slate-800 text-slate-300 font-mono">
          {dependencies.length} Total
        </span>
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        Supply chain inventory, unpinned dependencies, unused packages, and duplication diagnostics.
      </p>
    </div>

    <!-- Filter Buttons -->
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

  <!-- Search Box -->
  <div class="max-w-md">
    <input
      type="text"
      bind:value={searchQuery}
      placeholder="Search package name..."
      class="w-full px-3 py-1.5 bg-[#12151f] border border-[#262c3f] rounded text-slate-200 font-mono text-xs focus:outline-none focus:border-blue-500"
    />
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
            <th scope="col" class="py-2.5 px-3">Type</th>
            <th scope="col" class="py-2.5 px-3">Health Status</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[#262c3f]/50">
          {#each filteredDependencies as dep (dep.name + dep.ecosystem + String(dep.is_dev))}
            {@const findings = depFindingMap[dep.name] || []}
            <tr class="hover:bg-[#1a1e2e]/40">
              <td class="py-2.5 px-4 font-mono font-medium text-slate-100">{dep.name}</td>
              <td class="py-2.5 px-3 font-mono text-slate-400">{dep.ecosystem}</td>
              <td class="py-2.5 px-3 font-mono text-slate-300">{dep.version_req}</td>
              <td class="py-2.5 px-3 font-mono text-slate-400">{dep.resolved_version || "—"}</td>
              <td class="py-2.5 px-3">
                {#if dep.is_dev}
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-slate-800 text-slate-400">DEV</span>
                {:else if dep.is_direct}
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-blue-950 text-blue-300 border border-blue-900">DIRECT</span>
                {:else}
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-slate-800 text-slate-500">TRANSITIVE</span>
                {/if}
              </td>
              <td class="py-2.5 px-3">
                {#if findings.length === 0}
                  <span class="text-emerald-400 font-mono text-[10px]">HEALTHY</span>
                {:else}
                  <div class="flex flex-wrap items-center gap-1">
                    {#each findings as f (f.id)}
                      <span
                        class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-amber-950 text-amber-300 border border-amber-800"
                        title={f.description}
                      >
                        {f.rule_id.replace("deps/", "")}
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
