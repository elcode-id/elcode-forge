<script lang="ts">
  import { onMount } from "svelte";
  import type { Core } from "cytoscape";
  import { dashboardState } from "$lib/data/store.svelte.js";
  import { toElements, type GraphNodeData } from "$lib/graph/toElements.js";
  import { initCytoscape } from "$lib/graph/cytoscape.js";
  import type { Finding } from "@elcode-forge/schema";

  let containerEl: HTMLDivElement | null = $state(null);
  let cyInstance: Core | null = $state(null);
  let selectedNodeId: string | null = $state(null);
  let viewMode: "canvas" | "list" = $state("canvas");

  const bundle = $derived(dashboardState.bundle);
  const elements = $derived(bundle ? toElements(bundle) : { nodes: [], edges: [] });
  const selectedNode = $derived<GraphNodeData | null>(
    selectedNodeId
      ? elements.nodes.find((n) => n.data.id === selectedNodeId)?.data ?? null
      : null
  );

  // Inbound edges: modules that import this selected module
  const importedBy = $derived<string[]>(
    selectedNodeId
      ? elements.edges
          .filter((e) => e.data.target === selectedNodeId)
          .map((e) => e.data.source)
      : []
  );

  // Related findings for selected module
  const relatedFindings = $derived<Finding[]>(
    bundle && selectedNode
      ? (bundle.repository?.findings ?? []).filter((f: Finding) => {
          if (!f.location?.file_path) return false;
          return selectedNode.files.some((file) =>
            f.location?.file_path.includes(file) || file.includes(f.location?.file_path ?? "")
          );
        })
      : []
  );

  function mountCytoscape() {
    if (!containerEl || !bundle) return;
    if (cyInstance) {
      cyInstance.destroy();
      cyInstance = null;
    }

    cyInstance = initCytoscape({
      container: containerEl,
      elements,
      onSelectNode: (id) => {
        selectedNodeId = id;
      },
    });
  }

  onMount(() => {
    mountCytoscape();
    return () => {
      if (cyInstance) {
        cyInstance.destroy();
        cyInstance = null;
      }
    };
  });

  $effect(() => {
    // Re-mount or re-render if bundle/elements change
    if (elements && containerEl && viewMode === "canvas") {
      mountCytoscape();
    }
  });

  function handleSelectNode(nodeId: string) {
    selectedNodeId = nodeId;
    if (cyInstance) {
      cyInstance.$(":selected").unselect();
      const target = cyInstance.$(`#${nodeId}`);
      if (target.length) {
        target.select();
        cyInstance.center(target);
      }
    }
  }

  function handleFit() {
    if (cyInstance) {
      cyInstance.fit(undefined, 30);
    }
  }
</script>

