<script lang="ts">
  import {
    Check,
    ChevronRight,
    FileInput,
    FolderInput,
    Pencil,
    Plus,
    Search,
    Server,
    Trash2,
  } from "@lucide/svelte";

  import { t } from "$lib/i18n/index.svelte.js";
  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import SshConfigImportDialog from "$lib/components/ssh-config-import-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { HostGroupRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import {
    groupPrefix,
    lineColorFor,
    UNGROUPED_LINE,
  } from "$lib/session-stencil.js";

  interface Props {
    connections: ConnectionConfig[];
    hostGroups: HostGroupRecord[];
    onSelect: (conn: ConnectionConfig) => void;
    onEdit: (conn: ConnectionConfig) => void;
    onNew: () => void;
    onImported: () => Promise<unknown>;
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
    onImported,
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
  let isImporting = $state(false);
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

  // WebKit does not auto-scroll overflow containers during HTML5 drag; dragover
  // keeps firing while the pointer rests, so nudge the list near its edges.
  function handleListDragOver(event: DragEvent) {
    if (!draggingConnectionId) {
      return;
    }
    const list = event.currentTarget as HTMLElement;
    const rect = list.getBoundingClientRect();
    const edge = 48;
    if (event.clientY < rect.top + edge) {
      list.scrollBy(0, -12);
    } else if (event.clientY > rect.bottom - edge) {
      list.scrollBy(0, 12);
    }
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
      error = cause instanceof Error ? cause.message : t("Failed to create group");
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
      error = cause instanceof Error ? cause.message : t("Failed to move host");
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
      error = cause instanceof Error ? cause.message : t("Failed to delete group");
    } finally {
      deletingHostGroupId = null;
    }
  }

  function getAuthLabel(connection: ConnectionConfig): string {
    switch (connection.auth?.kind) {
      case "public_key_and_password":
        return t("Key + Password");
      case "public_key":
        return t("SSH Key");
      default:
        return t("Password");
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
        cause instanceof Error ? cause.message : t("Failed to delete connection");
    } finally {
      deletingConnectionId = null;
    }
  }
</script>

<div
  class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8"
