<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  const styles = {
    note: {
      container:
        "bg-sky-50 dark:bg-slate-800/60 dark:ring-1 dark:ring-slate-300/10",
      title: "text-sky-900 dark:text-sky-400",
      body: "text-sky-800 dark:text-slate-300",
    },
    warning: {
      container:
        "bg-amber-50 dark:bg-slate-800/60 dark:ring-1 dark:ring-slate-300/10",
      title: "text-amber-900 dark:text-amber-500",
      body: "text-amber-800 dark:text-slate-300",
    },
  };

  let {
    title,
    type = "note",
    children,
  }: {
    title: string;
    type?: keyof typeof styles;
    children?: Snippet;
  } = $props();
</script>

<div class="not-prose my-8 flex rounded-3xl p-6 {styles[type].container}">
  {#if type === "warning"}
    <Icon icon="warning" color="amber" class="h-8 w-8 flex-none" />
  {:else}
    <Icon icon="lightbulb" color="blue" class="h-8 w-8 flex-none" />
  {/if}
  <div class="ml-4 flex-auto">
    <p class="font-display text-xl font-normal {styles[type].title}">
      {title}
    </p>
    <div class="mt-2.5 text-sm leading-6 {styles[type].body}">
      {#if children}
        {@render children()}
      {/if}
    </div>
  </div>
</div>

