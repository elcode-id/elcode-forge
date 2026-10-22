<script lang="ts">
  import { dashboardState } from "$lib/data/store.svelte.js";
  import type { Finding } from "@elcode-forge/schema";

  const bundle = $derived(dashboardState.bundle);
  const allFindings = $derived<Finding[]>(bundle?.repository?.findings ?? []);

  // Filter security-relevant findings: category 'security', rule_id 'security/*', or 'license/*'
  const securityFindings = $derived<Finding[]>(
    allFindings.filter(
      (f) =>
        f.category?.toLowerCase() === "security" ||
        f.rule_id.toLowerCase().startsWith("security/") ||
        f.rule_id.toLowerCase().startsWith("license/")
    )
  );

  const vulnerabilityFindings = $derived<Finding[]>(
    securityFindings.filter((f) => f.rule_id.includes("vulnerable-dependency") || f.rule_id.includes("cve-") || f.rule_id.includes("ghsa-"))
  );

  const secretFindings = $derived<Finding[]>(
    securityFindings.filter((f) => f.rule_id.includes("secret") || f.rule_id.includes("private-key"))
  );

  const unsafePatternFindings = $derived<Finding[]>(
    securityFindings.filter(
      (f) =>
        f.rule_id.includes("eval") ||
        f.rule_id.includes("shell") ||
        f.rule_id.includes("dangerous") ||
        f.rule_id.includes("cors") ||
        f.rule_id.includes("pickle") ||
        f.rule_id.includes("subprocess") ||
        f.rule_id.includes("injection")
    )
  );

  const licenseFindings = $derived<Finding[]>(
    securityFindings.filter((f) => f.rule_id.startsWith("license/"))
  );

  // Filters state
  let searchQuery = $state("");
  let selectedSeverity: "all" | "critical" | "high" | "medium" | "low" = $state("all");
  let selectedCategory: "all" | "vulnerabilities" | "secrets" | "patterns" | "licenses" = $state("all");
  let selectedEcosystem: string = $state("all");

  const criticalCount = $derived<number>(securityFindings.filter((f) => f.severity === "critical").length);
  const highCount = $derived<number>(securityFindings.filter((f) => f.severity === "high").length);
  const mediumCount = $derived<number>(securityFindings.filter((f) => f.severity === "medium").length);
  const lowCount = $derived<number>(securityFindings.filter((f) => f.severity === "low" || f.severity === "info").length);

  const filteredFindings = $derived<Finding[]>(
    securityFindings.filter((f) => {
      if (selectedSeverity !== "all" && f.severity !== selectedSeverity) {
        return false;
      }
      if (selectedCategory === "vulnerabilities") {
        if (!f.rule_id.includes("vulnerable-dependency") && !f.rule_id.includes("cve-") && !f.rule_id.includes("ghsa-")) return false;
      } else if (selectedCategory === "secrets") {
        if (!f.rule_id.includes("secret") && !f.rule_id.includes("private-key")) return false;
      } else if (selectedCategory === "patterns") {
        if (!unsafePatternFindings.some((u) => u.id === f.id)) return false;
      } else if (selectedCategory === "licenses") {
        if (!f.rule_id.startsWith("license/")) return false;
      }

      if (selectedEcosystem !== "all") {
        const textToMatch = `${f.evidence || ""} ${f.title} ${f.description}`.toLowerCase();
        if (!textToMatch.includes(selectedEcosystem.toLowerCase())) {
          return false;
        }
      }

      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matches =
          f.title.toLowerCase().includes(q) ||
          f.description.toLowerCase().includes(q) ||
          f.rule_id.toLowerCase().includes(q) ||
          (f.evidence && f.evidence.toLowerCase().includes(q));
        if (!matches) return false;
      }

      return true;
    })
  );

  function extractCveId(finding: Finding): string | null {
    const match = (finding.evidence + " " + finding.description + " " + finding.title).match(/(CVE-\d{4}-\d{4,7}|GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4})/i);
    return match ? match[1].toUpperCase() : null;
  }

  function getAdvisoryUrl(cveId: string): string {
    if (cveId.startsWith("GHSA")) {
      return `https://github.com/advisories/${cveId}`;
    }
    return `https://nvd.nist.gov/vuln/detail/${cveId}`;
  }

  function generateRemediationCommand(finding: Finding): string | null {
    const ev = finding.evidence || "";
    // e.g. "Vulnerable package: lodash@4.17.15 (npm) | Fixed in: 4.17.19"
    const pkgMatch = ev.match(/Vulnerable package: ([^\s@]+)@([^\s]+) \((\w+)\)/);
    const fixedMatch = ev.match(/Fixed in: ([^\s|]+)/);

    if (pkgMatch && fixedMatch) {
      const [, pkgName, , eco] = pkgMatch;
      const fixedVer = fixedMatch[1];
      switch (eco.toLowerCase()) {
        case "npm":
          return `npm install ${pkgName}@^${fixedVer}`;
        case "pypi":
          return `pip install --upgrade "${pkgName}>=${fixedVer}"`;
        case "go":
          return `go get -u ${pkgName}@v${fixedVer.replace(/^v/, "")}`;
        case "composer":
          return `composer require ${pkgName}:^${fixedVer}`;
        case "cargo":
          return `cargo update -p ${pkgName} --precise ${fixedVer}`;
      }
    }
    return null;
  }

  let copiedCommand = $state<string | null>(null);
  function copyToClipboard(text: string) {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      navigator.clipboard.writeText(text);
      copiedCommand = text;
      setTimeout(() => {
        copiedCommand = null;
      }, 2000);
    }
  }
