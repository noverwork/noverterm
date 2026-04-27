<script lang="ts">
  import { ChevronLeft, ChevronRight, KeyRound, LogOut, Network, Plus, Search, Settings, Terminal, X } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { Session, SessionStatus } from "$lib/stores/session.svelte.js";

  type SidebarSection = "terminal" | "hosts" | "keys" | "forwards";

  let {
    sessions,
    activeSessionId,
    collapsed,
    onToggle,
    onActivateSession,
    onCloseSession,
    onLocalTerminal,
    onManageKeys,
    onPortForwards,
    onNewConnection,
    authEmail,
    onOpenSettings,
    onLogout,
    keyCount = 0,
    activeSection = "terminal",
  }: {
    sessions: Map<string, Session>;
    activeSessionId: string | null;
    collapsed: boolean;
    onToggle: () => void;
    onActivateSession: (id: string) => void;
    onCloseSession: (id: string) => void;
    onLocalTerminal?: () => void;
    onManageKeys?: () => void;
    onPortForwards?: () => void;
    onNewConnection?: () => void;
    authEmail?: string;
    onOpenSettings?: () => void;
    onLogout?: () => void;
    keyCount?: number;
    activeSection?: SidebarSection;
  } = $props();

  let searchQuery = $state("");

  const activeSessions = $derived(
    Array.from(sessions.values()).filter((session) => session.status !== "disconnected"),
  );

  const filteredSessions = $derived(
    activeSessions.filter((session) => {
      const query = searchQuery.toLowerCase();
      return (
        session.name.toLowerCase().includes(query) ||
        session.host.toLowerCase().includes(query) ||
        session.username.toLowerCase().includes(query)
      );
    }),
  );

  function statusBadge(status?: SessionStatus) {
    switch (status) {
      case "connected":
        return { tone: "bg-emerald-400 shadow-[0_0_14px_rgb(52_211_153/0.65)]", label: "Live", text: "text-emerald-300" };
      case "connecting":
        return { tone: "bg-amber-300 shadow-[0_0_14px_rgb(252_211_77/0.55)] animate-pulse", label: "Starting", text: "text-amber-300" };
      case "trust_required":
        return { tone: "bg-amber-300 shadow-[0_0_14px_rgb(252_211_77/0.55)]", label: "Trust", text: "text-amber-300" };
      case "error":
        return { tone: "bg-red-400 shadow-[0_0_14px_rgb(248_113_113/0.55)]", label: "Error", text: "text-red-300" };
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
      return "h-10 justify-start gap-2 rounded-2xl border-cyan-300/35 bg-cyan-300/12 text-cyan-50 shadow-[0_10px_30px_rgb(34_211_238/0.16)] ring-1 ring-cyan-300/10 hover:border-cyan-300/45 hover:bg-cyan-300/16 hover:text-white";
    }

    return "h-10 justify-start gap-2 rounded-2xl border-white/10 bg-white/[0.035] text-slate-200 hover:border-cyan-300/30 hover:bg-cyan-300/8 hover:text-white";
  }

  function iconButtonClass(section: SidebarSection): string {
    if (activeSection === section) {
      return "rounded-2xl border border-cyan-300/30 bg-cyan-300/12 text-cyan-100 shadow-[0_0_24px_rgb(34_211_238/0.15)] hover:bg-cyan-300/16 hover:text-white";
    }

    return "rounded-2xl text-slate-400 hover:bg-white/8 hover:text-white";
  }
</script>