>
  <section class="flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-stencil sm:p-6">
    <div
      class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between"
    >
      <div>
        <h1 class="text-2xl font-semibold tracking-tight text-stencil">
          {t("SSH Connections")}
        </h1>
      </div>

      <div class="flex items-center gap-2 self-start">
        <div class="relative">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-stencil-mute"
          />
          <Input
            type="search"
            bind:value={searchQuery}
            placeholder={t("Search hosts")}
            aria-label={t("Search connections")}
            class="h-8 w-48 rounded-[2px] border border-white/10 bg-rail pl-8 text-sm text-stencil placeholder:text-stencil-faint focus-visible:border-stencil"
          />
        </div>

        <Button
          onclick={() => (isImporting = true)}
          variant="outline"
          size="sm"
          class="gap-2 rounded-[2px] border-white/10 bg-transparent text-stencil hover:border-white/20 hover:bg-white/5"
        >
          <FileInput class="size-3.5" />
          {t("Import")}
        </Button>

        <Button
          onclick={onNew}
          variant="default"
          size="sm"
          class="gap-2 rounded-[2px] bg-stencil text-port hover:bg-white"
        >
          <Plus class="size-3.5" />
          {t("Add connection")}
        </Button>
      </div>
    </div>

    {#if error}
      <div
        class="mt-5 rounded-[2px] border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        role="alert"
      >
        {error}
      </div>
    {/if}

    <div
      class="-mx-2 mt-6 min-h-0 flex-1 overflow-y-auto px-2"
      role="presentation"
      ondragover={handleListDragOver}
    >
      {#if sortedConnections.length === 0}
        <div
          class="flex h-full min-h-[16rem] items-center justify-center rounded-[2px] border border-dashed border-white/10 bg-transparent px-4 py-8 text-center text-sm text-muted-foreground"
        >
          <div class="flex flex-col items-center gap-3">
            {t("No saved connections yet")}
            <Button
              onclick={() => (isImporting = true)}
              variant="outline"
              size="sm"
              class="gap-2 rounded-[2px] border-white/10 bg-transparent text-stencil hover:border-white/20 hover:bg-white/5"
            >
              <FileInput class="size-3.5" />
              {t("Import from {path}", { path: "~/.ssh/config" })}
            </Button>
          </div>
        </div>
      {:else}
        {#if isSearching && visibleSections.length === 0}
          <div
            class="rounded-[2px] border border-dashed border-white/10 bg-transparent px-4 py-8 text-center text-sm text-muted-foreground"
          >
            {t("No connections match your search.")}
          </div>
        {/if}

        {#each visibleSections as section (section.key)}
          <section
            role="group"
            aria-label={section.group?.name ?? t("Ungrouped")}
            class={dragOverKey === section.key && draggingConnectionId !== null
              ? "group/section -mx-2 mb-2 rounded-[2px] bg-white/[0.04] px-2 pb-1 ring-1 ring-stencil"
              : "group/section -mx-2 mb-2 rounded-[2px] px-2 pb-1"}
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
                    ? "size-3.5 shrink-0 text-stencil-mute transition-transform"
                    : "size-3.5 shrink-0 rotate-90 text-stencil-mute transition-transform"}
                />
                <span
                  class="h-2 w-3 shrink-0"
                  style:background-color={section.group
                    ? lineColorFor(section.group.id, hostGroups).fill
                    : UNGROUPED_LINE.fill}
                ></span>
                <span class="section-title truncate text-stencil-dim">
                  {section.group?.name ?? t("Ungrouped")}
                </span>
                {#if section.group}
                  <span
                    class="rounded-[2px] border border-white/10 bg-rib px-1.5 text-[11px] uppercase tracking-[0.14em] text-stencil-dim"
                    >{groupPrefix(section.group.name)}</span
                  >
                {/if}
                <span
                  class="rounded-[2px] border border-white/10 bg-rib px-1.5 text-[11px] text-stencil-mute"
                  >{section.connections.length}</span
                >
              </button>

              {#if section.group}
                <button
                  type="button"
                  class="ml-auto flex cursor-pointer items-center gap-1 rounded-[2px] px-1.5 py-1 text-stencil-mute opacity-0 transition hover:bg-seal/20 hover:text-red-300 focus-visible:opacity-100 group-hover/section:opacity-100"
                  aria-label={t("Delete group {name}", { name: section.group.name })}
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
                  class="my-3 rounded-[2px] border border-dashed border-white/10 bg-transparent px-4 py-4 text-center text-xs text-stencil-mute"
                >
                  {draggingConnectionId !== null
                    ? t("Drop here to move host")
                    : t("No hosts in this group — drag one here")}
                </div>
              {:else}
                <div class="grid gap-3 py-4 md:grid-cols-2 xl:grid-cols-3">
                  {#each section.connections as connection (connection.id)}
                    {@const line = connection.groupId
                      ? lineColorFor(connection.groupId, hostGroups)
                      : UNGROUPED_LINE}
                    <ContextMenu.Root>
                      <ContextMenu.Trigger class="contents">
                        <div
                          role="button"
                          tabindex="0"
                          draggable="true"
                          aria-label={t("Connect to {name}", { name: connection.name })}
                          ondragstart={(event) => handleDragStart(event, connection)}
                          ondragend={handleDragEnd}
                          onclick={() => onSelect(connection)}
                          onkeydown={(event) => {
                            if (event.key === "Enter") {
                              onSelect(connection);
                            }
                          }}
                          class={draggingConnectionId === connection.id
                            ? "group cursor-pointer rounded-[2px] border border-white/10 bg-rib px-4 py-4 opacity-40 transition hover:border-white/20 hover:bg-[#1f252c] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil"
                            : "group cursor-pointer rounded-[2px] border border-white/10 bg-rib px-4 py-4 transition hover:border-white/20 hover:bg-[#1f252c] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil"}
                        >
                          <div class="flex items-start gap-3">
                            <div
                              class={line.darkText
                                ? "mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-[2px] text-port"
                                : "mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-[2px] text-stencil"}
                              style:background-color={line.fill}
                            >
                              <Server class="size-5" />
                            </div>

                            <div class="min-w-0 flex-1">
                              <p class="truncate text-sm font-medium text-stencil">
                                {connection.name}
                              </p>
                              <p class="mt-2 truncate font-mono text-xs text-stencil-mute">
                                {connection.username}@{connection.host}:{connection.port}
                              </p>
                              <p class="mt-1 text-xs text-stencil-mute">
                                {getAuthLabel(connection)}
                              </p>
                            </div>

                            <div
                              class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
                            >
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                class="rounded-[2px] text-stencil-mute hover:bg-white/8 hover:text-stencil"
                                aria-label={t("Edit {name}", { name: connection.name })}
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
                                class="rounded-[2px] text-stencil-mute hover:bg-seal/20 hover:text-red-300"
                                aria-label={t("Delete {name}", { name: connection.name })}
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
                        class="min-w-44 rounded-[2px] border-white/10 bg-rail text-stencil"
                      >
                        <ContextMenu.Label class="max-w-56 truncate text-stencil-mute">
                          {connection.name}
                        </ContextMenu.Label>
                        <ContextMenu.Separator class="bg-white/10" />
                        <ContextMenu.Item
                          class="cursor-pointer rounded-[2px] focus:bg-white/8 focus:text-stencil"
                          onclick={() => onSelect(connection)}
                        >
                          {t("Connect")}
                        </ContextMenu.Item>
                        <ContextMenu.Item
                          class="cursor-pointer rounded-[2px] focus:bg-white/8 focus:text-stencil"
                          onclick={() => onEdit(connection)}
                        >
                          <Pencil class="size-3.5" />
                          {t("Edit")}
                        </ContextMenu.Item>
                        <ContextMenu.Sub>
                          <ContextMenu.SubTrigger
                            class="cursor-pointer rounded-[2px] focus:bg-white/8 focus:text-stencil data-[state=open]:bg-white/8 data-[state=open]:text-stencil"
                          >
                            <FolderInput class="size-3.5" />
                            {t("Move to group")}
                          </ContextMenu.SubTrigger>
                          <ContextMenu.SubContent
                            class="min-w-40 rounded-[2px] border-white/10 bg-rail text-stencil"
                          >
                            <ContextMenu.Item
                              class="cursor-pointer rounded-[2px] focus:bg-white/8 focus:text-stencil"
                              onclick={() => void handleChangeGroup(connection, null)}
                            >
                              {#if connection.groupId === null}
                                <Check class="size-3.5 text-stencil" />
                              {:else}
                                <span class="size-3.5"></span>
                              {/if}
                              {t("Ungrouped")}
                            </ContextMenu.Item>
                            {#each sortedHostGroups as group (group.id)}
                              <ContextMenu.Item
                                class="cursor-pointer rounded-[2px] focus:bg-white/8 focus:text-stencil"
                                onclick={() => void handleChangeGroup(connection, group.id)}
                              >
                                {#if connection.groupId === group.id}
                                  <Check class="size-3.5 text-stencil" />
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
                          class="cursor-pointer text-red-300 rounded-[2px] focus:bg-seal/20 focus:text-red-200"
                          disabled={deletingConnectionId === connection.id}
                          onclick={() => requestDelete(connection)}
                        >
                          <Trash2 class="size-3.5" />
                          {t("Delete")}
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
                placeholder={t("Group name")}
                aria-label={t("Group name")}
                class="h-8 w-36 rounded-[2px] border border-white/10 bg-rail px-2 text-sm text-stencil placeholder:text-stencil-faint focus:border-stencil focus:outline-none"
                disabled={isSavingGroup}
              />
              <Button
                type="submit"
                size="sm"
                class="rounded-[2px] bg-stencil text-port hover:bg-white"
                disabled={isSavingGroup}>{t("Add")}</Button
              >
              <Button
                type="button"
                variant="ghost"
                size="sm"
                class="rounded-[2px] text-stencil-dim hover:bg-white/8 hover:text-stencil"
                onclick={() => {
                  isCreatingGroup = false;
                  newGroupName = "";
                }}
                disabled={isSavingGroup}>{t("Cancel")}</Button
              >
            </form>
          {:else}
            <button
              type="button"
              class="mt-2 flex cursor-pointer items-center gap-1.5 text-sm font-medium text-stencil-mute transition hover:text-stencil"
              onclick={() => (isCreatingGroup = true)}
            >
              <Plus class="size-3.5" />
              {t("New group")}
            </button>
          {/if}
        {/if}
      {/if}
    </div>
  </section>
</div>

<SshConfigImportDialog
  open={isImporting}
  {onImported}
  onClose={() => (isImporting = false)}
/>

<DeleteConfirmDialog
  open={pendingDeleteConnection !== null}
  title={t("Delete saved connection?")}
  description={t("This removes the saved host profile and disconnects any sessions that were opened from it. This action cannot be undone.")}
  itemName={pendingDeleteConnection?.name}
  confirmLabel={t("Delete connection")}
  isDeleting={deletingConnectionId !== null}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDeleteConnection = null)}
/>

<DeleteConfirmDialog
  open={pendingDeleteHostGroup !== null}
  title={t("Delete host group?")}
  description={t("Hosts in this group will stay saved and move back to Ungrouped. This action cannot be undone.")}
  itemName={pendingDeleteHostGroup?.name}
  confirmLabel={t("Delete group")}
  isDeleting={deletingHostGroupId !== null}
  onConfirm={confirmDeleteGroup}
  onCancel={() => (pendingDeleteHostGroup = null)}
/>
