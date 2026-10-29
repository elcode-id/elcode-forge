<script lang="ts">
  import { page } from "$app/state";
  import { base } from "$app/paths";
  import { primaryPages } from "$lib/navigation";

  const normalize = (p: string) => {
    if (!p) return "/";
    const stripped = base && p.startsWith(base) ? p.slice(base.length) || "/" : p;
    return stripped.length > 1 && stripped.endsWith("/")
      ? stripped.slice(0, -1)
      : stripped;
  };

  const linkIndex = $derived(
    primaryPages.findIndex((p) => p.href === normalize(page.url.pathname))
  );

  const previousPage = $derived(linkIndex > 0 ? primaryPages[linkIndex - 1] : null);
  const nextPage = $derived(
    linkIndex > -1 && linkIndex < primaryPages.length - 1
      ? primaryPages[linkIndex + 1]
      : null
  );
</script>

{#if previousPage || nextPage}
  <dl class="mt-12 flex border-t border-slate-200 pt-6 dark:border-slate-800">
    {#if previousPage}
      <div>
        <dt class="font-display text-sm font-medium text-slate-900 dark:text-white">
          Previous
        </dt>
        <dd class="mt-1">
          <a
            href="{base}{previousPage.href}"
            class="flex flex-row-reverse items-center gap-x-1 text-base font-semibold text-slate-500 hover:text-slate-600 dark:text-slate-400 dark:hover:text-slate-300"
          >
            {previousPage.title}
            <svg
              viewBox="0 0 16 16"
              aria-hidden="true"
              class="h-4 w-4 flex-none fill-current -scale-x-100"
            >
              <path
                d="m9.182 13.423-1.17-1.16 3.505-3.505H3V7.065h8.517l-3.506-3.5L9.181 2.4l5.512 5.511-5.511 5.512Z"
              />
            </svg>
          </a>
        </dd>
      </div>
    {/if}
    {#if nextPage}
      <div class="ml-auto text-right">
        <dt class="font-display text-sm font-medium text-slate-900 dark:text-white">
          Next
        </dt>
        <dd class="mt-1">
          <a
            href="{base}{nextPage.href}"
            class="flex items-center gap-x-1 text-base font-semibold text-slate-500 hover:text-slate-600 dark:text-slate-400 dark:hover:text-slate-300"
          >
            {nextPage.title}
            <svg
              viewBox="0 0 16 16"
              aria-hidden="true"
              class="h-4 w-4 flex-none fill-current"
            >
              <path
                d="m9.182 13.423-1.17-1.16 3.505-3.505H3V7.065h8.517l-3.506-3.5L9.181 2.4l5.512 5.511-5.511 5.512Z"
              />
            </svg>
          </a>
        </dd>
      </div>
    {/if}
  </dl>
{/if}

