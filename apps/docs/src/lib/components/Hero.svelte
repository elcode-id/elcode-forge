<script lang="ts">
  import { base } from "$app/paths";
  import Button from "./Button.svelte";
  import HeroBackground from "./HeroBackground.svelte";

  interface CodeTab {
    name: string;
    lines: { tokens: { text: string; cls: string }[] }[];
  }

  const tabs: CodeTab[] = [
    {
      name: "eforge.config.yaml",
      lines: [
        { tokens: [{ text: "project", cls: "token property" }, { text: ":", cls: "token punctuation" }] },
        { tokens: [{ text: "  name", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: '"elcode-forge"', cls: "token string" }] },
        { tokens: [{ text: "extends", cls: "token property" }, { text: ":", cls: "token punctuation" }] },
        { tokens: [{ text: "  - ", cls: "token punctuation" }, { text: '"forge:recommended"', cls: "token string" }] },
        { tokens: [{ text: "  - ", cls: "token punctuation" }, { text: '"forge:security"', cls: "token string" }] },
        { tokens: [{ text: "policy", cls: "token property" }, { text: ":", cls: "token punctuation" }] },
        { tokens: [{ text: "  min_health_score", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: "85.0", cls: "token unit" }] },
        { tokens: [{ text: "  fail_on", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: '"high"', cls: "token string" }] },
        { tokens: [{ text: "ai", cls: "token property" }, { text: ":", cls: "token punctuation" }] },
        { tokens: [{ text: "  provider", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: '"anthropic"', cls: "token string" }] },
        { tokens: [{ text: "  api_key_env", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: '"EFORGE_AI_API_KEY"', cls: "token function" }] },
      ],
    },
    {
      name: "Terminal — eforge doctor",
      lines: [
        { tokens: [{ text: "$ ", cls: "token punctuation" }, { text: "eforge", cls: "token function" }, { text: " doctor --format terminal", cls: "token keyword" }] },
        { tokens: [{ text: "⚡ ELCODE Forge v1.1.0 — Polyglot Codebase Intelligence", cls: "token comment" }] },
        { tokens: [{ text: "✓ ", cls: "token unit" }, { text: "Languages: ", cls: "token keyword" }, { text: "Rust, TypeScript, Python (312 files)", cls: "token string" }] },
        { tokens: [{ text: "✓ ", cls: "token unit" }, { text: "Architecture DAG: ", cls: "token keyword" }, { text: "0 circular dependencies (Tarjan SCC)", cls: "token string" }] },
        { tokens: [{ text: "✓ ", cls: "token unit" }, { text: "Security & OSV: ", cls: "token keyword" }, { text: "0 leaked secrets, 0 critical CVEs", cls: "token string" }] },
        { tokens: [{ text: "✓ ", cls: "token unit" }, { text: "Policy Gate: ", cls: "token keyword" }, { text: "PASSED (.eforge/policy.yaml)", cls: "token string" }] },
        { tokens: [{ text: "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", cls: "token punctuation" }] },
        { tokens: [{ text: "Health Score: ", cls: "token property" }, { text: "96.4 / 100 ", cls: "token unit" }, { text: "(Grade A+) ", cls: "token function" }, { text: "[Exit Code: 0]", cls: "token comment" }] },
      ],
    },
    {
      name: ".github/workflows/eforge.yml",
      lines: [
        { tokens: [{ text: "- ", cls: "token punctuation" }, { text: "name", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: "ELCODE Forge Governance Gate", cls: "token string" }] },
        { tokens: [{ text: "  uses", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: "elcode-id/forge-action@v1", cls: "token function" }] },
        { tokens: [{ text: "  with", cls: "token property" }, { text: ":", cls: "token punctuation" }] },
        { tokens: [{ text: "    command", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: '"scan"', cls: "token string" }] },
        { tokens: [{ text: "    fail-on", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: '"high"', cls: "token string" }] },
        { tokens: [{ text: "    upload-sarif", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: "true", cls: "token unit" }] },
        { tokens: [{ text: "    pr-comment", cls: "token property" }, { text: ": ", cls: "token punctuation" }, { text: "true", cls: "token unit" }] },
      ],
    },
  ];

  let activeTab = $state(0);
</script>

