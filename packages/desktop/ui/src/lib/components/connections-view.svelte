<script lang="ts">
  import { ChevronDown, FolderInput, Pencil, Plus, Trash2 } from "@lucide/svelte";

  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import type { HostGroupRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";

  interface Props {
    connections: ConnectionConfig[];
    hostGroups: HostGroupRecord[];
    onSelect: (conn: ConnectionConfig) => void;
    onEdit: (conn: ConnectionConfig) => void;
    onNew: () => void;
    onDelete: (conn: ConnectionConfig) => Promise<void>;
    onCreateGroup: (name: string) => Promise<HostGroupRecord>;
    onDeleteGroup: (group: HostGroupRecord) => Promise<void>;
    onMoveToGroup: (
      conn: ConnectionConfig,
      groupId: string | null,
    ) => Promise<void>;
  }

  let {
    connections,
    hostGroups,
    onSelect,
    onEdit,
    onNew,
    onDelete,
    onCreateGroup,
    onDeleteGroup,
    onMoveToGroup,
  }: Props = $props();

  let error = $state<string | null>(null);
  let deletingConnectionId = $state<string | null>(null);
  let pendingDeleteConnection = $state<ConnectionConfig | null>(null);
  let deletingHostGroupId = $state<string | null>(null);
  let pendingDeleteHostGroup = $state<HostGroupRecord | null>(null);
  let activeGroupId = $state<string | null | "all">("all");
  let isCreatingGroup = $state(false);
  let newGroupName = $state("");
  let isSavingGroup = $state(false);
  let openGroupMenuId = $state<string | null>(null);
  let contextMenu = $state<{
    connection: ConnectionConfig;
    x: number;
    y: number;
  } | null>(null);

  let sortedConnections = $derived(
    [...connections].sort((a, b) => a.name.localeCompare(b.name)),
  );

  let sortedHostGroups = $derived(
    [...hostGroups].sort((a, b) => a.name.localeCompare(b.name)),
  );

  let ungroupedConnections = $derived(
    sortedConnections.filter((connection) => connection.groupId === null),
  );

  let activeHostGroup = $derived(
    typeof activeGroupId === "string" && activeGroupId !== "all"
      ? (hostGroups.find((hostGroup) => hostGroup.id === activeGroupId) ?? null)
      : null,
  );

  let visibleConnections = $derived.by(() => {
    if (activeGroupId === "all") {
      return sortedConnections;
    }

    if (activeGroupId === null) {
      return ungroupedConnections;
    }

    return sortedConnections.filter((connection) => connection.groupId === activeGroupId);
  });

  function getGroupCount(groupId: string | null): number {
    return connections.filter((connection) => connection.groupId === groupId).length;
  }

  function tabClass(groupId: string | null | "all"): string {
    const active = activeGroupId === groupId;
    return active
      ? "border-b-2 border-cyan-300 px-1 pb-2 pt-2 text-sm font-medium text-white"
      : "border-b-2 border-transparent px-1 pb-2 pt-2 text-sm font-medium text-slate-400 transition hover:text-white";
  }

  async function handleCreateGroup() {
    const name = newGroupName.trim();
    if (!name || isSavingGroup) {
      return;
    }

    isSavingGroup = true;
    error = null;
    try {
      const hostGroup = await onCreateGroup(name);
      activeGroupId = hostGroup.id;
      newGroupName = "";
      isCreatingGroup = false;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to create group";
    } finally {
      isSavingGroup = false;
    }
  }

  function toggleGroupMenu(event: Event, connectionId: string) {
    event.stopPropagation();
    openGroupMenuId = openGroupMenuId === connectionId ? null : connectionId;
  }

  function handleContextMenu(event: MouseEvent, connection: ConnectionConfig) {
    event.preventDefault();
    contextMenu = { connection, x: event.clientX, y: event.clientY };
  }

  async function handleChangeGroup(connection: ConnectionConfig, groupId: string | null) {
    openGroupMenuId = null;
    if (connection.groupId === groupId) {
      return;
    }
    error = null;
    try {
      await onMoveToGroup(connection, groupId);
      activeGroupId = groupId;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to move host";
    }
  }

  function groupNameLabel(groupId: string | null): string {
    if (groupId === null) return "Ungrouped";
    const group = hostGroups.find((g) => g.id === groupId);
    return group ? group.name : "Unknown";
  }

  function requestDeleteGroup(group: HostGroupRecord) {
    pendingDeleteHostGroup = group;
    error = null;
  }

  async function confirmDeleteGroup() {
    if (!pendingDeleteHostGroup) {
      return;
    }

    const group = pendingDeleteHostGroup;
    deletingHostGroupId = group.id;
    error = null;

    try {
      await onDeleteGroup(group);
      pendingDeleteHostGroup = null;
      activeGroupId = null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to delete group";
    } finally {
      deletingHostGroupId = null;
    }
  }

  function getAuthLabel(connection: ConnectionConfig): string {
    switch (connection.auth?.kind) {
      case "public_key_and_password":
        return "Key + Password";
      case "public_key":
        return "SSH Key";
      default:
        return "Password";
    }
  }

  function requestDelete(connection: ConnectionConfig) {
    pendingDeleteConnection = connection;
    error = null;
  }

  async function confirmDelete() {
    if (!pendingDeleteConnection) {
      return;
    }

    const connection = pendingDeleteConnection;
    deletingConnectionId = connection.id;
    error = null;

    try {
      await onDelete(connection);
      pendingDeleteConnection = null;
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Failed to delete connection";
    } finally {
      deletingConnectionId = null;
    }
  }
</script>

<svelte:window
  onclick={() => {
    openGroupMenuId = null;
    contextMenu = null;
  }}
  oncontextmenu={() => (contextMenu = null)}
/>

<div
  class="workspace-canvas flex h-full flex-col overflow-y-auto px-5 py-6 lg:px-8"
>
  <section class="ide-panel flex min-h-full flex-col p-5 text-white sm:p-6">
    <div
      class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between"
    >
      <div>
        <p class="section-title text-cyan-200/70">Inventory</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">
          SSH Connections
        </h1>
        <p class="mt-2 text-sm text-slate-500">
          Curated hosts, credentials, and terminal targets.
        </p>
      </div>

      <Button
        onclick={onNew}
        variant="default"
        size="sm"
        class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
      >
        <Plus class="size-3.5" />
        Add connection
      </Button>
    </div>

    {#if error}
      <div
        class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        role="alert"
      >
        {error}
      </div>
    {/if}

    <div class="mt-6 flex-1">
      {#if sortedConnections.length === 0}
        <div
          class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground"
        >
          No saved connections yet
        </div>
      {:else}
        <div class="mb-0 flex items-center gap-4 border-b border-white/10">
          <button
            type="button"
            class={tabClass("all")}
            onclick={() => (activeGroupId = "all")}
          >
            All
            <span
              class="ml-1.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] text-slate-400"
              >{connections.length}</span
            >
          </button>
          <button
            type="button"
            class={tabClass(null)}
            onclick={() => (activeGroupId = null)}
          >
            Ungrouped
            <span
              class="ml-1.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] text-slate-400"
              >{ungroupedConnections.length}</span
            >
          </button>
          {#each sortedHostGroups as hostGroup (hostGroup.id)}
            <button
              type="button"
              class={tabClass(hostGroup.id)}
              onclick={() => (activeGroupId = hostGroup.id)}
            >
              {hostGroup.name}
              <span
                class="ml-1.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] text-slate-400"
                >{getGroupCount(hostGroup.id)}</span
              >
            </button>
          {/each}

          {#if isCreatingGroup}
            <form
              class="flex items-center gap-2 pb-2 pl-3"
              onsubmit={(event) => {
                event.preventDefault();
                void handleCreateGroup();
              }}
            >
              <input
                bind:value={newGroupName}
                placeholder="Group name"
                class="h-7 w-36 rounded border border-white/10 bg-black/20 px-2 text-sm text-white placeholder:text-slate-500 focus:border-cyan-300/40 focus:outline-none"
                disabled={isSavingGroup}
              />
              <Button
                type="submit"
                size="sm"
                class="rounded bg-cyan-300 text-slate-950 hover:bg-cyan-200"
                disabled={isSavingGroup}>Add</Button
              >
              <Button
                type="button"
                variant="ghost"
                size="sm"
                class="rounded text-slate-400 hover:bg-white/8 hover:text-white"
                onclick={() => {
                  isCreatingGroup = false;
                  newGroupName = "";
                }}
                disabled={isSavingGroup}>Cancel</Button
              >
            </form>
          {:else}
            <button
              type="button"
              class="flex items-center gap-1.5 border-b-2 border-transparent pb-2 pl-1 pt-2 text-sm font-medium text-slate-400 transition hover:text-white"
              onclick={() => (isCreatingGroup = true)}
            >
              <Plus class="size-3.5" />
              Group
            </button>
          {/if}

          {#if activeHostGroup}
            <button
              type="button"
              class="flex items-center gap-1.5 border-b-2 border-transparent pb-2 pl-1 pt-2 text-sm font-medium text-red-400 transition hover:text-red-300"
              onclick={() => requestDeleteGroup(activeHostGroup)}
              disabled={deletingHostGroupId === activeHostGroup.id}
            >
              <Trash2 class="size-3.5" />
              Delete group
            </button>
          {/if}
        </div>

        <div
          role="list"
          aria-label="SSH connections"
          class="grid gap-3 px-1 py-5 md:grid-cols-2 xl:grid-cols-3"
        >
          {#if visibleConnections.length === 0}
            <div
              class="rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground md:col-span-2 xl:col-span-3"
            >
              No connections in this group yet.
            </div>
          {/if}

          {#each visibleConnections as connection (connection.id)}
            <article
              role="listitem"
              oncontextmenu={(event) => handleContextMenu(event, connection)}
              class="group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]"
            >
              <div class="flex items-start gap-3">
                <div class="min-w-0 flex-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="truncate text-sm font-medium text-white"
                      >{connection.name}</span
                    >
                    <div class="relative">
                      <button
                        type="button"
                        class="flex items-center gap-1 rounded-lg border border-white/10 bg-white/5 px-1.5 py-0.5 text-[11px] text-slate-400 transition hover:border-white/20 hover:text-slate-300"
                        onclick={(event) => toggleGroupMenu(event, connection.id)}
                      >
                        {groupNameLabel(connection.groupId)}
                        <ChevronDown class="size-2.5" />
                      </button>

                      {#if openGroupMenuId === connection.id}
                        <div
                          class="absolute top-full left-0 z-50 mt-1 min-w-[12rem] rounded-xl border border-white/12 bg-[#0d1117] p-1 shadow-2xl shadow-black/50"
                        >
                          <button
                            type="button"
                            class={connection.groupId === null
                              ? "flex w-full items-center gap-2 rounded-lg bg-cyan-300/10 px-3 py-1.5 text-xs text-cyan-100"
                              : "flex w-full items-center gap-2 rounded-lg px-3 py-1.5 text-xs text-slate-300 transition hover:bg-white/6"}
                            onclick={() => void handleChangeGroup(connection, null)}
                          >
                            Ungrouped
                          </button>
                          {#each sortedHostGroups as group (group.id)}
                            <button
                              type="button"
                              class={connection.groupId === group.id
                                ? "flex w-full items-center gap-2 rounded-lg bg-cyan-300/10 px-3 py-1.5 text-xs text-cyan-100"
                                : "flex w-full items-center gap-2 rounded-lg px-3 py-1.5 text-xs text-slate-300 transition hover:bg-white/6"}
                              onclick={() => void handleChangeGroup(connection, group.id)}
                            >
                              {group.name}
                            </button>
                          {/each}
                        </div>
                      {/if}
                    </div>
                  </div>

                  <p class="mt-2 truncate font-mono text-xs text-slate-400">
                    {connection.username}@{connection.host}:{connection.port}
                  </p>
                  <p class="mt-1 text-xs text-slate-500">
                    {getAuthLabel(connection)}
                  </p>
                </div>
              </div>

              <div class="mt-4 flex flex-wrap items-center gap-2">
                <Button
                  variant="ghost"
                  size="xs"
                  class="gap-1.5 rounded-xl bg-white/[0.035] text-slate-200 hover:bg-cyan-300/10 hover:text-white"
                  onclick={() => onSelect(connection)}
                >
                  Connect
                </Button>
                <Button
                  variant="ghost"
                  size="xs"
                  class="gap-1.5 rounded-xl text-slate-400 hover:bg-white/7 hover:text-white"
                  onclick={() => onEdit(connection)}
                >
                  <Pencil class="size-3" />
                  Edit
                </Button>
                <Button
                  variant="ghost"
                  size="xs"
                  class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                  onclick={() => requestDelete(connection)}
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

{#if contextMenu}
  {@const conn = contextMenu.connection}
  <div
    class="fixed z-[9999] min-w-[14rem] rounded-xl border border-white/12 bg-[#0d1117] p-1 shadow-2xl shadow-black/50"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
  >
    <div class="px-3 py-2 text-xs font-medium text-slate-500">
      Move to group
    </div>
    <button
      type="button"
      class={conn.groupId === null
        ? "flex w-full items-center gap-2 rounded-lg bg-cyan-300/10 px-3 py-1.5 text-xs text-cyan-100"
        : "flex w-full items-center gap-2 rounded-lg px-3 py-1.5 text-xs text-slate-300 transition hover:bg-white/6"}
      onclick={() => void handleChangeGroup(conn, null)}
    >
      Ungrouped
    </button>
    {#each sortedHostGroups as group (group.id)}
      <button
        type="button"
        class={conn.groupId === group.id
          ? "flex w-full items-center gap-2 rounded-lg bg-cyan-300/10 px-3 py-1.5 text-xs text-cyan-100"
          : "flex w-full items-center gap-2 rounded-lg px-3 py-1.5 text-xs text-slate-300 transition hover:bg-white/6"}
        onclick={() => void handleChangeGroup(conn, group.id)}
      >
        <FolderInput class="size-3" />
        {group.name}
      </button>
    {/each}
  </div>
{/if}

<DeleteConfirmDialog
  open={pendingDeleteConnection !== null}
  title="Delete saved connection?"
  description="This removes the saved host profile and disconnects any sessions that were opened from it. This action cannot be undone."
  itemName={pendingDeleteConnection?.name}
  confirmLabel="Delete connection"
  isDeleting={deletingConnectionId !== null}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDeleteConnection = null)}
/>

<DeleteConfirmDialog
  open={pendingDeleteHostGroup !== null}
  title="Delete host group?"
  description="Hosts in this group will stay saved and move back to Ungrouped. This action cannot be undone."
  itemName={pendingDeleteHostGroup?.name}
  confirmLabel="Delete group"
  isDeleting={deletingHostGroupId !== null}
  onConfirm={confirmDeleteGroup}
  onCancel={() => (pendingDeleteHostGroup = null)}
/>
