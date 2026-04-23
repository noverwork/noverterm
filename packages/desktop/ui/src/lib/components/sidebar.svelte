<script lang="ts">
  import { ChevronLeft, ChevronRight, KeyRound, Pencil, Plus, Search, Terminal, Trash2 } from "@lucide/svelte";

import { Button } from "$lib/components/ui/button/index.js";
import { Input } from "$lib/components/ui/input/index.js";
import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";
import type { SessionStatus } from "$lib/stores/session.svelte.js";
import { findConnectionSession } from "$lib/view-models/auth-and-sessions.js";

  let {
    connections,
    sessions,
    activeSessionId,
    collapsed,
    onToggle,
    onSelect,
    onAdd,
    onEdit,
    onDelete,
    onLocalTerminal,
    onManageKeys,
    keyCount = 0,
  }: {
    connections: ConnectionConfig[];
    sessions: Map<string, { id: string; name: string; status: SessionStatus; connectionId?: string | null }>;
    activeSessionId: string | null;
    collapsed: boolean;
    onToggle: () => void;
    onSelect: (conn: ConnectionConfig) => void;
    onAdd: () => void;
    onEdit: (conn: ConnectionConfig) => void;
    onDelete: (conn: ConnectionConfig) => void;
    onLocalTerminal?: () => void;
    onManageKeys?: () => void;
    keyCount?: number;
  } = $props();

  let searchQuery = $state("");

  const filteredConnections = $derived(
    connections
      .filter(
        (connection) =>
          connection.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          connection.host.toLowerCase().includes(searchQuery.toLowerCase()) ||
          connection.username.toLowerCase().includes(searchQuery.toLowerCase()),
      )
      .sort((a, b) => a.name.localeCompare(b.name)),
  );

  function getConnectionSession(conn: ConnectionConfig) {
    return findConnectionSession(sessions.values(), conn);
  }

  function statusBadge(status?: SessionStatus) {
    switch (status) {
      case "connected":
        return { tone: "bg-emerald-400", label: "Live" };
      case "connecting":
        return { tone: "bg-amber-400 animate-pulse", label: "Starting" };
      case "error":
        return { tone: "bg-red-400", label: "Error" };
      default:
        return { tone: "bg-slate-500", label: "Saved" };
    }
  }

  const connectedConnections = $derived(filteredConnections.filter((conn) => {
    const session = getConnectionSession(conn);
    return session?.status === "connected" || session?.status === "connecting";
  }));

  const otherConnections = $derived(filteredConnections.filter((conn) => !connectedConnections.some((connected) => connected.id === conn.id)));
</script>

