<script lang="ts">
  import "../app.css";
  import type { Snippet } from "svelte";
  import { onDestroy, onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
  import { AlertCircle, Loader2 } from "@lucide/svelte";

  import AuthShell from "$lib/components/auth-shell.svelte";
  import SettingsModal from "$lib/components/settings-modal.svelte";
  import Sidebar from "$lib/components/sidebar.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import {
    createAppShellStore,
    setAppShellContext,
  } from "$lib/stores/app-shell.svelte.js";
  import TerminalView from "$lib/terminal/terminal.svelte";

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
  const isTerminalRoute = $derived(routePath === terminalPath);
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

    return "terminal";
  });

  onMount(async () => {
    await app.init();
  });

  onDestroy(() => {
    app.cleanup();
  });

  async function activateSession(id: string) {
    app.activateSession(id);
    await goto(terminalPath);
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

  async function closeSessionAndNavigate(id: string) {
    const isLastSession = app.sessionStore.sessions.size === 1;
    app.closeSession(id);
    if (isLastSession && routePath === terminalPath) {
      await goto(connectionsPath);
    }
  }

  async function goHome() {
    app.sessionStore.setActiveSession(null);
    await goto(connectionsPath);
  }

  async function retryActiveConnection() {
    await app.retryActiveConnection();
    await goto(terminalPath);
  }

  async function trustActiveHost() {
    const trusted = await app.trustActiveHost();
    if (trusted) {
      await goto(terminalPath);
    }
  }

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

<svelte:window onkeydown={handleGlobalKeydown} />

<QueryClientProvider client={queryClient}>
  {#if app.isLoading}
    <div class="flex min-h-screen items-center justify-center bg-background">
      <div class="flex flex-col items-center gap-4">
        <Loader2 class="size-8 animate-spin text-primary" />
        <p class="text-sm text-muted-foreground">Restoring session...</p>
      </div>
    </div>
  {:else if app.isUnauthenticated}
    <AuthShell
      onLogin={app.login}
      onSignup={app.signup}
      onForgotPassword={app.forgotPassword}
      onResetPassword={app.resetAccountPassword}
      isLoading={app.isLoading}
      error={app.error}
    />
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
        onManageKeys={() => goto("/keys")}
        onPortForwards={() => goto("/forwards")}
        onNewConnection={() => goto("/connections")}
        onGoHome={goHome}
        authEmail={app.authStatus?.email ?? ""}
        onOpenSettings={app.openSettings}
        onLogout={app.logout}
        connectionCount={app.connections.length}
        keyCount={app.keys.length}
        forwardCount={app.savedPortForwards.length}
        activeSection={activeSidebarSection}
      />

      <div class="flex min-h-0 min-w-0 flex-1 flex-col bg-[#080c13]/72">
        <div class="flex h-10 shrink-0 items-center gap-1 overflow-x-auto border-b border-white/10 px-4">
          {#each app.activeSessions as session (session.id)}
            {@const isActive = session.id === app.sessionStore.activeSessionId}
            <div
              class={isActive
                ? "group flex shrink-0 items-center gap-2 rounded-lg border border-cyan-300/30 bg-cyan-300/10 px-3 py-1.5 text-sm text-white transition hover:bg-cyan-300/14"
                : "group flex shrink-0 items-center gap-2 rounded-lg border border-transparent px-3 py-1.5 text-sm text-slate-400 transition hover:border-white/10 hover:bg-white/[0.045] hover:text-white"}
            >
              <button
                type="button"
                class="flex min-w-0 flex-1 items-center gap-2 text-left"
                onclick={() => activateSession(session.id)}
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
              </button>
              <button
                type="button"
                class="flex size-5 shrink-0 items-center justify-center rounded text-slate-500 opacity-0 transition-opacity hover:bg-red-400/10 hover:text-red-300 group-hover:opacity-100"
                onclick={(event) => {
                  event.stopPropagation();
                  void closeSessionAndNavigate(session.id);
                }}
                aria-label={`Close ${session.name}`}
              >
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="size-3">
                  <path d="M18 6 6 18"/><path d="m6 6 12 12"/>
                </svg>
              </button>
            </div>
          {/each}
        </div>

        <div class="relative flex min-h-0 flex-1 flex-col overflow-hidden">
          {#if !hasTerminalErrorOverlay}
            {@render children()}
          {/if}

          {#if isTerminalRoute && app.mountedTerminalSessions.length > 0}
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
                      onClose={() =>
                        app.sessionStore.updateSession(session.id, {
                          status: "disconnected",
                        })}
                    />
                    {#if session.status === "connecting"}
                      <div class="absolute inset-0 z-20 flex flex-col items-center justify-center bg-[#080c13]/90">
                        <Loader2
                          class="mb-4 size-8 animate-spin text-amber-200"
                        />
                        <p class="text-sm font-semibold text-white">
                          Connecting to {session.name}
                        </p>
                        <p class="mt-2 text-xs text-slate-500">
                          Negotiating terminal session…
                        </p>
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          {#if isTerminalRoute && (app.activeSession?.status === "error" || app.activeSession?.status === "trust_required")}
            <div class="absolute inset-0 z-20 flex h-full flex-col items-center justify-center p-8">
              <div
                class={app.activeSession.status === "trust_required"
                  ? "w-full max-w-xl overflow-hidden rounded-[1.75rem] border border-white/10 bg-slate-950/88 text-left shadow-2xl shadow-black/40 ring-1 ring-amber-300/10 backdrop-blur-xl"
                  : "max-w-lg rounded-[2rem] border border-red-300/20 bg-red-400/8 p-8 shadow-2xl shadow-black/30"}
              >
                {#if app.activeSession.trustPrompt}
                  <div class="border-b border-white/10 px-6 py-5 sm:px-7">
                    <div class="flex items-start gap-4">
                      <div
                        class="grid size-11 shrink-0 place-items-center rounded-2xl border border-amber-300/20 bg-amber-300/10 text-amber-200 shadow-[0_0_24px_rgb(252_211_77/0.08)]"
                      >
                        <AlertCircle class="size-5" />
                      </div>
                      <div class="min-w-0 flex-1">
                        <div class="flex flex-wrap items-center gap-2">
                          <span
                            class="rounded-full border border-amber-300/20 bg-amber-300/10 px-2.5 py-1 text-[11px] font-medium uppercase tracking-[0.18em] text-amber-200"
                          >
                            SSH host identity
                          </span>
                        </div>
                        <h2
                          class="mt-3 text-xl font-semibold tracking-tight text-white"
                        >
                          Verify SSH host identity
                        </h2>
                        <p class="mt-2 text-sm leading-6 text-slate-400">
                          Confirm this fingerprint before opening a terminal
                          session.
                        </p>
                      </div>
                    </div>
                  </div>

                  <div class="px-6 py-5 sm:px-7">
                    <dl class="grid gap-3 text-sm">
                      <div
                        class="flex items-center justify-between gap-4 rounded-2xl border border-white/8 bg-white/[0.035] px-4 py-3"
                      >
                        <dt
                          class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500"
                        >
                          Host
                        </dt>
                        <dd class="truncate font-mono text-slate-100">
                          {app.activeSession.trustPrompt.host}:{app
                            .activeSession.trustPrompt.port}
                        </dd>
                      </div>
                      <div
                        class="flex items-center justify-between gap-4 rounded-2xl border border-white/8 bg-white/[0.035] px-4 py-3"
                      >
                        <dt
                          class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500"
                        >
                          Algorithm
                        </dt>
                        <dd class="font-mono text-slate-100">
                          {app.activeSession.trustPrompt.algorithm}
                        </dd>
                      </div>
                      <div
                        class="rounded-2xl border border-white/8 bg-black/25 p-4"
                      >
                        <dt
                          class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500"
                        >
                          Fingerprint
                        </dt>
                        <dd
                          class="mt-3 break-all rounded-xl border border-amber-300/10 bg-amber-300/[0.06] px-3 py-2.5 font-mono text-sm leading-6 text-amber-100"
                        >
                          {app.activeSession.trustPrompt.fingerprint}
                        </dd>
                      </div>
                    </dl>
                    <p class="mt-4 text-xs leading-5 text-slate-500">
                      Only continue if this fingerprint matches the server you
                      expect. It will be saved locally in Tauri's trust JSON.
                    </p>
                  </div>

                  {#if app.trustError}
                    <p
                      class="mx-6 mb-4 rounded-2xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-200 sm:mx-7"
                    >
                      {app.trustError}
                    </p>
                  {/if}
                  <div
                    class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-white/[0.025] px-6 py-4 sm:px-7"
                  >
                    {#if app.activeSession.connectionId}
                      <Button
                        onclick={trustActiveHost}
                        disabled={app.trustConfirming}
                        class="order-2 gap-2 rounded-2xl bg-amber-300 px-4 text-amber-950 hover:bg-amber-200 sm:order-none"
                      >
                        {#if app.trustConfirming}
                          <Loader2 class="size-4 animate-spin" />
                        {/if}
                        Trust host and retry
                      </Button>
                    {:else}
                      <p class="max-w-md text-sm leading-6 text-slate-400">
                        Save this connection first to trust the host and retry
                        automatically.
                      </p>
                    {/if}
                    <Button
                      variant="outline"
                      onclick={() =>
                        app.sessionStore.removeSession(
                          app.activeSession?.id ?? "",
                        )}
                      class="rounded-2xl border-white/10 bg-white/4 px-4 text-white hover:bg-white/8"
                    >
                      Cancel
                    </Button>
                  </div>
                {:else if app.activeSession.trustMismatch}
                  <div
                    class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300"
                  >
                    <AlertCircle class="size-7" />
                  </div>
                  <h2 class="mt-5 text-center text-xl font-semibold text-white">
                    Connection failed
                  </h2>
                  <p
                    class="mx-auto mt-2 max-w-md text-center text-sm leading-6 text-slate-400"
                  >
                    {app.activeSession.error ?? "Unknown error"}
                  </p>
                  <div
                    class="mt-5 rounded-2xl border border-red-300/25 bg-red-300/8 p-4 text-left"
                  >
                    <p class="text-sm font-semibold text-red-100">
                      Saved fingerprint does not match.
                    </p>
                    <dl class="mt-3 grid gap-2 text-xs text-slate-300">
                      <div class="space-y-1">
                        <dt class="text-slate-500">Expected</dt>
                        <dd
                          class="break-all rounded-xl bg-black/30 px-3 py-2 font-mono"
                        >
                          {app.activeSession.trustMismatch.expected_fingerprint}
                        </dd>
                      </div>
                      <div class="space-y-1">
                        <dt class="text-slate-500">Presented</dt>
                        <dd
                          class="break-all rounded-xl bg-black/30 px-3 py-2 font-mono text-red-100"
                        >
                          {app.activeSession.trustMismatch
                            .presented_fingerprint}
                        </dd>
                      </div>
                    </dl>
                    <p class="mt-3 text-xs leading-5 text-slate-400">
                      This may indicate the server changed keys or a
                      man-in-the-middle risk. Not updating trust automatically.
                    </p>
                  </div>
                  <div class="mt-6 flex justify-center">
                    <Button
                      onclick={retryActiveConnection}
                      class="gap-2 rounded-2xl bg-red-300 px-5 text-red-950 hover:bg-red-200"
                    >
                      Retry session
                    </Button>
                  </div>
                {:else}
                  <div
                    class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300"
                  >
                    <AlertCircle class="size-7" />
                  </div>
                  <h2 class="mt-5 text-center text-xl font-semibold text-white">
                    Connection failed
                  </h2>
                  <p
                    class="mx-auto mt-2 max-w-md text-center text-sm leading-6 text-slate-400"
                  >
                    {app.activeSession.error ?? "Unknown error"}
                  </p>
                  <div class="mt-6 flex justify-center">
                    <Button
                      onclick={retryActiveConnection}
                      class="gap-2 rounded-2xl bg-red-300 px-5 text-red-950 hover:bg-red-200"
                    >
                      Retry session
                    </Button>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>

      <SettingsModal open={app.showSettings} onClose={app.closeSettings} />
    </div>
  {/if}
</QueryClientProvider>
