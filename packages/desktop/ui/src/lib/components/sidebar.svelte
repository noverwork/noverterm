<script lang="ts">
  import {
    KeyRound,
    Network,
    Server,
    Settings,
    FileText,
    FolderOpen,
  } from "@lucide/svelte";

  import type { Snippet } from "svelte";

  import type { SnippetRecord } from "$lib/api/types.js";
  import ShortcutPanel from "$lib/components/shortcut-panel.svelte";
  import { Button } from "$lib/components/ui/button/index.js";

  type SidebarSection = "terminal" | "hosts" | "keys" | "forwards" | "known-hosts" | "snippets" | "sftp";

  let {
    onLocalTerminal,
    pinnedSnippets = [],
    onLaunchSnippet,
    onReorderSnippets,
    onEditSnippet,
    onUnpinSnippet,
    onAddShortcut,
    onManageKeys,
    onManageKnownHosts,
    onPortForwards,
    onNewConnection,
    onGoHome,
    onSnippets,
    onSftp,
    onOpenSettings,
    connectionCount = 0,
    keyCount = 0,
    forwardCount = 0,
    snippetCount = 0,
    activeSection = "terminal",
  }: {
    onLocalTerminal?: () => void;
    pinnedSnippets?: SnippetRecord[];
    onLaunchSnippet?: (snippet: SnippetRecord) => void;
    onReorderSnippets?: (ids: string[]) => void;
    onEditSnippet?: (snippet: SnippetRecord) => void;
    onUnpinSnippet?: (snippet: SnippetRecord) => void;
    onAddShortcut?: () => void;
    onManageKeys?: () => void;
    onManageKnownHosts?: () => void;
    onPortForwards?: () => void;
    onNewConnection?: () => void;
    onGoHome?: () => void;
    onSnippets?: () => void;
    onSftp?: () => void;
    onOpenSettings?: () => void;
    connectionCount?: number;
    keyCount?: number;
    forwardCount?: number;
    snippetCount?: number;
    activeSection?: SidebarSection;
  } = $props();

  const TOOLTIP_CLASS =
    "pointer-events-none absolute left-full top-1/2 z-50 ml-2 -translate-y-1/2 whitespace-nowrap rounded-lg border border-white/10 bg-slate-950/95 px-2 py-1 text-[11px] font-medium text-slate-100 opacity-0 shadow-lg shadow-black/45 backdrop-blur transition-opacity duration-150 group-hover/tip:opacity-100";

  function navButtonClass(section: SidebarSection): string {
    if (activeSection === section) {
      return "h-10 w-full justify-center rounded-2xl px-0 border-cyan-300/35 bg-cyan-300/12 text-cyan-50 hover:border-cyan-300/45 hover:bg-cyan-300/16 hover:text-white";
    }

    return "h-10 w-full justify-center rounded-2xl px-0 border-white/10 bg-white/[0.035] text-slate-200 hover:border-white/10 hover:bg-white/8 hover:text-white";
  }

  function withCount(label: string, count?: number): string {
    return count === undefined ? label : `${label} (${count})`;
  }
</script>

{#snippet navItem(
  onclick: () => void,
  section: SidebarSection,
  label: string,
  icon: Snippet,
  count?: number,
)}
  <div class="group/tip relative">
    <Button
      {onclick}
      variant="outline"
      size="sm"
      class={navButtonClass(section)}
      aria-label={withCount(label, count)}
    >
      {@render icon()}
    </Button>
    <span class={TOOLTIP_CLASS} aria-hidden="true">
      {withCount(label, count)}
    </span>
  </div>
{/snippet}

{#snippet serverIcon()}
  <Server class="size-3.5" />
{/snippet}

{#snippet sftpIcon()}
  <FolderOpen class="size-3.5" />
{/snippet}

{#snippet keyIcon()}
  <KeyRound class="size-3.5" />
{/snippet}

{#snippet snippetIcon()}
  <FileText class="size-3.5" />
{/snippet}

{#snippet forwardIcon()}
  <Network class="size-3.5" />
{/snippet}

<aside
  class="sidebar relative z-30 flex w-[4.5rem] shrink-0 flex-col border-r border-white/10 backdrop-blur-2xl"
>
  <div
    class="pointer-events-none absolute inset-y-0 right-0 w-px bg-gradient-to-b from-transparent via-cyan-300/25 to-transparent"
  ></div>

  <div class="flex items-center justify-center px-3 pt-4 pb-3">
    <button
      type="button"
      class="cursor-pointer rounded-2xl transition-colors hover:bg-white/[0.035] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/30"
      onclick={onGoHome}
      aria-label="Go to Connections"
    >
      <div
        class="grid size-10 shrink-0 place-items-center rounded-2xl border border-white/10 bg-white/5 text-slate-300"
      >
        <img src="/favicon.png" alt="" class="size-7 rounded-xl" />
      </div>
    </button>
  </div>

  <div class="flex min-h-0 flex-1 flex-col">
    <div class="px-3 py-4">
      <div class="grid gap-2">
        {#if onNewConnection}
          {@render navItem(onNewConnection, "hosts", "Connections", serverIcon, connectionCount)}
        {/if}
        {#if onSftp}
          {@render navItem(onSftp, "sftp", "SFTP", sftpIcon)}
        {/if}
        {#if onManageKeys}
          {@render navItem(onManageKeys, "keys", "Keys", keyIcon, keyCount)}
        {/if}
        {#if onSnippets}
          {@render navItem(onSnippets, "snippets", "Snippets", snippetIcon, snippetCount)}
        {/if}
        {#if onPortForwards}
          {@render navItem(onPortForwards, "forwards", "Forwards", forwardIcon, forwardCount)}
        {/if}
        {#if onManageKnownHosts}
          {@render navItem(onManageKnownHosts, "known-hosts", "Known Hosts", serverIcon)}
        {/if}
      </div>
    </div>

    {#if onLaunchSnippet && onReorderSnippets && onEditSnippet && onUnpinSnippet && onAddShortcut}
      <div class="mx-4 h-px shrink-0 bg-white/8"></div>
      <div class="sidebar-shortcuts min-h-0 flex-1 overflow-y-auto px-3 py-4">
        <ShortcutPanel
          snippets={pinnedSnippets}
          localTerminalActive={activeSection === "terminal"}
          {onLocalTerminal}
          onLaunch={onLaunchSnippet}
          onReorder={onReorderSnippets}
          onEdit={onEditSnippet}
          onUnpin={onUnpinSnippet}
          onAdd={onAddShortcut}
        />
      </div>
    {:else}
      <div class="flex-1"></div>
    {/if}

    <div class="border-t border-white/10 p-3">
      {#if onOpenSettings}
        <div class="group/tip relative">
          <Button
            onclick={onOpenSettings}
            variant="ghost"
            size="sm"
            class="h-10 w-full justify-center rounded-2xl px-0 text-slate-300 hover:bg-white/7 hover:text-white"
            aria-label="Settings"
          >
            <Settings class="size-3.5" />
          </Button>
          <span class={TOOLTIP_CLASS} aria-hidden="true">Settings</span>
        </div>
      {/if}
    </div>
  </div>
</aside>
