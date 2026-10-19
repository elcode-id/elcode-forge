<script lang="ts">
  interface Props {
    values: number[];
    width?: number;
    height?: number;
    color?: string;
    strokeWidth?: number;
    label?: string;
  }

  let {
    values = [],
    width = 120,
    height = 32,
    color = "#38bdf8",
    strokeWidth = 2,
    label = "Trend sparkline",
  }: Props = $props();

  const points = $derived.by<string>(() => {
    if (!values || values.length === 0) return "";
    if (values.length === 1) {
      return `0,${height / 2} ${width},${height / 2}`;
    }

    const min = Math.min(...values);
    const max = Math.max(...values);
    const range = max - min || 1;
    const padding = 4;
    const effectiveHeight = height - padding * 2;

    const step = width / (values.length - 1);
    return values
      .map((val, idx) => {
        const x = idx * step;
        const normalized = (val - min) / range;
        // Invert Y because SVG coordinate 0 is top
        const y = height - padding - normalized * effectiveHeight;
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  });

  const latestValue = $derived(values.length > 0 ? values[values.length - 1] : null);
</script>

<div class="inline-flex items-center gap-2">
  {#if values.length > 0}
    <svg
      {width}
      {height}
      viewBox="0 0 {width} {height}"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      aria-label="{label}: latest {latestValue}"
      class="overflow-visible"
    >
      <polyline
        fill="none"
        stroke={color}
        stroke-width={strokeWidth}
        stroke-linecap="round"
        stroke-linejoin="round"
        points={points}
      />
    </svg>
  {:else}
    <span class="text-xs text-slate-500 font-mono italic">No data</span>
  {/if}
</div>
