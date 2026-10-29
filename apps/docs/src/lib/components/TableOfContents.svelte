<script lang="ts">
  import { page } from "$app/state";

  interface TocSubItem {
    id: string;
    title: string;
  }

  interface TocSection {
    id: string;
    title: string;
    children: TocSubItem[];
  }

  let sections = $state<TocSection[]>([]);
  let currentId = $state<string>("");

  const slugify = (text: string) =>
    text
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/(^-|-$)/g, "");

  const scanArticleHeadings = () => {
    if (typeof document === "undefined") return;
    const article = document.querySelector("article.syntax-article");
    if (!article) {
      sections = [];
      return;
    }

    const nodes = Array.from(article.querySelectorAll("h2, h3")) as HTMLElement[];
    const tree: TocSection[] = [];
    const usedIds = new Set<string>();

    for (const el of nodes) {
      if (el.closest(".not-toc")) continue;
      const rawTitle = (el.textContent || "").trim();
      if (!rawTitle) continue;

      if (!el.id) {
        let baseId = slugify(rawTitle) || "section";
        let candidate = baseId;
        let idx = 2;
        while (usedIds.has(candidate) || document.getElementById(candidate)) {
          candidate = `${baseId}-${idx++}`;
        }
        el.id = candidate;
      }
      usedIds.add(el.id);

      if (el.tagName === "H2") {
        tree.push({ id: el.id, title: rawTitle, children: [] });
      } else if (el.tagName === "H3" && tree.length > 0) {
        tree[tree.length - 1].children.push({ id: el.id, title: rawTitle });
      }
    }

    sections = tree;
    currentId = tree[0]?.id || "";
  };

  $effect(() => {
    // Re-scan whenever pathname changes
    const _pathname = page.url.pathname;
    if (typeof window === "undefined") return;

    const timer = setTimeout(() => {
      scanArticleHeadings();
    }, 30);

    const onScroll = () => {
      if (sections.length === 0) return;
      const allIds = sections.flatMap((s) => [s.id, ...s.children.map((c) => c.id)]);
      const scrollY = window.scrollY;
      let active = allIds[0] || "";
      for (const id of allIds) {
        const el = document.getElementById(id);
        if (!el) continue;
        const top = scrollY + el.getBoundingClientRect().top - 120;
        if (scrollY >= top) {
          active = id;
        } else {
          break;
        }
      }
      currentId = active;
    };

    window.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      clearTimeout(timer);
      window.removeEventListener("scroll", onScroll);
    };
  });

  const isSectionActive = (sec: TocSection) =>
    sec.id === currentId || sec.children.some((c) => c.id === currentId);
</script>

<div class="hidden xl:sticky xl:top-19 xl:-mr-6 xl:block xl:h-[calc(100vh-4.75rem)] xl:flex-none xl:overflow-y-auto xl:py-16 xl:pr-6">
  <nav aria-labelledby="on-this-page-title" class="w-56">
    {#if sections.length > 0}
      <h2
        id="on-this-page-title"
        class="font-display text-sm font-medium text-slate-900 dark:text-white"
      >
        On this page
      </h2>
      <ol role="list" class="mt-4 space-y-3 text-sm">
        {#each sections as section}
          <li>
            <h3>
              <a
                href="#{section.id}"
                class={isSectionActive(section)
                  ? "text-sky-500 font-medium"
                  : "font-normal text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-300"}
              >
                {section.title}
              </a>
            </h3>
            {#if section.children.length > 0}
              <ol
                role="list"
                class="mt-2 space-y-2.5 pl-4 text-xs text-slate-500 dark:text-slate-400 border-l border-slate-200 dark:border-slate-800"
              >
                {#each section.children as sub}
                  <li>
                    <a
                      href="#{sub.id}"
                      class={currentId === sub.id
                        ? "text-sky-500 font-medium"
                        : "hover:text-slate-600 dark:hover:text-slate-300"}
                    >
                      {sub.title}
                    </a>
                  </li>
                {/each}
              </ol>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
  </nav>
</div>

