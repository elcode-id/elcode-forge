<script lang="ts" generics="T extends Record<string, unknown>">
  import type { Snippet } from "svelte";
  import type { Column } from "./types.js";

  interface Props {
    columns: Column[];
    data: T[];
    emptyMessage?: string;
    rowSnippet?: Snippet<[T, string]>;
  }

  let {
    columns,
    data,
    emptyMessage = "No records found",
    rowSnippet,
  }: Props = $props();
</script>

<div class="overflow-x-auto border border-slate-800 rounded-lg">
  <table class="w-full text-left text-xs">
    <thead class="bg-slate-900/90 text-slate-400 font-mono uppercase tracking-wider border-b border-slate-800">
      <tr>
        {#each columns as col (col.key)}
          <th
            scope="col"
            class="px-3.5 py-2.5 font-medium {col.align === 'right' ? 'text-right' : col.align === 'center' ? 'text-center' : 'text-left'}"
            style:width={col.width}
          >
            {col.header}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody class="divide-y divide-slate-800/60 bg-slate-950/40">
      {#if data.length === 0}
        <tr>
          <td colspan={columns.length} class="px-4 py-8 text-center text-slate-500 italic">
            {emptyMessage}
          </td>
        </tr>
      {:else}
        {#each data as row, idx (idx)}
          <tr class="hover:bg-slate-900/50 transition-colors">
            {#each columns as col (col.key)}
              <td class="px-3.5 py-2.5 text-slate-200 {col.align === 'right' ? 'text-right' : col.align === 'center' ? 'text-center' : 'text-left'}">
                {#if rowSnippet}
                  {@render rowSnippet(row, col.key)}
                {:else}
                  {String(row[col.key] ?? "-")}
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      {/if}
    </tbody>
  </table>
</div>
