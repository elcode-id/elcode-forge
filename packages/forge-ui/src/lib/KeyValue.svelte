<script lang="ts">
  interface Props {
    label: string;
    value: string | number;
    mono?: boolean;
    hint?: string;
  }

  let { label, value, mono = false, hint }: Props = $props();
</script>

<div class="flex items-center justify-between py-2 border-b border-slate-800/60 last:border-b-0 text-xs">
  <span class="text-slate-400 font-medium flex items-center gap-1">
    {label}
    {#if hint}
      <span class="text-[10px] text-slate-500">({hint})</span>
    {/if}
  </span>
  <span class="text-slate-200 {mono ? 'font-mono' : ''} font-medium">
    {value}
  </span>
</div>
