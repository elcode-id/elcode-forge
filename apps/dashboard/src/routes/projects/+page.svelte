<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { dashboardState } from "$lib/data/store.svelte.js";
  import {
    checkServerHealth,
    fetchProjects,
    pullSnapshot,
    snapshotToReportBundle,
    type ProjectSummary,
  } from "$lib/data/server.js";

  let searchQuery = $state("");
  let showConfigModal = $state(false);
  let serverUrlInput = $state(dashboardState.serverConfig.serverUrl);
  let tokenInput = $state(dashboardState.serverConfig.token);
  let testStatus = $state<{ text: string; ok: boolean } | null>(null);
  let isLoading = $state(false);

  let filteredProjects = $derived.by(() => {
    const list = dashboardState.serverProjects;
    if (!searchQuery.trim()) return list;
    const q = searchQuery.toLowerCase().trim();
    return list.filter(
      (p) => p.name.toLowerCase().includes(q) || p.repo_id.toLowerCase().includes(q)
    );
  });

  async function refreshProjects() {
    isLoading = true;
    dashboardState.setError(null);
    const health = await checkServerHealth(
      dashboardState.serverConfig.serverUrl,
      dashboardState.serverConfig.token
    );
    dashboardState.setServerConnected(health.ok);

    if (health.ok) {
      const res = await fetchProjects(
        dashboardState.serverConfig.serverUrl,
        dashboardState.serverConfig.token
      );
      if (res.ok) {
        dashboardState.setServerProjects(res.projects);
      } else {
        dashboardState.setError(res.error || "Failed to query projects list");
      }
    } else {
      dashboardState.setError(health.error || "Cannot reach Forge Server");
    }
    isLoading = false;
  }

  async function handleTestConnection() {
    testStatus = null;
    const res = await checkServerHealth(serverUrlInput, tokenInput);
    if (res.ok) {
      testStatus = {
        text: `Connected successfully! (version: ${res.version || "0.8.0"})`,
        ok: true,
      };
    } else {
      testStatus = {
        text: res.error || "Connection failed",
        ok: false,
      };
    }
  }

  function handleSaveConfig() {
    dashboardState.setServerConfig({
      ...dashboardState.serverConfig,
      serverUrl: serverUrlInput.trim(),
      token: tokenInput.trim(),
      mode: "cloud",
    });
    showConfigModal = false;
    refreshProjects();
  }

  async function handleSelectProject(project: ProjectSummary) {
    isLoading = true;
    const branch = project.latest_branch || "main";
    const res = await pullSnapshot(
      dashboardState.serverConfig.serverUrl,
      project.repo_id,
      branch,
      dashboardState.serverConfig.token
    );

    if (res.ok && res.snapshot) {
      const bundle = snapshotToReportBundle(res.snapshot, project.name);
      dashboardState.setBundle(bundle, {
        kind: "server",
        serverUrl: dashboardState.serverConfig.serverUrl,
        repoId: project.repo_id,
        branch,
      });
      dashboardState.setServerConfig({
        ...dashboardState.serverConfig,
        activeRepoId: project.repo_id,
        activeBranch: branch,
      });
      goto("/");
    } else {
      dashboardState.setError(res.error || `Failed to pull snapshot for ${project.repo_id}`);
    }
    isLoading = false;
  }

  onMount(() => {
    if (dashboardState.serverConfig.mode === "cloud" || dashboardState.serverProjects.length === 0) {
      refreshProjects();
    }
  });
</script>

