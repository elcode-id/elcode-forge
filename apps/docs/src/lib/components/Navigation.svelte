<script lang="ts">
  import { page } from "$app/state";
  import { base } from "$app/paths";
  import { navigation } from "$lib/navigation";

  let {
    class: className = "",
    onLinkClick,
  }: {
    class?: string;
    onLinkClick?: () => void;
  } = $props();

  const normalize = (p: string) => {
    if (!p) return "/";
    const stripped = base && p.startsWith(base) ? p.slice(base.length) || "/" : p;
    return stripped.length > 1 && stripped.endsWith("/")
      ? stripped.slice(0, -1)
      : stripped;
  };

  const currentPath = $derived(normalize(page.url.pathname));
  const currentHash = $derived(page.url.hash);

  const isLinkActive = (href: string) => {
    const [linkPath, linkHash] = href.split("#");
    const normLinkPath = linkPath || "/";
    if (linkHash) {
      return currentPath === normLinkPath && currentHash === `#${linkHash}`;
    }
    return currentPath === normLinkPath && (!currentHash || href === normLinkPath);
  };
</script>

<nav class="text-base lg:text-sm {className}">
  <ul role="list" class="space-y-9">
    {#each navigation as section}
      <li>
        <h2 class="font-display font-medium text-slate-900 dark:text-white">
          {section.title}
        </h2>
        <ul
          role="list"
          class="mt-2 space-y-2 border-l-2 border-slate-100 lg:mt-4 lg:space-y-4 lg:border-slate-200 dark:border-slate-800"
        >
          {#each section.links as link}
            {@const active = isLinkActive(link.href)}
            <li class="relative">
              <a
                href="{base}{link.href}"
                onclick={onLinkClick}
                class="block w-full pl-3.5 before:pointer-events-none before:absolute before:top-1/2 before:-left-1 before:h-1.5 before:w-1.5 before:-translate-y-1/2 before:rounded-full {active
                  ? 'font-semibold text-sky-500 before:bg-sky-500'
                  : 'text-slate-500 before:hidden before:bg-slate-300 hover:text-slate-600 hover:before:block dark:text-slate-400 dark:before:bg-slate-700 dark:hover:text-slate-300'}"
              >
                {link.title}
              </a>
            </li>
          {/each}
        </ul>
      </li>
    {/each}
  </ul>
</nav>

