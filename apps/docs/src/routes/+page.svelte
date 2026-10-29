<svelte:head>
  <title>Overview | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-10">
  <!-- Hero Section -->
  <div class="relative overflow-hidden rounded-2xl bg-gradient-to-br from-slate-900 via-[#12151f] to-slate-900 border border-[#262c3f] p-8 sm:p-12">
    <div class="max-w-3xl space-y-4">
      <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-blue-500/10 border border-blue-500/20 text-blue-400 text-xs font-mono">
        <span>Production Ready</span>
        <span>•</span>
        <span>Version 1.0.0 GA</span>
      </div>
      <h1 class="text-3xl sm:text-5xl font-extrabold tracking-tight text-white">
        Deterministic Code Intelligence & Enterprise Governance
      </h1>
      <p class="text-base sm:text-lg text-slate-300 leading-relaxed">
        ELCODE Forge is an autonomous, local-first static analysis engine, architecture inspector, and Policy-as-Code gate built under the ELCODE Studio Manifesto: <em>"Software should feel like an instrument, not a compromise."</em>
      </p>
      <div class="flex flex-wrap gap-3 pt-4">
        <a
          href="/policy"
          class="px-5 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-medium text-sm transition-all shadow-lg shadow-blue-600/20"
        >
          Explore Policy-as-Code
        </a>
        <a
          href="/compliance"
          class="px-5 py-2.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 font-medium text-sm border border-slate-700 transition-all"
        >
          Compliance Audits
        </a>
      </div>
    </div>
  </div>

  <!-- Key Pillars Grid -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-3">
      <div class="w-10 h-10 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400 font-bold">
        🛡️
      </div>
      <h3 class="text-lg font-bold text-white">Policy-as-Code Engine</h3>
      <p class="text-sm text-slate-400 leading-relaxed">
        Declare non-negotiable architectural limits, min health scores, zero-tolerance secret rules, and time-bounded exception waivers evaluated deterministically at CI time.
      </p>
      <a href="/policy" class="inline-flex items-center gap-1.5 text-xs font-mono text-blue-400 hover:text-blue-300">
        Read Specification →
      </a>
    </div>

    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-3">
      <div class="w-10 h-10 rounded-lg bg-purple-500/10 border border-purple-500/20 flex items-center justify-center text-purple-400 font-bold">
        📋
      </div>
      <h3 class="text-lg font-bold text-white">Automated Compliance</h3>
      <p class="text-sm text-slate-400 leading-relaxed">
        Export auditable proof of compliance for SOC 2 Type II, ISO/IEC 27001:2022, and OWASP Top 10 with deterministic exit codes and structured evidence.
      </p>
      <a href="/compliance" class="inline-flex items-center gap-1.5 text-xs font-mono text-purple-400 hover:text-purple-300">
        Compliance Mapping →
      </a>
    </div>

    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-3">
      <div class="w-10 h-10 rounded-lg bg-amber-500/10 border border-amber-500/20 flex items-center justify-center text-amber-400 font-bold">
        🏢
      </div>
      <h3 class="text-lg font-bold text-white">Multi-Repo Aggregator</h3>
      <p class="text-sm text-slate-400 leading-relaxed">
        Centralize organization health with self-hostable `forge-server`, pre-aggregated materialized rollups, cross-repository vulnerability matrix, and append-only audit stream.
      </p>
      <a href="/self-hosting" class="inline-flex items-center gap-1.5 text-xs font-mono text-amber-400 hover:text-amber-300">
        Deployment Guide →
      </a>
    </div>
  </div>

  <!-- CLI Fast Reference -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono flex items-center gap-2">
      <span>Command-Line Interface Quickstart</span>
    </h2>
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs font-mono">
      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-slate-400 font-bold uppercase tracking-wider text-[11px]">Primary Diagnostics & Scans</div>
        <div class="text-slate-200"># Fast repository health inspection</div>
        <div class="text-blue-400">eforge doctor</div>
        <div class="text-slate-200 pt-2"># Full AST analysis and rule verification</div>
        <div class="text-blue-400">eforge scan --fail-on high</div>
        <div class="text-slate-200 pt-2"># Git working-tree change review</div>
        <div class="text-blue-400">eforge review --fast</div>
      </div>

      <div class="bg-black/40 border border-slate-800 rounded-lg p-4 space-y-2">
        <div class="text-slate-400 font-bold uppercase tracking-wider text-[11px]">Governance & Compliance</div>
        <div class="text-slate-200"># Assert Policy-as-Code gate</div>
        <div class="text-emerald-400">eforge policy check</div>
        <div class="text-slate-200 pt-2"># Export SOC 2 Type II audit report</div>
        <div class="text-emerald-400">eforge audit --standard soc2 --format markdown</div>
        <div class="text-slate-200 pt-2"># Push baseline to Forge Server</div>
        <div class="text-emerald-400">eforge sync push</div>
      </div>
    </div>
  </div>
</div>
