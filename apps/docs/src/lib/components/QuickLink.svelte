<script lang="ts">
  import Icon, { type IconName } from "./Icon.svelte";

  let {
    title,
    description,
    href,
    icon = "installation",
  }: {
    title: string;
    description: string;
    href: string;
    icon?: IconName;
  } = $props();
</script>

<div class="group relative rounded-xl border border-slate-200 dark:border-slate-800">
  <div
    class="absolute -inset-px rounded-xl border-2 border-transparent opacity-0 [background:linear-gradient(var(--quick-links-hover-bg,#f0f9ff),var(--quick-links-hover-bg,#f0f9ff))_padding-box,linear-gradient(to_top,#818cf8,#22d3ee,#0ea5e9)_border-box] group-hover:opacity-100 transition-opacity dark:[--quick-links-hover-bg:#1e293b]"
  ></div>
  <div class="relative overflow-hidden rounded-xl p-6">
    <Icon {icon} class="h-8 w-8" />
    <h2 class="mt-4 font-display text-base font-medium text-slate-900 dark:text-white">
      <a {href} class="focus:outline-none">
        <span class="absolute -inset-px rounded-xl"></span>
        {title}
      </a>
    </h2>
    <p class="mt-1 text-sm text-slate-700 dark:text-slate-400">
      {description}
    </p>
  </div>
</div>

