<svelte:head>
  <title>Compliance Audits | ELCODE Forge Documentation</title>
</svelte:head>

<div class="space-y-8 max-w-4xl">
  <div>
    <div class="text-xs font-mono uppercase tracking-wider text-purple-400 mb-1">Audit & Governance</div>
    <h1 class="text-3xl font-extrabold text-white">Automated Compliance Mapping</h1>
    <p class="text-slate-400 mt-2 text-base leading-relaxed">
      Execute compliance audits directly from the CLI and export structured evidence for SOC 2 Type II, ISO/IEC 27001:2022, and OWASP Top 10 (2021).
    </p>
  </div>

  <!-- CLI Command Reference -->
  <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-6 space-y-4">
    <h2 class="text-lg font-bold text-white font-mono">1. Running Compliance Audits</h2>
    <div class="bg-black/50 border border-slate-800 rounded-lg p-4 font-mono text-xs text-slate-300 space-y-2">
      <div class="text-slate-400"># Audit against SOC 2 Type II controls</div>
      <div class="text-purple-400">eforge audit --standard soc2 --format terminal</div>
      <div class="text-slate-400 pt-2"># Export auditable Markdown report for security auditors</div>
      <div class="text-purple-400">eforge audit --standard iso27001 --format markdown -o audit-report.md</div>
      <div class="text-slate-400 pt-2"># Automated CI gate (exits with code 4 if any clause fails)</div>
      <div class="text-purple-400">eforge audit --standard owasp --format json</div>
    </div>
  </div>

  <!-- Evaluated Standards -->
  <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-5 space-y-3">
      <div class="text-sm font-bold text-white font-mono flex items-center gap-2">
        <span class="w-2 h-2 rounded-full bg-blue-400"></span>
        <span>SOC 2 Type II</span>
      </div>
      <ul class="text-xs text-slate-400 space-y-1.5 font-mono">
        <li>• <strong class="text-slate-200">CC6.1:</strong> Logical Access & Secret Scans</li>
        <li>• <strong class="text-slate-200">CC6.6:</strong> Boundary Protection & Architecture</li>
        <li>• <strong class="text-slate-200">CC7.1:</strong> Vulnerability Management</li>
        <li>• <strong class="text-slate-200">CC8.1:</strong> Change Risk & Test Coverage</li>
      </ul>
    </div>

    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-5 space-y-3">
      <div class="text-sm font-bold text-white font-mono flex items-center gap-2">
        <span class="w-2 h-2 rounded-full bg-purple-400"></span>
        <span>OWASP Top 10 (2021)</span>
      </div>
      <ul class="text-xs text-slate-400 space-y-1.5 font-mono">
        <li>• <strong class="text-slate-200">A01:</strong> Broken Access Control</li>
        <li>• <strong class="text-slate-200">A02:</strong> Cryptographic Failures</li>
        <li>• <strong class="text-slate-200">A03:</strong> Injection Flaws & Unsafe Code</li>
        <li>• <strong class="text-slate-200">A06:</strong> Vulnerable & Outdated Packages</li>
      </ul>
    </div>

    <div class="bg-[#12151f] border border-[#262c3f] rounded-xl p-5 space-y-3">
      <div class="text-sm font-bold text-white font-mono flex items-center gap-2">
        <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
        <span>ISO/IEC 27001:2022</span>
      </div>
      <ul class="text-xs text-slate-400 space-y-1.5 font-mono">
        <li>• <strong class="text-slate-200">Control 8.8:</strong> Technical Vulnerability Mgmt</li>
        <li>• <strong class="text-slate-200">Control 8.28:</strong> Secure Coding Guidelines</li>
      </ul>
    </div>
  </div>
</div>
