<script lang="ts">
  import { onMount } from "svelte";

  type ThemeMode = "light" | "dark" | "system";

  let theme = $state<ThemeMode>("dark");
  let open = $state(false);

  const applyTheme = (mode: ThemeMode) => {
    if (typeof document === "undefined") return;
    const isDark =
      mode === "dark" ||
      (mode === "system" &&
        window.matchMedia("(prefers-color-scheme: dark)").matches);
    if (isDark) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  };

  const selectTheme = (mode: ThemeMode) => {
    theme = mode;
    open = false;
    try {
      localStorage.setItem("eforge-docs-theme", mode);
    } catch (_) {}
    applyTheme(mode);
  };

  onMount(() => {
    try {
      const saved = localStorage.getItem("eforge-docs-theme") as ThemeMode | null;
      if (saved === "light" || saved === "dark" || saved === "system") {
        theme = saved;
      }
    } catch (_) {}
    applyTheme(theme);
  });
</script>

<div class="relative z-10">
  <button
    type="button"
    onclick={() => (open = !open)}
    class="flex h-6 w-6 items-center justify-center rounded-lg shadow-md ring-1 shadow-black/5 ring-black/5 dark:bg-slate-700 dark:ring-white/5 dark:ring-inset"
    aria-label="Theme"
  >
    <!-- Light Icon -->
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      class="h-4 w-4 dark:hidden {theme === 'system' ? 'fill-slate-400' : 'fill-sky-400'}"
    >
      <path
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M7 1a1 1 0 0 1 2 0v1a1 1 0 1 1-2 0V1Zm4 7a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm2.657-5.657a1 1 0 0 0-1.414 0l-.707.707a1 1 0 0 0 1.414 1.414l.707-.707a1 1 0 0 0 0-1.414Zm-1.415 11.313-.707-.707a1 1 0 0 1 1.415-1.415l.707.708a1 1 0 0 1-1.415 1.414ZM16 7.999a1 1 0 0 0-1-1h-1a1 1 0 1 0 0 2h1a1 1 0 0 0 1-1ZM7 14a1 1 0 1 1 2 0v1a1 1 0 1 1-2 0v-1Zm-2.536-2.464a1 1 0 0 0-1.414 0l-.707.707a1 1 0 0 0 1.414 1.414l.707-.707a1 1 0 0 0 0-1.414Zm0-8.486A1 1 0 0 1 3.05 4.464l-.707-.707a1 1 0 0 1 1.414-1.414l.707.707ZM3 8a1 1 0 0 0-1-1H1a1 1 0 0 0 0 2h1a1 1 0 0 0 1-1Z"
      />
    </svg>
    <!-- Dark Icon -->
    <svg
      aria-hidden="true"
      viewBox="0 0 16 16"
      class="hidden h-4 w-4 dark:block {theme === 'system' ? 'fill-slate-400' : 'fill-sky-400'}"
    >
      <path
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M7.23 3.333C7.757 2.905 7.68 2 7 2a6 6 0 1 0 0 12c.68 0 .758-.905.23-1.332A5.989 5.989 0 0 1 5 8c0-1.885.87-3.568 2.23-4.668ZM12 5a1 1 0 0 1 1 1 1 1 0 0 0 1 1 1 1 0 1 1 0 2 1 1 0 0 0-1 1 1 1 0 1 1-2 0 1 1 0 0 0-1-1 1 1 0 1 1 0-2 1 1 0 0 0 1-1 1 1 0 0 1 1-1Z"
      />
    </svg>
  </button>

  {#if open}
    <button
      type="button"
      class="fixed inset-0 z-10 cursor-default"
      aria-label="Close theme menu"
      onclick={() => (open = false)}
    ></button>
    <div
      class="absolute top-full right-0 z-20 mt-3 w-36 space-y-1 rounded-xl bg-white p-3 text-sm font-medium shadow-md ring-1 shadow-black/5 ring-black/5 dark:bg-slate-800 dark:ring-white/5"
    >
      <button
        type="button"
        onclick={() => selectTheme("light")}
        class="flex w-full cursor-pointer items-center rounded-[0.625rem] p-1 select-none hover:bg-slate-100 dark:hover:bg-slate-900/40 {theme === 'light'
          ? 'text-sky-500'
          : 'text-slate-700 dark:text-slate-400'}"
      >
        <div class="rounded-md bg-white p-1 shadow-sm ring-1 ring-slate-900/5 dark:bg-slate-700 dark:ring-white/5 dark:ring-inset">
          <svg
            aria-hidden="true"
            viewBox="0 0 16 16"
            class="h-4 w-4 {theme === 'light' ? 'fill-sky-400' : 'fill-slate-400'}"
          >
            <path
              fill-rule="evenodd"
              clip-rule="evenodd"
              d="M7 1a1 1 0 0 1 2 0v1a1 1 0 1 1-2 0V1Zm4 7a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm2.657-5.657a1 1 0 0 0-1.414 0l-.707.707a1 1 0 0 0 1.414 1.414l.707-.707a1 1 0 0 0 0-1.414Zm-1.415 11.313-.707-.707a1 1 0 0 1 1.415-1.415l.707.708a1 1 0 0 1-1.415 1.414ZM16 7.999a1 1 0 0 0-1-1h-1a1 1 0 1 0 0 2h1a1 1 0 0 0 1-1ZM7 14a1 1 0 1 1 2 0v1a1 1 0 1 1-2 0v-1Zm-2.536-2.464a1 1 0 0 0-1.414 0l-.707.707a1 1 0 0 0 1.414 1.414l.707-.707a1 1 0 0 0 0-1.414Zm0-8.486A1 1 0 0 1 3.05 4.464l-.707-.707a1 1 0 0 1 1.414-1.414l.707.707ZM3 8a1 1 0 0 0-1-1H1a1 1 0 0 0 0 2h1a1 1 0 0 0 1-1Z"
            />
          </svg>
        </div>
        <div class="ml-3">Light</div>
      </button>

      <button
        type="button"
        onclick={() => selectTheme("dark")}
        class="flex w-full cursor-pointer items-center rounded-[0.625rem] p-1 select-none hover:bg-slate-100 dark:hover:bg-slate-900/40 {theme === 'dark'
          ? 'text-sky-500'
          : 'text-slate-700 dark:text-slate-400'}"
      >
        <div class="rounded-md bg-white p-1 shadow-sm ring-1 ring-slate-900/5 dark:bg-slate-700 dark:ring-white/5 dark:ring-inset">
          <svg
            aria-hidden="true"
            viewBox="0 0 16 16"
            class="h-4 w-4 {theme === 'dark' ? 'fill-sky-400' : 'fill-slate-400'}"
          >
            <path
              fill-rule="evenodd"
              clip-rule="evenodd"
              d="M7.23 3.333C7.757 2.905 7.68 2 7 2a6 6 0 1 0 0 12c.68 0 .758-.905.23-1.332A5.989 5.989 0 0 1 5 8c0-1.885.87-3.568 2.23-4.668ZM12 5a1 1 0 0 1 1 1 1 1 0 0 0 1 1 1 1 0 1 1 0 2 1 1 0 0 0-1 1 1 1 0 1 1-2 0 1 1 0 0 0-1-1 1 1 0 1 1 0-2 1 1 0 0 0 1-1 1 1 0 0 1 1-1Z"
            />
          </svg>
        </div>
        <div class="ml-3">Dark</div>
      </button>

      <button
        type="button"
        onclick={() => selectTheme("system")}
        class="flex w-full cursor-pointer items-center rounded-[0.625rem] p-1 select-none hover:bg-slate-100 dark:hover:bg-slate-900/40 {theme === 'system'
          ? 'text-sky-500'
          : 'text-slate-700 dark:text-slate-400'}"
      >
        <div class="rounded-md bg-white p-1 shadow-sm ring-1 ring-slate-900/5 dark:bg-slate-700 dark:ring-white/5 dark:ring-inset">
          <svg
            aria-hidden="true"
            viewBox="0 0 16 16"
            class="h-4 w-4 {theme === 'system' ? 'fill-sky-400' : 'fill-slate-400'}"
          >
            <path
              fill-rule="evenodd"
              clip-rule="evenodd"
              d="M1 4a3 3 0 0 1 3-3h8a3 3 0 0 1 3 3v4a3 3 0 0 1-3 3h-1.5l.31 1.242c.084.333.36.573.63.808.091.08.182.158.264.24A1 1 0 0 1 11 15H5a1 1 0 0 1-.704-1.71c.082-.082.173-.16.264-.24.27-.235.546-.475.63-.808L5.5 11H4a3 3 0 0 1-3-3V4Zm3-1a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H4Z"
            />
          </svg>
        </div>
        <div class="ml-3">System</div>
      </button>
    </div>
  {/if}
</div>

