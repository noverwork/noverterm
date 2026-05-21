<script lang="ts">
  import { onMount } from "svelte";
  import {
    KeyRound,
    LogOut,
    Network,
    Server,
    Settings,
    Terminal,
    FileText,
  } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";

  type SidebarSection = "terminal" | "hosts" | "keys" | "forwards" | "known-hosts" | "snippets";

  let {
    onLocalTerminal,
    onK9sTerminal,
    onClaudeCodeTerminal,
    onOpencodeTerminal,
    onManageKeys,
    onManageKnownHosts,
    onPortForwards,
    onNewConnection,
    onGoHome,
    onSnippets,
    authEmail,
    onOpenSettings,
    onLogout,
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
    onManageKeys?: () => void;
    onManageKnownHosts?: () => void;
    onPortForwards?: () => void;
    onNewConnection?: () => void;
    onGoHome?: () => void;
    onSnippets?: () => void;
    authEmail?: string;
    onOpenSettings?: () => void;
    onLogout?: () => void;
    connectionCount?: number;
    keyCount?: number;
    forwardCount?: number;
    snippetCount?: number;
    activeSection?: SidebarSection;
  } = $props();

  let appVersion = $state<string | null>(null);

  onMount(async () => {
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      appVersion = await getVersion();
    } catch {
      appVersion = null;
    }
  });

  function navButtonClass(section: SidebarSection): string {
    if (activeSection === section) {
      return "h-10 w-full justify-start gap-2 rounded-2xl border-cyan-300/35 bg-cyan-300/12 text-cyan-50 shadow-[0_10px_30px_rgb(34_211_238/0.16)] ring-1 ring-cyan-300/10 hover:border-cyan-300/45 hover:bg-cyan-300/16 hover:text-white";
    }

    return "h-10 w-full justify-start gap-2 rounded-2xl border-white/10 bg-white/[0.035] text-slate-200 hover:border-cyan-300/30 hover:bg-cyan-300/8 hover:text-white";
  }

  function navCountBadgeClass(section: SidebarSection): string {
    if (activeSection === section) {
      return "ml-auto rounded-full bg-cyan-200/20 px-1.5 py-0.5 text-[10px] text-cyan-50";
    }

    return "ml-auto rounded-full bg-cyan-300/12 px-1.5 py-0.5 text-[10px] text-cyan-200";
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
</script>

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
  class="sidebar relative flex w-[18rem] shrink-0 flex-col overflow-hidden border-r border-white/10 shadow-[18px_0_60px_rgb(0_0_0/0.28)] backdrop-blur-2xl"
>
  <div
    class="pointer-events-none absolute inset-y-0 right-0 w-px bg-gradient-to-b from-transparent via-cyan-300/25 to-transparent"
  ></div>

  <div class="flex items-center justify-between px-4 pt-4 pb-3">
    <button
      type="button"
      class="flex min-w-0 cursor-pointer items-center gap-3 rounded-2xl text-left transition-colors hover:bg-white/[0.035] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/30"
      onclick={onGoHome}
      aria-label="Go to Connections"
    >
      <div
        class="grid size-10 shrink-0 place-items-center rounded-2xl border border-cyan-300/20 bg-cyan-300/12 text-cyan-100 shadow-[0_0_24px_rgb(34_211_238/0.14)] ring-1 ring-cyan-200/5"
      >
        <img src="/favicon.png" alt="" class="size-7 rounded-xl" />
      </div>
      <div class="min-w-0">
        <p
          class="truncate text-sm font-semibold uppercase tracking-[0.28em] text-white"
        >
          NOVERTERM
        </p>
        {#if appVersion}
          <p class="mt-1 text-[10px] font-medium text-slate-500">
            v{appVersion}
          </p>
        {/if}
      </div>
    </button>
  </div>

  <div class="border-b border-white/8 px-4 pb-4">
    {#if onLocalTerminal || onK9sTerminal || onClaudeCodeTerminal || onOpencodeTerminal}
      <div class="flex items-center gap-2">
        {#if onLocalTerminal}
          <Button
            onclick={onLocalTerminal}
            variant="outline"
            size="icon-lg"
            class={localTerminalIconButtonClass()}
            aria-label="Open local terminal"
            title="Local terminal"
          >
            <Terminal class="size-4" />
          </Button>
        {/if}
        {#if onK9sTerminal}
          <Button
            onclick={onK9sTerminal}
            variant="outline"
            size="icon-lg"
            class={k9sIconButtonClass()}
            aria-label="Open k9s terminal"
            title="k9s terminal"
          >
            {@render k9sIcon()}
          </Button>
        {/if}
        {#if onClaudeCodeTerminal}
          <Button
            onclick={onClaudeCodeTerminal}
            variant="outline"
            size="icon-lg"
            class={claudeCodeIconButtonClass()}
            aria-label="Open Claude Code"
            title="Claude Code"
          >
            {@render claudeIcon()}
          </Button>
        {/if}
        {#if onOpencodeTerminal}
          <Button
            onclick={onOpencodeTerminal}
            variant="outline"
            size="icon-lg"
            class={opencodeIconButtonClass()}
            aria-label="Open OpenCode"
            title="OpenCode"
          >
            {@render opencodeIcon()}
          </Button>
        {/if}
      </div>
    {/if}
  </div>

  <div class="flex min-h-0 flex-1 flex-col">
    <div class="px-4 py-4">
      <div class="grid gap-2">
        <div class="grid gap-2">
          {#if onNewConnection}
            <Button
              onclick={onNewConnection}
              variant="outline"
              size="sm"
              class={navButtonClass("hosts")}
            >
              <Server class="size-3.5" />
              Connections
              <span class={navCountBadgeClass("hosts")}>{connectionCount}</span>
            </Button>
          {/if}
          {#if onManageKeys}
            <Button
              onclick={onManageKeys}
              variant="outline"
              size="sm"
              class={navButtonClass("keys")}
            >
              <KeyRound class="size-3.5" />
              Keys
              <span class={navCountBadgeClass("keys")}>{keyCount}</span>
            </Button>
          {/if}
          {#if onPortForwards}
            <Button
              onclick={onPortForwards}
              variant="outline"
              size="sm"
              class={navButtonClass("forwards")}
            >
              <Network class="size-3.5" />
              Forwards
              <span class={navCountBadgeClass("forwards")}>{forwardCount}</span>
            </Button>
          {/if}
          {#if onManageKnownHosts}
            <Button
              onclick={onManageKnownHosts}
              variant="outline"
              size="sm"
              class={navButtonClass("known-hosts")}
            >
              <Server class="size-3.5" />
              Known Hosts
            </Button>
          {/if}
          {#if onSnippets}
            <Button
              onclick={onSnippets}
              variant="outline"
              size="sm"
              class={navButtonClass("snippets")}
            >
              <FileText class="size-3.5" />
              Snippets
              <span class={navCountBadgeClass("snippets")}>{snippetCount}</span>
            </Button>
          {/if}
        </div>
      </div>
    </div>

    <div class="flex-1"></div>

    <div class="border-t border-white/10 p-3">
      <div
        class="rounded-3xl border border-white/10 bg-white/[0.035] p-3 shadow-inner shadow-white/[0.02]"
      >
        {#if authEmail}
          <p class="truncate px-1 text-xs font-medium text-slate-300">
            {authEmail}
          </p>
          <p
            class="mt-1 px-1 text-[10px] uppercase tracking-[0.2em] text-slate-600"
          >
            Authenticated
          </p>
        {/if}

        <div class="mt-3 grid gap-1.5">
          {#if onOpenSettings}
            <Button
              onclick={onOpenSettings}
              variant="ghost"
              size="sm"
              class="w-full justify-start gap-2 rounded-2xl text-slate-300 hover:bg-white/7 hover:text-white"
            >
              <Settings class="size-3.5" />
              Settings
            </Button>
          {/if}

          {#if onLogout}
            <Button
              onclick={onLogout}
              variant="ghost"
              size="sm"
              class="w-full justify-start gap-2 rounded-2xl text-slate-500 hover:bg-red-400/10 hover:text-red-300"
            >
              <LogOut class="size-3.5" />
              Log out
            </Button>
          {/if}
        </div>
      </div>
    </div>
  </div>
</aside>
