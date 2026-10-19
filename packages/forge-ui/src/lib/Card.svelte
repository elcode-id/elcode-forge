<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title?: string;
    description?: string;
    children?: Snippet;
    actions?: Snippet;
  }

  let { title, description, children, actions }: Props = $props();
</script>

<div class="elcode-card bg-slate-900/80 border border-slate-800 rounded-lg p-5">
  {#if title || actions}
    <div class="flex items-center justify-between pb-3 border-b border-slate-800/80 mb-4">
      <div>
        {#if title}
          <h3 class="text-sm font-semibold tracking-tight text-slate-100">{title}</h3>
        {/if}
        {#if description}
          <p class="text-xs text-slate-400 mt-0.5">{description}</p>
        {/if}
      </div>
      {#if actions}
        <div class="flex items-center gap-2">
          {@render actions()}
        </div>
      {/if}
    </div>
  {/if}
  {#if children}
    <div>
      {@render children()}
    </div>
  {/if}
</div>