<div class="space-y-6">
  <!-- Studio Header -->
  <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 pb-4 border-b border-[#262c3f]">
    <div>
      <div class="flex items-center gap-3">
        <h1 class="text-2xl font-bold tracking-tight text-white font-mono">
          Project Workspace Portfolio
        </h1>
        <span
          class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-mono {dashboardState.serverConnected
            ? 'bg-emerald-950/80 text-emerald-400 border border-emerald-800/80'
            : 'bg-amber-950/80 text-amber-400 border border-amber-800/80'}"
        >
          <span class="w-1.5 h-1.5 rounded-full {dashboardState.serverConnected ? 'bg-emerald-400' : 'bg-amber-400'}"></span>
          {dashboardState.serverConnected ? "Connected" : "Disconnected"}
        </span>
      </div>
      <p class="text-xs text-slate-400 font-mono mt-1">
        Connected to <span class="text-slate-300">{dashboardState.serverConfig.serverUrl}</span> •
        Total Repositories: {dashboardState.serverProjects.length}
      </p>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-2">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Filter by name or repo ID..."
        class="px-3 py-1.5 bg-[#12151f] border border-[#262c3f] rounded text-xs font-mono text-slate-200 placeholder-slate-500 w-56 focus:outline-none focus:border-blue-500"
      />
      <button
        onclick={refreshProjects}
        disabled={isLoading}
        class="px-3 py-1.5 rounded bg-[#1a1e2e] hover:bg-slate-800 border border-[#262c3f] text-xs font-medium text-slate-200 transition-colors disabled:opacity-50"
      >
        {isLoading ? "Syncing..." : "Refresh"}
      </button>
      <button
        onclick={() => (showConfigModal = true)}
        class="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-xs font-medium text-white transition-colors"
      >
        Server Settings
      </button>
    </div>
  </div>

  <!-- Projects Grid -->
  {#if filteredProjects.length === 0}
    <div class="p-12 rounded-xl border border-dashed border-[#262c3f] bg-[#12151f]/50 text-center">
      <div class="w-12 h-12 rounded-full bg-slate-800 flex items-center justify-center mx-auto mb-4 text-slate-400 font-mono">
        ☁️
      </div>
      <h3 class="text-base font-semibold text-white mb-2">No Connected Repositories Found</h3>
      <p class="text-xs text-slate-400 max-w-md mx-auto mb-6">
        {#if !dashboardState.serverConnected}
          Could not establish connection to the Forge Server at <code>{dashboardState.serverConfig.serverUrl}</code>. Check your server settings or network connectivity.
        {:else if searchQuery}
          No repositories match search query "<code>{searchQuery}</code>".
        {:else}
          Your Forge Server is running but has no snapshots registered yet. Synchronize a project from your terminal:
        {/if}
      </p>

      <div class="inline-block text-left bg-[#090a0f] border border-[#262c3f] rounded-lg p-3 text-xs font-mono text-slate-300">
        <p class="text-slate-500 mb-1"># Push analysis snapshot to server:</p>
        <p class="text-emerald-400">eforge login --server {dashboardState.serverConfig.serverUrl}</p>
        <p class="text-emerald-400">eforge sync</p>
      </div>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
      {#each filteredProjects as project (project.repo_id)}
        <div class="bg-[#12151f] border border-[#262c3f] hover:border-slate-600 rounded-xl p-5 flex flex-col justify-between transition-all group">
          <div>
            <!-- Card Header -->
            <div class="flex items-start justify-between gap-3 mb-3">
              <div>
                <h3 class="font-bold text-white text-base group-hover:text-blue-400 transition-colors font-mono">
                  {project.name}
                </h3>
                <span class="text-xs text-slate-500 font-mono">
                  {project.repo_id}
                </span>
              </div>

              <!-- Health Score Pill -->
              {#if project.latest_health !== null}
                <div
                  class="px-2.5 py-1 rounded-lg text-xs font-mono font-bold {project.latest_health >= 85
                    ? 'bg-emerald-950/80 text-emerald-400 border border-emerald-800/80'
                    : project.latest_health >= 70
                    ? 'bg-amber-950/80 text-amber-400 border border-amber-800/80'
                    : 'bg-red-950/80 text-red-400 border border-red-800/80'}"
                >
                  {project.latest_health}/100
                </div>
              {:else}
                <span class="px-2 py-0.5 rounded text-xs bg-slate-800 text-slate-500 font-mono">
                  No Score
                </span>
              {/if}
            </div>

            <!-- Details -->
            <div class="space-y-1.5 py-3 border-y border-[#262c3f]/80 text-xs font-mono">
              <div class="flex justify-between text-slate-400">
                <span>Branch:</span>
                <span class="text-slate-200">{project.latest_branch || "main"}</span>
              </div>
              <div class="flex justify-between text-slate-400">
                <span>Commit:</span>
                <span class="text-slate-300 font-mono">
                  {project.latest_commit ? project.latest_commit.slice(0, 8) : "—"}
                </span>
              </div>
              <div class="flex justify-between text-slate-400">
                <span>Open Findings:</span>
                <span class="{project.findings_count && project.findings_count > 0 ? 'text-amber-400' : 'text-slate-200'}">
                  {project.findings_count ?? 0}
                </span>
              </div>
              <div class="flex justify-between text-slate-400">
                <span>Last Updated:</span>
                <span class="text-slate-400">
                  {new Date(project.updated_at * 1000).toLocaleDateString()}
                </span>
              </div>
            </div>
          </div>

          <!-- Card Actions -->
          <div class="flex items-center gap-2 mt-4 pt-2">
            <button
              onclick={() => handleSelectProject(project)}
              class="flex-1 px-3 py-1.5 rounded bg-blue-600/90 hover:bg-blue-600 text-white text-xs font-medium text-center transition-colors font-mono"
            >
              Open Dashboard
            </button>
            <a
              href="/compare?repo={encodeURIComponent(project.repo_id)}"
              class="px-3 py-1.5 rounded bg-[#1a1e2e] hover:bg-slate-800 border border-[#262c3f] text-slate-300 text-xs font-medium transition-colors font-mono"
            >
              PR Diff
            </a>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Server Config Modal -->
{#if showConfigModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl max-w-md w-full p-6 shadow-2xl">
      <div class="flex items-center justify-between mb-4 pb-2 border-b border-[#262c3f]">
        <h3 class="font-bold text-white text-base font-mono">Forge Server Configuration</h3>
        <button
          onclick={() => (showConfigModal = false)}
          class="text-slate-400 hover:text-white transition-colors"
        >
          ✕
        </button>
      </div>

      <div class="space-y-4 text-xs font-mono">
        <div>
          <label for="server-url-input" class="block text-slate-300 font-medium mb-1">Server URL</label>
          <input
            id="server-url-input"
            type="text"
            bind:value={serverUrlInput}
            placeholder="http://localhost:8080"
            class="w-full px-3 py-2 bg-[#090a0f] border border-[#262c3f] rounded text-slate-200 focus:outline-none focus:border-blue-500"
          />
          <p class="text-slate-500 text-[11px] mt-1">Default address for local `forge-server` deployment.</p>
        </div>

        <div>
          <label for="server-token-input" class="block text-slate-300 font-medium mb-1">Personal Access Token / API Key</label>
          <input
            id="server-token-input"
            type="password"
            bind:value={tokenInput}
            placeholder="Optional for public reads, required for push"
            class="w-full px-3 py-2 bg-[#090a0f] border border-[#262c3f] rounded text-slate-200 focus:outline-none focus:border-blue-500"
          />
          <p class="text-slate-500 text-[11px] mt-1">Stored securely in your local browser storage.</p>
        </div>

        {#if testStatus}
          <div
            class="p-2.5 rounded text-xs {testStatus.ok
              ? 'bg-emerald-950/70 text-emerald-300 border border-emerald-800'
              : 'bg-red-950/70 text-red-300 border border-red-800'}"
          >
            {testStatus.text}
          </div>
        {/if}

        <div class="flex items-center justify-between pt-4 border-t border-[#262c3f]">
          <button
            onclick={handleTestConnection}
            type="button"
            class="px-3 py-1.5 rounded bg-[#1a1e2e] hover:bg-slate-800 border border-[#262c3f] text-slate-300 transition-colors font-medium"
          >
            Test Connection
          </button>
          <div class="flex items-center gap-2">
            <button
              onclick={() => (showConfigModal = false)}
              type="button"
              class="px-3 py-1.5 rounded text-slate-400 hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              onclick={handleSaveConfig}
              type="button"
              class="px-4 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white font-medium transition-colors"
            >
              Save & Connect
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
