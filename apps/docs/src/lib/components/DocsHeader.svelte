<script lang="ts">
  import { page } from "$app/state";
  import { base } from "$app/paths";
  import { primaryPages } from "$lib/navigation";

  let {
    title,
    section,
  }: {
    title?: string;
    section?: string;
  } = $props();

  const normalize = (p: string) => {
    if (!p) return "/";
    const stripped = base && p.startsWith(base) ? p.slice(base.length) || "/" : p;
    return stripped.length > 1 && stripped.endsWith("/")
      ? stripped.slice(0, -1)
      : stripped;
  };

  const currentMeta = $derived(
    primaryPages.find((p) => p.href === normalize(page.url.pathname))
  );

  const resolvedSection = $derived(section ?? currentMeta?.section);
  const resolvedTitle = $derived(title ?? currentMeta?.title);
</script>

{#if resolvedSection || resolvedTitle}
  <header class="mb-9 space-y-1">
    {#if resolvedSection}
      <p class="font-display text-sm font-medium text-sky-500">
        {resolvedSection}
      </p>
    {/if}
    {#if resolvedTitle}
      <h1 class="font-display text-3xl tracking-tight text-slate-900 dark:text-white">
        {resolvedTitle}
      </h1>
    {/if}
  </header>
{/if}

