<script lang="ts">
  import type { Snippet } from "svelte";

  let { children }: { children?: Snippet } = $props();
</script>

<div class="not-prose not-toc my-12 grid grid-cols-1 gap-6 sm:grid-cols-2">
  {#if children}
    {@render children()}
  {/if}
</div>
