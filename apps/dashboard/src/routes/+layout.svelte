<script lang="ts">
  import type { Snippet } from "svelte";
  import "../app.css";
  import { dashboardState } from "$lib/data/store.svelte.js";
  import { loadFromFile, loadFromApi } from "$lib/data/source.js";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let apiInput = $state("http://127.0.0.1:7878");

  async function handleFileSelect(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0]) {
      dashboardState.setLoading(true);
      const res = await loadFromFile(target.files[0]);
      if (res.ok) {
        dashboardState.setBundle(res.bundle, res.source);
      } else {
        dashboardState.setError(res.errors.join(", "));
      }
    }
  }

  async function handleConnectApi() {
    dashboardState.setLoading(true);
    const res = await loadFromApi(apiInput);
    if (res.ok) {
      dashboardState.setBundle(res.bundle, res.source);
    } else {
      dashboardState.setError(res.errors.join(", "));
    }
  }

  function toggleConnectionMode() {
    const nextMode = dashboardState.serverConfig.mode === "cloud" ? "local" : "cloud";
    dashboardState.setMode(nextMode);
  }
</script>

<div class="min-h-screen flex flex-col bg-[#090a0f] text-slate-100">
  <!-- Header Bar -->
  <header class="border-b border-[#262c3f] bg-[#12151f]/80 backdrop-blur sticky top-0 z-50 px-6 py-3">
    <div class="max-w-7xl mx-auto flex items-center justify-between">
      <div class="flex items-center gap-3">
        <span class="w-2.5 h-2.5 rounded-full bg-blue-500"></span>
        <span class="font-bold tracking-tight text-sm font-mono">ELCODE Forge</span>
        <span class="text-xs px-2 py-0.5 rounded bg-slate-800 text-slate-400 font-mono">v1.0.0</span>

        <nav class="flex items-center gap-1 ml-4" aria-label="Main Navigation">
          <a
            href="/"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Overview
          </a>
          <a
            href="/projects"
            class="px-2.5 py-1 rounded text-xs text-blue-300 hover:text-white hover:bg-slate-800 transition-colors font-medium flex items-center gap-1"
          >
            <span>Projects</span>
            <span class="w-1.5 h-1.5 rounded-full bg-blue-400"></span>
          </a>
          <a
            href="/compare"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Compare
          </a>
          <a
            href="/governance"
            class="px-2.5 py-1 rounded text-xs text-purple-300 hover:text-white hover:bg-slate-800 transition-colors font-medium flex items-center gap-1"
          >
            <span>Governance</span>
            <span class="w-1.5 h-1.5 rounded-full bg-purple-400"></span>
          </a>
          <a
            href="/architecture"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Architecture
          </a>
          <a
            href="/findings"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Findings
          </a>
          <a
            href="/rules"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Rules
          </a>
          <a
            href="/security"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Security
          </a>
          <a
            href="/dependencies"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Dependencies
          </a>
          <a
            href="/changes"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Changes
          </a>
          <a
            href="/trends"
            class="px-2.5 py-1 rounded text-xs text-slate-300 hover:text-white hover:bg-slate-800 transition-colors font-medium"
          >
            Trends
          </a>
        </nav>
      </div>

      <!-- Data Source Switcher & Mode Toggle -->
      <div class="flex items-center gap-3 text-xs">
        <!-- Mode Switcher Pill -->
        <button
          onclick={toggleConnectionMode}
          class="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-mono font-medium border transition-colors {dashboardState.serverConfig.mode === 'cloud'
            ? 'bg-blue-950/80 text-blue-300 border-blue-800 hover:bg-blue-900/80'
            : 'bg-slate-900 text-slate-400 border-slate-700 hover:bg-slate-800'}"
          title="Toggle between Local File Mode and Team Cloud Mode"
        >
          <span class="w-1.5 h-1.5 rounded-full {dashboardState.serverConfig.mode === 'cloud' ? 'bg-blue-400' : 'bg-slate-500'}"></span>
          <span>{dashboardState.serverConfig.mode === 'cloud' ? 'Team Cloud' : 'Local File'}</span>
        </button>

        {#if dashboardState.serverConfig.mode === 'cloud'}
          <!-- Team Cloud Server Info -->
          <a
            href="/projects"
            class="flex items-center gap-2 px-2.5 py-1 rounded bg-[#1a1e2e] border border-[#262c3f] hover:border-slate-600 transition-colors"
          >
            <span class="w-1.5 h-1.5 rounded-full {dashboardState.serverConnected ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
            <span class="text-slate-400">Server:</span>
            <span class="font-mono text-slate-200">
              {dashboardState.serverConfig.activeRepoId ? `${dashboardState.serverConfig.activeRepoId}` : 'Browse Projects'}
            </span>
          </a>
        {:else}
          <!-- Local File / Live Daemon Info -->
          {#if dashboardState.source}
            <div class="flex items-center gap-2 px-2.5 py-1 rounded bg-[#1a1e2e] border border-[#262c3f]">
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
              <span class="text-slate-400">
                {dashboardState.source.kind === 'file' ? 'File:' : dashboardState.source.kind === 'live' ? 'Live:' : 'Cloud:'}
              </span>
              <span class="font-mono text-slate-200 truncate max-w-xs">
                {dashboardState.source.kind === 'file'
                  ? dashboardState.source.name
                  : dashboardState.source.kind === 'live'
                  ? dashboardState.source.baseUrl
                  : `${dashboardState.source.repoId}@${dashboardState.source.branch}`}
              </span>
            </div>
          {/if}

          <label class="px-3 py-1.5 rounded bg-[#1a1e2e] hover:bg-slate-800 border border-[#262c3f] cursor-pointer transition-colors font-medium">
            <span>Load JSON</span>
            <input
              id="file-upload"
              data-testid="file-upload"
              type="file"
              accept=".json"
              class="hidden"
              onchange={handleFileSelect}
            />
          </label>

          <div class="flex items-center gap-1">
            <input
              type="text"
              bind:value={apiInput}
              placeholder="http://127.0.0.1:7878"
              class="px-2.5 py-1.5 bg-[#090a0f] border border-[#262c3f] rounded font-mono text-xs w-44 focus:outline-none focus:border-blue-500"
            />
            <button
              onclick={handleConnectApi}
              class="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white font-medium transition-colors"
            >
              Connect
            </button>
          </div>
        {/if}
      </div>
    </div>
  </header>

  <!-- Error banner -->
  {#if dashboardState.error}
    <div class="bg-red-950/70 border-b border-red-800/80 px-6 py-2.5 text-xs text-red-200 text-center font-mono">
      {dashboardState.error}
    </div>
  {/if}

  <!-- Main Content -->
  <main class="flex-1 max-w-7xl w-full mx-auto px-6 py-8">
    {#if children}
      {@render children()}
    {/if}
  </main>

  <!-- Footer -->
  <footer class="border-t border-[#262c3f] py-4 text-center text-xs text-slate-500 font-mono">
    ELCODE Forge — Know your code. Ship with confidence.
  </footer>
</div>
