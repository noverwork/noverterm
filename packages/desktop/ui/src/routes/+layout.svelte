<script lang="ts">
  import "../app.css";
  import type { Snippet } from "svelte";
  import { flip } from "svelte/animate";
  import { cubicOut } from "svelte/easing";
  import { slide } from "svelte/transition";
  import { onDestroy, onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
  import {
    AlertCircle,
    FileText,
    FolderOpen,
    KeyRound,
    Loader2,
    Network,
    Server,
  } from "@lucide/svelte";

  import ConnectionStatusOverlay from "$lib/components/connection-status-overlay.svelte";
  import SettingsModal from "$lib/components/settings-modal.svelte";
  import Sidebar from "$lib/components/sidebar.svelte";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import {
    createAppShellStore,
    setAppShellContext,
  } from "$lib/stores/app-shell.svelte.js";
  import TerminalView from "$lib/terminal/terminal.svelte";
  import { checkForAppUpdate } from "$lib/updater/auto-update.js";

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        refetchOnWindowFocus: false,
      },
    },
  });

  const app = createAppShellStore(queryClient);
  setAppShellContext(app);

  const routePath = $derived($page.url.pathname);
  const connectionsPath = "/connections";
  const terminalPath = "/terminal";
  const sftpPath = "/sftp";
  const isTerminalRoute = $derived(routePath === terminalPath);
  const isSftpRoute = $derived(routePath === sftpPath);
  const isTerminalVisible = $derived(
    isTerminalRoute &&
      (app.activeSession?.status === "connected" ||
        app.activeSession?.status === "connecting"),
  );
  const hasTerminalErrorOverlay = $derived(
    isTerminalRoute &&
      (app.activeSession?.status === "error" ||
        app.activeSession?.status === "trust_required"),
  );
  const hasTerminalDisconnectedOverlay = $derived(
    isTerminalRoute && app.activeSession?.status === "disconnected",
  );
  const sessionTabEdgeSize = 72;
  const sessionTabMaxScrollSpeed = 0.8;
  let sessionTabsElement = $state<HTMLDivElement | null>(null);
  let canScrollSessionTabsLeft = $state(false);
  let canScrollSessionTabsRight = $state(false);
  let sessionTabScrollFrame = 0;
  let sessionTabScrollVelocity = 0;
  let sessionTabLastFrameTime = 0;
  let dragState = $state<{
    sessionId: string;
    startX: number;
    active: boolean;
  } | null>(null);
  let dragOverSessionId = $state<string | null>(null);
  const activeSidebarSection = $derived.by(() => {
    if (routePath.startsWith("/connections")) {
      return "hosts";
    }

    if (routePath.startsWith("/keys")) {
      return "keys";
    }

    if (routePath.startsWith("/forwards")) {
      return "forwards";
    }

    if (routePath.startsWith("/known-hosts")) {
      return "known-hosts";
    }

    if (routePath.startsWith("/snippets")) {
      return "snippets";
    }

    if (routePath.startsWith("/sftp")) {
      return "sftp";
    }

    return "terminal";
  });
  const sidebarPages = {
    hosts: { label: "Connections", icon: Server },
    keys: { label: "Keys", icon: KeyRound },
    forwards: { label: "Forwards", icon: Network },
    "known-hosts": { label: "Known Hosts", icon: Server },
    snippets: { label: "Snippets", icon: FileText },
    sftp: { label: "SFTP", icon: FolderOpen },
  } as const;
  const currentPage = $derived(
    activeSidebarSection === "terminal"
      ? null
      : sidebarPages[activeSidebarSection],
  );

  onMount(async () => {
    window.addEventListener("contextmenu", handleGlobalContextMenu);
    await app.init();
    void checkForAppUpdate();
    updateSessionTabScrollIndicators();
  });

  onDestroy(() => {
    window.removeEventListener("contextmenu", handleGlobalContextMenu);
    stopSessionTabAutoScroll();
    app.cleanup();
  });

  function handleGlobalContextMenu(event: MouseEvent) {
    event.preventDefault();
  }

  async function activateSession(id: string) {
    app.activateSession(id);
    // SvelteKit resets focus to <body> after navigation, which blurs the
    // terminal the tab switch just focused.
    await goto(terminalPath, { keepFocus: true });
  }

  async function openLocalTerminal() {
    await app.connectLocalTerminal();
    await goto(terminalPath);
  }

  async function openK9sTerminal() {
    await app.openK9sTerminal();
    await goto(terminalPath);
  }

  async function openClaudeCodeTerminal() {
    await app.openClaudeCodeTerminal();
    await goto(terminalPath);
  }

  async function openOpencodeTerminal() {
    await app.openOpencodeTerminal();
    await goto(terminalPath);
  }

  async function openOmpTerminal() {
    await app.openOmpTerminal();
    await goto(terminalPath);
  }

  async function openHerdrTerminal() {
    await app.openHerdrTerminal();
    await goto(terminalPath);
  }

  async function closeSessionAndNavigate(id: string) {
    await closeSessionIdsAndNavigate([id]);
  }

  async function closeSessionIdsAndNavigate(ids: string[]) {
    if (ids.length === 0) {
      return;
    }

    const activeSessionIds = app.activeSessions.map((session) => session.id);
    const closingAllSessions = activeSessionIds.every((id) => ids.includes(id));

    for (const id of ids) {
      app.closeSession(id);
    }

    if (closingAllSessions && routePath === terminalPath) {
      await goto(connectionsPath);
    }
  }

  function sessionIdsBefore(index: number): string[] {
    return app.activeSessions.slice(0, index).map((session) => session.id);
  }

  function sessionIdsAfter(index: number): string[] {
    return app.activeSessions.slice(index + 1).map((session) => session.id);
  }

  function allSessionIds(): string[] {
    return app.activeSessions.map((session) => session.id);
  }

  async function goHome() {
    app.sessionStore.setActiveSession(null);
    await goto(connectionsPath);
  }

  async function retryActiveConnection() {
    await app.retryActiveConnection();
    await goto(terminalPath);
  }

  async function trustNewHostKey() {
    await app.trustNewHostKey();
  }

  async function trustActiveHost() {
    const trusted = await app.trustActiveHost();
    if (trusted) {
      await goto(terminalPath);
    }
  }

  function stopSessionTabAutoScroll() {
    sessionTabScrollVelocity = 0;
    sessionTabLastFrameTime = 0;

    if (sessionTabScrollFrame !== 0) {
      cancelAnimationFrame(sessionTabScrollFrame);
      sessionTabScrollFrame = 0;
    }
  }

  function updateSessionTabScrollIndicators() {
    if (!sessionTabsElement) {
      canScrollSessionTabsLeft = false;
      canScrollSessionTabsRight = false;
      return;
    }

    const maxScrollLeft =
      sessionTabsElement.scrollWidth - sessionTabsElement.clientWidth;
    canScrollSessionTabsLeft = sessionTabsElement.scrollLeft > 1;
    canScrollSessionTabsRight =
      sessionTabsElement.scrollLeft < maxScrollLeft - 1;
  }

  function stepSessionTabAutoScroll(timestamp: number) {
    if (!sessionTabsElement || sessionTabScrollVelocity === 0) {
      stopSessionTabAutoScroll();
      return;
    }

    const elapsed =
      sessionTabLastFrameTime === 0
        ? 16
        : Math.min(timestamp - sessionTabLastFrameTime, 32);
    sessionTabLastFrameTime = timestamp;
    sessionTabsElement.scrollLeft += sessionTabScrollVelocity * elapsed;
    updateSessionTabScrollIndicators();
    sessionTabScrollFrame = requestAnimationFrame(stepSessionTabAutoScroll);
  }

  function startSessionTabAutoScroll(velocity: number) {
    sessionTabScrollVelocity = velocity;

    if (sessionTabScrollFrame === 0) {
      sessionTabScrollFrame = requestAnimationFrame(stepSessionTabAutoScroll);
    }
  }

  function handleSessionTabsPointerMove(event: MouseEvent | PointerEvent) {
    if (!sessionTabsElement) {
      return;
    }

    const maxScrollLeft =
      sessionTabsElement.scrollWidth - sessionTabsElement.clientWidth;
    if (maxScrollLeft <= 0) {
      stopSessionTabAutoScroll();
      updateSessionTabScrollIndicators();
      return;
    }

    const bounds = sessionTabsElement.getBoundingClientRect();
    const leftDistance = event.clientX - bounds.left;
    const rightDistance = bounds.right - event.clientX;

    if (
      leftDistance < sessionTabEdgeSize &&
      sessionTabsElement.scrollLeft > 0
    ) {
      const intensity =
        (sessionTabEdgeSize - leftDistance) / sessionTabEdgeSize;
      startSessionTabAutoScroll(-sessionTabMaxScrollSpeed * intensity);
      return;
    }

    if (
      rightDistance < sessionTabEdgeSize &&
      sessionTabsElement.scrollLeft < maxScrollLeft
    ) {
      const intensity =
        (sessionTabEdgeSize - rightDistance) / sessionTabEdgeSize;
      startSessionTabAutoScroll(sessionTabMaxScrollSpeed * intensity);
      return;
    }

    stopSessionTabAutoScroll();
    updateSessionTabScrollIndicators();
  }

  function scheduleSessionTabScrollIndicatorUpdate(activeSessionCount: number) {
    requestAnimationFrame(() => {
      if (activeSessionCount !== app.activeSessions.length) {
        return;
      }

      updateSessionTabScrollIndicators();
    });
  }

  function findSessionTabAtPoint(
    clientX: number,
    clientY: number,
  ): { id: string; element: Element } | null {
    const el = document.elementFromPoint(clientX, clientY);
    const tab = el?.closest("[data-session-id]");
    const id = tab?.getAttribute("data-session-id");
    return tab && id ? { id, element: tab } : null;
  }

  function handleTabMouseDown(event: MouseEvent, sessionId: string) {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement;
    if (target.closest("[aria-label^='Close']")) return;
    dragState = { sessionId, startX: event.clientX, active: false };
  }

  function handleTabMouseMove(event: MouseEvent | PointerEvent) {
    if (!dragState) return;

    if (!dragState.active) {
      const dx = Math.abs(event.clientX - dragState.startX);
      if (dx < 6) return;
      dragState.active = true;
    }

    const target = findSessionTabAtPoint(event.clientX, event.clientY);
    if (!target || target.id === dragState.sessionId) {
      dragOverSessionId = null;
      return;
    }

    const fromIndex = app.activeSessions.findIndex(
      (session) => session.id === dragState?.sessionId,
    );
    const toIndex = app.activeSessions.findIndex(
      (session) => session.id === target.id,
    );
    if (fromIndex === -1 || toIndex === -1) return;

    const rect = target.element.getBoundingClientRect();
    const crossingRight =
      fromIndex < toIndex && event.clientX > rect.left + rect.width * 0.6;
    const crossingLeft =
      fromIndex > toIndex && event.clientX < rect.left + rect.width * 0.4;
    if (!crossingRight && !crossingLeft) return;

    dragOverSessionId = target.id;
    app.reorderSessions(dragState.sessionId, target.id);
  }

  function handleTabMouseUp() {
    dragState = null;
    dragOverSessionId = null;
  }

  function handleWindowMouseMove(event: MouseEvent) {
    if (dragState) {
      handleTabMouseMove(event);
      return;
    }

    if (!sessionTabsElement) return;
    const bounds = sessionTabsElement.getBoundingClientRect();
    if (event.clientY < bounds.top || event.clientY > bounds.bottom) return;
    handleSessionTabsPointerMove(event);
  }

  $effect(() => {
    scheduleSessionTabScrollIndicatorUpdate(app.activeSessions.length);
  });

  function handleGlobalKeydown(event: KeyboardEvent) {
    const mod = event.metaKey || event.ctrlKey;
    const target = event.target as HTMLElement;
    const isInput =
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT" ||
      target.isContentEditable;

    if (mod && event.key === ",") {
      event.preventDefault();
      app.openSettings();
      return;
    }

    if (mod && (event.key === "t" || event.key === "T") && !isInput) {
      event.preventDefault();
      void goto(terminalPath);
      return;
    }

    if (mod && (event.key === "w" || event.key === "W") && !isInput) {
      event.preventDefault();
      if (app.sessionStore.activeSessionId) {
        void closeSessionAndNavigate(app.sessionStore.activeSessionId);
      }
      return;
    }

    if (mod && event.key >= "1" && event.key <= "9" && !isInput) {
      event.preventDefault();
      const index = Number.parseInt(event.key, 10) - 1;
      if (index < app.activeSessions.length) {
        void activateSession(app.activeSessions[index].id);
      }
    }
  }
