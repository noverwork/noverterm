<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal, Loader2, AlertCircle, Zap, Keyboard } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import Sidebar from "$lib/components/sidebar.svelte";
  import TerminalTabs from "$lib/components/terminal-tabs.svelte";
  import ConnectionForm from "$lib/components/connection-form.svelte";
  import TerminalView from "$lib/terminal/terminal.svelte";
  import SettingsModal from "$lib/components/settings-modal.svelte";
  import { createSessionStore, type Session } from "$lib/stores/session.svelte.js";
  import {
    loadConfig,
    saveConnections,
    saveConfig,
    addConnection,
    updateConnection,
    deleteConnection,
    type ConnectionConfig,
    type TerminalConfig,
  } from "$lib/config.js";

  const store = createSessionStore();

  let sidebarCollapsed = $state(false);
  let showConnectionForm = $state(false);
  let editingConnection = $state<ConnectionConfig | null>(null);
  let connections = $state<ConnectionConfig[]>([]);
  let terminalConfig = $state<TerminalConfig>({
    theme: "dark",
    fontSize: 14,
    fontFamily: "JetBrains Mono, Fira Code, monospace",
    cursorStyle: "block",
    cursorBlink: true,
    scrollback: 5000,
  });

  // Settings modal
  let showSettings = $state(false);

  // Terminal controller for toolbar actions
  let termContainer = $state<HTMLDivElement | null>(null);
  let termController = $state<any>(null);

  // Quick connect form state
  let qcHost = $state("");
  let qcPort = $state(22);
  let qcUsername = $state("");
  let qcAuthType = $state<"password" | "key">("password");
  let qcPassword = $state("");
  let qcKeyPath = $state("");
  let qcConnecting = $state(false);
  let qcSubmitted = $state(false);
  let qcTouched = $state<Record<string, boolean>>({});

  const qcErrors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (!qcHost.trim()) errs.host = "Host is required";
    if (qcPort < 1 || qcPort > 65535) errs.port = "Port must be 1-65535";
    if (!qcUsername.trim()) errs.username = "Username is required";
    if (qcAuthType === "password" && !qcPassword) errs.password = "Password is required";
    if (qcAuthType === "key" && !qcKeyPath.trim()) errs.keyPath = "Key path is required";
    return errs;
  });

  const qcIsValid = $derived(Object.keys(qcErrors).length === 0);

  function markQcTouched(field: string) {
    qcTouched[field] = true;
  }

  function showQcError(field: string) {
    return (qcSubmitted || qcTouched[field]) && qcErrors[field];
  }

  const sortedConnections = $derived(
    [...connections].sort((a, b) => a.name.localeCompare(b.name))
  );

  async function handleQuickConnect() {
    qcSubmitted = true;
    if (!qcIsValid || qcConnecting) return;
    qcConnecting = true;
    try {
      const credential = qcAuthType === "password" ? qcPassword : qcKeyPath;
      await store.connectToHost(
        qcHost.trim(),
        qcPort,
        qcUsername.trim(),
        qcAuthType,
        credential,
        `${qcUsername.trim()}@${qcHost.trim()}:${qcPort}`,
        80,
        24
      );
      qcHost = "";
      qcPort = 22;
      qcUsername = "";
      qcPassword = "";
      qcKeyPath = "";
      qcSubmitted = false;
      qcTouched = {};
    } catch {
      // Error is handled by the session store (status becomes "error")
    } finally {
      qcConnecting = false;
    }
  }

  const activeSession = $derived(
    store.activeSessionId ? store.sessions.get(store.activeSessionId) : undefined
  );

  const activeSessions = $derived(
    Array.from(store.sessions.values()).filter(
      (s) => s.status !== "disconnected"
    ) as Session[]
  );

  onMount(async () => {
    await store.init();
    const config = await loadConfig();
    terminalConfig = config.terminal;
    connections = config.connections;
    // Apply theme from config
    if (terminalConfig.theme === "dark") {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }

    if (store.sessions.size === 0) {
      try {
        await store.connectLocal("Local Terminal");
      } catch {
        void 0;
      }
    }
  });

  onDestroy(() => {
    store.cleanup();
  });

  async function handleSelectConnection(conn: ConnectionConfig) {
    const existing = Array.from(store.sessions.values()).find(
      (s) => s.name === `${conn.username}@${conn.host}:${conn.port}` && s.status === "connected"
    );
    if (existing) {
      store.setActiveSession(existing.id);
      return;
    }

    const credential = conn.authType === "password" ? conn.password! : conn.keyPath!;
    await store.connectToHost(
      conn.host,
      conn.port,
      conn.username,
      conn.authType,
      credential,
      `${conn.username}@${conn.host}:${conn.port}`,
      80,
      24
    );
  }

  async function handleSaveConnection(data: Omit<ConnectionConfig, "id"> & { id?: string }) {
    if (data.id) {
      await updateConnection(data as ConnectionConfig);
    } else {
      const newConn = await addConnection(data);
      connections = [...connections, newConn];
    }
    connections = (await loadConfig()).connections;
    showConnectionForm = false;
    editingConnection = null;
  }

  async function handleDeleteConnection(conn: ConnectionConfig) {
    await deleteConnection(conn.id);
    connections = (await loadConfig()).connections;
  }

  function handleSessionClose(id: string) {
    store.disconnectSession(id);
  }

  function handleSettingsSave(config: TerminalConfig) {
    terminalConfig = config;
    saveConfig({ terminal: config, connections, lastActiveSessionId: store.activeSessionId ?? undefined });
    showSettings = false;
  }

  // Global keyboard shortcuts
  function handleGlobalKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    const target = e.target as HTMLElement;
    const isInput = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT" || target.isContentEditable;

    if (mod && e.key === ",") {
      e.preventDefault();
      showSettings = true;
      return;
    }

    if (mod && e.key === "t") {
      e.preventDefault();
      editingConnection = null;
      showConnectionForm = true;
      return;
    }

    if (mod && e.key === "w") {
      e.preventDefault();
      if (store.activeSessionId) {
        handleSessionClose(store.activeSessionId);
      }
      return;
    }

    if (mod && e.key >= "1" && e.key <= "9" && !isInput) {
      e.preventDefault();
      const idx = parseInt(e.key) - 1;
      if (idx < activeSessions.length) {
        store.setActiveSession(activeSessions[idx].id);
      }
      return;
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="flex h-screen w-screen overflow-hidden bg-background">
  <Sidebar
    {connections}
    sessions={store.sessions}
    activeSessionId={store.activeSessionId}
    collapsed={sidebarCollapsed}
    onToggle={() => (sidebarCollapsed = !sidebarCollapsed)}
    onSelect={handleSelectConnection}
    onAdd={() => {
      editingConnection = null;
      showConnectionForm = true;
    }}
    onEdit={(conn) => {
      editingConnection = conn;
      showConnectionForm = true;
    }}
    onDelete={handleDeleteConnection}
    onLocalTerminal={() => store.connectLocal?.("Local Terminal")}
  />

  <div class="flex flex-col flex-1 min-w-0">
    <TerminalTabs
      sessions={activeSessions}
      activeSessionId={store.activeSessionId}
      onActivate={(id) => store.setActiveSession(id)}
      onClose={handleSessionClose}
      onNew={() => {
        editingConnection = null;
        showConnectionForm = true;
      }}
      onNewLocal={() => store.connectLocal("Local Terminal")}
    />

    <div class="relative flex flex-1 min-h-0 flex-col overflow-hidden">
      {#if !activeSession}
        <div class="flex flex-col h-full overflow-y-auto">
          <!-- Header -->
          <div class="flex flex-col items-center justify-center pt-12 pb-6 text-center">
            <div class="relative mb-4">
              <div class="absolute inset-0 blur-3xl bg-primary/20 rounded-full"></div>
              <Terminal class="relative size-12 text-primary" />
            </div>
            <h1 class="text-2xl font-bold tracking-tight">Noverterm</h1>
            <p class="text-sm text-muted-foreground mt-1">SSH Terminal</p>
          </div>

          <!-- Quick Connect -->
          <div class="mx-auto w-full max-w-lg px-6 pb-6">
            <div class="flex items-center gap-2 mb-3">
              <Zap class="size-4 text-primary" />
              <h2 class="text-sm font-semibold">Quick Connect</h2>
            </div>
            <form
              class="space-y-3"
              onsubmit={(e) => {
                e.preventDefault();
                handleQuickConnect();
              }}
            >
              <div class="grid grid-cols-3 gap-2">
                <div class="col-span-2 space-y-1">
                  <Input
                    id="qc-host"
                    bind:value={qcHost}
                    onblur={() => markQcTouched("host")}
                    placeholder="Host (e.g. 192.168.1.1)"
                    class={showQcError('host') ? 'border-destructive' : ''}
                  />
                  {#if showQcError("host")}
                    <p class="text-xs text-destructive">{showQcError("host")}</p>
                  {/if}
                </div>
                <div class="space-y-1">
                  <Input
                    id="qc-port"
                    type="number"
                    bind:value={qcPort}
                    onblur={() => markQcTouched("port")}
                    placeholder="22"
                    class={showQcError('port') ? 'border-destructive' : ''}
                  />
                  {#if showQcError("port")}
                    <p class="text-xs text-destructive">{showQcError("port")}</p>
                  {/if}
                </div>
              </div>

              <div class="space-y-1">
                <Input
                  id="qc-username"
                  bind:value={qcUsername}
                  onblur={() => markQcTouched("username")}
                  placeholder="Username"
                  class={showQcError('username') ? 'border-destructive' : ''}
                />
                {#if showQcError("username")}
                  <p class="text-xs text-destructive">{showQcError("username")}</p>
                {/if}
              </div>

              <div class="space-y-1">
                <div class="flex gap-2">
                  <Button
                    type="button"
                    variant={qcAuthType === "password" ? "default" : "outline"}
                    size="sm"
                    class="flex-1"
                    onclick={() => (qcAuthType = "password")}
                  >
                    Password
                  </Button>
                  <Button
                    type="button"
                    variant={qcAuthType === "key" ? "default" : "outline"}
                    size="sm"
                    class="flex-1"
                    onclick={() => (qcAuthType = "key")}
                  >
                    SSH Key
                  </Button>
                </div>
              </div>

              {#if qcAuthType === "password"}
                <div class="space-y-1">
                  <Input
                    id="qc-password"
                    type="password"
                    bind:value={qcPassword}
                    onblur={() => markQcTouched("password")}
                    placeholder="Password"
                    class={showQcError('password') ? 'border-destructive' : ''}
                  />
                  {#if showQcError("password")}
                    <p class="text-xs text-destructive">{showQcError("password")}</p>
                  {/if}
                </div>
              {:else}
                <div class="space-y-1">
                  <Input
                    id="qc-keypath"
                    bind:value={qcKeyPath}
                    onblur={() => markQcTouched("keyPath")}
                    placeholder="~/.ssh/id_ed25519"
                    class={showQcError('keyPath') ? 'border-destructive' : ''}
                  />
                  {#if showQcError("keyPath")}
                    <p class="text-xs text-destructive">{showQcError("keyPath")}</p>
                  {/if}
                </div>
              {/if}

              <Button
                type="submit"
                class="w-full gap-2"
                disabled={qcConnecting}
              >
                {#if qcConnecting}
                  <Loader2 class="size-4 animate-spin" />
                  Connecting...
                {:else}
                  <Zap class="size-4" />
                  Quick Connect
                {/if}
              </Button>
            </form>
          </div>

          <!-- Saved Connections -->
          <div class="mx-auto w-full max-w-lg px-6 pb-6">
            <div class="flex items-center gap-2 mb-3">
              <Terminal class="size-4 text-muted-foreground" />
              <h2 class="text-sm font-semibold">Saved Connections</h2>
            </div>
            {#if sortedConnections.length > 0}
              <div class="space-y-1">
                {#each sortedConnections as conn (conn.id)}
                  <button
                    class="w-full text-left px-3 py-2 rounded-lg transition-colors hover:bg-accent/50 cursor-pointer flex items-center gap-3 group"
                    onclick={() => handleSelectConnection(conn)}
                  >
                    <div class="flex flex-col min-w-0 flex-1">
                      <div class="flex items-center gap-2">
                        <span class="text-sm font-medium truncate">{conn.name}</span>
                        <div class="size-2 rounded-full shrink-0 {(() => {
                          const session = Array.from(store.sessions.values()).find(
                            (s) => s.name === `${conn.username}@${conn.host}:${conn.port}`
                          );
                          if (!session) return 'bg-muted-foreground/50';
                          if (session.status === 'connected') return 'bg-green-500';
                          if (session.status === 'connecting') return 'bg-yellow-500 animate-pulse';
                          if (session.status === 'error') return 'bg-red-500';
                          return 'bg-muted-foreground/50';
                        })()}"></div>
                      </div>
                      <span class="text-xs text-muted-foreground truncate">
                        {conn.username}@{conn.host}:{conn.port}
                      </span>
                    </div>
                  </button>
                {/each}
              </div>
            {:else}
              <p class="text-sm text-muted-foreground">
                No saved connections. Add one from the sidebar or use Quick Connect above.
              </p>
            {/if}
          </div>

          <!-- Keyboard Shortcuts -->
          <div class="mx-auto w-full max-w-lg px-6 pb-8 mt-auto">
            <div class="rounded-lg border border-border bg-card/50 p-3">
              <div class="flex items-center gap-2 mb-2">
                <Keyboard class="size-3.5 text-muted-foreground" />
                <span class="text-xs font-medium text-muted-foreground">Keyboard Shortcuts</span>
              </div>
              <div class="grid grid-cols-2 gap-x-4 gap-y-1.5 text-xs text-muted-foreground">
                <div class="flex items-center gap-2">
                  <kbd class="px-1.5 py-0.5 rounded bg-muted text-[10px] font-mono">⌘/Ctrl+T</kbd>
                  <span>New Connection</span>
                </div>
                <div class="flex items-center gap-2">
                  <kbd class="px-1.5 py-0.5 rounded bg-muted text-[10px] font-mono">⌘/Ctrl+W</kbd>
                  <span>Close Tab</span>
                </div>
                <div class="flex items-center gap-2">
                  <kbd class="px-1.5 py-0.5 rounded bg-muted text-[10px] font-mono">⌘/Ctrl+,</kbd>
                  <span>Settings</span>
                </div>
                <div class="flex items-center gap-2">
                  <kbd class="px-1.5 py-0.5 rounded bg-muted text-[10px] font-mono">⌘/Ctrl+1-9</kbd>
                  <span>Switch to Tab N</span>
                </div>
              </div>
            </div>
          </div>
        </div>
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
          <Button
            onclick={() => {
              const conn = connections.find(
                (c) => `${c.username}@${c.host}:${c.port}` === activeSession.name
              );
              if (conn) handleSelectConnection(conn);
            }}
            class="gap-2"
          >
            Retry
          </Button>
        </div>
      {:else}
        <div bind:this={termContainer} class="flex-1 min-h-0 overflow-hidden">
          {#key activeSession.id}
            <TerminalView
              sessionId={activeSession.id}
              sessionType={activeSession.type}
              config={terminalConfig}
              bind:controller={termController}
              onClose={() => store.updateSession(activeSession.id, { status: "disconnected" })}
            />
          {/key}
        </div>
      {/if}
    </div>
  </div>

  {#if showConnectionForm}
    <ConnectionForm
      connection={editingConnection}
      onSave={handleSaveConnection}
      onCancel={() => {
        showConnectionForm = false;
        editingConnection = null;
      }}
    />
  {/if}

  <SettingsModal
    open={showSettings}
    config={terminalConfig}
    onSave={handleSettingsSave}
    onClose={() => (showSettings = false)}
  />
</div>
