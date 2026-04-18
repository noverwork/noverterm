<script lang="ts">
  import { Terminal, Plus, Search, ChevronLeft, ChevronRight, Trash2, Pencil } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";
  import type { SessionStatus } from "$lib/stores/session.svelte.js";

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
  }: {
    connections: ConnectionConfig[];
    sessions: Map<string, { id: string; name: string; status: SessionStatus }>;
    activeSessionId: string | null;
    collapsed: boolean;
    onToggle: () => void;
    onSelect: (conn: ConnectionConfig) => void;
    onAdd: () => void;
    onEdit: (conn: ConnectionConfig) => void;
    onDelete: (conn: ConnectionConfig) => void;
    onLocalTerminal?: () => void;
  } = $props();

  let searchQuery = $state("");

  const filteredConnections = $derived(
    connections.filter(
      (c) =>
        c.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.host.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.username.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  function statusColor(status: SessionStatus): string {
    switch (status) {
      case "connected":
        return "bg-green-500";
      case "connecting":
        return "bg-yellow-500 animate-pulse";
      case "error":
        return "bg-red-500";
      default:
        return "bg-muted-foreground/50";
    }
  }

  function isConnected(conn: ConnectionConfig): boolean {
    return Array.from(sessions.values()).some(
      (s) => s.status === "connected" && s.name === `${conn.username}@${conn.host}:${conn.port}`
    );
  }

  function getConnectionSession(conn: ConnectionConfig): { id: string; name: string; status: SessionStatus } | undefined {
    return Array.from(sessions.values()).find(
      (s) => s.name === `${conn.username}@${conn.host}:${conn.port}`
    );
  }
</script>

<div
  class="sidebar flex flex-col border-r border-border bg-sidebar transition-all duration-200 {collapsed ? 'w-12' : 'w-72'}"
>
  <div class="flex items-center justify-between p-3 border-b border-border">
    {#if !collapsed}
      <div class="flex items-center gap-2">
        <Terminal class="size-4 text-primary" />
        <span class="text-sm font-semibold">Connections</span>
      </div>
    {/if}
    <Button variant="ghost" size="icon-xs" onclick={onToggle}>
      {#if collapsed}
        <ChevronRight class="size-3.5" />
      {:else}
        <ChevronLeft class="size-3.5" />
      {/if}
    </Button>
  </div>

  {#if !collapsed}
    <div class="p-3 space-y-2">
      <div class="relative">
        <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
        <Input
          bind:value={searchQuery}
          placeholder="Search connections..."
          class="pl-8 h-8 text-sm"
        />
      </div>
      <Button onclick={onAdd} variant="outline" size="sm" class="w-full gap-2">
        <Plus class="size-3.5" />
        Add Connection
      </Button>
      {#if onLocalTerminal}
        <Button onclick={onLocalTerminal} variant="outline" size="sm" class="w-full gap-2">
          <Terminal class="size-3.5" />
          Local Terminal
        </Button>
      {/if}
    </div>

    <div class="flex-1 overflow-y-auto px-2 pb-2">
      {#each filteredConnections as conn (conn.id)}
        <button
          class="w-full text-left px-3 py-2.5 rounded-lg mb-1 transition-colors hover:bg-sidebar-accent group flex items-center gap-3 {(() => { const session = getConnectionSession(conn); return session && session.id === activeSessionId ? 'bg-sidebar-accent' : ''; })()}"
          onclick={() => onSelect(conn)}
        >
          <div class="flex flex-col min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium truncate">{conn.name}</span>
              <div class="size-2 rounded-full shrink-0 {(() => { const session = getConnectionSession(conn); return session ? statusColor(session.status) : 'bg-muted-foreground/50'; })()}"></div>
            </div>
            <span class="text-xs text-muted-foreground truncate">
              {conn.username}@{conn.host}:{conn.port}
            </span>
          </div>
          <div class="hidden group-hover:flex items-center gap-1 shrink-0">
            <Button
              variant="ghost"
              size="icon-xs"
              class="size-6"
              onclick={(e) => {
                e.stopPropagation();
                onEdit(conn);
              }}
            >
              <Pencil class="size-3" />
            </Button>
            <Button
              variant="ghost"
              size="icon-xs"
              class="size-6 text-destructive hover:text-destructive"
              onclick={(e) => {
                e.stopPropagation();
                onDelete(conn);
              }}
            >
              <Trash2 class="size-3" />
            </Button>
          </div>
        </button>
      {/each}

      {#if filteredConnections.length === 0}
        <div class="text-center py-8 text-muted-foreground text-sm">
          {#if searchQuery}
            No connections match "{searchQuery}"
          {:else}
            No connections yet
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