<div class="sidebar flex flex-col border-r border-border/70 bg-sidebar/95 backdrop-blur-md transition-all duration-200 {collapsed ? 'w-16' : 'w-80'}">
  <div class="flex items-center justify-between border-b border-border/70 px-4 py-4">
    {#if !collapsed}
      <div>
        <p class="section-title">Workspace</p>
        <div class="mt-2 flex items-center gap-2">
          <Terminal class="size-4 text-primary" />
          <span class="text-sm font-semibold">Connections</span>
        </div>
      </div>
    {:else}
      <div class="mx-auto flex size-10 items-center justify-center rounded-2xl bg-primary/10 text-primary">
        <Terminal class="size-4" />
      </div>
    {/if}

    <Button variant="ghost" size="icon-xs" onclick={onToggle} class="text-muted-foreground hover:text-foreground">
      {#if collapsed}
        <ChevronRight class="size-3.5" />
      {:else}
        <ChevronLeft class="size-3.5" />
      {/if}
    </Button>
  </div>

  {#if !collapsed}
    <div class="space-y-4 px-4 py-4">
      <div class="relative">
        <Search class="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
        <Input bind:value={searchQuery} placeholder="Search hosts or usernames" class="h-10 rounded-xl border-border/70 bg-background/60 pl-9 text-sm" />
      </div>

      <div class="grid gap-2">
        <Button onclick={onAdd} variant="default" size="sm" class="w-full justify-start gap-2 rounded-xl">
          <Plus class="size-3.5" />
          Create connection
        </Button>
        {#if onLocalTerminal}
          <Button onclick={onLocalTerminal} variant="outline" size="sm" class="w-full justify-start gap-2 rounded-xl">
            <Terminal class="size-3.5" />
            Open local terminal
          </Button>
        {/if}
        {#if onManageKeys}
          <Button onclick={onManageKeys} variant="outline" size="sm" class="w-full justify-start gap-2 rounded-xl">
            <KeyRound class="size-3.5" />
            Manage keys
            {#if keyCount > 0}
              <span class="ml-auto rounded-full bg-white/10 px-1.5 py-0.5 text-[10px]">{keyCount}</span>
            {/if}
          </Button>
        {/if}
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-3 pb-3">
      {#if connectedConnections.length > 0}
        <div class="mb-5">
          <p class="section-title px-2 text-emerald-300/80">Active</p>
          <div class="mt-2 space-y-1">
            {#each connectedConnections as conn (conn.id)}
              {@const session = getConnectionSession(conn)}
              {@const status = statusBadge(session?.status)}
              <button
                class={session?.id === activeSessionId
                  ? "group flex w-full cursor-pointer items-start gap-3 rounded-2xl border border-primary/30 bg-primary/10 px-3 py-3 text-left transition-colors"
                  : "group flex w-full cursor-pointer items-start gap-3 rounded-2xl border border-transparent px-3 py-3 text-left transition-colors hover:border-white/8 hover:bg-white/[0.04]"}
                onclick={() => onSelect(conn)}
              >
                <div class="mt-1 size-2.5 shrink-0 rounded-full {status.tone}"></div>
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <span class="truncate text-sm font-medium">{conn.name}</span>
                    <span class="rounded-full border border-white/10 bg-white/6 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-slate-300">{status.label}</span>
                  </div>
                  <p class="mt-1 truncate text-xs text-muted-foreground">{conn.username}@{conn.host}:{conn.port}</p>
                </div>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <div>
        <p class="section-title px-2">Saved</p>
        <div class="mt-2 space-y-1">
          {#each otherConnections as conn (conn.id)}
            {@const session = getConnectionSession(conn)}
            {@const status = statusBadge(session?.status)}
            <button
              class={session?.id === activeSessionId
                ? "group flex w-full cursor-pointer items-start gap-3 rounded-2xl border border-primary/30 bg-primary/10 px-3 py-3 text-left transition-colors"
                : "group flex w-full cursor-pointer items-start gap-3 rounded-2xl border border-transparent px-3 py-3 text-left transition-colors hover:border-white/8 hover:bg-white/[0.04]"}
              onclick={() => onSelect(conn)}
            >
              <div class="mt-1 size-2.5 shrink-0 rounded-full {status.tone}"></div>
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="truncate text-sm font-medium">{conn.name}</span>
                  <span class="rounded-full border border-white/10 bg-white/6 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-slate-300">{status.label}</span>
                </div>
                <p class="mt-1 truncate text-xs text-muted-foreground">{conn.username}@{conn.host}:{conn.port}</p>
              </div>
              <div class="hidden items-center gap-1 shrink-0 group-hover:flex">
                <Button variant="ghost" size="icon-xs" class="size-7 text-muted-foreground hover:text-foreground" onclick={(event) => { event.stopPropagation(); onEdit(conn); }}>
                  <Pencil class="size-3" />
                </Button>
                <Button variant="ghost" size="icon-xs" class="size-7 text-muted-foreground hover:text-destructive" onclick={(event) => { event.stopPropagation(); onDelete(conn); }}>
                  <Trash2 class="size-3" />
                </Button>
              </div>
            </button>
          {/each}

          {#if filteredConnections.length === 0}
            <div class="rounded-2xl border border-dashed border-white/10 bg-white/[0.03] px-4 py-6 text-center text-sm text-muted-foreground">
              {#if searchQuery}
                No saved connections match “{searchQuery}”.
              {:else}
                Create your first SSH connection to turn this into a reusable command center.
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
