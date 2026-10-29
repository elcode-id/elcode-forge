<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
</script>

<div class="min-h-screen flex flex-col bg-[#0b0d13] text-slate-100">
  <header class="sticky top-0 z-50 backdrop-blur-md bg-[#0b0d13]/85 border-b border-[#262c3f]">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 h-16 flex items-center justify-between">
      <div class="flex items-center gap-6">
        <a href="/" class="flex items-center gap-3 group">
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

        <nav class="hidden md:flex items-center gap-1 text-sm font-medium">
          <a
            href="/"
            class="px-3 py-1.5 rounded-lg transition-colors {page.url.pathname === '/' ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Overview
          </a>
          <a
            href="/policy"
            class="px-3 py-1.5 rounded-lg transition-colors {page.url.pathname.startsWith('/policy') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Policy-as-Code
          </a>
          <a
            href="/compliance"
            class="px-3 py-1.5 rounded-lg transition-colors {page.url.pathname.startsWith('/compliance') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Compliance Audits
          </a>
          <a
            href="/self-hosting"
            class="px-3 py-1.5 rounded-lg transition-colors {page.url.pathname.startsWith('/self-hosting') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
          >
            Self-Hosting
          </a>
          <a
            href="/provenance"
            class="px-3 py-1.5 rounded-lg transition-colors {page.url.pathname.startsWith('/provenance') ? 'text-white bg-slate-800/80' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/60'}"
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
  </header>

  <main class="flex-1 max-w-7xl mx-auto px-4 sm:px-6 py-8 w-full">
    <slot />
  </main>

  <footer class="border-t border-[#262c3f] bg-[#0b0d13] py-8 text-center text-xs text-slate-500 font-mono">
    <div class="max-w-7xl mx-auto px-4 flex flex-col sm:flex-row items-center justify-between gap-4">
      <div>
        ELCODE Forge v1.0.0 GA — Know your code. Ship with confidence.
      </div>
      <div class="flex items-center gap-4 text-slate-400">
        <a href="https://elcode.id" target="_blank" rel="noreferrer" class="hover:text-white">Studio Manifesto</a>
        <span>•</span>
        <a href="https://github.com/elcode-id/elcode-forge" target="_blank" rel="noreferrer" class="hover:text-white">Source Code</a>
      </div>
    </div>
  </footer>
</div>
