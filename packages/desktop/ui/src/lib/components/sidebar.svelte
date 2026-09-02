<script lang="ts">
  import {
    KeyRound,
    Network,
    Server,
    Settings,
    Terminal,
    FileText,
    FolderOpen,
    LayoutGrid,
  } from "@lucide/svelte";

  import type { Snippet } from "svelte";

  import { Button } from "$lib/components/ui/button/index.js";

  type SidebarSection = "terminal" | "hosts" | "keys" | "forwards" | "known-hosts" | "snippets" | "sftp";

  let {
    onLocalTerminal,
    onK9sTerminal,
    onClaudeCodeTerminal,
    onOpencodeTerminal,
    onHerdrTerminal,
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
    onK9sTerminal?: () => void;
    onClaudeCodeTerminal?: () => void;
    onOpencodeTerminal?: () => void;
    onHerdrTerminal?: () => void;
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
    "pointer-events-none absolute left-full top-1/2 z-50 ml-2 -translate-y-1/2 whitespace-nowrap rounded-lg border border-white/10 bg-slate-950/95 px-2 py-1 text-[11px] font-medium text-slate-100 opacity-0 shadow-[0_8px_24px_rgb(0_0_0/0.45)] backdrop-blur transition-opacity duration-150 group-hover/tip:opacity-100";

  function navButtonClass(section: SidebarSection): string {
    if (activeSection === section) {
      return "h-10 w-full justify-center rounded-2xl px-0 border-cyan-300/35 bg-cyan-300/12 text-cyan-50 shadow-[0_10px_30px_rgb(34_211_238/0.16)] ring-1 ring-cyan-300/10 hover:border-cyan-300/45 hover:bg-cyan-300/16 hover:text-white";
    }

    return "h-10 w-full justify-center rounded-2xl px-0 border-white/10 bg-white/[0.035] text-slate-200 hover:border-cyan-300/30 hover:bg-cyan-300/8 hover:text-white";
  }

  function localTerminalIconButtonClass(): string {
    if (activeSection === "terminal") {
      return "rounded-2xl border-emerald-300/24 bg-emerald-300/12 text-emerald-100 shadow-[0_0_22px_rgb(52_211_153/0.12)] ring-1 ring-emerald-300/10 hover:border-emerald-300/35 hover:bg-emerald-300/16 hover:text-white";
    }

    return "rounded-2xl border-emerald-300/12 bg-emerald-300/[0.045] text-emerald-200/85 hover:border-emerald-300/22 hover:bg-emerald-300/8 hover:text-emerald-50";
  }

  function k9sIconButtonClass(): string {
    return "rounded-2xl border-cyan-300/12 bg-cyan-300/[0.045] text-cyan-200/85 hover:border-cyan-300/22 hover:bg-cyan-300/8 hover:text-cyan-50";
  }

  function claudeCodeIconButtonClass(): string {
    return "rounded-2xl border-violet-300/12 bg-violet-300/[0.045] text-violet-200/85 hover:border-violet-300/22 hover:bg-violet-300/8 hover:text-violet-50";
  }

  function opencodeIconButtonClass(): string {
    return "rounded-2xl border-orange-300/12 bg-orange-300/[0.045] text-orange-200/85 hover:border-orange-300/22 hover:bg-orange-300/8 hover:text-orange-50";
  }

  function herdrIconButtonClass(): string {
    return "rounded-2xl border-rose-300/12 bg-rose-300/[0.045] text-rose-200/85 hover:border-rose-300/22 hover:bg-rose-300/8 hover:text-rose-50";
  }

  function withCount(label: string, count?: number): string {
    return count === undefined ? label : `${label} (${count})`;
  }
</script>

{#snippet terminalIcon()}
  <Terminal class="size-4" />
{/snippet}

{#snippet herdrIcon()}
  <LayoutGrid class="size-4" />
{/snippet}

{#snippet quickAction(
  onclick: () => void,
  buttonClass: string,
  label: string,
  icon: Snippet,
)}
  <div class="group/tip relative">
    <Button
      {onclick}
      variant="outline"
      size="icon-lg"
      class={buttonClass}
      aria-label={label}
    >
      {@render icon()}
    </Button>
    <span class={TOOLTIP_CLASS} aria-hidden="true">
      {label}
    </span>
  </div>
{/snippet}

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

{#snippet claudeIcon()}
  <svg
    viewBox="0 0 24 24"
    class="size-4"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M12 3.5 13.9 9l5.6-1.9-3.1 5 4.9 3.3-5.9.8.5 5.8-3.9-4.4L8.1 22l.5-5.8-5.9-.8L7.6 12 4.5 7.1 10.1 9 12 3.5Z" />
    <path d="M12 9.2v5.6" />
    <path d="m9.2 12 5.6 0" />
  </svg>
{/snippet}

{#snippet opencodeIcon()}
  <svg
    viewBox="0 0 24 24"
    class="size-4"
    fill="none"
    aria-hidden="true"
  >
    <rect
      x="3"
      y="5"
      width="18"
      height="14"
      rx="4"
      stroke="currentColor"
      stroke-width="1.8"
    />
    <path
      d="m9 10-2 2 2 2M15 10l2 2-2 2M13 8.5l-2 7"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
{/snippet}

{#snippet k9sIcon()}
  <svg
    viewBox="0 0 24 24"
    class="size-4"
    fill="none"
    aria-hidden="true"
  >
    <rect
      x="3"
      y="5"
      width="18"
      height="14"
      rx="4"
      stroke="currentColor"
      stroke-width="1.8"
    />
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
{/snippet}

<aside
  class="sidebar relative z-30 flex w-[4.5rem] shrink-0 flex-col border-r border-white/10 shadow-[18px_0_60px_rgb(0_0_0/0.28)] backdrop-blur-2xl"
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
        class="grid size-10 shrink-0 place-items-center rounded-2xl border border-cyan-300/20 bg-cyan-300/12 text-cyan-100 shadow-[0_0_24px_rgb(34_211_238/0.14)] ring-1 ring-cyan-200/5"
      >
        <img src="/favicon.png" alt="" class="size-7 rounded-xl" />
      </div>
    </button>
  </div>

  <div class="border-b border-white/8 px-3 pb-4">
    {#if onLocalTerminal || onK9sTerminal || onClaudeCodeTerminal || onOpencodeTerminal || onHerdrTerminal}
      <div class="flex flex-col items-center gap-2">
        {#if onLocalTerminal}
          {@render quickAction(
            onLocalTerminal,
            localTerminalIconButtonClass(),
            "Local terminal",
            terminalIcon,
          )}
        {/if}
        {#if onK9sTerminal}
          {@render quickAction(onK9sTerminal, k9sIconButtonClass(), "k9s", k9sIcon)}
        {/if}
        {#if onClaudeCodeTerminal}
          {@render quickAction(
            onClaudeCodeTerminal,
            claudeCodeIconButtonClass(),
            "Claude Code",
            claudeIcon,
          )}
        {/if}
        {#if onOpencodeTerminal}
          {@render quickAction(
            onOpencodeTerminal,
            opencodeIconButtonClass(),
            "OpenCode",
            opencodeIcon,
          )}
        {/if}
        {#if onHerdrTerminal}
          {@render quickAction(onHerdrTerminal, herdrIconButtonClass(), "Herdr", herdrIcon)}
        {/if}
      </div>
    {/if}
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

    <div class="flex-1"></div>

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
