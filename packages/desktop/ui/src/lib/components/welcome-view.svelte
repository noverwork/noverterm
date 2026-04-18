<script lang="ts">
  import {
    ArrowRight,
    Keyboard,
    Loader2,
    Plus,
    Server,
    Terminal,
    Zap,
  } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { SessionStatus } from "$lib/stores/session.svelte.js";
  import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";
  import { findConnectionSession } from "$lib/view-models/auth-and-sessions.js";

  let {
    connections,
    sessions,
    activeSessionId,
    onSelectConnection,
    onQuickConnect,
    onLocalTerminal,
    onOpenConnectionManager,
    quickConnectState,
  }: {
    connections: ConnectionConfig[];
    sessions: Map<string, { id: string; name: string; status: SessionStatus; connectionId?: string | null }>;
    activeSessionId: string | null;
    onSelectConnection: (conn: ConnectionConfig) => void;
    onQuickConnect: (host: string, port: number, username: string, password: string, privateKey: string) => void;
    onLocalTerminal: () => void;
    onOpenConnectionManager: () => void;
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

  const sortedConnections = $derived([...connections].sort((a, b) => a.name.localeCompare(b.name)));
  const connectedConnections = $derived(
    sortedConnections.filter((conn) =>
      findConnectionSession(sessions.values(), conn)?.status === "connected",
    ),
  );
  const suggestedConnections = $derived(
    sortedConnections.filter((conn) => !connectedConnections.some((connected) => connected.id === conn.id)).slice(0, 4),
  );

  const qcErrors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (!quickConnectState.host.trim()) errs.host = "Host is required";
    if (quickConnectState.port < 1 || quickConnectState.port > 65535) errs.port = "Port must be 1-65535";
    if (!quickConnectState.username.trim()) errs.username = "Username is required";
    if (!quickConnectState.password && !quickConnectState.privateKey.trim()) errs.auth = "Password or private key is required";
    return errs;
  });

  function markQcTouched(field: string) {
    quickConnectState.touched[field] = true;
  }

  function showQcError(field: string) {
    return (quickConnectState.submitted || quickConnectState.touched[field]) && qcErrors[field];
  }

  function handleQuickConnectSubmit(event: Event) {
    event.preventDefault();
    quickConnectState.submitted = true;
    if (Object.keys(qcErrors).length > 0 || quickConnectState.connecting) return;

    onQuickConnect(
      quickConnectState.host.trim(),
      quickConnectState.port,
      quickConnectState.username.trim(),
      quickConnectState.password,
      quickConnectState.privateKey.trim(),
    );
  }

  function connectionMeta(conn: ConnectionConfig) {
    const session = findConnectionSession(sessions.values(), conn);
    return {
      isActive: session?.id === activeSessionId,
      label: session?.status === "connected" ? "Live" : conn.authMode === "publickey_password" ? "Key + password" : conn.authMode === "publickey" ? "SSH key" : "Password",
    };
  }
</script>

