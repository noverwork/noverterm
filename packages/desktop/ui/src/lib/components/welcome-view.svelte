<script lang="ts">
  import { Terminal, Loader2, AlertCircle, Zap, Keyboard } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { Session, SessionStatus } from "$lib/stores/session.svelte.js";
  import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";

  let {
    connections,
    sessions,
    activeSessionId,
    onSelectConnection,
    onQuickConnect,
    onLocalTerminal,
    quickConnectState,
  }: {
    connections: ConnectionConfig[];
    sessions: Map<string, { id: string; name: string; status: SessionStatus }>;
    activeSessionId: string | null;
      onSelectConnection: (conn: ConnectionConfig) => void;
      onQuickConnect: (host: string, port: number, username: string, password: string, privateKey: string) => void;
      onLocalTerminal: () => void;
      quickConnectState: {
        host: string;
        port: number;
        username: string;
        password: string;
        privateKey: string;
        connecting: boolean;
        submitted: boolean;
        touched: Record<string, boolean>;
      };
  } = $props();

  const sortedConnections = $derived(
    [...connections].sort((a, b) => a.name.localeCompare(b.name))
  );

  const qcErrors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (!quickConnectState.host.trim()) errs.host = "Host is required";
    if (quickConnectState.port < 1 || quickConnectState.port > 65535) errs.port = "Port must be 1-65535";
    if (!quickConnectState.username.trim()) errs.username = "Username is required";
    if (!quickConnectState.password && !quickConnectState.privateKey.trim()) errs.auth = "Password or private key is required";
    return errs;
  });

  const qcIsValid = $derived(Object.keys(qcErrors).length === 0);

  function markQcTouched(field: string) {
    quickConnectState.touched[field] = true;
  }

  function showQcError(field: string) {
    return (quickConnectState.submitted || quickConnectState.touched[field]) && qcErrors[field];
  }

  function handleQuickConnectSubmit(e: Event) {
    e.preventDefault();
    quickConnectState.submitted = true;
    if (!qcIsValid || quickConnectState.connecting) return;
    onQuickConnect(
      quickConnectState.host.trim(),
      quickConnectState.port,
      quickConnectState.username.trim(),
      quickConnectState.password,
      quickConnectState.privateKey.trim()
    );
  }

  function connectionStatus(conn: ConnectionConfig): string {
    const session = Array.from(sessions.values()).find(
      (s) => s.name === `${conn.username}@${conn.host}:${conn.port}`
    );
    if (!session) return "bg-muted-foreground/50";
    if (session.status === "connected") return "bg-green-500";
    if (session.status === "connecting") return "bg-yellow-500 animate-pulse";
    if (session.status === "error") return "bg-red-500";
    return "bg-muted-foreground/50";
  }
</script>

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
    <form class="space-y-3" onsubmit={handleQuickConnectSubmit}>
      <div class="grid grid-cols-3 gap-2">
        <div class="col-span-2 space-y-1">
          <Input
            id="qc-host"
            value={quickConnectState.host}
            oninput={(e) => quickConnectState.host = (e.target as HTMLInputElement).value}
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
            value={quickConnectState.port}
            oninput={(e) => quickConnectState.port = parseInt((e.target as HTMLInputElement).value) || 22}
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
          value={quickConnectState.username}
          oninput={(e) => quickConnectState.username = (e.target as HTMLInputElement).value}
          onblur={() => markQcTouched("username")}
          placeholder="Username"
          class={showQcError('username') ? 'border-destructive' : ''}
        />
        {#if showQcError("username")}
          <p class="text-xs text-destructive">{showQcError("username")}</p>
        {/if}
      </div>

      <div class="space-y-1">
        <Input
            id="qc-password"
            type="password"
            value={quickConnectState.password}
            oninput={(e) => quickConnectState.password = (e.target as HTMLInputElement).value}
            onblur={() => markQcTouched("password")}
            placeholder="Password (optional if private key is provided)"
          />
      </div>

      <div class="space-y-1">
        <Input
          id="qc-private-key"
          value={quickConnectState.privateKey}
          oninput={(e) => quickConnectState.privateKey = (e.target as HTMLInputElement).value}
          onblur={() => markQcTouched("privateKey")}
          placeholder="Paste SSH private key (optional if password provided)"
          class={showQcError('auth') ? 'border-destructive' : ''}
        />
        {#if showQcError("auth")}
          <p class="text-xs text-destructive">{showQcError("auth")}</p>
        {/if}
      </div>

      <Button
        type="submit"
        class="w-full gap-2"
        disabled={quickConnectState.connecting}
      >
        {#if quickConnectState.connecting}
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
            onclick={() => onSelectConnection(conn)}
          >
            <div class="flex flex-col min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium truncate">{conn.name}</span>
                <div class="size-2 rounded-full shrink-0 {connectionStatus(conn)}"></div>
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
        No saved connections. Add hosts from the backend or use Quick Connect above.
      </p>
    {/if}
  </div>

  <!-- Local Terminal -->
  <div class="mx-auto w-full max-w-lg px-6 pb-6">
    <Button variant="outline" class="w-full gap-2" onclick={onLocalTerminal}>
      <Terminal class="size-4" />
      Open Local Terminal
    </Button>
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
