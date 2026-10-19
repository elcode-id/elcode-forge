<script lang="ts">
  import { dashboardState } from "$lib/data/store.svelte.js";
  import {
    ScoreRing,
    SeverityBadge,
    Card,
    EmptyState,
    KeyValue,
    DataTable,
  } from "@elcode-forge/ui";

  let bundle = $derived(dashboardState.bundle);

  let severityCounts = $derived.by(() => {
    if (!bundle) return { critical: 0, high: 0, medium: 0, low: 0, info: 0 };
    const counts = { critical: 0, high: 0, medium: 0, low: 0, info: 0 };
    for (const f of bundle.repository.findings) {
      const sev = f.severity.toLowerCase() as keyof typeof counts;
      if (counts[sev] !== undefined) counts[sev]++;
    }
    return counts;
  });

  let hotspotColumns = [
    { key: "path", header: "File Path" },
    { key: "commit_count", header: "Commits", align: "right" as const },
    { key: "contributors_count", header: "Authors", align: "right" as const },
  ];

  let criticalFindings = $derived.by(() => {
    if (!bundle) return [];
    return bundle.repository.findings
      .filter((f: { severity: string }) => f.severity === "critical" || f.severity === "high")
      .slice(0, 5);
  });
</script>

{#if !bundle}
  <EmptyState
    title="No Intelligence Bundle Loaded"
    description="Connect to a live 'eforge serve' daemon or load an exported ReportBundle JSON file to inspect codebase health, architecture, security, and blast radius."
  />
{:else}
  <div class="space-y-6">
    <!-- Top Summary Banner -->
    <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 pb-4 border-b border-[#262c3f]">
      <div>
        <h1 class="text-2xl font-bold tracking-tight text-white font-mono">
          {bundle.repository.name}
        </h1>
        <p class="text-xs text-slate-400 font-mono mt-1">
          {bundle.repository.root_path}
          {#if bundle.revision}
            <span class="mx-2">•</span>
            <span class="text-blue-400">commit {bundle.revision.head_sha.slice(0, 10)}</span>
          {/if}
        </p>
      </div>

      <div class="flex items-center gap-2">
        {#each bundle.repository.detected_frameworks as fw (fw)}
          <span class="px-2 py-0.5 text-xs rounded bg-blue-950/70 border border-blue-800 text-blue-300 font-mono uppercase">
            {fw}
          </span>
        {/each}
        {#each bundle.repository.detected_languages as lang (lang)}
          <span class="px-2 py-0.5 text-xs rounded bg-slate-800 border border-slate-700 text-slate-300 font-mono uppercase">
            {lang}
          </span>
        {/each}
      </div>
    </div>

    <!-- Health Score & Findings Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Health Ring Card -->
      <Card title="Overall Health">
        <div class="flex flex-col items-center justify-center py-4">
          <ScoreRing score={bundle.repository.health?.overall ?? 0} size={140} strokeWidth={12} />
          <div class="mt-4 text-center">
            <span class="text-xs text-slate-400">
              Deterministic 8-dimension health model
            </span>
          </div>
        </div>
      </Card>

      <!-- Findings Breakdown -->
      <Card title="Findings by Severity">
        <div class="space-y-2 py-2">
          <div class="flex items-center justify-between py-1.5 px-3 rounded bg-red-950/30 border border-red-900/50">
            <SeverityBadge severity="critical" />
            <span class="font-mono text-sm font-bold text-red-400">{severityCounts.critical}</span>
          </div>
          <div class="flex items-center justify-between py-1.5 px-3 rounded bg-orange-950/30 border border-orange-900/50">
            <SeverityBadge severity="high" />
            <span class="font-mono text-sm font-bold text-orange-400">{severityCounts.high}</span>
          </div>
          <div class="flex items-center justify-between py-1.5 px-3 rounded bg-amber-950/30 border border-amber-900/50">
            <SeverityBadge severity="medium" />
            <span class="font-mono text-sm font-bold text-amber-400">{severityCounts.medium}</span>
          </div>
          <div class="flex items-center justify-between py-1.5 px-3 rounded bg-blue-950/30 border border-blue-900/50">
            <SeverityBadge severity="low" />
            <span class="font-mono text-sm font-bold text-blue-400">{severityCounts.low}</span>
          </div>
          <div class="flex items-center justify-between py-1.5 px-3 rounded bg-slate-900 border border-slate-800">
            <SeverityBadge severity="info" />
            <span class="font-mono text-sm font-bold text-slate-400">{severityCounts.info}</span>
          </div>
        </div>
      </Card>

      <!-- Inventory Key Values -->
      <Card title="Repository Inventory">
        <div class="divide-y divide-slate-800/80">
          <KeyValue label="Files Analyzed" value={bundle.repository.files.length} mono />
          <KeyValue label="Modules Inferred" value={bundle.repository.modules.length} mono />
          <KeyValue label="Dependencies" value={bundle.repository.dependencies.length} mono />
          <KeyValue label="Circular Dep Cycles" value={bundle.architecture.circular_dependencies.length} mono />
          <KeyValue label="Hotspots Tracked" value={bundle.hotspots.length} mono />
          <KeyValue label="Report Schema" value={`v${bundle.schema_version}`} mono />
        </div>
      </Card>
    </div>

    <!-- Dimension Breakdown Grid -->
    {#if bundle.repository.health?.dimensions}
      <Card title="Health Dimensions Breakdown">
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {#each bundle.repository.health.dimensions as dim (dim.dimension)}
            <div class="p-3.5 rounded bg-[#12151f] border border-[#262c3f]">
              <div class="flex items-center justify-between mb-2">
                <span class="text-xs font-semibold text-slate-300">{dim.dimension}</span>
                <span class="font-mono text-xs font-bold px-2 py-0.5 rounded bg-[#1a1e2e] text-slate-200">
                  {dim.score} / 100
                </span>
              </div>
              <div class="text-[11px] text-slate-400 space-y-1">
                {#each dim.reasons.slice(0, 2) as reason, rIdx (rIdx)}
                  <p class="truncate">• {reason}</p>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </Card>
    {/if}

    <!-- Churn Hotspots Table -->
    {#if bundle.hotspots.length > 0}
      <Card title="Top Churn Hotspots" description="Files with the highest commit activity and churn">
        <DataTable columns={hotspotColumns} data={bundle.hotspots.slice(0, 5)} />
      </Card>
    {/if}

    <!-- High Severity Findings List -->
    {#if criticalFindings.length > 0}
      <Card title="Critical & High Findings" description="Top issues requiring engineering attention">
        <div class="space-y-3">
          {#each criticalFindings as f (f.id)}
            <div class="p-3.5 rounded bg-[#12151f] border border-[#262c3f] flex items-start justify-between gap-4">
              <div class="space-y-1">
                <div class="flex items-center gap-2">
                  <SeverityBadge severity={f.severity} />
                  <span class="text-xs font-mono text-slate-400">{f.rule_id}</span>
                  {#if f.location}
                    <span class="text-xs font-mono text-blue-400">
                      {f.location.file_path}{#if f.location.start_line}:{f.location.start_line}{/if}
                    </span>
                  {/if}
                </div>
                <h4 class="text-sm font-semibold text-slate-200">{f.title}</h4>
                <p class="text-xs text-slate-400">{f.description}</p>
              </div>
            </div>
          {/each}
        </div>
      </Card>
    {/if}
  </div>
{/if}