<div class="overflow-hidden bg-slate-900 dark:-mt-19 dark:-mb-32 dark:pt-19 dark:pb-32">
  <div class="py-16 sm:px-2 lg:relative lg:px-0 lg:py-20">
    <div class="mx-auto grid max-w-2xl grid-cols-1 items-center gap-x-8 gap-y-16 px-4 lg:max-w-8xl lg:grid-cols-2 lg:px-8 xl:gap-x-16 xl:px-12">
      <!-- Left Column: Display Title, Subtitle & CTA Buttons -->
      <div class="relative z-10 md:text-center lg:text-left">
        <img
          class="pointer-events-none absolute right-full bottom-full -mr-72 -mb-56 opacity-50 select-none"
          src="{base}/images/blur-cyan.png"
          alt=""
          width="530"
          height="530"
        />
        <div class="relative">
          <p class="inline bg-gradient-to-r from-indigo-200 via-sky-400 to-indigo-200 bg-clip-text font-display text-5xl tracking-tight text-transparent">
            Know your code. Ship with confidence.
          </p>
          <p class="mt-3 text-2xl tracking-tight text-slate-400">
            Deterministic codebase intelligence, polyglot architecture analysis, Policy-as-Code, and native MCP server in a single Rust binary.
          </p>
          <div class="mt-8 flex flex-wrap gap-4 md:justify-center lg:justify-start">
            <Button href="{base}/#installation">Get started</Button>
            <Button
              href="https://github.com/elcode-id/elcode-forge"
              target="_blank"
              rel="noreferrer"
              variant="secondary"
            >
              View on GitHub
            </Button>
          </div>
        </div>
      </div>

      <!-- Right Column: Circuit SVG Background + Frosted Code Window -->
      <div class="relative lg:static xl:pl-10">
        <div class="pointer-events-none absolute inset-x-[-50vw] -top-32 -bottom-48 [mask-image:linear-gradient(transparent,white,white)] lg:-top-32 lg:right-0 lg:-bottom-32 lg:left-[calc(50%+14rem)] lg:[mask-image:none] dark:[mask-image:linear-gradient(transparent,white,transparent)] lg:dark:[mask-image:linear-gradient(white,white,transparent)]">
          <HeroBackground class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 lg:left-0 lg:translate-x-0 lg:translate-y-[-60%]" />
        </div>
        <div class="relative">
          <img
            class="pointer-events-none absolute -top-64 -right-64 select-none"
            src="{base}/images/blur-cyan.png"
            alt=""
            width="530"
            height="530"
          />
          <img
            class="pointer-events-none absolute -right-44 -bottom-40 select-none"
            src="{base}/images/blur-indigo.png"
            alt=""
            width="567"
            height="567"
          />
          <div class="absolute inset-0 rounded-2xl bg-gradient-to-tr from-sky-300 via-sky-300/70 to-blue-300 opacity-10 blur-lg"></div>
          <div class="absolute inset-0 rounded-2xl bg-gradient-to-tr from-sky-300 via-sky-300/70 to-blue-300 opacity-10"></div>
          <div class="relative rounded-2xl bg-[#0A101F]/80 ring-1 ring-white/10 backdrop-blur-sm">
            <div class="absolute -top-px right-11 left-20 h-px bg-gradient-to-r from-sky-300/0 via-sky-300/70 to-sky-300/0"></div>
            <div class="absolute right-20 -bottom-px left-11 h-px bg-gradient-to-r from-blue-400/0 via-blue-400 to-blue-400/0"></div>
            <div class="pt-4 pl-4">
              <!-- Traffic Lights SVG -->
              <svg aria-hidden="true" viewBox="0 0 42 10" fill="none" class="h-2.5 w-auto stroke-slate-500/30">
                <circle cx="5" cy="5" r="4.5" />
                <circle cx="21" cy="5" r="4.5" />
                <circle cx="37" cy="5" r="4.5" />
              </svg>

              <!-- Interactive File Tabs -->
              <div class="mt-4 flex flex-wrap gap-2 pr-4 text-xs">
                {#each tabs as tab, idx}
                  <button
                    type="button"
                    onclick={() => (activeTab = idx)}
                    class="flex h-6 rounded-full transition-colors {activeTab === idx
                      ? 'bg-gradient-to-r from-sky-400/30 via-sky-400 to-sky-400/30 p-px font-medium text-sky-300'
                      : 'text-slate-500 hover:text-slate-300'}"
                  >
                    <span
                      class="flex items-center rounded-full px-2.5 {activeTab === idx
                        ? 'bg-slate-800'
                        : ''}"
                    >
                      {tab.name}
                    </span>
                  </button>
                {/each}
              </div>

              <!-- Code Body -->
              <div class="mt-6 flex items-start px-1 text-sm font-mono">
                <div
                  aria-hidden="true"
                  class="border-r border-slate-300/5 pr-4 font-mono text-slate-600 select-none leading-6"
                >
                  {#each tabs[activeTab].lines as _, index}
                    {(index + 1).toString().padStart(2, "0")}<br />
                  {/each}
                </div>
                <pre class="syntax-pre flex overflow-x-auto pb-6 leading-6"><code class="px-4">{#each tabs[activeTab].lines as line}<div>{#each line.tokens as token}<span class={token.cls}>{token.text}</span>{/each}</div>{/each}</code></pre>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>

