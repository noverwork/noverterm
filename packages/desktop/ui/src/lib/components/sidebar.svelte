<script lang="ts">
  import {
    KeyRound,
    LogOut,
    Network,
    Server,
    Settings,
    Terminal,
    X,
  } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import type { Session, SessionStatus } from "$lib/stores/session.svelte.js";

  type SidebarSection = "terminal" | "hosts" | "keys" | "forwards";

  let {
    sessions,
    activeSessionId,
    onActivateSession,
    onCloseSession,
    onLocalTerminal,
    onManageKeys,
    onPortForwards,
    onNewConnection,
    onGoHome,
    authEmail,
    onOpenSettings,
    onLogout,
    connectionCount = 0,
    keyCount = 0,
    forwardCount = 0,
    activeSection = "terminal",
  }: {
    sessions: Map<string, Session>;
    activeSessionId: string | null;
    onActivateSession: (id: string) => void;
    onCloseSession: (id: string) => void;
    onLocalTerminal?: () => void;
    onManageKeys?: () => void;
    onPortForwards?: () => void;
    onNewConnection?: () => void;
    onGoHome?: () => void;
    authEmail?: string;
    onOpenSettings?: () => void;
    onLogout?: () => void;
    connectionCount?: number;
    keyCount?: number;
    forwardCount?: number;
    activeSection?: SidebarSection;
  } = $props();

  const activeSessions = $derived(
    Array.from(sessions.values()).filter(
      (session) => session.status !== "disconnected",
    ),
  );

  function statusBadge(status?: SessionStatus) {
    switch (status) {
      case "connected":
        return {
          tone: "bg-emerald-400 shadow-[0_0_14px_rgb(52_211_153/0.65)]",
          label: "Live",
          text: "text-emerald-300",
        };
      case "connecting":
        return {
          tone: "bg-amber-300 shadow-[0_0_14px_rgb(252_211_77/0.55)] animate-pulse",
          label: "Starting",
          text: "text-amber-300",
        };
      case "trust_required":
        return {
          tone: "bg-amber-300 shadow-[0_0_14px_rgb(252_211_77/0.55)]",
          label: "Trust",
          text: "text-amber-300",
        };
      case "error":
        return {
          tone: "bg-red-400 shadow-[0_0_14px_rgb(248_113_113/0.55)]",
          label: "Error",
          text: "text-red-300",
        };
      default:
        return { tone: "bg-slate-500", label: "Saved", text: "text-slate-400" };
    }
  }

  function sessionSubtitle(session: Session) {
    if (session.type === "local") {
      return "Local shell";
    }

    return `${session.username}@${session.host}:${session.port}`;
  }

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

  function localTerminalButtonClass(): string {
    if (activeSection === "terminal") {
      return "h-10 w-full justify-start gap-2 rounded-2xl border-emerald-300/18 bg-emerald-300/8 text-emerald-100 shadow-[0_10px_26px_rgb(52_211_153/0.08)] ring-1 ring-emerald-300/8 hover:border-emerald-300/25 hover:bg-emerald-300/12 hover:text-white";
    }

    return "h-10 w-full justify-start gap-2 rounded-2xl border-emerald-300/12 bg-emerald-300/[0.045] text-emerald-200/85 hover:border-emerald-300/22 hover:bg-emerald-300/8 hover:text-emerald-50";
  }
</script>

<aside
  class="sidebar relative flex w-[18rem] shrink-0 flex-col overflow-hidden border-r border-white/10 shadow-[18px_0_60px_rgb(0_0_0/0.28)] backdrop-blur-2xl"
>
  <div
    class="pointer-events-none absolute inset-y-0 right-0 w-px bg-gradient-to-b from-transparent via-cyan-300/25 to-transparent"
  ></div>

  <div class="flex items-center justify-between px-4 py-4">
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
      </div>
    </button>
  </div>

  <div class="flex min-h-0 flex-1 flex-col">
    <div class="px-4 pb-4">
      <div class="grid gap-2">
        {#if onLocalTerminal}
          <Button
            onclick={onLocalTerminal}
            variant="outline"
            size="sm"
            class={localTerminalButtonClass()}
          >
            <Terminal class="size-3.5" />
            Local terminal
          </Button>
        {/if}
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
        </div>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-3 pb-3" data-sidebar-scroll>
      {#if activeSessions.length > 0}
        <div class="mb-6">
          <div class="flex items-center justify-between px-2">
            <p class="section-title text-emerald-300/80">Active</p>
            <span
              class="rounded-full border border-emerald-300/15 bg-emerald-300/8 px-2 py-0.5 text-[10px] font-medium text-emerald-200"
              >{activeSessions.length}</span
            >
          </div>
          <div class="mt-2 space-y-1.5">
            {#each activeSessions as session (session.id)}
              {@const status = statusBadge(session.status)}
              <div
                class={session.id === activeSessionId
                  ? "group flex items-start gap-2 rounded-2xl border border-cyan-300/30 bg-cyan-300/10 p-1.5 text-white shadow-[0_10px_32px_rgb(34_211_238/0.10)] transition"
                  : "group flex items-start gap-2 rounded-2xl border border-transparent p-1.5 text-slate-300 transition hover:border-white/10 hover:bg-white/[0.045] hover:text-white"}
              >
                <button
                  class="flex min-w-0 flex-1 cursor-pointer items-start gap-3 rounded-xl px-1.5 py-1.5 text-left"
                  onclick={() => onActivateSession(session.id)}
                >
                  <span
                    class="mt-1.5 size-2.5 shrink-0 rounded-full {status.tone}"
                  ></span>
                  <span class="min-w-0 flex-1">
                    <span class="flex items-center gap-2">
                      <span class="truncate text-sm font-semibold"
                        >{session.name}</span
                      >
                      <span
                        class="rounded-full border border-white/10 bg-black/20 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide {status.text}"
                        >{status.label}</span
                      >
                    </span>
                    <span class="mt-1 block truncate text-xs text-slate-500"
                      >{sessionSubtitle(session)}</span
                    >
                  </span>
                </button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  class="size-7 shrink-0 rounded-xl text-slate-500 opacity-0 transition-opacity hover:bg-red-400/10 hover:text-red-300 group-hover:opacity-100 group-focus-within:opacity-100"
                  onclick={() => onCloseSession(session.id)}
                  aria-label={`Close ${session.name}`}
                >
                  <X class="size-3" />
                </Button>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>

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
