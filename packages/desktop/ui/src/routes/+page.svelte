<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { AlertCircle, Loader2, LogOut, Settings } from "@lucide/svelte";

  import ConnectionForm from "$lib/components/connection-form.svelte";
  import LoginForm from "$lib/components/login-form.svelte";
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
  import TerminalView from "$lib/terminal/terminal.svelte";

  const bootstrapStore = createBootstrapStore();
  const sessionStore = createSessionStore();

  let sidebarCollapsed = $state(false);
  let showSettings = $state(false);
  let showConnectionForm = $state(false);
  let editingConnection = $state<ConnectionConfig | null>(null);
  let connectionFormError = $state<string | null>(null);
  let connectionSaving = $state(false);

  let qcHost = $state("");
  let qcPort = $state(22);
  let qcUsername = $state("");
  let qcPassword = $state("");
  let qcPrivateKey = $state("");
  let qcConnecting = $state(false);
  let qcSubmitted = $state(false);
  let qcTouched = $state<Record<string, boolean>>({});

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

  const quickConnectState = $derived({
    host: qcHost,
    port: qcPort,
    username: qcUsername,
    password: qcPassword,
    privateKey: qcPrivateKey,
    connecting: qcConnecting,
    submitted: qcSubmitted,
    touched: qcTouched,
  });

  async function openInitialLocalTerminal() {
    if (sessionStore.sessions.size > 0) return;
    try {
      await sessionStore.connectLocal("Local Terminal");
    } catch {
      // Ignore initial local-terminal failures and keep the app usable.
    }
  }

  onMount(async () => {
    await sessionStore.init();
    await bootstrapStore.init();

    if (bootstrapStore.isAuthenticated) {
      bootstrapStore.applyTheme(bootstrapStore.getTerminalConfig().theme);
      await openInitialLocalTerminal();
    }
  });

  onDestroy(() => {
    sessionStore.cleanup();
  });

  async function handleLogin(username: string, password: string) {
    await bootstrapStore.login(username, password);
    if (bootstrapStore.isAuthenticated) {
      bootstrapStore.applyTheme(bootstrapStore.getTerminalConfig().theme);
      await openInitialLocalTerminal();
    }
  }

  async function handleLogout() {
    await bootstrapStore.logout();
  }

  async function handleSelectConnection(connection: ConnectionConfig) {
    const existing = Array.from(sessionStore.sessions.values()).find(
      (session) => session.connectionId === connection.id && session.status !== "disconnected",
    );
    if (existing) {
      sessionStore.setActiveSession(existing.id);
      return;
    }

    await sessionStore.connectSavedConnection(connection, 80, 24);
  }

  async function handleQuickConnect(
    host: string,
    port: number,
    username: string,
    password: string,
    privateKey: string,
  ) {
    qcConnecting = true;
    try {
      await sessionStore.connectDirect(
        {
          host,
          port,
          username,
          password: password || undefined,
          privateKey: privateKey || undefined,
        },
        80,
        24,
      );
      qcHost = "";
      qcPort = 22;
      qcUsername = "";
      qcPassword = "";
      qcPrivateKey = "";
      qcSubmitted = false;
      qcTouched = {};
    } finally {
      qcConnecting = false;
    }
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
    const confirmed = window.confirm(`Delete saved connection \"${connection.name}\"?`);
    if (!confirmed) return;

    await bootstrapStore.deleteConnection(connection);
    sessionStore.disconnectConnectionSessions(connection.id);
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

    openNewConnectionForm();
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
      openNewConnectionForm();
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
        sessionStore.setActiveSession(activeSessions[index].id);
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
  <LoginForm onLogin={handleLogin} isLoading={bootstrapStore.isLoading} error={bootstrapStore.error} />
{:else if bootstrapStore.isError}
  <div class="flex min-h-screen flex-col items-center justify-center bg-background px-4">
    <div class="flex flex-col items-center text-center max-w-md">
      <AlertCircle class="size-12 text-destructive mb-4" />
      <h1 class="text-xl font-semibold mb-2">Connection Failed</h1>
      <p class="text-sm text-muted-foreground mb-6">
        {bootstrapStore.error ?? "Unable to connect to the backend. Remote features are unavailable."}
      </p>
      <div class="flex gap-3">
        <Button onclick={() => sessionStore.connectLocal("Local Terminal")} class="gap-2">
          Open Local Terminal
        </Button>
        <Button variant="outline" onclick={() => bootstrapStore.init()} class="gap-2">
          Retry
        </Button>
      </div>
    </div>
  </div>
{:else}
  <div class="flex h-screen w-screen overflow-hidden bg-background">
    <Sidebar
      {connections}
      sessions={sessionStore.sessions}
      activeSessionId={sessionStore.activeSessionId}
      collapsed={sidebarCollapsed}
      onToggle={() => (sidebarCollapsed = !sidebarCollapsed)}
      onSelect={handleSelectConnection}
      onAdd={openNewConnectionForm}
      onEdit={openEditConnectionForm}
      onDelete={handleDeleteConnection}
      onLocalTerminal={() => sessionStore.connectLocal("Local Terminal")}
    />

    <div class="flex flex-col flex-1 min-w-0">
      <div class="flex items-center justify-between px-3 py-1.5 border-b border-border bg-background">
        <TerminalTabs
          sessions={activeSessions}
          activeSessionId={sessionStore.activeSessionId}
          onActivate={(id) => sessionStore.setActiveSession(id)}
          onClose={handleSessionClose}
          onNew={openNewConnectionForm}
          onNewLocal={() => sessionStore.connectLocal("Local Terminal")}
        />
        <div class="flex items-center gap-2 shrink-0">
          <span class="text-xs text-muted-foreground">{bootstrapStore.authStatus?.username ?? ""}</span>
          <Button variant="ghost" size="icon-xs" onclick={() => (showSettings = true)}>
            <Settings class="size-3.5" />
          </Button>
          <Button variant="ghost" size="icon-xs" onclick={handleLogout}>
            <LogOut class="size-3.5" />
          </Button>
        </div>
      </div>

      <div class="relative flex flex-1 min-h-0 flex-col overflow-hidden">
        {#if !activeSession}
          <WelcomeView
            {connections}
            sessions={sessionStore.sessions}
            activeSessionId={sessionStore.activeSessionId}
            onSelectConnection={handleSelectConnection}
            onQuickConnect={handleQuickConnect}
            onLocalTerminal={() => sessionStore.connectLocal("Local Terminal")}
            {quickConnectState}
          />
        {:else if activeSession.status === "connecting"}
          <div class="flex flex-col items-center justify-center h-full">
            <Loader2 class="size-8 animate-spin text-primary mb-4" />
            <p class="text-muted-foreground">Connecting to {activeSession.name}...</p>
          </div>
        {:else if activeSession.status === "error"}
          <div class="flex flex-col items-center justify-center h-full text-center p-8">
            <AlertCircle class="size-12 text-destructive mb-4" />
            <h2 class="text-xl font-semibold mb-2">Connection Failed</h2>
            <p class="text-muted-foreground mb-4 max-w-md">{activeSession.error ?? "Unknown error"}</p>
            <Button onclick={retryActiveConnection} class="gap-2">
              Retry
            </Button>
          </div>
        {:else}
          <div class="relative flex-1 min-h-0 overflow-hidden">
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
        error={connectionFormError}
        isSaving={connectionSaving}
        onSave={handleSaveConnection}
        onCancel={closeConnectionForm}
      />
    {/if}
  </div>
{/if}
