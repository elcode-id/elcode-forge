<script lang="ts">
  let {
    logomarkOnly = false,
    class: className = "",
  }: {
    logomarkOnly?: boolean;
    class?: string;
  } = $props();
</script>

{#if logomarkOnly}
  <svg aria-hidden="true" viewBox="0 0 36 36" fill="none" class={className}>
    <g fill="none" stroke="#38BDF8" stroke-linejoin="round" stroke-width="3">
      <path d="M10.308 5L18 17.5 10.308 30 2.615 17.5 10.308 5z" />
      <path d="M18 17.5L10.308 5h15.144l7.933 12.5M18 17.5h15.385L25.452 30H10.308L18 17.5z" />
    </g>
  </svg>
{:else}
  <div class="inline-flex items-center gap-3 {className}">
    <svg aria-hidden="true" viewBox="0 0 36 36" fill="none" class="h-9 w-9 flex-none">
      <g fill="none" stroke="#38BDF8" stroke-linejoin="round" stroke-width="3">
        <path d="M10.308 5L18 17.5 10.308 30 2.615 17.5 10.308 5z" />
        <path d="M18 17.5L10.308 5h15.144l7.933 12.5M18 17.5h15.385L25.452 30H10.308L18 17.5z" />
      </g>
    </svg>
    <div class="flex items-center gap-2">
      <span class="font-display text-lg font-semibold tracking-tight text-slate-900 dark:text-sky-100">
        ELCODE Forge
      </span>
      <span class="rounded-full bg-sky-400/10 px-2 py-0.5 font-mono text-[11px] font-medium text-sky-600 ring-1 ring-inset ring-sky-400/20 dark:text-sky-400">
        v1.0.0
      </span>
    </div>
  </div>
{/if}

