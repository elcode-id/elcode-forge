<script lang="ts">
  import "../app.css";
  import type { Snippet } from "svelte";
  import { page } from "$app/state";
  import { base } from "$app/paths";

  let { children }: { children?: Snippet } = $props();

  const isHome = $derived(
    page.url.pathname === base || page.url.pathname === base + "/" || page.url.pathname === "/"
  );

  const isActive = (path: string) => {
    const target = base ? `${base}${path}` : path;
    return page.url.pathname.startsWith(target);
  };
</script>

<div class="min-h-screen flex flex-col bg-[#0b0d13] text-slate-100">
  <header class="sticky top-0 z-50 backdrop-blur-md bg-[#0b0d13]/85 border-b border-[#262c3f]">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between">
      <div class="flex items-center gap-5">
        <a href="{base}/" class="flex items-center gap-3 group shrink-0">
          <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-indigo-600 flex items-center justify-center font-bold text-white shadow-lg shadow-blue-500/20 group-hover:scale-105 transition-transform">
            ⚡
          </div>
          <div class="flex flex-col">
            <span class="font-bold tracking-tight text-white flex items-center gap-2">
              ELCODE Forge
              <span class="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-blue-950/80 text-blue-400 border border-blue-800/40">Docs</span>
            </span>
            <span class="text-[10px] text-slate-400 font-mono">v1.0.0 GA</span>
          </div>
        </a>

        <nav class="hidden xl:flex items-center gap-1 text-xs font-medium">
          <a
            href="{base}/"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isHome ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Overview & Install
          </a>
          <a
            href="{base}/configuration"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isActive('/configuration') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Configuration
          </a>
          <a
            href="{base}/cli"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isActive('/cli') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            CLI Reference
          </a>
          <a
            href="{base}/rules"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isActive('/rules') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Custom Rules
          </a>
          <a
            href="{base}/mcp"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isActive('/mcp') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            MCP Server
          </a>
          <a
            href="{base}/policy"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isActive('/policy') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Policy-as-Code
          </a>
          <a
            href="{base}/compliance"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isActive('/compliance') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Security & Compliance
          </a>
          <a
            href="{base}/self-hosting"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isActive('/self-hosting') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Self-Hosting
          </a>
          <a
            href="{base}/provenance"
            class="px-2.5 py-1.5 rounded-lg transition-colors {isActive('/provenance') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            SLSA Provenance
          </a>
        </nav>
      </div>

      <div class="flex items-center gap-3">
        <a
          href="https://github.com/elcode-id/elcode-forge"
          target="_blank"
          rel="noreferrer"
          class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-900 border border-slate-800 hover:border-slate-700 text-xs font-mono text-slate-300 transition-colors"
        >
          <span>GitHub</span>
          <span class="text-slate-500">↗</span>
        </a>
      </div>
    </div>

    <!-- Mobile & Tablet Sub-Navigation -->
    <div class="xl:hidden flex items-center gap-2 px-4 py-2 overflow-x-auto border-t border-slate-800/60 text-xs font-mono scrollbar-none">
      <a href="{base}/" class="px-2 py-1 rounded whitespace-nowrap {isHome ? 'text-white bg-slate-800' : 'text-slate-400'}">Overview & Install</a>
      <a href="{base}/configuration" class="px-2 py-1 rounded whitespace-nowrap {isActive('/configuration') ? 'text-white bg-slate-800' : 'text-slate-400'}">Configuration</a>
      <a href="{base}/cli" class="px-2 py-1 rounded whitespace-nowrap {isActive('/cli') ? 'text-white bg-slate-800' : 'text-slate-400'}">CLI Reference</a>
      <a href="{base}/rules" class="px-2 py-1 rounded whitespace-nowrap {isActive('/rules') ? 'text-white bg-slate-800' : 'text-slate-400'}">Custom Rules</a>
      <a href="{base}/mcp" class="px-2 py-1 rounded whitespace-nowrap {isActive('/mcp') ? 'text-white bg-slate-800' : 'text-slate-400'}">MCP Server</a>
      <a href="{base}/policy" class="px-2 py-1 rounded whitespace-nowrap {isActive('/policy') ? 'text-white bg-slate-800' : 'text-slate-400'}">Policy-as-Code</a>
      <a href="{base}/compliance" class="px-2 py-1 rounded whitespace-nowrap {isActive('/compliance') ? 'text-white bg-slate-800' : 'text-slate-400'}">Security & Compliance</a>
      <a href="{base}/self-hosting" class="px-2 py-1 rounded whitespace-nowrap {isActive('/self-hosting') ? 'text-white bg-slate-800' : 'text-slate-400'}">Self-Hosting</a>
      <a href="{base}/provenance" class="px-2 py-1 rounded whitespace-nowrap {isActive('/provenance') ? 'text-white bg-slate-800' : 'text-slate-400'}">Provenance</a>
    </div>
  </header>

  <main class="flex-1 max-w-7xl mx-auto px-4 sm:px-6 py-8 w-full">
    {#if children}
      {@render children()}
    {/if}
  </main>

  <footer class="border-t border-[#262c3f] bg-[#0b0d13] py-8 text-center text-xs text-slate-500 font-mono">
    <div class="max-w-7xl mx-auto px-4 flex flex-col sm:flex-row items-center justify-between gap-4">
      <div>
        ELCODE Forge (`eforge`) v1.0.0 GA — Know your code. Ship with confidence.
      </div>
      <div class="flex items-center gap-4 text-slate-400">
        <a href="https://elcode.id" target="_blank" rel="noreferrer" class="hover:text-white">Studio Manifesto</a>
        <span>•</span>
        <a href="https://github.com/elcode-id/elcode-forge" target="_blank" rel="noreferrer" class="hover:text-white">Source Code</a>
      </div>
    </div>
  </footer>
</div>
