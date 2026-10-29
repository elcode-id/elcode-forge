<script lang="ts">
  import type { Snippet } from "svelte";

  const variantStyles = {
    primary:
      "rounded-full bg-sky-300 py-2 px-4 text-sm font-semibold text-slate-900 hover:bg-sky-200 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-sky-300/50 active:bg-sky-500 transition-colors",
    secondary:
      "rounded-full bg-slate-800 py-2 px-4 text-sm font-medium text-white hover:bg-slate-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white/50 active:text-slate-400 transition-colors",
  };

  let {
    variant = "primary",
    href,
    target,
    rel,
    class: className = "",
    children,
  }: {
    variant?: keyof typeof variantStyles;
    href?: string;
    target?: string;
    rel?: string;
    class?: string;
    children?: Snippet;
  } = $props();
</script>

{#if href}
  <a {href} {target} {rel} class="{variantStyles[variant]} {className}">
    {#if children}
      {@render children()}
    {/if}
  </a>
{:else}
  <button type="button" class="{variantStyles[variant]} {className}">
    {#if children}
      {@render children()}
    {/if}
  </button>
{/if}

