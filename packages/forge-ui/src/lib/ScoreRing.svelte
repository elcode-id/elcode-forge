<script lang="ts">
  interface Props {
    score: number;
    size?: number;
    strokeWidth?: number;
    label?: string;
  }

  let {
    score = 0,
    size = 120,
    strokeWidth = 10,
    label = "Health Score",
  }: Props = $props();

  let clampedScore = $derived(Math.max(0, Math.min(100, Math.round(score))));
  let radius = $derived((size - strokeWidth) / 2);
  let circumference = $derived(2 * Math.PI * radius);
  let strokeDashoffset = $derived(circumference - (clampedScore / 100) * circumference);

  let color = $derived(
    clampedScore >= 80 ? "var(--elcode-health-good, #10b981)" :
    clampedScore >= 60 ? "var(--elcode-health-warning, #f59e0b)" :
    "var(--elcode-health-bad, #ef4444)"
  );
</script>

<div
  class="relative inline-flex flex-col items-center justify-center"
  role="region"
  aria-label={label}
>
  <svg
    width={size}
    height={size}
    class="transform -rotate-90"
    aria-hidden="true"
  >
    <circle
      cx={size / 2}
      cy={size / 2}
      r={radius}
      stroke="var(--elcode-bg-subtle, #1a1e2e)"
      stroke-width={strokeWidth}
      fill="transparent"
    />
    <circle
      cx={size / 2}
      cy={size / 2}
      r={radius}
      stroke={color}
      stroke-width={strokeWidth}
      fill="transparent"
      stroke-dasharray={circumference}
      stroke-dashoffset={strokeDashoffset}
      stroke-linecap="round"
      class="transition-all duration-300 ease-out"
    />
  </svg>
  <div class="absolute inset-0 flex flex-col items-center justify-center select-none">
    <span class="text-2xl font-bold font-mono tracking-tight" style:color={color}>
      {clampedScore}
    </span>
    <span class="text-[10px] uppercase font-mono tracking-wider text-slate-400">
      / 100
    </span>
  </div>
</div>
