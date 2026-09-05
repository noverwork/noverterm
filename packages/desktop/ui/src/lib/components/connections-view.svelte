<script lang="ts">
  import {
    Check,
    ChevronRight,
    FolderInput,
    Pencil,
    Plus,
    Search,
    Server,
    Trash2,
  } from "@lucide/svelte";

  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { HostGroupRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";

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

  const UNGROUPED_KEY = "ungrouped";
  const COLLAPSED_STORAGE_KEY = "noverterm.connections.collapsedGroups";

  interface GroupSection {
    key: string;
    group: HostGroupRecord | null;
    connections: ConnectionConfig[];
  }

  let error = $state<string | null>(null);
  let deletingConnectionId = $state<string | null>(null);
  let pendingDeleteConnection = $state<ConnectionConfig | null>(null);
  let deletingHostGroupId = $state<string | null>(null);
  let pendingDeleteHostGroup = $state<HostGroupRecord | null>(null);
  let isCreatingGroup = $state(false);
  let newGroupName = $state("");
  let isSavingGroup = $state(false);
  let searchQuery = $state("");
  let collapsedKeys = $state<string[]>(loadCollapsedKeys());
  let draggingConnectionId = $state<string | null>(null);
  let dragOverKey = $state<string | null>(null);

  let sortedConnections = $derived(
    [...connections].sort((a, b) => a.name.localeCompare(b.name)),
  );

  let sortedHostGroups = $derived(
    [...hostGroups].sort((a, b) => a.name.localeCompare(b.name)),
  );

  let isSearching = $derived(searchQuery.trim().length > 0);

  let sections = $derived.by(() => {
    const query = searchQuery.trim().toLowerCase();
    const matches = (connection: ConnectionConfig) =>
      !query ||
      connection.name.toLowerCase().includes(query) ||
      connection.host.toLowerCase().includes(query) ||
      connection.username.toLowerCase().includes(query);

    const result: GroupSection[] = sortedHostGroups.map((group) => ({
      key: group.id,
      group,
      connections: sortedConnections.filter(
        (connection) => connection.groupId === group.id && matches(connection),
      ),
    }));

    result.push({
      key: UNGROUPED_KEY,
      group: null,
      connections: sortedConnections.filter(
        (connection) => connection.groupId === null && matches(connection),
      ),
    });

    return result;
  });

  let visibleSections = $derived(
    sections.filter((section) => {
      if (isSearching) {
        return section.connections.length > 0;
      }
      if (section.group === null) {
        return section.connections.length > 0 || draggingConnectionId !== null;
      }
      return true;
    }),
  );

  function loadCollapsedKeys(): string[] {
    try {
      const raw = localStorage.getItem(COLLAPSED_STORAGE_KEY);
      const parsed: unknown = raw ? JSON.parse(raw) : [];
      return Array.isArray(parsed) ? parsed.filter((v) => typeof v === "string") : [];
    } catch {
      return [];
    }
  }

  function toggleCollapsed(key: string) {
    collapsedKeys = collapsedKeys.includes(key)
      ? collapsedKeys.filter((k) => k !== key)
      : [...collapsedKeys, key];
    try {
      localStorage.setItem(COLLAPSED_STORAGE_KEY, JSON.stringify(collapsedKeys));
    } catch {
      // best-effort persistence only
    }
  }

  function isCollapsed(key: string): boolean {
    return !isSearching && collapsedKeys.includes(key);
  }

  function handleDragStart(event: DragEvent, connection: ConnectionConfig) {
    draggingConnectionId = connection.id;
    if (event.dataTransfer) {
      event.dataTransfer.setData("text/plain", connection.id);
      event.dataTransfer.effectAllowed = "move";
    }
  }

  function handleDragEnd() {
    draggingConnectionId = null;
    dragOverKey = null;
  }

  function handleDragOver(event: DragEvent, key: string) {
    if (!draggingConnectionId) {
      return;
    }
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "move";
    }
    dragOverKey = key;
  }

  function handleDragLeave(event: DragEvent) {
    const element = event.currentTarget as HTMLElement;
    if (event.relatedTarget instanceof Node && element.contains(event.relatedTarget)) {
      return;
    }
    dragOverKey = null;
  }

  async function handleDrop(event: DragEvent, groupId: string | null) {
    event.preventDefault();
    const id = draggingConnectionId;
    draggingConnectionId = null;
    dragOverKey = null;
    if (!id) {
      return;
    }
    const connection = connections.find((c) => c.id === id);
    if (connection) {
      await handleChangeGroup(connection, groupId);
    }
  }

  async function handleCreateGroup() {
    const name = newGroupName.trim();
    if (!name || isSavingGroup) {
      return;
    }

    isSavingGroup = true;
    error = null;
    try {
      await onCreateGroup(name);
      newGroupName = "";
      isCreatingGroup = false;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to create group";
    } finally {
      isSavingGroup = false;
    }
  }

  async function handleChangeGroup(connection: ConnectionConfig, groupId: string | null) {
    if (connection.groupId === groupId) {
      return;
    }
    error = null;
    try {
      await onMoveToGroup(connection, groupId);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to move host";
    }
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

<div
  class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8"
>
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
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

      <div class="flex items-center gap-2 self-start">
        <div class="relative">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-slate-500"
          />
          <Input
            type="search"
            bind:value={searchQuery}
            placeholder="Search hosts"
            aria-label="Search connections"
            class="h-8 w-48 rounded-2xl border-white/10 bg-white/[0.03] pl-8 text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:ring-cyan-300/20"
          />
        </div>

        <Button
          onclick={onNew}
          variant="default"
          size="sm"
          class="gap-2 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
        >
          <Plus class="size-3.5" />
          Add connection
        </Button>
      </div>
    </div>

    {#if error}
      <div
        class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        role="alert"
      >
        {error}
      </div>
    {/if}

    <div class="-mx-2 mt-6 min-h-0 flex-1 overflow-y-auto px-2">
      {#if sortedConnections.length === 0}
        <div
          class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground"
        >
          No saved connections yet
        </div>
      {:else}
        {#if isSearching && visibleSections.length === 0}
          <div
            class="rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground"
          >
            No connections match your search.
          </div>
        {/if}

        {#each visibleSections as section (section.key)}
          <section
            role="group"
            aria-label={section.group?.name ?? "Ungrouped"}
            class={dragOverKey === section.key && draggingConnectionId !== null
              ? "group/section -mx-2 mb-2 rounded-2xl bg-cyan-300/[0.05] px-2 pb-1 ring-1 ring-cyan-300/35"
              : "group/section -mx-2 mb-2 rounded-2xl px-2 pb-1"}
            ondragover={(event) => handleDragOver(event, section.key)}
            ondragleave={handleDragLeave}
            ondrop={(event) => void handleDrop(event, section.group?.id ?? null)}
          >
            <div class="flex items-center gap-1 border-b border-white/8 pb-2 pt-2">
              <button
                type="button"
                class="flex min-w-0 cursor-pointer items-center gap-2 text-left"
                onclick={() => toggleCollapsed(section.key)}
                aria-expanded={!isCollapsed(section.key)}
              >
                <ChevronRight
                  class={isCollapsed(section.key)
                    ? "size-3.5 shrink-0 text-slate-500 transition-transform"
                    : "size-3.5 shrink-0 rotate-90 text-slate-500 transition-transform"}
                />
                <span class="section-title truncate text-slate-300">
                  {section.group?.name ?? "Ungrouped"}
                </span>
                <span
                  class="rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] text-slate-400"
                  >{section.connections.length}</span
                >
              </button>

              {#if section.group}
                <button
                  type="button"
                  class="ml-auto flex cursor-pointer items-center gap-1 rounded-lg px-1.5 py-1 text-slate-500 opacity-0 transition hover:bg-red-400/10 hover:text-red-300 focus-visible:opacity-100 group-hover/section:opacity-100"
                  aria-label={`Delete group ${section.group.name}`}
                  onclick={() => section.group && requestDeleteGroup(section.group)}
                  disabled={deletingHostGroupId === section.group.id}
                >
                  <Trash2 class="size-3.5" />
                </button>
              {/if}
            </div>

            {#if !isCollapsed(section.key)}
              {#if section.connections.length === 0}
                <div
                  class="my-3 rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.02] px-4 py-4 text-center text-xs text-slate-500"
                >
                  {draggingConnectionId !== null
                    ? "Drop here to move host"
                    : "No hosts in this group — drag one here"}
                </div>
              {:else}
                <div class="grid gap-3 py-4 md:grid-cols-2 xl:grid-cols-3">
                  {#each section.connections as connection (connection.id)}
                    <ContextMenu.Root>
                      <ContextMenu.Trigger class="contents">
                        <div
                          role="button"
                          tabindex="0"
                          draggable="true"
                          aria-label={`Connect to ${connection.name}`}
                          ondragstart={(event) => handleDragStart(event, connection)}
                          ondragend={handleDragEnd}
                          onclick={() => onSelect(connection)}
                          onkeydown={(event) => {
                            if (event.key === "Enter") {
                              onSelect(connection);
                            }
                          }}
                          class={draggingConnectionId === connection.id
                            ? "group cursor-pointer rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 opacity-40 transition hover:border-cyan-300/30 hover:bg-white/[0.055] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
                            : "group cursor-pointer rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-cyan-300/30 hover:bg-white/[0.055] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"}
                        >
                          <div class="flex items-start gap-3">
                            <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                              <Server class="size-5" />
                            </div>

                            <div class="min-w-0 flex-1">
                              <p class="truncate text-sm font-medium text-white">
                                {connection.name}
                              </p>
                              <p class="mt-2 truncate font-mono text-xs text-slate-400">
                                {connection.username}@{connection.host}:{connection.port}
                              </p>
                              <p class="mt-1 text-xs text-slate-500">
                                {getAuthLabel(connection)}
                              </p>
                            </div>

                            <div
                              class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
                            >
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                class="rounded-xl text-slate-400 hover:bg-white/8 hover:text-white"
                                aria-label={`Edit ${connection.name}`}
                                onclick={(event) => {
                                  event.stopPropagation();
                                  onEdit(connection);
                                }}
                              >
                                <Pencil class="size-3.5" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                class="rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                                aria-label={`Delete ${connection.name}`}
                                disabled={deletingConnectionId === connection.id}
                                onclick={(event) => {
                                  event.stopPropagation();
                                  requestDelete(connection);
                                }}
                              >
                                <Trash2 class="size-3.5" />
                              </Button>
                            </div>
                          </div>
                        </div>
                      </ContextMenu.Trigger>

                      <ContextMenu.Content
                        class="min-w-44 border-white/10 bg-slate-950/96 text-slate-100 shadow-2xl shadow-black/45"
                      >
                        <ContextMenu.Label class="max-w-56 truncate text-slate-400">
                          {connection.name}
                        </ContextMenu.Label>
                        <ContextMenu.Separator class="bg-white/10" />
                        <ContextMenu.Item
                          class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                          onclick={() => onSelect(connection)}
                        >
                          Connect
                        </ContextMenu.Item>
                        <ContextMenu.Item
                          class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                          onclick={() => onEdit(connection)}
                        >
                          <Pencil class="size-3.5" />
                          Edit
                        </ContextMenu.Item>
                        <ContextMenu.Sub>
                          <ContextMenu.SubTrigger
                            class="cursor-pointer focus:bg-cyan-300/10 focus:text-white data-[state=open]:bg-cyan-300/10 data-[state=open]:text-white"
                          >
                            <FolderInput class="size-3.5" />
                            Move to group
                          </ContextMenu.SubTrigger>
                          <ContextMenu.SubContent
                            class="min-w-40 border-white/10 bg-slate-950/96 text-slate-100 shadow-2xl shadow-black/45"
                          >
                            <ContextMenu.Item
                              class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                              onclick={() => void handleChangeGroup(connection, null)}
                            >
                              {#if connection.groupId === null}
                                <Check class="size-3.5 text-cyan-200" />
                              {:else}
                                <span class="size-3.5"></span>
                              {/if}
                              Ungrouped
                            </ContextMenu.Item>
                            {#each sortedHostGroups as group (group.id)}
                              <ContextMenu.Item
                                class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                                onclick={() => void handleChangeGroup(connection, group.id)}
                              >
                                {#if connection.groupId === group.id}
                                  <Check class="size-3.5 text-cyan-200" />
                                {:else}
                                  <span class="size-3.5"></span>
                                {/if}
                                {group.name}
                              </ContextMenu.Item>
                            {/each}
                          </ContextMenu.SubContent>
                        </ContextMenu.Sub>
                        <ContextMenu.Separator class="bg-white/10" />
                        <ContextMenu.Item
                          class="cursor-pointer text-red-300 focus:bg-red-400/10 focus:text-red-200"
                          disabled={deletingConnectionId === connection.id}
                          onclick={() => requestDelete(connection)}
                        >
                          <Trash2 class="size-3.5" />
                          Delete
                        </ContextMenu.Item>
                      </ContextMenu.Content>
                    </ContextMenu.Root>
                  {/each}
                </div>
              {/if}
            {/if}
          </section>
        {/each}

        {#if !isSearching}
          {#if isCreatingGroup}
            <form
              class="mt-2 flex items-center gap-2"
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
                class="rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
                disabled={isSavingGroup}>Add</Button
              >
              <Button
                type="button"
                variant="ghost"
                size="sm"
                class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white"
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
              class="mt-2 flex cursor-pointer items-center gap-1.5 text-sm font-medium text-slate-400 transition hover:text-white"
              onclick={() => (isCreatingGroup = true)}
            >
              <Plus class="size-3.5" />
              New group
            </button>
          {/if}
        {/if}
      {/if}
    </div>
  </section>
</div>

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