<div class="workspace-surface flex h-full flex-col overflow-y-auto text-white">
  <div class="mx-auto flex w-full max-w-7xl flex-1 flex-col gap-6 px-5 py-6 lg:px-8">
    <div class="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
      <div class="panel-glass rounded-[1.75rem] p-6 lg:p-8">
        <div class="hero-chip w-fit">
          <Terminal class="size-3.5" />
          Command center
        </div>

        <div class="mt-5 max-w-2xl space-y-4">
          <h1 class="text-3xl font-semibold tracking-tight text-white lg:text-4xl">
            Launch remote or local sessions without leaving your workspace.
          </h1>
          <p class="text-sm leading-7 text-slate-300 lg:text-base">
            Start from a saved host, open a local terminal, or spin up a one-off connection. Noverterm
            keeps credentials managed while your desktop runtime handles the actual terminal session.
          </p>
        </div>

        <div class="mt-6 grid gap-4 md:grid-cols-3">
          <button class="metric-card cursor-pointer text-left transition-colors hover:bg-white/8" onclick={() => suggestedConnections[0] && onSelectConnection(suggestedConnections[0])}>
            <div class="flex items-center justify-between">
              <div class="flex size-11 items-center justify-center rounded-2xl bg-primary/15 text-primary">
                <Server class="size-5" />
              </div>
              <ArrowRight class="size-4 text-slate-500" />
            </div>
            <h2 class="mt-5 text-base font-semibold text-white">Open a saved host</h2>
            <p class="mt-2 text-sm leading-6 text-slate-400">Jump back into your most common SSH connections with a single click.</p>
          </button>

          <button class="metric-card cursor-pointer text-left transition-colors hover:bg-white/8" onclick={onOpenConnectionManager}>
            <div class="flex items-center justify-between">
              <div class="flex size-11 items-center justify-center rounded-2xl bg-primary/15 text-primary">
                <Plus class="size-5" />
              </div>
              <ArrowRight class="size-4 text-slate-500" />
            </div>
            <h2 class="mt-5 text-base font-semibold text-white">Create a connection</h2>
            <p class="mt-2 text-sm leading-6 text-slate-400">Add a new host with password, SSH key, or hybrid auth and keep it ready for later.</p>
          </button>

          <button class="metric-card cursor-pointer text-left transition-colors hover:bg-white/8" onclick={onLocalTerminal}>
            <div class="flex items-center justify-between">
              <div class="flex size-11 items-center justify-center rounded-2xl bg-primary/15 text-primary">
                <Terminal class="size-5" />
              </div>
              <ArrowRight class="size-4 text-slate-500" />
            </div>
            <h2 class="mt-5 text-base font-semibold text-white">Open local terminal</h2>
            <p class="mt-2 text-sm leading-6 text-slate-400">Use the same workspace for local debugging, scripts, and quick command checks.</p>
          </button>
        </div>
      </div>

      <div class="panel-glass rounded-[1.75rem] p-6">
        <div class="flex items-center gap-2">
          <Zap class="size-4 text-primary" />
          <p class="section-title text-slate-400">Quick connect</p>
        </div>

        <form class="mt-4 space-y-4" onsubmit={handleQuickConnectSubmit}>
          <div class="grid gap-3 sm:grid-cols-[1.8fr_0.8fr]">
            <div class="space-y-2">
              <label for="qc-host" class="text-sm font-medium text-slate-100">Host</label>
              <Input id="qc-host" value={quickConnectState.host} oninput={(e) => quickConnectState.host = (e.target as HTMLInputElement).value} onblur={() => markQcTouched("host")} placeholder="prod.example.com" class={showQcError("host") ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} />
              {#if showQcError("host")}
                <p class="text-xs text-destructive" role="alert">{showQcError("host")}</p>
              {/if}
            </div>
            <div class="space-y-2">
              <label for="qc-port" class="text-sm font-medium text-slate-100">Port</label>
              <Input id="qc-port" type="number" value={quickConnectState.port} oninput={(e) => quickConnectState.port = parseInt((e.target as HTMLInputElement).value) || 22} onblur={() => markQcTouched("port")} placeholder="22" class={showQcError("port") ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} />
              {#if showQcError("port")}
                <p class="text-xs text-destructive" role="alert">{showQcError("port")}</p>
              {/if}
            </div>
          </div>

          <div class="space-y-2">
            <label for="qc-username" class="text-sm font-medium text-slate-100">Username</label>
            <Input id="qc-username" value={quickConnectState.username} oninput={(e) => quickConnectState.username = (e.target as HTMLInputElement).value} onblur={() => markQcTouched("username")} placeholder="deploy" class={showQcError("username") ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} />
            {#if showQcError("username")}
              <p class="text-xs text-destructive" role="alert">{showQcError("username")}</p>
            {/if}
          </div>

          <div class="grid gap-3">
            <div class="space-y-2">
              <label for="qc-password" class="text-sm font-medium text-slate-100">Password</label>
              <Input id="qc-password" type="password" value={quickConnectState.password} oninput={(e) => quickConnectState.password = (e.target as HTMLInputElement).value} onblur={() => markQcTouched("password")} placeholder="Optional when using a private key" class="border-white/10 bg-white/5 text-white placeholder:text-slate-500" />
            </div>
            <div class="space-y-2">
              <label for="qc-private-key" class="text-sm font-medium text-slate-100">Private key</label>
              <textarea
                id="qc-private-key"
                rows="4"
                value={quickConnectState.privateKey}
                oninput={(e) => quickConnectState.privateKey = (e.target as HTMLTextAreaElement).value}
                onblur={() => markQcTouched("privateKey")}
                placeholder="Paste a private key for one-off access"
                class={showQcError("auth") ? "flex w-full rounded-xl border border-destructive bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring" : "flex w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"}
              ></textarea>
              {#if showQcError("auth")}
                <p class="text-xs text-destructive" role="alert">{showQcError("auth")}</p>
              {/if}
            </div>
          </div>

          <Button type="submit" class="w-full gap-2" disabled={quickConnectState.connecting}>
            {#if quickConnectState.connecting}
              <Loader2 class="size-4 animate-spin" />
              Establishing session…
            {:else}
              <Zap class="size-4" />
              Connect now
            {/if}
          </Button>
        </form>
      </div>
    </div>

    <div class="grid gap-6 xl:grid-cols-[1.15fr_0.85fr]">
      <div class="panel-glass rounded-[1.75rem] p-6">
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="section-title text-slate-400">Saved hosts</p>
            <h2 class="mt-2 text-xl font-semibold text-white">Recommended next actions</h2>
          </div>
          <Button variant="outline" class="border-white/10 bg-white/4 text-white hover:bg-white/8" onclick={onOpenConnectionManager}>
            Manage connections
          </Button>
        </div>

        <div class="mt-5 grid gap-3">
          {#if connectedConnections.length > 0}
            <div class="space-y-3">
              <p class="section-title text-emerald-300/80">Connected now</p>
              {#each connectedConnections.slice(0, 3) as conn (conn.id)}
                {@const meta = connectionMeta(conn)}
                <button class="flex w-full cursor-pointer items-center justify-between rounded-2xl border border-emerald-400/15 bg-emerald-400/8 px-4 py-3 text-left transition-colors hover:bg-emerald-400/10" onclick={() => onSelectConnection(conn)}>
                  <div>
                    <p class="font-medium text-white">{conn.name}</p>
                    <p class="mt-1 text-sm text-slate-300">{conn.username}@{conn.host}:{conn.port}</p>
                  </div>
                  <div class="rounded-full border border-emerald-300/30 bg-emerald-300/10 px-2.5 py-1 text-xs font-medium text-emerald-100">{meta.isActive ? "Active" : "Live"}</div>
                </button>
              {/each}
            </div>
          {/if}

          <div class="space-y-3">
            <p class="section-title text-slate-400">Saved connections</p>
            {#if suggestedConnections.length > 0}
              {#each suggestedConnections as conn (conn.id)}
                {@const meta = connectionMeta(conn)}
                <button class="flex w-full cursor-pointer items-center justify-between rounded-2xl border border-white/8 bg-white/[0.04] px-4 py-3 text-left transition-colors hover:bg-white/[0.07]" onclick={() => onSelectConnection(conn)}>
                  <div>
                    <p class="font-medium text-white">{conn.name}</p>
                    <p class="mt-1 text-sm text-slate-400">{conn.username}@{conn.host}:{conn.port}</p>
                  </div>
                  <div class="rounded-full border border-white/10 bg-white/5 px-2.5 py-1 text-xs font-medium text-slate-300">{meta.label}</div>
                </button>
              {/each}
            {:else}
              <div class="rounded-2xl border border-dashed border-white/10 bg-white/[0.03] p-5 text-sm text-slate-400">
                You don’t have any saved SSH hosts yet. Create a connection to make this workspace feel
                like a real command center.
              </div>
            {/if}
          </div>
        </div>
      </div>

      <div class="space-y-6">
        <div class="panel-glass rounded-[1.75rem] p-6">
          <div class="flex items-center gap-2">
            <Keyboard class="size-4 text-primary" />
            <p class="section-title text-slate-400">Speed up your flow</p>
          </div>
          <div class="mt-4 grid gap-3">
            <div class="metric-card flex items-center justify-between">
              <span class="text-sm text-slate-300">Open connection manager</span>
              <kbd class="rounded-lg border border-white/10 bg-white/6 px-2 py-1 text-xs text-slate-200">⌘/Ctrl + T</kbd>
            </div>
            <div class="metric-card flex items-center justify-between">
              <span class="text-sm text-slate-300">Close active tab</span>
              <kbd class="rounded-lg border border-white/10 bg-white/6 px-2 py-1 text-xs text-slate-200">⌘/Ctrl + W</kbd>
            </div>
            <div class="metric-card flex items-center justify-between">
              <span class="text-sm text-slate-300">Open settings</span>
              <kbd class="rounded-lg border border-white/10 bg-white/6 px-2 py-1 text-xs text-slate-200">⌘/Ctrl + ,</kbd>
            </div>
          </div>
        </div>

        <div class="panel-glass rounded-[1.75rem] p-6">
          <p class="section-title text-slate-400">Workspace posture</p>
          <div class="mt-4 grid gap-3 sm:grid-cols-2">
            <div class="metric-card">
              <p class="text-3xl font-semibold text-white">{connections.length}</p>
              <p class="mt-2 text-sm text-slate-400">Saved connections available</p>
            </div>
            <div class="metric-card">
              <p class="text-3xl font-semibold text-white">{Array.from(sessions.values()).filter((session) => session.status === "connected").length}</p>
              <p class="mt-2 text-sm text-slate-400">Live sessions in this workspace</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