<div class="space-y-6">
  <!-- Title & Controls Header -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#262c3f]">
    <div>
      <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
        <span>Architecture & Dependency Graph</span>
        {#if bundle?.architecture?.circular_dependencies?.length}
          <span class="px-2 py-0.5 rounded text-xs bg-red-950/80 text-red-300 border border-red-800">
            {bundle.architecture.circular_dependencies.length} Circular Dependencies
          </span>
        {/if}
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        Interactive module dependency structure, circular cycles, and deep module inspection.
      </p>
    </div>

    <div class="flex items-center gap-2">
      <!-- Toggle between Canvas and Accessible Table -->
      <div class="inline-flex rounded-md border border-[#262c3f] bg-[#12151f] p-0.5 text-xs">
        <button
          onclick={() => (viewMode = "canvas")}
          class="px-3 py-1 rounded transition-colors {viewMode === 'canvas' ? 'bg-blue-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
          aria-pressed={viewMode === "canvas"}
        >
          Graph Canvas
        </button>
        <button
          onclick={() => (viewMode = "list")}
          class="px-3 py-1 rounded transition-colors {viewMode === 'list' ? 'bg-blue-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
          aria-pressed={viewMode === "list"}
        >
          Module List (Accessible)
        </button>
      </div>

      {#if viewMode === "canvas"}
        <button
          onclick={handleFit}
          class="px-3 py-1 rounded bg-[#1a1e2e] hover:bg-slate-800 border border-[#262c3f] text-xs font-medium text-slate-200 transition-colors"
          title="Fit graph to view"
        >
          Fit View
        </button>
      {/if}
    </div>
  </div>

  {#if !bundle}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center">
      <p class="text-sm text-slate-400">No report loaded. Please load a report JSON file or connect to a live eforge serve instance.</p>
    </div>
  {:else if elements.nodes.length === 0}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center">
      <p class="text-sm text-slate-400">No modules detected in this repository.</p>
    </div>
  {:else}
    <!-- Main Content Area: Graph/List + Inspector Detail Panel -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 items-start">
      <!-- Left 2 Cols: Canvas or Accessible Table -->
      <div class="lg:col-span-2 space-y-4">
        {#if viewMode === "canvas"}
          <div class="relative rounded-lg border border-[#262c3f] bg-[#0d1017] overflow-hidden">
            <!-- Canvas viewport -->
            <div
              bind:this={containerEl}
              class="w-full h-[620px]"
              role="region"
              aria-label="Interactive architecture graph"
            ></div>

            <!-- Legend Overlay -->
            <div class="absolute bottom-3 left-3 bg-[#12151f]/90 backdrop-blur border border-[#262c3f] rounded px-3 py-2 text-xs flex items-center gap-4 text-slate-300">
              <span class="flex items-center gap-1.5">
                <span class="w-2.5 h-2.5 rounded-sm bg-[#161f30] border border-[#26354f]"></span>
                Module
              </span>
              <span class="flex items-center gap-1.5">
                <span class="w-2.5 h-2.5 rounded-sm bg-[#2a151b] border border-[#ef4444]"></span>
                In Cycle
              </span>
              <span class="flex items-center gap-1.5">
                <span class="w-3 h-0.5 bg-[#ef4444]"></span>
                Cyclic Edge
              </span>
            </div>
          </div>
        {:else}
          <!-- Keyboard-accessible module table (WCAG AA non-visual alternative) -->
          <div class="rounded-lg border border-[#262c3f] bg-[#12151f] overflow-hidden">
            <table class="w-full text-left text-xs text-slate-300" aria-label="Modules list">
              <thead class="bg-[#1a1e2e] text-slate-400 uppercase font-mono text-[10px] tracking-wider border-b border-[#262c3f]">
                <tr>
                  <th scope="col" class="py-2.5 px-4">Module</th>
                  <th scope="col" class="py-2.5 px-3">Path</th>
                  <th scope="col" class="py-2.5 px-3">Files</th>
                  <th scope="col" class="py-2.5 px-3">Imports</th>
                  <th scope="col" class="py-2.5 px-3">Status</th>
                  <th scope="col" class="py-2.5 px-3 text-right">Action</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-[#262c3f]/50">
                {#each elements.nodes as node (node.data.id)}
                  <tr class="hover:bg-[#1a1e2e]/50 {selectedNodeId === node.data.id ? 'bg-blue-950/20' : ''}">
                    <td class="py-2.5 px-4 font-mono font-medium text-slate-100">{node.data.name}</td>
                    <td class="py-2.5 px-3 font-mono text-slate-400">{node.data.rootPath}</td>
                    <td class="py-2.5 px-3">{node.data.fileCount}</td>
                    <td class="py-2.5 px-3">{node.data.importCount}</td>
                    <td class="py-2.5 px-3">
                      {#if node.data.hasCycle}
                        <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-red-950 text-red-300 border border-red-800">
                          CYCLE
                        </span>
                      {:else}
                        <span class="text-slate-500 font-mono text-[10px]">OK</span>
                      {/if}
                    </td>
                    <td class="py-2.5 px-3 text-right">
                      <button
                        onclick={() => handleSelectNode(node.data.id)}
                        class="px-2 py-1 rounded bg-[#262c3f] hover:bg-blue-600 text-slate-200 hover:text-white transition-colors"
                      >
                        Inspect
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>

      <!-- Right Col: Detail Panel (eforge explain parity) -->
      <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-5 space-y-5">
        {#if !selectedNode}
          <div class="text-center py-12 text-slate-400 space-y-2">
            <p class="font-medium text-slate-300 text-sm">No Module Selected</p>
            <p class="text-xs">Click any module node in the graph or choose inspect in the module table to view its architectural breakdown.</p>
          </div>
        {:else}
          <div class="border-b border-[#262c3f] pb-3">
            <div class="flex items-center justify-between">
              <span class="text-[10px] font-mono uppercase text-blue-400 font-semibold tracking-wider">Module Inspector</span>
              {#if selectedNode.hasCycle}
                <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-red-950 text-red-300 border border-red-800">
                  CIRCULAR DEPENDENCY
                </span>
              {/if}
            </div>
            <h2 class="text-lg font-bold font-mono text-white mt-1 break-all">{selectedNode.name}</h2>
            <p class="text-xs font-mono text-slate-400 mt-0.5 break-all">{selectedNode.rootPath}</p>
          </div>

          <!-- Metrics Quick Bar -->
          <div class="grid grid-cols-3 gap-2 text-center">
            <div class="p-2 rounded bg-[#1a1e2e] border border-[#262c3f]">
              <div class="text-[10px] text-slate-400 uppercase">Files</div>
              <div class="text-sm font-bold text-slate-100 mt-0.5 font-mono">{selectedNode.fileCount}</div>
            </div>
            <div class="p-2 rounded bg-[#1a1e2e] border border-[#262c3f]">
              <div class="text-[10px] text-slate-400 uppercase">Imports</div>
              <div class="text-sm font-bold text-slate-100 mt-0.5 font-mono">{selectedNode.importCount}</div>
            </div>
            <div class="p-2 rounded bg-[#1a1e2e] border border-[#262c3f]">
              <div class="text-[10px] text-slate-400 uppercase">Inbound</div>
              <div class="text-sm font-bold text-slate-100 mt-0.5 font-mono">{importedBy.length}</div>
            </div>
          </div>

          <!-- Outbound Imports -->
          <div class="space-y-1.5">
            <h3 class="text-xs font-semibold text-slate-200">Outbound Imports ({selectedNode.imports.length})</h3>
            {#if selectedNode.imports.length === 0}
              <p class="text-xs text-slate-500 italic">No dependencies imported.</p>
            {:else}
              <div class="max-h-28 overflow-y-auto space-y-1 pr-1 font-mono text-xs">
                {#each selectedNode.imports as imp (imp)}
                  <div class="px-2 py-1 rounded bg-[#090a0f] border border-[#262c3f] text-slate-300 break-all">
                    {imp}
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Inbound Imported By -->
          <div class="space-y-1.5">
            <h3 class="text-xs font-semibold text-slate-200">Imported By ({importedBy.length})</h3>
            {#if importedBy.length === 0}
              <p class="text-xs text-slate-500 italic">No internal modules import this module directly.</p>
            {:else}
              <div class="max-h-28 overflow-y-auto space-y-1 pr-1 font-mono text-xs">
                {#each importedBy as caller (caller)}
                  <div class="px-2 py-1 rounded bg-[#090a0f] border border-[#262c3f] text-slate-300 break-all">
                    {caller}
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Files -->
          <div class="space-y-1.5">
            <h3 class="text-xs font-semibold text-slate-200">Files ({selectedNode.files.length})</h3>
            <div class="max-h-32 overflow-y-auto space-y-1 pr-1 font-mono text-xs">
              {#each selectedNode.files as file (file)}
                <div class="px-2 py-1 rounded bg-[#090a0f] border border-[#262c3f] text-slate-400 break-all">
                  {file}
                </div>
              {/each}
            </div>
          </div>

          <!-- Related Findings -->
          <div class="space-y-1.5">
            <h3 class="text-xs font-semibold text-slate-200">Related Findings ({relatedFindings.length})</h3>
            {#if relatedFindings.length === 0}
              <p class="text-xs text-emerald-400">No rule violations or security findings in this module.</p>
            {:else}
              <div class="max-h-36 overflow-y-auto space-y-1.5 pr-1 text-xs">
                {#each relatedFindings as finding (finding.id)}
                  <div class="p-2 rounded bg-red-950/20 border border-red-900/40 text-slate-300">
                    <div class="flex items-center gap-1.5 font-mono text-[10px] text-red-300">
                      <span class="font-bold uppercase">{finding.severity}</span>
                      <span>•</span>
                      <span>{finding.rule_id}</span>
                    </div>
                    <div class="font-medium text-slate-100 mt-0.5">{finding.title}</div>
                    <div class="text-slate-400 text-[11px] mt-1">{finding.description}</div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
