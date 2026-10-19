<script lang="ts">
  type Severity = "critical" | "high" | "medium" | "low" | "info";

  interface Props {
    severity: Severity | string;
  }

  let { severity }: Props = $props();

  let normalized = $derived(severity.toLowerCase());

  let bgClass = $derived(
    normalized === "critical" ? "bg-red-950/60 text-red-400 border-red-800/80" :
    normalized === "high" ? "bg-orange-950/60 text-orange-400 border-orange-800/80" :
    normalized === "medium" ? "bg-amber-950/60 text-amber-400 border-amber-800/80" :
    normalized === "low" ? "bg-blue-950/60 text-blue-400 border-blue-800/80" :
    "bg-slate-900 text-slate-400 border-slate-700"
  );
</script>

<span
  class="inline-flex items-center px-2 py-0.5 rounded text-xs font-mono font-medium uppercase tracking-wider border {bgClass}"
  role="status"
  aria-label={`Severity: ${normalized}`}
>
  {normalized}
</span>
