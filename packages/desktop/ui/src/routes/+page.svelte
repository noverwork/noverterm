<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { AlertCircle, Loader2 } from "@lucide/svelte";

  import AuthShell from "$lib/components/auth-shell.svelte";
  import ConnectionForm from "$lib/components/connection-form.svelte";
  import ConnectionsView from "$lib/components/connections-view.svelte";
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

  const terminalConfig = $derived(bootstrapStore.getTerminalConfig());
  const connections = $derived(bootstrapStore.getConnections());
  const recentConnectionIds = $derived(bootstrapStore.getRecentConnectionIds());
  const keys = $derived(bootstrapStore.getKeys());

  async function openInitialLocalTerminal() {
    if (sessionStore.sessions.size > 0) return;
    try {
      await sessionStore.connectLocal("Local Terminal");
    } catch {
      // Ignore initial local-terminal failures and keep the app usable.
    }
  }

  onMount(async () => {
    await loadAppSettings();
    await sessionStore.init();
    await bootstrapStore.init();

    if (bootstrapStore.isAuthenticated) {
      await openInitialLocalTerminal();
    }
  });

  onDestroy(() => {
    sessionStore.cleanup();
  });

  async function handleLogin(email: string, password: string) {
    await bootstrapStore.login(email, password);
    if (bootstrapStore.isAuthenticated) {
      await openInitialLocalTerminal();
    }
  }

  async function handleSignup(email: string, password: string) {
    await bootstrapStore.register(email, password);
    if (bootstrapStore.isAuthenticated) {
      await openInitialLocalTerminal();
    }
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

  function switchView(view: "terminal" | "connections" | "keys") {
    currentView = view;
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
            keys={bootstrapStore.getKeys()}
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
          {:else if activeSession.status === "error"}
            <div class="flex h-full flex-col items-center justify-center p-8 text-center">
              <div class="max-w-lg rounded-[2rem] border border-red-300/20 bg-red-400/8 p-8 shadow-2xl shadow-black/30">
                <div class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300">
                  <AlertCircle class="size-7" />
                </div>
                <h2 class="mt-5 text-xl font-semibold text-white">Connection failed</h2>
                <p class="mx-auto mt-2 max-w-md text-sm leading-6 text-slate-400">{activeSession.error ?? "Unknown error"}</p>
                <Button onclick={retryActiveConnection} class="mt-6 gap-2 rounded-2xl bg-red-300 text-red-950 hover:bg-red-200">
                  Retry session
                </Button>
              </div>
            </div>
          {:else}
            <div class="relative min-h-0 flex-1 overflow-hidden p-3">
              <div class="terminal-frame relative h-full overflow-hidden rounded-[1.35rem] border border-white/10 bg-black/50 shadow-2xl shadow-black/45">
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
        keys={bootstrapStore.getKeys()}
        error={connectionFormError}
        isSaving={connectionSaving}
        onSave={handleSaveConnection}
        onCancel={closeConnectionForm}
      />
    {/if}
  </div>
{/if}
