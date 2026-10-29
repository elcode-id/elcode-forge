<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { base } from "$app/paths";
  import { searchIndex, type SearchEntry } from "$lib/navigation";

  let open = $state(false);
  let query = $state("");
  let selectedIndex = $state(0);
  let modifierKey = $state("Ctrl ");
  let inputEl = $state<HTMLInputElement | null>(null);

  const results = $derived.by<SearchEntry[]>(() => {
    const q = query.trim().toLowerCase();
    if (!q) return [];
    const terms = q.split(/\s+/);
    return searchIndex
      .filter((entry) => {
        const hay = `${entry.title} ${entry.pageTitle} ${entry.sectionTitle} ${entry.keywords}`.toLowerCase();
        return terms.every((t) => hay.includes(t));
      })
      .slice(0, 7);
  });

  $effect(() => {
    if (results.length > 0 && selectedIndex >= results.length) {
      selectedIndex = 0;
    }
  });

  $effect(() => {
    if (open && inputEl) {
      setTimeout(() => inputEl?.focus(), 10);
    }
  });

  onMount(() => {
    modifierKey = /(Mac|iPhone|iPod|iPad)/i.test(navigator.platform) ? "⌘" : "Ctrl ";

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "k" && (event.metaKey || event.ctrlKey)) {
        event.preventDefault();
        open = !open;
        if (!open) query = "";
      } else if (open && event.key === "Escape") {
        event.preventDefault();
        open = false;
        query = "";
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  const selectResult = (entry: SearchEntry) => {
    open = false;
    query = "";
    goto(`${base}${entry.href}`);
  };

  const handleInputKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (results.length > 0) {
        selectedIndex = (selectedIndex + 1) % results.length;
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (results.length > 0) {
        selectedIndex = (selectedIndex - 1 + results.length) % results.length;
      }
    } else if (e.key === "Enter" && results[selectedIndex]) {
      e.preventDefault();
      selectResult(results[selectedIndex]);
    }
  };
</script>

<button
  type="button"
  onclick={() => {
    open = true;
    selectedIndex = 0;
  }}
  class="group flex h-6 w-6 items-center justify-center sm:justify-start md:h-auto md:w-80 md:flex-none md:rounded-lg md:py-2.5 md:pr-3.5 md:pl-4 md:text-sm md:ring-1 md:ring-slate-200 md:hover:ring-slate-300 lg:w-96 dark:md:bg-slate-800/75 dark:md:ring-white/5 dark:md:ring-inset dark:md:hover:bg-slate-700/40 dark:md:hover:ring-slate-500"
>
  <svg
    aria-hidden="true"
    viewBox="0 0 20 20"
    class="h-5 w-5 flex-none fill-slate-400 group-hover:fill-slate-500 md:group-hover:fill-slate-400 dark:fill-slate-500"
  >
    <path
      d="M16.293 17.707a1 1 0 0 0 1.414-1.414l-1.414 1.414ZM9 14a5 5 0 0 1-5-5H2a7 7 0 0 0 7 7v-2ZM4 9a5 5 0 0 1 5-5V2a7 7 0 0 0-7 7h2Zm5-5a5 5 0 0 1 5 5h2a7 7 0 0 0-7-7v2Zm8.707 12.293-3.757-3.757-1.414 1.414 3.757 3.757 1.414-1.414ZM14 9a4.98 4.98 0 0 1-1.464 3.536l1.414 1.414A6.98 6.98 0 0 0 16 9h-2Zm-1.464 3.536A4.98 4.98 0 0 1 9 14v2a6.98 6.98 0 0 0 4.95-2.05l-1.414-1.414Z"
    />
  </svg>
  <span class="sr-only md:not-sr-only md:ml-2 md:text-slate-500 md:dark:text-slate-400">
    Search docs
  </span>
  <kbd class="ml-auto hidden font-medium text-slate-400 md:block dark:text-slate-500">
    <kbd class="font-sans">{modifierKey}</kbd><kbd class="font-sans">K</kbd>
  </kbd>
</button>

{#if open}
  <div class="fixed inset-0 z-50" role="dialog" aria-modal="true" aria-label="Search documentation">
    <button
      type="button"
      class="fixed inset-0 bg-slate-900/50 backdrop-blur-sm cursor-default"
      aria-label="Close search modal"
      onclick={() => {
        open = false;
        query = "";
      }}
    ></button>

    <div class="fixed inset-0 overflow-y-auto px-4 py-4 sm:px-6 sm:py-20 md:py-32 lg:px-8 lg:py-[15vh] pointer-events-none">
      <div class="pointer-events-auto mx-auto transform-gpu overflow-hidden rounded-xl bg-white shadow-xl sm:max-w-xl dark:bg-slate-800 dark:ring-1 dark:ring-slate-700">
        <div class="group relative flex h-12">
          <svg
            aria-hidden="true"
            viewBox="0 0 20 20"
            class="pointer-events-none absolute top-0 left-4 h-full w-5 fill-slate-400 dark:fill-slate-500"
          >
            <path
              d="M16.293 17.707a1 1 0 0 0 1.414-1.414l-1.414 1.414ZM9 14a5 5 0 0 1-5-5H2a7 7 0 0 0 7 7v-2ZM4 9a5 5 0 0 1 5-5V2a7 7 0 0 0-7 7h2Zm5-5a5 5 0 0 1 5 5h2a7 7 0 0 0-7-7v2Zm8.707 12.293-3.757-3.757-1.414 1.414 3.757 3.757 1.414-1.414ZM14 9a4.98 4.98 0 0 1-1.464 3.536l1.414 1.414A6.98 6.98 0 0 0 16 9h-2Zm-1.464 3.536A4.98 4.98 0 0 1 9 14v2a6.98 6.98 0 0 0 4.95-2.05l-1.414-1.414Z"
            />
          </svg>
          <input
            bind:this={inputEl}
            bind:value={query}
            onkeydown={handleInputKeyDown}
            type="search"
            placeholder="Find something in ELCODE Forge..."
            class="flex-auto appearance-none bg-transparent pl-12 pr-4 text-slate-900 outline-none placeholder:text-slate-400 sm:text-sm dark:text-white"
          />
        </div>

        {#if query.trim() !== ""}
          <div class="border-t border-slate-200 bg-white px-2 py-3 dark:border-slate-400/10 dark:bg-slate-800">
            {#if results.length === 0}
              <p class="px-4 py-8 text-center text-sm text-slate-700 dark:text-slate-400">
                No results for &ldquo;<span class="text-slate-900 dark:text-white">{query}</span>&rdquo;
              </p>
            {:else}
              <ul role="listbox">
                {#each results as result, idx}
                  <li role="option" aria-selected={selectedIndex === idx}>
                    <button
                      type="button"
                      onmouseenter={() => (selectedIndex = idx)}
                      onclick={() => selectResult(result)}
                      class="group block w-full text-left rounded-lg px-3 py-2 {selectedIndex === idx
                        ? 'bg-slate-100 dark:bg-slate-700/30'
                        : ''}"
                    >
                      <div
                        class="text-sm {selectedIndex === idx
                          ? 'text-sky-600 dark:text-sky-400 font-medium'
                          : 'text-slate-700 dark:text-slate-300'}"
                      >
                        {result.title}
                      </div>
                      <div class="mt-0.5 truncate text-xs whitespace-nowrap text-slate-500 dark:text-slate-400">
                        <span>{result.sectionTitle}</span>
                        <span class="mx-2 text-slate-300 dark:text-slate-700">/</span>
                        <span>{result.pageTitle}</span>
                      </div>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

