<script lang="ts">
  import { base } from "$app/paths";
  import Logo from "./Logo.svelte";
  import Navigation from "./Navigation.svelte";

  let isOpen = $state(false);

  const close = () => {
    isOpen = false;
  };
</script>

<button
  type="button"
  onclick={() => (isOpen = true)}
  class="relative"
  aria-label="Open navigation"
>
  <svg
    aria-hidden="true"
    viewBox="0 0 24 24"
    fill="none"
    stroke-width="2"
    stroke-linecap="round"
    class="h-6 w-6 stroke-slate-500"
  >
    <path d="M4 7h16M4 12h16M4 17h16" />
  </svg>
</button>

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-start overflow-y-auto bg-slate-900/50 pr-10 backdrop-blur-sm lg:hidden"
    role="dialog"
    aria-modal="true"
    aria-label="Navigation"
  >
    <!-- Backdrop click area -->
    <button
      type="button"
      class="fixed inset-0 cursor-default"
      aria-label="Close navigation backdrop"
      onclick={close}
    ></button>

    <div class="relative z-10 min-h-full w-full max-w-xs bg-white px-4 pt-5 pb-12 sm:px-6 dark:bg-slate-900">
      <div class="flex items-center">
        <button
          type="button"
          onclick={close}
          aria-label="Close navigation"
        >
          <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            fill="none"
            stroke-width="2"
            stroke-linecap="round"
            class="h-6 w-6 stroke-slate-500"
          >
            <path d="M5 5l14 14M19 5l-14 14" />
          </svg>
        </button>
        <a href="{base}/" onclick={close} class="ml-6" aria-label="Home page">
          <Logo logomarkOnly class="h-9 w-9" />
        </a>
      </div>
      <Navigation class="mt-5 px-1" onLinkClick={close} />
    </div>
  </div>
{/if}

