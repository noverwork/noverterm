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
  import { t } from "$lib/i18n/index.svelte.js";

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
    "pointer-events-none absolute left-full top-1/2 z-50 ml-2 -translate-y-1/2 whitespace-nowrap rounded-[2px] border border-white/10 bg-rail px-2 py-1 text-[11px] uppercase tracking-[0.14em] text-stencil opacity-0 transition-opacity duration-150 group-hover/tip:opacity-100";

  function navButtonClass(section: SidebarSection): string {
    if (activeSection === section) {
      return "h-7 w-10 justify-center rounded-[2px] px-0 border-stencil bg-transparent text-stencil hover:bg-white/5 dark:bg-transparent dark:border-stencil";
    }

    return "h-7 w-10 justify-center rounded-[2px] px-0 border-transparent bg-transparent text-stencil-mute hover:border-white/10 hover:bg-white/5 hover:text-stencil dark:bg-transparent dark:border-transparent";
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
  class="sidebar relative z-30 flex w-14 shrink-0 flex-col border-r border-rib"
>
  <div class="flex items-center justify-center px-2 pt-3 pb-2">
    <button
      type="button"
      class="cursor-pointer rounded-[2px] transition-colors hover:bg-white/5 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil"
      onclick={onGoHome}
      aria-label={t("Go to Connections")}
    >
      <div
        class="grid size-10 shrink-0 place-items-center rounded-[2px] border border-rib"
      >
        <img src="/favicon.png" alt="" class="size-7 rounded-[2px]" />
      </div>
    </button>
  </div>

  <div class="flex min-h-0 flex-1 flex-col">
    <div class="px-2 py-3">
      <div class="grid justify-items-center gap-1">
        {#if onNewConnection}
          {@render navItem(onNewConnection, "hosts", t("Connections"), serverIcon, connectionCount)}
        {/if}
        {#if onSftp}
          {@render navItem(onSftp, "sftp", "SFTP", sftpIcon)}
        {/if}
        {#if onManageKeys}
          {@render navItem(onManageKeys, "keys", t("Keys"), keyIcon, keyCount)}
        {/if}
        {#if onSnippets}
          {@render navItem(onSnippets, "snippets", t("Snippets"), snippetIcon, snippetCount)}
        {/if}
        {#if onPortForwards}
          {@render navItem(onPortForwards, "forwards", t("Forwards"), forwardIcon, forwardCount)}
        {/if}
        {#if onManageKnownHosts}
          {@render navItem(onManageKnownHosts, "known-hosts", t("Known Hosts"), serverIcon)}
        {/if}
      </div>
    </div>

    {#if onLaunchSnippet && onReorderSnippets && onEditSnippet && onUnpinSnippet && onAddShortcut}
      <div class="mx-3 h-px shrink-0 bg-rib"></div>
      <div class="sidebar-shortcuts min-h-0 flex-1 overflow-y-auto px-2 py-3">
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

    <div class="flex justify-center border-t border-rib p-2">
      {#if onOpenSettings}
        <div class="group/tip relative">
          <Button
            onclick={onOpenSettings}
            variant="ghost"
            size="sm"
            class="h-7 w-10 justify-center rounded-[2px] px-0 text-stencil-mute hover:bg-white/5 hover:text-stencil"
            aria-label={t("Settings")}
          >
            <Settings class="size-3.5" />
          </Button>
          <span class={TOOLTIP_CLASS} aria-hidden="true">{t("Settings")}</span>
        </div>
      {/if}
    </div>
  </div>
</aside>
