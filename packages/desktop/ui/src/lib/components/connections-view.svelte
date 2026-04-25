<script lang="ts">
  import { Pencil, Plus, Trash2 } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";
  import type { SessionStatus } from "$lib/stores/session.svelte.js";
  import { findConnectionSession } from "$lib/view-models/auth-and-sessions.js";

  interface SessionLookup {
    id: string;
    name: string;
    status: SessionStatus;
    connectionId?: string | null;
  }

  interface Props {
    connections: ConnectionConfig[];
    sessions: Map<string, SessionLookup>;
    activeSessionId: string | null;
    onSelect: (conn: ConnectionConfig) => void;
    onEdit: (conn: ConnectionConfig) => void;
    onNew: () => void;
    onDelete: (conn: ConnectionConfig) => Promise<void>;
  }

  let {
    connections,
    sessions,
    activeSessionId,
    onSelect,
    onEdit,
    onNew,
    onDelete,
  }: Props = $props();

  let error = $state<string | null>(null);
  let deletingConnectionId = $state<string | null>(null);

  let sortedConnections = $derived(
    [...connections].sort((a, b) => a.name.localeCompare(b.name)),
  );

  function getConnectionSession(connection: ConnectionConfig) {
    return findConnectionSession(sessions.values(), connection);
  }

  function getStatusMeta(status?: SessionStatus) {
    switch (status) {
      case "connected":
        return {
          label: "Connected",
          badgeClass: "border-emerald-400/30 bg-emerald-400/15 text-emerald-300",
          dotClass: "bg-emerald-400",
        };
      case "connecting":
        return {
          label: "Connecting",
          badgeClass: "border-amber-400/30 bg-amber-400/15 text-amber-300",
          dotClass: "bg-amber-400 animate-pulse",
        };
      case "error":
        return {
          label: "Error",
          badgeClass: "border-red-400/30 bg-red-400/15 text-red-300",
          dotClass: "bg-red-400",
        };
      default:
        return {
          label: "Saved",
          badgeClass: "border-white/10 bg-white/6 text-slate-300",
          dotClass: "bg-slate-500",
        };
    }
  }

  function getAuthLabel(connection: ConnectionConfig): string {
    switch (connection.authMode) {
      case "publickey_password":
        return "Key + Password";
      case "publickey":
        return "SSH Key";
      default:
        return "Password";
    }
  }

  async function handleDelete(connection: ConnectionConfig) {
    if (!window.confirm(`Delete saved connection "${connection.name}"?`)) {
      return;
    }

    deletingConnectionId = connection.id;
    error = null;

    try {
      await onDelete(connection);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to delete connection";
    } finally {
      deletingConnectionId = null;
    }
  }
</script>

<div class="workspace-canvas flex h-full flex-col overflow-y-auto px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-full flex-col p-5 text-white sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Inventory</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">SSH Connections</h1>
        <p class="mt-2 text-sm text-slate-500">Curated hosts, credentials, and active terminal targets.</p>
      </div>

      <Button onclick={onNew} variant="default" size="sm" class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
        <Plus class="size-3.5" />
        Add connection
      </Button>
    </div>

    {#if error}
      <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    <div class="mt-6 flex-1">
      {#if sortedConnections.length === 0}
        <div class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
          No saved connections yet
        </div>
      {:else}
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each sortedConnections as connection (connection.id)}
            {@const session = getConnectionSession(connection)}
            {@const status = getStatusMeta(session?.status)}
            {@const isActive = session?.id === activeSessionId}

            <article class={isActive
              ? "group rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8 px-4 py-4 shadow-[0_16px_42px_rgb(34_211_238/0.10)]"
              : "group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]"}>
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 flex-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="truncate text-sm font-medium text-white">{connection.name}</span>
                    <span class="rounded-full border px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide {status.badgeClass}">
                      {status.label}
                    </span>
                  </div>

                  <p class="mt-2 truncate font-mono text-xs text-slate-400">{connection.username}@{connection.host}:{connection.port}</p>
                  <p class="mt-1 text-xs text-slate-500">{getAuthLabel(connection)}</p>
                </div>

                <div class="mt-1 size-2.5 shrink-0 rounded-full {status.dotClass}"></div>
              </div>

              <div class="mt-4 flex flex-wrap items-center gap-2">
                <Button variant="ghost" size="xs" class="gap-1.5 rounded-xl bg-white/[0.035] text-slate-200 hover:bg-cyan-300/10 hover:text-white" onclick={() => onSelect(connection)}>
                  Open
                </Button>
                <Button variant="ghost" size="xs" class="gap-1.5 rounded-xl text-slate-400 hover:bg-white/7 hover:text-white" onclick={() => onEdit(connection)}>
                  <Pencil class="size-3" />
                  Edit
                </Button>
                <Button
                  variant="ghost"
                  size="xs"
                  class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                  onclick={() => handleDelete(connection)}
                  disabled={deletingConnectionId === connection.id}
                >
                  <Trash2 class="size-3" />
                  Delete
                </Button>
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</div>