</script>

<svelte:window
  onkeydown={handleGlobalKeydown}
  onmousemove={handleWindowMouseMove}
  onmouseup={handleTabMouseUp}
/>

<QueryClientProvider client={queryClient}>
  {#if app.isLoading}
    <div class="flex min-h-screen items-center justify-center bg-background">
      <div class="flex flex-col items-center gap-4">
        <Loader2 class="size-8 animate-spin text-primary" />
        <p class="text-sm text-muted-foreground">Loading...</p>
      </div>
    </div>
  {:else if app.isError}
    <div
      class="auth-shell flex min-h-screen items-center justify-center px-4 py-8"
    >
      <div
        class="w-full max-w-2xl rounded-[2rem] border border-white/10 bg-slate-950/78 p-6 text-center shadow-2xl backdrop-blur-2xl sm:p-8"
      >
        <div
          class="mx-auto flex size-16 items-center justify-center rounded-[1.5rem] bg-destructive/10 text-destructive"
        >
          <AlertCircle class="size-8" />
        </div>
        <h1 class="mt-5 text-2xl font-semibold text-white">
          Backend connection unavailable
        </h1>
        <p class="mx-auto mt-3 max-w-xl text-sm leading-7 text-slate-300">
          {app.error ??
            "Unable to connect to the backend. Remote features are unavailable."}
        </p>
        <div class="mt-8 flex flex-wrap justify-center gap-3">
          <Button
            variant="outline"
            onclick={() => void app.init()}
            class="gap-2 border-white/10 bg-white/4 text-white hover:bg-white/8"
          >
            Retry
          </Button>
        </div>
      </div>
    </div>
  {:else}
    <div
      class="workspace-canvas flex h-screen w-screen overflow-hidden bg-background"
    >
      <Sidebar
        onLocalTerminal={openLocalTerminal}
        onK9sTerminal={openK9sTerminal}
        onClaudeCodeTerminal={openClaudeCodeTerminal}
        onOpencodeTerminal={openOpencodeTerminal}
        onOmpTerminal={openOmpTerminal}
        onHerdrTerminal={openHerdrTerminal}
        onManageKeys={() => goto("/keys")}
        onManageKnownHosts={() => goto("/known-hosts")}
        onPortForwards={() => goto("/forwards")}
        onSnippets={() => goto("/snippets")}
        onSftp={() => goto("/sftp")}
        onNewConnection={() => goto("/connections")}
        onGoHome={goHome}
        onOpenSettings={app.openSettings}
        connectionCount={app.connections.length}
        keyCount={app.keys.length}
        forwardCount={app.savedPortForwards.length}
        activeSection={activeSidebarSection}
      />

      <div class="flex min-h-0 min-w-0 flex-1 flex-col bg-[#080c13]/72">
        <div class="flex h-11 shrink-0 border-b border-white/10">
          {#if currentPage}
            <div
              class="flex shrink-0 items-center overflow-hidden border-r border-white/10 px-3"
              transition:slide={{ axis: "x", duration: 180, easing: cubicOut }}
            >
              <span
                class="flex items-center gap-2 whitespace-nowrap rounded-lg border border-cyan-300/30 bg-cyan-300/10 px-3 py-1 text-sm font-medium text-cyan-50"
              >
                <currentPage.icon class="size-3.5 text-cyan-200" />
                {currentPage.label}
              </span>
            </div>
          {/if}

          <div
            class="relative h-full min-w-0 flex-1 transition-opacity duration-150 {currentPage
              ? 'opacity-45 hover:opacity-100'
              : ''}"
          >
          {#if canScrollSessionTabsLeft}
            <div
              class="pointer-events-none absolute inset-y-0 left-0 z-20 flex w-12 items-center bg-gradient-to-r from-[#080c13] via-[#080c13]/90 to-transparent pl-1 text-cyan-200/45"
              aria-hidden="true"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="size-4"
              >
                <path d="m15 18-6-6 6-6" />
              </svg>
            </div>
          {/if}

          <div
            bind:this={sessionTabsElement}
            role="group"
            aria-label="Active terminal sessions"
            class="session-tabs flex h-full items-center gap-1 overflow-x-auto px-3"
            onscroll={updateSessionTabScrollIndicators}
            onpointerleave={stopSessionTabAutoScroll}
          >
            {#each app.activeSessions as session, sessionIndex (session.id)}
              {@const isActive =
                isTerminalRoute &&
                session.id === app.sessionStore.activeSessionId}
              {@const isDragging =
                dragState?.active && dragState.sessionId === session.id}
              {@const isDragOver = dragOverSessionId === session.id}
              <div
                data-session-id={session.id}
                class="shrink-0"
                animate:flip={{ duration: 140, easing: cubicOut }}
              >
                <ContextMenu.Root>
                  <ContextMenu.Trigger class="contents">
                    <div
                      class={isActive
                        ? `group relative flex shrink-0 items-center gap-2 rounded-lg border border-cyan-300/30 bg-cyan-300/10 px-3 py-1 text-sm text-white transition hover:bg-cyan-300/14${isDragging ? " opacity-40" : ""}${isDragOver ? " ring-1 ring-cyan-300/50" : ""}`
                        : `group relative flex shrink-0 items-center gap-2 rounded-lg border border-transparent px-3 py-1 text-sm text-slate-400 transition hover:border-white/10 hover:bg-white/[0.045] hover:text-white${isDragging ? " opacity-40" : ""}${isDragOver ? " ring-1 ring-cyan-300/50" : ""}`}
                    >
                      <button
                        type="button"
                        class="absolute inset-y-0 left-0 right-7 cursor-pointer rounded-l-lg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/50"
                        onmousedown={(event) =>
                          handleTabMouseDown(event, session.id)}
                        onclick={() => activateSession(session.id)}
                        aria-label={`Switch to ${session.name}`}
                      ></button>
                      <span
                        class="pointer-events-none relative z-10 flex min-w-0 flex-1 items-center gap-2 text-left"
                      >
                        <span
                          class={session.status === "connected"
                            ? "size-2 shrink-0 rounded-full bg-emerald-400 shadow-[0_0_10px_rgb(52_211_153/0.55)]"
                            : session.status === "connecting"
                              ? "size-2 shrink-0 rounded-full bg-amber-300 shadow-[0_0_10px_rgb(252_211_77/0.45)] animate-pulse"
                              : session.status === "trust_required"
                                ? "size-2 shrink-0 rounded-full bg-amber-300 shadow-[0_0_10px_rgb(252_211_77/0.45)]"
                                : "size-2 shrink-0 rounded-full bg-red-400 shadow-[0_0_10px_rgb(248_113_113/0.45)]"}
                        ></span>
                        <span class="truncate font-medium">{session.name}</span>
                      </span>
                      <button
                        type="button"
                        class="relative z-20 flex size-6 shrink-0 items-center justify-center rounded text-slate-500 opacity-0 transition-opacity hover:bg-red-400/10 hover:text-red-300 group-hover:opacity-100"
                        onclick={(event) => {
                          event.stopPropagation();
                          void closeSessionAndNavigate(session.id);
                        }}
                        aria-label={`Close ${session.name}`}
                      >
                        <svg
                          xmlns="http://www.w3.org/2000/svg"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          class="size-3"
                        >
                          <path d="M18 6 6 18" /><path d="m6 6 12 12" />
                        </svg>
                      </button>
                    </div>
                  </ContextMenu.Trigger>
                  <ContextMenu.Content
                    class="min-w-44 border-white/10 bg-slate-950/96 text-slate-100 shadow-2xl shadow-black/45"
                  >
                    <ContextMenu.Label class="max-w-56 truncate text-slate-400">
                      {session.name}
                    </ContextMenu.Label>
                    <ContextMenu.Separator class="bg-white/10" />
                    <ContextMenu.Item
                      class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                      onclick={() => void closeSessionAndNavigate(session.id)}
                    >
                      Close
                    </ContextMenu.Item>
                    <ContextMenu.Item
                      class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                      disabled={session.type !== "local" &&
                        !session.connectionId}
                      onclick={() => void app.duplicateSession(session.id)}
                    >
                      Duplicate Tab
                    </ContextMenu.Item>
                    <ContextMenu.Item
                      class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                      disabled={sessionIndex === 0}
                      onclick={() =>
                        void closeSessionIdsAndNavigate(
                          sessionIdsBefore(sessionIndex),
                        )}
                    >
                      Close Tabs to the Left
                    </ContextMenu.Item>
                    <ContextMenu.Item
                      class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                      disabled={sessionIndex === app.activeSessions.length - 1}
                      onclick={() =>
                        void closeSessionIdsAndNavigate(
                          sessionIdsAfter(sessionIndex),
                        )}
                    >
                      Close Tabs to the Right
                    </ContextMenu.Item>
                    <ContextMenu.Separator class="bg-white/10" />
                    <ContextMenu.Item
                      class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                      onclick={() =>
                        void closeSessionIdsAndNavigate(allSessionIds())}
                    >
                      Close All Tabs
                    </ContextMenu.Item>
                  </ContextMenu.Content>
                </ContextMenu.Root>
              </div>
            {/each}
          </div>

          {#if canScrollSessionTabsRight}
            <div
              class="pointer-events-none absolute inset-y-0 right-0 z-20 flex w-12 items-center justify-end bg-gradient-to-l from-[#080c13] via-[#080c13]/90 to-transparent pr-1 text-cyan-200/45"
              aria-hidden="true"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="size-4"
              >
                <path d="m9 18 6-6-6-6" />
              </svg>
            </div>
          {/if}
          </div>
        </div>

        <div class="relative flex min-h-0 flex-1 flex-col overflow-hidden">
          {#if isSftpRoute}
            {#if children}
              {@render children()}
            {/if}
          {:else if isTerminalRoute}
            {#if !hasTerminalErrorOverlay && !hasTerminalDisconnectedOverlay}
              {@render children()}
            {/if}

            {#if isTerminalRoute && app.activeSession?.status === "disconnected"}
              <div
                class="absolute inset-0 z-20 flex h-full flex-col items-center justify-center p-8"
                aria-live="polite"
              >
                <div
                  class="w-full max-w-lg rounded-[2rem] border border-red-300/20 bg-red-400/8 p-8 text-center shadow-2xl shadow-black/30"
                >
                  <div
                    class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300"
                  >
                    <AlertCircle class="size-7" />
                  </div>
                  <h2 class="mt-5 text-xl font-semibold text-white">
                    Terminal disconnected
                  </h2>
                  <p
                    class="mx-auto mt-2 max-w-md text-sm leading-6 text-slate-400"
                  >
                    {app.activeSession.name} is no longer connected. The tab stays
                    open so you can see it failed and decide whether to retry or close
                    it.
                  </p>
                  <div class="mt-6 flex flex-wrap justify-center gap-3">
                    {#if app.activeSession.connectionId}
                      <Button
                        onclick={retryActiveConnection}
                        class="gap-2 rounded-2xl bg-red-300 px-5 text-red-950 hover:bg-red-200"
                      >
                        Retry session
                      </Button>
                    {/if}
                    <Button
                      variant="outline"
                      onclick={() =>
                        void closeSessionAndNavigate(
                          app.activeSession?.id ?? "",
                        )}
                      class="rounded-2xl border-white/10 bg-white/4 px-5 text-white hover:bg-white/8"
                    >
                      Close tab
                    </Button>
                  </div>
                </div>
              </div>
            {/if}

            {#if isTerminalRoute && (app.activeSession?.status === "error" || app.activeSession?.status === "trust_required")}
              <ConnectionStatusOverlay
                status={app.activeSession.status}
                name={app.activeSession.name}
                error={app.activeSession.error}
                trustPrompt={app.activeSession.trustPrompt}
                trustMismatch={app.activeSession.trustMismatch}
                trustError={app.trustError}
                trustConfirming={app.trustConfirming}
                canTrust={Boolean(app.activeSession.connectionId)}
                onRetry={retryActiveConnection}
                onTrust={trustActiveHost}
                onReplaceTrust={trustNewHostKey}
                onCancel={app.activeSession.trustPrompt
                  ? () =>
                      app.sessionStore.removeSession(app.activeSession?.id ?? "")
                  : undefined}
              />
            {/if}
          {:else}
            {@render children()}
          {/if}

          {#if app.mountedTerminalSessions.length > 0}
            <div
              class={isTerminalVisible
                ? "absolute inset-0 z-10 flex h-full min-h-0 flex-col overflow-hidden p-3"
                : "pointer-events-none absolute inset-0 z-10 flex h-full min-h-0 flex-col overflow-hidden p-3 opacity-0"}
            >
              <div
                class="terminal-frame relative min-h-0 flex-1 overflow-hidden rounded-[1.35rem] border border-white/10 bg-[#080c13]/72 shadow-2xl shadow-black/45"
              >
                {#each app.mountedTerminalSessions as session (session.id)}
                  <div
                    class={isTerminalVisible &&
                    session.id === app.visibleTerminalSessionId
                      ? "absolute inset-0 z-10 min-h-0 overflow-hidden opacity-100 pointer-events-auto"
                      : "absolute inset-0 z-0 min-h-0 overflow-hidden opacity-0 pointer-events-none"}
                  >
                    <!-- A connecting session still has a placeholder id the backend rejects. -->
                    {#if session.status !== "connecting"}
                      <TerminalView
                        sessionId={session.id}
                        sessionType={session.type}
                        active={isTerminalVisible &&
                          session.id === app.visibleTerminalSessionId}
                        config={app.terminalConfig}
                        subscribeOutput={(callback) =>
                          app.sessionStore.subscribeSessionOutput(
                            session.id,
                            callback,
                          )}
                        onClose={() => {
                          if (
                            app.sessionStore.sessions.get(session.id)
                              ?.status !== "error"
                          ) {
                            app.sessionStore.updateSession(session.id, {
                              status: "disconnected",
                            });
                          }
                        }}
                        onError={(error) =>
                          app.sessionStore.updateSession(session.id, {
                            status: "error",
                            error,
                          })}
                        onRequestClose={() =>
                          void closeSessionAndNavigate(session.id)}
                      />
                    {:else}
                      <ConnectionStatusOverlay
                        status="connecting"
                        name={session.name}
                      />
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>

      <SettingsModal open={app.showSettings} onClose={app.closeSettings} />
    </div>
  {/if}
</QueryClientProvider>