</script>

<div class="space-y-6">
  <!-- Title & Context -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#262c3f]">
    <div>
      <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
        <span>Security & Vulnerability Intelligence</span>
        {#if securityFindings.length > 0}
          <span class="px-2 py-0.5 rounded text-xs font-mono {criticalCount + highCount > 0 ? 'bg-rose-950 text-rose-300 border border-rose-800' : 'bg-amber-950 text-amber-300 border border-amber-800'}">
            {securityFindings.length} {securityFindings.length === 1 ? 'Finding' : 'Findings'}
          </span>
        {:else}
          <span class="px-2 py-0.5 rounded text-xs font-mono bg-emerald-950 text-emerald-300 border border-emerald-800">
            Clean (0 Vulnerabilities)
          </span>
        {/if}
      </h1>
      <p class="text-xs text-slate-400 mt-1">
        Deterministic OSV supply-chain vulnerability audit, redacted secret prevention, and license compatibility analysis.
      </p>
    </div>

    <!-- Severity Filter Toggle -->
    <div class="inline-flex rounded-md border border-[#262c3f] bg-[#12151f] p-0.5 text-xs">
      <button
        onclick={() => (selectedSeverity = "all")}
        class="px-2 py-1 rounded transition-colors {selectedSeverity === 'all' ? 'bg-blue-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
      >
        All ({securityFindings.length})
      </button>
      <button
        onclick={() => (selectedSeverity = "critical")}
        class="px-2 py-1 rounded transition-colors {selectedSeverity === 'critical' ? 'bg-rose-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
      >
        Critical ({criticalCount})
      </button>
      <button
        onclick={() => (selectedSeverity = "high")}
        class="px-2 py-1 rounded transition-colors {selectedSeverity === 'high' ? 'bg-amber-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
      >
        High ({highCount})
      </button>
      <button
        onclick={() => (selectedSeverity = "medium")}
        class="px-2 py-1 rounded transition-colors {selectedSeverity === 'medium' ? 'bg-yellow-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
      >
        Medium ({mediumCount})
      </button>
      <button
        onclick={() => (selectedSeverity = "low")}
        class="px-2 py-1 rounded transition-colors {selectedSeverity === 'low' ? 'bg-slate-600 text-white font-medium' : 'text-slate-400 hover:text-slate-200'}"
      >
        Low ({lowCount})
      </button>
    </div>
  </div>

  <!-- Metric KPI Cards -->
  <div class="grid grid-cols-2 sm:grid-cols-4 gap-4 text-xs">
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4">
      <div class="text-[11px] font-mono uppercase text-rose-400">CVE / OSV Advisories</div>
      <div class="text-2xl font-bold font-mono text-rose-300 mt-1">{vulnerabilityFindings.length}</div>
    </div>
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4">
      <div class="text-[11px] font-mono uppercase text-amber-400">Hardcoded Secrets</div>
      <div class="text-2xl font-bold font-mono text-amber-300 mt-1">{secretFindings.length}</div>
    </div>
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4">
      <div class="text-[11px] font-mono uppercase text-blue-400">Unsafe Code Patterns</div>
      <div class="text-2xl font-bold font-mono text-blue-300 mt-1">{unsafePatternFindings.length}</div>
    </div>
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-4">
      <div class="text-[11px] font-mono uppercase text-purple-400">License Violations</div>
      <div class="text-2xl font-bold font-mono text-purple-300 mt-1">{licenseFindings.length}</div>
    </div>
  </div>

  <!-- Filters & Search Toolbar -->
  <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
    <!-- Category Tabs -->
    <div class="flex flex-wrap items-center gap-1.5 text-xs">
      <button
        onclick={() => (selectedCategory = "all")}
        class="px-2.5 py-1 rounded font-mono text-[11px] border transition-colors {selectedCategory === 'all'
          ? 'bg-blue-600 text-white border-blue-500 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        All Categories
      </button>
      <button
        onclick={() => (selectedCategory = "vulnerabilities")}
        class="px-2.5 py-1 rounded font-mono text-[11px] border transition-colors {selectedCategory === 'vulnerabilities'
          ? 'bg-rose-950 text-rose-300 border-rose-800 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        Vulnerabilities ({vulnerabilityFindings.length})
      </button>
      <button
        onclick={() => (selectedCategory = "secrets")}
        class="px-2.5 py-1 rounded font-mono text-[11px] border transition-colors {selectedCategory === 'secrets'
          ? 'bg-amber-950 text-amber-300 border-amber-800 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        Secrets ({secretFindings.length})
      </button>
      <button
        onclick={() => (selectedCategory = "patterns")}
        class="px-2.5 py-1 rounded font-mono text-[11px] border transition-colors {selectedCategory === 'patterns'
          ? 'bg-blue-950 text-blue-300 border-blue-800 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        Unsafe Patterns ({unsafePatternFindings.length})
      </button>
      <button
        onclick={() => (selectedCategory = "licenses")}
        class="px-2.5 py-1 rounded font-mono text-[11px] border transition-colors {selectedCategory === 'licenses'
          ? 'bg-purple-950 text-purple-300 border-purple-800 font-medium'
          : 'bg-[#12151f] text-slate-400 border-[#262c3f] hover:border-slate-600'}"
      >
        Licenses ({licenseFindings.length})
      </button>
    </div>

    <!-- Search Input -->
    <div class="w-full sm:max-w-xs">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search CVE, package, or rule..."
        class="w-full px-3 py-1.5 bg-[#12151f] border border-[#262c3f] rounded text-slate-200 font-mono text-xs focus:outline-none focus:border-blue-500"
      />
    </div>
  </div>

  {#if !bundle}
    <div class="rounded-lg border border-[#262c3f] bg-[#12151f] p-12 text-center text-slate-400 text-sm">
      No report loaded.
    </div>
  {:else if filteredFindings.length === 0}
    <div class="rounded-lg border border-emerald-900/40 bg-emerald-950/10 p-12 text-center space-y-2">
      <p class="text-emerald-400 font-semibold text-base">Zero Findings Matching Current Filters</p>
      <p class="text-xs text-slate-400 max-w-md mx-auto">
        No security vulnerabilities, leaked credentials, unsafe code patterns, or license policy violations found.
      </p>
    </div>
  {:else}
    <!-- Security Findings Stack -->
    <div class="space-y-4">
      {#each filteredFindings as finding (finding.id)}
        {@const cveId = extractCveId(finding)}
        {@const remediationCmd = generateRemediationCommand(finding)}
        <div class="rounded-lg border {finding.severity === 'critical' ? 'border-rose-700 bg-rose-950/10' : finding.severity === 'high' ? 'border-rose-900/60 bg-[#12151f]' : 'border-[#262c3f] bg-[#12151f]'} p-5 text-xs space-y-3.5">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span class="px-2 py-0.5 rounded text-[10px] font-mono uppercase font-bold {finding.severity === 'critical' ? 'bg-rose-950 text-rose-300 border border-rose-800' : finding.severity === 'high' ? 'bg-amber-950 text-amber-300 border border-amber-800' : 'bg-blue-950 text-blue-300 border border-blue-800'}">
                {finding.severity}
              </span>
              <span class="font-mono text-slate-300 font-semibold">{finding.rule_id}</span>
              {#if cveId}
                <a
                  href={getAdvisoryUrl(cveId)}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-rose-900/80 text-rose-200 border border-rose-700 hover:bg-rose-800 transition-colors inline-flex items-center gap-1"
                >
                  <span>{cveId}</span>
                  <span class="text-[9px]">↗</span>
                </a>
              {/if}
              <span class="text-slate-600">•</span>
              <span class="text-slate-400 font-mono text-[10px]">Confidence: {finding.confidence}</span>
            </div>

            {#if finding.location?.file_path}
              <div class="font-mono text-[11px] text-slate-400">
                <span>{finding.location.file_path}</span>
                {#if finding.location.start_line}
                  <span class="text-slate-500">:{finding.location.start_line}</span>
                {/if}
              </div>
            {/if}
          </div>

          <div>
            <h2 class="text-sm font-semibold text-slate-100">{finding.title}</h2>
            <p class="text-slate-300 mt-1 leading-relaxed">{finding.description}</p>
          </div>

          <!-- Structured Evidence Box -->
          {#if finding.evidence}
            <div class="space-y-1">
              <div class="text-[10px] uppercase font-mono text-slate-400">Diagnostic Evidence:</div>
              <pre class="p-2.5 rounded bg-[#090a0f] border border-[#262c3f] text-rose-300/90 font-mono text-[11px] overflow-x-auto whitespace-pre-wrap break-all">{finding.evidence}</pre>
            </div>
          {/if}

          <!-- Actionable Upgrade / Remediation Command -->
          {#if remediationCmd}
            <div class="p-3 rounded bg-[#090a0f] border border-emerald-900/50 flex flex-col sm:flex-row sm:items-center justify-between gap-2">
              <div class="flex items-center gap-2">
                <span class="text-[10px] font-mono uppercase font-bold text-emerald-400">Fix Command:</span>
                <code class="text-slate-200 font-mono text-xs select-all">{remediationCmd}</code>
              </div>
              <button
                onclick={() => copyToClipboard(remediationCmd)}
                class="px-2.5 py-1 rounded bg-[#1a1e2e] border border-slate-700 text-slate-300 hover:text-white font-mono text-[11px] transition-colors"
              >
                {copiedCommand === remediationCmd ? "Copied!" : "Copy"}
              </button>
            </div>
          {/if}

          <!-- Recommendation / Guidance -->
          {#if finding.recommendation}
            <div class="p-2.5 rounded bg-[#1a1e2e] border border-[#262c3f] text-slate-300">
              <span class="font-semibold text-slate-200">Mitigation:</span>
              <span class="ml-1.5">{finding.recommendation}</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
