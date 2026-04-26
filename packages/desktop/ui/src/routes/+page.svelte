<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { AlertCircle, Loader2 } from "@lucide/svelte";

  import AuthShell from "$lib/components/auth-shell.svelte";
  import ConnectionForm from "$lib/components/connection-form.svelte";
  import ConnectionsView from "$lib/components/connections-view.svelte";
  import PortForwardPanel from "$lib/components/port-forward-panel.svelte";
  import SshKeysView from "$lib/components/ssh-keys-view.svelte";
  import SettingsModal from "$lib/components/settings-modal.svelte";
  import Sidebar from "$lib/components/sidebar.svelte";
  import TerminalTabs from "$lib/components/terminal-tabs.svelte";
  import WelcomeView from "$lib/components/welcome-view.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import type {
    ConnectionConfig,
    SaveConnectionInput,
    TerminalConfig,
  } from "$lib/stores/bootstrap.svelte.js";
  import { createBootstrapStore } from "$lib/stores/bootstrap.svelte.js";
  import { createSessionStore, type Session } from "$lib/stores/session.svelte.js";
  import { loadAppSettings } from "$lib/api/api-client.js";
  import type { SshKeyRecord } from "$lib/api/types.js";
  import TerminalView from "$lib/terminal/terminal.svelte";

  const bootstrapStore = createBootstrapStore();
  const sessionStore = createSessionStore();

  let sidebarCollapsed = $state(false);
  let showSettings = $state(false);
  let showConnectionForm = $state(false);
  let currentView = $state<"terminal" | "connections" | "keys">("terminal");
  let editingConnection = $state<ConnectionConfig | null>(null);
  let connectionFormError = $state<string | null>(null);
  let connectionSaving = $state(false);
  let trustConfirming = $state(false);
  let trustError = $state<string | null>(null);

  const activeSession = $derived(
    sessionStore.activeSessionId ? sessionStore.sessions.get(sessionStore.activeSessionId) : undefined,
  );

  const activeSessions = $derived(
    Array.from(sessionStore.sessions.values()).filter(
      (session) => session.status !== "disconnected",
    ) as Session[],
  );

  const mountedTerminalSessions = $derived(
    activeSessions.filter((session) => session.status === "connected") as Session[],
  );

  const activeSessionPortForwards = $derived(
    activeSession
      ? Array.from(sessionStore.portForwards.values()).filter(
          (forward) => forward.session_id === activeSession.id,
        )
      : [],
  );

  const terminalConfig = $derived(bootstrapStore.getTerminalConfig());
  const connections = $derived(bootstrapStore.getConnections());
  const recentConnectionIds = $derived(bootstrapStore.getRecentConnectionIds());
  const keys = $derived(bootstrapStore.getKeys());

  onMount(async () => {
    await loadAppSettings();
    await sessionStore.init();
    await bootstrapStore.init();
  });

  onDestroy(() => {
    sessionStore.cleanup();
  });

  async function handleLogin(email: string, password: string) {
    await bootstrapStore.login(email, password);
  }

  async function handleSignup(email: string, password: string) {
    await bootstrapStore.register(email, password);
  }

  async function handleLogout() {
    await bootstrapStore.logout();
  }

  async function handleSelectConnection(connection: ConnectionConfig) {
    currentView = "terminal";
    await sessionStore.connectSavedConnection(connection, 80, 24);
    void bootstrapStore.recordRecentConnection(connection.id).catch(() => undefined);
  }

  function handleActivateSession(id: string) {
    sessionStore.setActiveSession(id);
    currentView = "terminal";
  }

  function openNewConnectionForm() {
    editingConnection = null;
    connectionFormError = null;
    showConnectionForm = true;
  }

  function openEditConnectionForm(connection: ConnectionConfig) {
    editingConnection = connection;
    connectionFormError = null;
    showConnectionForm = true;
  }

  function closeConnectionForm() {
    showConnectionForm = false;
    editingConnection = null;
    connectionFormError = null;
    connectionSaving = false;
  }

  async function handleSaveConnection(connection: SaveConnectionInput) {
    connectionSaving = true;
    connectionFormError = null;

    try {
      await bootstrapStore.saveConnection(connection);
      closeConnectionForm();
    } catch (error) {
      connectionFormError = error instanceof Error ? error.message : String(error);
    } finally {
      connectionSaving = false;
    }
  }

  async function handleDeleteConnection(connection: ConnectionConfig) {
    const confirmed = window.confirm(`Delete saved connection "${connection.name}"?`);
    if (!confirmed) return;

    await bootstrapStore.deleteConnection(connection);
    sessionStore.disconnectConnectionSessions(connection.id);
  }

  async function handleSaveKey(name: string, privateKey: string, passphrase: string) {
    await bootstrapStore.saveKey({
      name,
      kind: "inline",
      encrypted_private_key: privateKey,
      encrypted_passphrase: passphrase || null,
    });
  }

  async function handleDeleteKey(key: SshKeyRecord) {
    await bootstrapStore.deleteKey(key);
  }

  async function handleUpdateKey(keyId: string, name: string, privateKey: string, passphrase: string) {
    await bootstrapStore.updateKey(keyId, {
      name,
      kind: "inline",
      encrypted_private_key: privateKey,
      encrypted_passphrase: passphrase || null,
    });
  }

  function handleSessionClose(id: string) {
    void sessionStore.disconnectSession(id);
  }

  async function handleSettingsSave(config: TerminalConfig) {
    await bootstrapStore.saveTerminalConfig(config);
    showSettings = false;
  }

  function retryActiveConnection() {
    if (!activeSession) return;

    if (activeSession.connectionId) {
      const connection = connections.find((candidate) => candidate.id === activeSession.connectionId);
      if (connection) {
        void handleSelectConnection(connection);
      }
      return;
    }

    currentView = "connections";
  }

  async function trustActiveHost() {
    if (!activeSession?.trustPrompt || !activeSession.connectionId) return;

    const connection = connections.find((candidate) => candidate.id === activeSession.connectionId);
    if (!connection) {
      trustError = "Saved connection not found. Open Connections and try again.";
      return;
    }

    const prompt = activeSession.trustPrompt;
    const failedSessionId = activeSession.id;
    trustConfirming = true;
    trustError = null;

    try {
      await sessionStore.confirmHostTrust({
        host: prompt.host,
        port: prompt.port,
        algorithm: prompt.algorithm,
        fingerprint: prompt.fingerprint,
      });
      sessionStore.removeSession(failedSessionId);
      await handleSelectConnection(connection);
    } catch (error) {
      trustError = error instanceof Error ? error.message : String(error);
    } finally {
      trustConfirming = false;
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
      showSettings = true;
      return;
    }

    if (mod && (event.key === "t" || event.key === "T") && !isInput) {
      event.preventDefault();
      currentView = "terminal";
      return;
    }

    if (mod && (event.key === "w" || event.key === "W") && !isInput) {
      event.preventDefault();
      if (sessionStore.activeSessionId) {
        handleSessionClose(sessionStore.activeSessionId);
      }
      return;
    }

    if (mod && event.key >= "1" && event.key <= "9" && !isInput) {
      event.preventDefault();
      const index = Number.parseInt(event.key, 10) - 1;
      if (index < activeSessions.length) {
        handleActivateSession(activeSessions[index].id);
      }
    }
  }

