<script lang="ts" module>
  import {
    Activity,
    Bot,
    Cloud,
    Code,
    Container,
    Cpu,
    Database,
    FileText,
    Folder,
    Gauge,
    GitBranch,
    Globe,
    HardDrive,
    LayoutGrid,
    RefreshCw,
    Rocket,
    Server,
    Shield,
    Sparkles,
    SquareTerminal,
    Terminal,
    Wrench,
    Zap,
  } from "@lucide/svelte";

  const LUCIDE_ICONS = {
    terminal: Terminal,
    "square-terminal": SquareTerminal,
    server: Server,
    database: Database,
    container: Container,
    cloud: Cloud,
    bot: Bot,
    sparkles: Sparkles,
    code: Code,
    "git-branch": GitBranch,
    rocket: Rocket,
    activity: Activity,
    gauge: Gauge,
    cpu: Cpu,
    "hard-drive": HardDrive,
    "file-text": FileText,
    folder: Folder,
    globe: Globe,
    shield: Shield,
    wrench: Wrench,
    zap: Zap,
    "refresh-cw": RefreshCw,
    "layout-grid": LayoutGrid,
  };

  // Hand-drawn marks for tools that have no Lucide equivalent.
  const BRAND_ICONS = ["k9s", "claude", "opencode", "omp"] as const;

  export const PIN_ICON_NAMES: string[] = [...BRAND_ICONS, ...Object.keys(LUCIDE_ICONS)];
</script>

<script lang="ts">
  let { name, class: className = "size-4" }: { name: string; class?: string } = $props();

  const Lucide = $derived(LUCIDE_ICONS[name as keyof typeof LUCIDE_ICONS] ?? FileText);
</script>

{#if name === "k9s"}
  <svg viewBox="0 0 24 24" class={className} fill="none" aria-hidden="true">
    <rect x="3" y="5" width="18" height="14" rx="4" stroke="currentColor" stroke-width="1.8" />
    <path
      d="M8 9v6M8 12.5 11.5 9M9.8 11.5 12 15"
      stroke="currentColor"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <path
      d="M14.2 15h2.2c.9 0 1.6-.7 1.6-1.6v-.1c0-.9-.7-1.6-1.6-1.6h-.8c-.8 0-1.4-.6-1.4-1.4S14.8 9 15.6 9h2"
      stroke="currentColor"
      stroke-width="1.7"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
{:else if name === "claude"}
  <svg
    viewBox="0 0 24 24"
    class={className}
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path
      d="M12 3.5 13.9 9l5.6-1.9-3.1 5 4.9 3.3-5.9.8.5 5.8-3.9-4.4L8.1 22l.5-5.8-5.9-.8L7.6 12 4.5 7.1 10.1 9 12 3.5Z"
    />
    <path d="M12 9.2v5.6" />
    <path d="m9.2 12 5.6 0" />
  </svg>
{:else if name === "opencode"}
  <svg viewBox="0 0 24 24" class={className} fill="none" aria-hidden="true">
    <rect x="3" y="5" width="18" height="14" rx="4" stroke="currentColor" stroke-width="1.8" />
    <path
      d="m9 10-2 2 2 2M15 10l2 2-2 2M13 8.5l-2 7"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
{:else if name === "omp"}
  <span class="grid place-items-center text-base font-semibold leading-none {className}" aria-hidden="true"
    >π</span
  >
{:else}
  <Lucide class={className} aria-hidden="true" />
{/if}
