<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    description?: string;
    action?: Snippet;
  }

  let { title, description, action }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center py-12 px-4 text-center border border-dashed border-slate-800 rounded-lg bg-slate-900/30">
  <div class="w-10 h-10 rounded-full bg-slate-800/80 flex items-center justify-center text-slate-400 mb-3">
    <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
    </svg>
  </div>
  <h4 class="text-sm font-medium text-slate-200">{title}</h4>
  {#if description}
    <p class="text-xs text-slate-400 max-w-sm mt-1">{description}</p>
  {/if}
  {#if action}
    <div class="mt-4">
      {@render action()}
    </div>
  {/if}
</div>