</script>

<svelte:window onkeydown={handleGlobalKeydown} />

{#if bootstrapStore.isLoading}
  <div class="flex min-h-screen items-center justify-center bg-background">
    <div class="flex flex-col items-center gap-4">
      <Loader2 class="size-8 animate-spin text-primary" />
      <p class="text-sm text-muted-foreground">Restoring session...</p>
    </div>
  </div>
{:else if bootstrapStore.isUnauthenticated}
  <AuthShell onLogin={handleLogin} onSignup={handleSignup} isLoading={bootstrapStore.isLoading} error={bootstrapStore.error} />
{:else if bootstrapStore.isError}
  <div class="auth-shell flex min-h-screen items-center justify-center px-4 py-8">
    <div class="w-full max-w-2xl rounded-[2rem] border border-white/10 bg-slate-950/78 p-6 text-center shadow-2xl backdrop-blur-2xl sm:p-8">
      <div class="mx-auto flex size-16 items-center justify-center rounded-[1.5rem] bg-destructive/10 text-destructive">
        <AlertCircle class="size-8" />
      </div>
      <h1 class="mt-5 text-2xl font-semibold text-white">Backend connection unavailable</h1>
      <p class="mx-auto mt-3 max-w-xl text-sm leading-7 text-slate-300">
        {bootstrapStore.error ?? "Unable to connect to the backend. Remote features are unavailable."}
      </p>
      <div class="mt-8 flex flex-wrap justify-center gap-3">
        <Button variant="outline" onclick={() => bootstrapStore.init()} class="gap-2 border-white/10 bg-white/4 text-white hover:bg-white/8">
          Retry
        </Button>
      </div>
    </div>
  </div>
{:else}
  <div class="workspace-canvas flex h-screen w-screen overflow-hidden bg-background">
    <Sidebar
      {connections}
      sessions={sessionStore.sessions}
      {recentConnectionIds}
      activeSessionId={sessionStore.activeSessionId}
      collapsed={sidebarCollapsed}
      onToggle={() => (sidebarCollapsed = !sidebarCollapsed)}
      onSelect={handleSelectConnection}
      onEdit={(conn) => { currentView = "connections"; openEditConnectionForm(conn); }}
      onDelete={handleDeleteConnection}
      onLocalTerminal={() => { sessionStore.connectLocal("Local Terminal"); currentView = "terminal"; }}
      onManageKeys={() => { currentView = "keys"; }}
      onNewConnection={() => { currentView = "connections"; }}
      authEmail={bootstrapStore.authStatus?.email ?? ""}
      onOpenSettings={() => (showSettings = true)}
      onLogout={handleLogout}
      keyCount={bootstrapStore.getKeys().length}
    />

    <div class="flex min-h-0 min-w-0 flex-1 flex-col bg-[#080c13]/72">
      <div class="shrink-0">
        <TerminalTabs
          sessions={activeSessions}
          activeSessionId={sessionStore.activeSessionId}
          onActivate={handleActivateSession}
          onClose={handleSessionClose}
        />
      </div>

      <div class="relative flex min-h-0 flex-1 flex-col overflow-hidden">
        {#if currentView === "connections"}
          <ConnectionsView
            {connections}
            sessions={sessionStore.sessions}
            activeSessionId={sessionStore.activeSessionId}
            onSelect={handleSelectConnection}
            onEdit={openEditConnectionForm}
            onNew={openNewConnectionForm}
            onDelete={handleDeleteConnection}
          />
        {:else if currentView === "keys"}
          <SshKeysView
            keys={keys}
            onSave={handleSaveKey}
            onUpdate={handleUpdateKey}
            onDelete={handleDeleteKey}
          />
        {:else}
          {#if !activeSession}
            <WelcomeView
              sessions={sessionStore.sessions}
              sessionStore={{
                sessions: sessionStore.sessions,
                activeSessionId: sessionStore.activeSessionId,
                setActiveSession: sessionStore.setActiveSession,
                connectLocal: (name: string) => sessionStore.connectLocal(name),
              }}
              terminalConfig={terminalConfig}
              onOpenConnectionManager={() => (currentView = "connections")}
              onManageKeys={() => (currentView = "keys")}
            />
          {:else if activeSession.status === "connecting"}
            <div class="flex h-full flex-col items-center justify-center p-8">
              <div class="rounded-[2rem] border border-amber-300/15 bg-amber-300/8 p-8 text-center shadow-2xl shadow-black/30">
                <Loader2 class="mx-auto mb-4 size-8 animate-spin text-amber-200" />
                <p class="text-sm font-semibold text-white">Connecting to {activeSession.name}</p>
                <p class="mt-2 text-xs text-slate-500">Negotiating terminal session…</p>
              </div>
            </div>
          {:else if activeSession.status === "error" || activeSession.status === "trust_required"}
            <div class="flex h-full flex-col items-center justify-center p-8">
              <div class={activeSession.status === "trust_required"
                ? "w-full max-w-xl overflow-hidden rounded-[1.75rem] border border-white/10 bg-slate-950/88 text-left shadow-2xl shadow-black/40 ring-1 ring-amber-300/10 backdrop-blur-xl"
                : "max-w-lg rounded-[2rem] border border-red-300/20 bg-red-400/8 p-8 shadow-2xl shadow-black/30"}>
                {#if activeSession.trustPrompt}
                  <div class="border-b border-white/10 px-6 py-5 sm:px-7">
                    <div class="flex items-start gap-4">
                      <div class="grid size-11 shrink-0 place-items-center rounded-2xl border border-amber-300/20 bg-amber-300/10 text-amber-200 shadow-[0_0_24px_rgb(252_211_77/0.08)]">
                        <AlertCircle class="size-5" />
                      </div>
                      <div class="min-w-0 flex-1">
                        <div class="flex flex-wrap items-center gap-2">
                          <span class="rounded-full border border-amber-300/20 bg-amber-300/10 px-2.5 py-1 text-[11px] font-medium uppercase tracking-[0.18em] text-amber-200">
                            SSH host identity
                          </span>
                        </div>
                        <h2 class="mt-3 text-xl font-semibold tracking-tight text-white">Verify SSH host identity</h2>
                        <p class="mt-2 text-sm leading-6 text-slate-400">
                          Confirm this fingerprint before opening a terminal session.
                        </p>
                      </div>
                    </div>
                  </div>

                  <div class="px-6 py-5 sm:px-7">
                    <dl class="grid gap-3 text-sm">
                      <div class="flex items-center justify-between gap-4 rounded-2xl border border-white/8 bg-white/[0.035] px-4 py-3">
                        <dt class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500">Host</dt>
                        <dd class="truncate font-mono text-slate-100">{activeSession.trustPrompt.host}:{activeSession.trustPrompt.port}</dd>
                      </div>
                      <div class="flex items-center justify-between gap-4 rounded-2xl border border-white/8 bg-white/[0.035] px-4 py-3">
                        <dt class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500">Algorithm</dt>
                        <dd class="font-mono text-slate-100">{activeSession.trustPrompt.algorithm}</dd>
                      </div>
                      <div class="rounded-2xl border border-white/8 bg-black/25 p-4">
                        <dt class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500">Fingerprint</dt>
                        <dd class="mt-3 break-all rounded-xl border border-amber-300/10 bg-amber-300/[0.06] px-3 py-2.5 font-mono text-sm leading-6 text-amber-100">
                          {activeSession.trustPrompt.fingerprint}
                        </dd>
                      </div>
                    </dl>
                    <p class="mt-4 text-xs leading-5 text-slate-500">
                      Only continue if this fingerprint matches the server you expect. It will be saved locally in Tauri's trust JSON.
                    </p>
                  </div>

                  {#if trustError}
                    <p class="mx-6 mb-4 rounded-2xl border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-200 sm:mx-7">{trustError}</p>
                  {/if}
                  <div class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-white/[0.025] px-6 py-4 sm:px-7">
                    {#if activeSession.connectionId}
                      <Button
                        onclick={trustActiveHost}
                        disabled={trustConfirming}
                        class="order-2 gap-2 rounded-2xl bg-amber-300 px-4 text-amber-950 hover:bg-amber-200 sm:order-none"
                      >
                        {#if trustConfirming}
                          <Loader2 class="size-4 animate-spin" />
                        {/if}
                        Trust host and retry
                      </Button>
                    {:else}
                      <p class="max-w-md text-sm leading-6 text-slate-400">
                        Save this connection first to trust the host and retry automatically.
                      </p>
                    {/if}
                    <Button variant="outline" onclick={() => sessionStore.removeSession(activeSession.id)} class="rounded-2xl border-white/10 bg-white/4 px-4 text-white hover:bg-white/8">
                      Cancel
                    </Button>
                  </div>
                {:else if activeSession.trustMismatch}
                  <div class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300">
                    <AlertCircle class="size-7" />
                  </div>
                  <h2 class="mt-5 text-center text-xl font-semibold text-white">Connection failed</h2>
                  <p class="mx-auto mt-2 max-w-md text-center text-sm leading-6 text-slate-400">{activeSession.error ?? "Unknown error"}</p>
                  <div class="mt-5 rounded-2xl border border-red-300/25 bg-red-300/8 p-4 text-left">
                    <p class="text-sm font-semibold text-red-100">Saved fingerprint does not match.</p>
                    <dl class="mt-3 grid gap-2 text-xs text-slate-300">
                      <div class="space-y-1">
                        <dt class="text-slate-500">Expected</dt>
                        <dd class="break-all rounded-xl bg-black/30 px-3 py-2 font-mono">{activeSession.trustMismatch.expected_fingerprint}</dd>
                      </div>
                      <div class="space-y-1">
                        <dt class="text-slate-500">Presented</dt>
                        <dd class="break-all rounded-xl bg-black/30 px-3 py-2 font-mono text-red-100">{activeSession.trustMismatch.presented_fingerprint}</dd>
                      </div>
                    </dl>
                    <p class="mt-3 text-xs leading-5 text-slate-400">This may indicate the server changed keys or a man-in-the-middle risk. Not updating trust automatically.</p>
                  </div>
                  <Button onclick={retryActiveConnection} class="mt-6 gap-2 rounded-2xl bg-red-300 text-red-950 hover:bg-red-200">
                    Retry session
                  </Button>
                {:else}
                  <div class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300">
                    <AlertCircle class="size-7" />
                  </div>
                  <h2 class="mt-5 text-center text-xl font-semibold text-white">Connection failed</h2>
                  <p class="mx-auto mt-2 max-w-md text-center text-sm leading-6 text-slate-400">{activeSession.error ?? "Unknown error"}</p>
                  <Button onclick={retryActiveConnection} class="mt-6 gap-2 rounded-2xl bg-red-300 text-red-950 hover:bg-red-200">
                    Retry session
                  </Button>
                {/if}
              </div>
            </div>
          {:else}
            <div class="relative flex min-h-0 flex-1 flex-col overflow-hidden p-3">
              {#if activeSession.type === "ssh" && activeSession.status === "connected"}
                <PortForwardPanel
                  session={activeSession}
                  forwards={activeSessionPortForwards}
                  onStart={sessionStore.startLocalPortForward}
                  onStop={sessionStore.stopLocalPortForward}
                />
              {/if}
              <div class="terminal-frame relative min-h-0 flex-1 overflow-hidden rounded-[1.35rem] border border-white/10 bg-black/50 shadow-2xl shadow-black/45">
              {#each mountedTerminalSessions as session (session.id)}
                <div class:hidden={session.id !== sessionStore.activeSessionId} class="absolute inset-0 min-h-0 overflow-hidden">
                  <TerminalView
                    sessionId={session.id}
                    sessionType={session.type}
                    active={session.id === sessionStore.activeSessionId}
                    config={terminalConfig}
                    onClose={() => sessionStore.updateSession(session.id, { status: "disconnected" })}
                  />
                </div>
              {/each}
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>

    <SettingsModal
      open={showSettings}
      config={terminalConfig}
      onSave={handleSettingsSave}
      onClose={() => (showSettings = false)}
    />

    {#if showConnectionForm}
      <ConnectionForm
        connection={editingConnection}
        keys={keys}
        error={connectionFormError}
        isSaving={connectionSaving}
        onSave={handleSaveConnection}
        onCancel={closeConnectionForm}
      />
    {/if}
  </div>
{/if}