<aside class="sidebar relative flex flex-col border-r border-white/10 bg-[#070b12]/96 shadow-[18px_0_60px_rgb(0_0_0/0.28)] backdrop-blur-2xl transition-all duration-300 {collapsed ? 'w-[4.5rem]' : 'w-[20.5rem]'}">
  <div class="pointer-events-none absolute inset-y-0 right-0 w-px bg-gradient-to-b from-transparent via-cyan-300/25 to-transparent"></div>

  <div class="flex items-center justify-between px-4 py-4">
    {#if !collapsed}
      <div class="min-w-0">
        <div class="flex items-center gap-3">
          <div class="grid size-10 place-items-center rounded-2xl border border-cyan-300/18 bg-cyan-300/10 text-cyan-200 shadow-[0_0_24px_rgb(34_211_238/0.12)]">
            <Terminal class="size-4" />
          </div>
          <div class="min-w-0">
            <p class="text-[10px] font-semibold uppercase tracking-[0.28em] text-cyan-200/70">Noverterm</p>
            <p class="truncate text-sm font-semibold text-white">Command Center</p>
          </div>
        </div>
      </div>
    {:else}
      <div class="mx-auto grid size-11 place-items-center rounded-2xl border border-cyan-300/18 bg-cyan-300/10 text-cyan-200">
        <Terminal class="size-4" />
      </div>
    {/if}

    <Button
      variant="ghost"
      size="icon-xs"
      onclick={onToggle}
      class="rounded-xl text-slate-500 hover:bg-white/7 hover:text-white"
      aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
      aria-expanded={!collapsed}
      title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
    >
      {#if collapsed}
        <ChevronRight class="size-3.5" />
      {:else}
        <ChevronLeft class="size-3.5" />
      {/if}
    </Button>
  </div>

  {#if !collapsed}
    <div class="px-4 pb-4">
      <div class="relative">
        <Search class="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-slate-500" />
        <Input bind:value={searchQuery} placeholder="Search host, user, label" class="h-10 rounded-2xl border-white/10 bg-white/[0.045] pl-9 text-sm text-slate-100 placeholder:text-slate-600 focus-visible:border-cyan-300/40 focus-visible:ring-cyan-300/20" />
      </div>

      <div class="mt-3 grid gap-2">
        {#if onLocalTerminal}
          <Button onclick={onLocalTerminal} variant="default" size="sm" class={activeSection === "terminal"
            ? "h-10 justify-start gap-2 rounded-2xl bg-cyan-300 text-slate-950 shadow-[0_12px_34px_rgb(34_211_238/0.24)] ring-1 ring-cyan-100/20 hover:bg-cyan-200"
            : "h-10 justify-start gap-2 rounded-2xl bg-cyan-300/12 text-cyan-100 shadow-[0_12px_34px_rgb(34_211_238/0.10)] hover:bg-cyan-300/18 hover:text-white"}>
            <Terminal class="size-3.5" />
            Local terminal
          </Button>
        {/if}
        <div class="grid grid-cols-2 gap-2">
          {#if onNewConnection}
            <Button onclick={onNewConnection} variant="outline" size="sm" class={navButtonClass("hosts")}>
              <Plus class="size-3.5" />
              Hosts
            </Button>
          {/if}
          {#if onManageKeys}
            <Button onclick={onManageKeys} variant="outline" size="sm" class={navButtonClass("keys")}>
              <KeyRound class="size-3.5" />
              Keys
              {#if keyCount > 0}
                <span class={activeSection === "keys"
                  ? "ml-auto rounded-full bg-cyan-200/20 px-1.5 py-0.5 text-[10px] text-cyan-50"
                  : "ml-auto rounded-full bg-cyan-300/12 px-1.5 py-0.5 text-[10px] text-cyan-200"}>{keyCount}</span>
              {/if}
            </Button>
          {/if}
          {#if onPortForwards}
            <Button onclick={onPortForwards} variant="outline" size="sm" class={navButtonClass("forwards")}>
              <Network class="size-3.5" />
              Forwards
            </Button>
          {/if}
        </div>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-3 pb-3">
      {#if activeSessions.length > 0}
        <div class="mb-6">
          <div class="flex items-center justify-between px-2">
            <p class="section-title text-emerald-300/80">Active</p>
            <span class="rounded-full border border-emerald-300/15 bg-emerald-300/8 px-2 py-0.5 text-[10px] font-medium text-emerald-200">{activeSessions.length}</span>
          </div>
          <div class="mt-2 space-y-1.5">
            {#each filteredSessions as session (session.id)}
              {@const status = statusBadge(session.status)}
              <div class={session.id === activeSessionId
                ? "group flex items-start gap-2 rounded-2xl border border-cyan-300/30 bg-cyan-300/10 p-1.5 text-white shadow-[0_10px_32px_rgb(34_211_238/0.10)] transition"
                : "group flex items-start gap-2 rounded-2xl border border-transparent p-1.5 text-slate-300 transition hover:border-white/10 hover:bg-white/[0.045] hover:text-white"}>
                <button class="flex min-w-0 flex-1 cursor-pointer items-start gap-3 rounded-xl px-1.5 py-1.5 text-left" onclick={() => onActivateSession(session.id)}>
                  <span class="mt-1.5 size-2.5 shrink-0 rounded-full {status.tone}"></span>
                  <span class="min-w-0 flex-1">
                    <span class="flex items-center gap-2">
                      <span class="truncate text-sm font-semibold">{session.name}</span>
                      <span class="rounded-full border border-white/10 bg-black/20 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide {status.text}">{status.label}</span>
                    </span>
                    <span class="mt-1 block truncate text-xs text-slate-500">{sessionSubtitle(session)}</span>
                  </span>
                </button>
                <Button variant="ghost" size="icon-xs" class="size-7 shrink-0 rounded-xl text-slate-500 opacity-0 transition-opacity hover:bg-red-400/10 hover:text-red-300 group-hover:opacity-100 group-focus-within:opacity-100" onclick={() => onCloseSession(session.id)} aria-label={`Close ${session.name}`}>
                  <X class="size-3" />
                </Button>
              </div>
            {/each}
            {#if filteredSessions.length === 0}
              <div class="rounded-3xl border border-dashed border-white/10 bg-white/[0.025] px-4 py-7 text-center text-sm leading-6 text-slate-500">
                No active sessions match “{searchQuery}”.
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <div class="border-t border-white/10 p-3">
      <div class="rounded-3xl border border-white/10 bg-white/[0.035] p-3 shadow-inner shadow-white/[0.02]">
        {#if authEmail}
          <p class="truncate px-1 text-xs font-medium text-slate-300">{authEmail}</p>
          <p class="mt-1 px-1 text-[10px] uppercase tracking-[0.2em] text-slate-600">Authenticated</p>
        {/if}

        <div class="mt-3 grid gap-1.5">
          {#if onOpenSettings}
            <Button onclick={onOpenSettings} variant="ghost" size="sm" class="w-full justify-start gap-2 rounded-2xl text-slate-300 hover:bg-white/7 hover:text-white">
              <Settings class="size-3.5" />
              Settings
            </Button>
          {/if}

          {#if onLogout}
            <Button onclick={onLogout} variant="ghost" size="sm" class="w-full justify-start gap-2 rounded-2xl text-slate-500 hover:bg-red-400/10 hover:text-red-300">
              <LogOut class="size-3.5" />
              Log out
            </Button>
          {/if}
        </div>
      </div>
    </div>
  {:else}
    <div class="flex flex-1 flex-col items-center gap-2 px-2 py-4">
      {#if onLocalTerminal}
        <Button variant="ghost" size="icon-sm" class={iconButtonClass("terminal")} onclick={onLocalTerminal} aria-label="Open local terminal" title="Local terminal">
          <Terminal class="size-4" />
        </Button>
      {/if}
      {#if onNewConnection}
        <Button variant="ghost" size="icon-sm" class={iconButtonClass("hosts")} onclick={onNewConnection} aria-label="Connections" title="Connections">
          <Plus class="size-4" />
        </Button>
      {/if}
      {#if onManageKeys}
        <Button variant="ghost" size="icon-sm" class={iconButtonClass("keys")} onclick={onManageKeys} aria-label="SSH keys" title="SSH keys">
          <KeyRound class="size-4" />
        </Button>
      {/if}
      {#if onPortForwards}
        <Button variant="ghost" size="icon-sm" class={iconButtonClass("forwards")} onclick={onPortForwards} aria-label="Port forwards" title="Port forwards">
          <Network class="size-4" />
        </Button>
      {/if}
    </div>
    <div class="flex flex-col items-center gap-2 border-t border-white/10 px-2 py-4">
      {#if onOpenSettings}
        <Button variant="ghost" size="icon-sm" class="rounded-2xl text-slate-400 hover:bg-white/8 hover:text-white" onclick={onOpenSettings} aria-label="Settings" title="Settings">
          <Settings class="size-4" />
        </Button>
      {/if}
      {#if onLogout}
        <Button variant="ghost" size="icon-sm" class="rounded-2xl text-slate-500 hover:bg-red-400/10 hover:text-red-300" onclick={onLogout} aria-label="Log out" title="Log out">
          <LogOut class="size-4" />
        </Button>
      {/if}
    </div>
  {/if}
</aside>
