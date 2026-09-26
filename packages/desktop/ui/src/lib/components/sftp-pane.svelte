<script lang="ts">
  import { ChevronsUpDown, Laptop, Loader2, Server, TerminalSquare } from "@lucide/svelte";

  import FileList from "./file-browser/FileList.svelte";
  import CreateFolderDialog from "./file-browser/CreateFolderDialog.svelte";
  import RenameDialog from "./file-browser/RenameDialog.svelte";
  import DeleteConfirmDialog from "./file-browser/DeleteConfirmDialog.svelte";
  import ConnectionStatusOverlay from "./connection-status-overlay.svelte";
  import {
    LOCAL_MACHINE,
    machineKey,
    sftpStore,
    type PaneSide,
    type SftpPane,
  } from "$lib/stores/sftp.svelte.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import type { Session } from "$lib/stores/session.svelte.js";
  import type { FileEntry } from "$lib/types/sftp.js";

  interface Props {
    pane: SftpPane;
    connections: ConnectionConfig[];
    sshSessions: Session[];
    onConnect: (connection: ConnectionConfig) => Promise<void>;
    onOpenSession: (session: Session) => Promise<void>;
    onUseLocal: () => Promise<void>;
    onDisconnect: () => Promise<void>;
    onRetry?: () => Promise<void>;
    onTrust?: () => Promise<void>;
    onReplaceTrust?: () => Promise<void>;
  }

  let {
    pane,
    connections,
    sshSessions,
    onConnect,
    onOpenSession,
    onUseLocal,
    onDisconnect,
    onRetry,
    onTrust,
    onReplaceTrust,
  }: Props = $props();

  const sideLabel = $derived(pane.side === "left" ? "Left" : "Right");
  const connectionStatus = $derived(
    pane.isConnecting ? "connecting"
      : pane.trustPrompt ? "trust_required"
      : pane.trustMismatch || pane.connectionError ? "error"
      : null,
  );
  const pickerDisabled = $derived(connectionStatus !== null || pane.isClosing);
  const otherMachine = $derived(sftpStore.otherPane(pane).machine);
  const takenTitle = "Already open on the other side";

  let showCreateFolderDialog = $state(false);
  let renameEntry = $state<FileEntry | null>(null);
  let deleteEntry = $state<FileEntry | null>(null);
  let isDragOver = $state(false);

  function formatHost(connection: { host: string; port: number; username: string }): string {
    const host = connection.host.includes(":") ? `[${connection.host}]` : connection.host;
    return `${connection.username}@${host}:${connection.port}`;
  }

  function handleNavigate(entry: FileEntry) {
    if (entry.file_type === "Dir") {
      void pane.navigate(pane.path ? `${pane.path}/${entry.name}` : entry.name);
    }
  }

  function parentDirectoryPath(currentPath: string): string | null {
    if (!currentPath || currentPath === "/") {
      return null;
    }
    if (currentPath === "~") {
      return "/";
    }
    const lastSlash = currentPath.lastIndexOf("/");
    if (lastSlash <= 0) {
      return "/";
    }
    return currentPath.slice(0, lastSlash);
  }

  function handleNavigateUp() {
    const parent = parentDirectoryPath(pane.path);
    if (parent !== null) {
      void pane.navigate(parent);
    }
  }

  function handlePathKeydown(event: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (event.key === "Enter") {
      const path = event.currentTarget.value.trim();
      if (!path) return;
      event.currentTarget.blur();
      void pane.navigate(path);
    } else if (event.key === "Escape") {
      event.currentTarget.value = pane.path;
      event.currentTarget.blur();
    }
  }

  async function handleCreateFolder(name: string) {
    await pane.mkdir(name);
    showCreateFolderDialog = false;
  }

  async function handleRename(newName: string) {
    if (!renameEntry) return;
    await pane.rename(renameEntry, newName);
    renameEntry = null;
  }

  async function handleDelete() {
    if (!deleteEntry) return;
    await pane.remove(deleteEntry);
    deleteEntry = null;
  }

  function handleDragOver(event: DragEvent): void {
    if (!event.dataTransfer || !pane.isReady) return;
    if (!event.dataTransfer.types.includes("application/x-sftp-entry")) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    isDragOver = true;
  }

  function handleDragLeave(event: DragEvent): void {
    if (event.currentTarget instanceof HTMLElement) {
      const related = event.relatedTarget as Node | null;
      if (related && event.currentTarget.contains(related)) {
        return;
      }
    }
    isDragOver = false;
  }

  function handleDrop(event: DragEvent): void {
    event.preventDefault();
    isDragOver = false;
    const raw = event.dataTransfer?.getData("application/x-sftp-entry");
    if (!raw) {
      console.warn("[SFTP][SftpPane] DOM drop without SFTP payload", { side: pane.side });
      return;
    }

    let payload: { panel: PaneSide; entry: FileEntry };
    try {
      payload = JSON.parse(raw);
    } catch {
      console.warn("[SFTP][SftpPane] DOM drop payload parse failed", { side: pane.side, raw });
      return;
    }

    if (payload.panel === pane.side) {
      return;
    }
    void sftpStore.transfer(sftpStore.pane(payload.panel), payload.entry);
  }
</script>

<div class="relative flex min-h-0 w-1/2 flex-col {pane.side === 'left' ? 'border-r border-white/10' : ''}" data-side={pane.side} aria-busy={pane.isConnecting}>
  <div class="flex min-h-0 flex-1 flex-col" inert={connectionStatus !== null}>
  {#if pane.isReady}
    <button
      type="button"
      class="group flex min-w-0 items-center gap-3 border-b border-white/8 bg-cyan-300/[0.035] px-4 py-3 text-left transition hover:bg-cyan-300/[0.08] focus-visible:bg-cyan-300/[0.08] focus-visible:outline-none"
      onclick={() => pane.isLocal ? pane.chooseMachine() : onDisconnect()}
      title={pane.isLocal ? "Switch machine" : "Disconnect and switch machine"}
      aria-label="Switch machine"
      data-testid="{pane.side}-connection-identity"
    >
      {#if pane.isLocal}
        <Laptop class="size-4 shrink-0 text-cyan-200" />
      {:else}
        <Server class="size-4 shrink-0 text-cyan-200" />
      {/if}
      <div class="min-w-0 flex-1">
        <p class="truncate text-sm font-medium text-white">{pane.label}</p>
        <p class="break-all text-xs text-slate-400">
          {pane.isLocal ? "This machine" : pane.connection ? formatHost(pane.connection) : ""}
        </p>
      </div>
      <ChevronsUpDown class="size-4 shrink-0 text-slate-500 transition group-hover:text-white" />
    </button>
    <div class="flex items-center justify-between border-b border-white/8 px-4 py-3">
      <input
        type="text"
        class="min-w-0 flex-1 truncate rounded border border-transparent bg-transparent px-1 py-0.5 text-xs text-slate-400 outline-none hover:border-white/10 focus:border-cyan-300/30 focus:bg-white/5 focus:text-white"
        value={pane.path}
        placeholder={pane.isLocal ? "~" : "/"}
        spellcheck="false"
        title="Type a path and press Enter"
        aria-label="{sideLabel} path"
        data-testid="{pane.side}-path-input"
        onkeydown={handlePathKeydown}
      />
      <div class="flex shrink-0 items-center gap-1">
        <button
          type="button"
          class="rounded-lg p-1.5 text-slate-400 hover:bg-white/10 hover:text-white"
          onclick={() => pane.refresh()}
          title="Refresh"
        >
          <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" />
            <path d="M21 3v5h-5" />
          </svg>
        </button>
        <button
          type="button"
          class="rounded-lg p-1.5 text-slate-400 hover:bg-white/10 hover:text-white"
          onclick={() => showCreateFolderDialog = true}
          title="New folder"
        >
          <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            <line x1="12" y1="11" x2="12" y2="17" />
            <line x1="9" y1="14" x2="15" y2="14" />
          </svg>
        </button>
        <button
          type="button"
          class="rounded-lg p-1.5 text-slate-400 hover:bg-white/10 hover:text-white disabled:opacity-30"
          onclick={() => renameEntry = pane.selected}
          disabled={!pane.selected}
          title="Rename"
        >
          <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M12 20h9" />
            <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z" />
          </svg>
        </button>
        <button
          type="button"
          class="rounded-lg p-1.5 text-slate-400 hover:bg-red-400/10 hover:text-red-300 disabled:opacity-30"
          onclick={() => deleteEntry = pane.selected}
          disabled={!pane.selected}
          title="Delete"
        >
          <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          </svg>
        </button>
      </div>
    </div>
    <div
      class="flex-1 overflow-auto p-2 {isDragOver ? 'bg-cyan-300/10 ring-2 ring-cyan-300/40 ring-inset rounded-lg' : ''}"
      ondragover={handleDragOver}
      ondragleave={handleDragLeave}
      ondrop={handleDrop}
      role="region"
      aria-label="{sideLabel} file drop zone"
      data-testid="{pane.side}-drop-zone"
    >
      <FileList
        files={pane.files}
        selected={pane.selected}
        loading={pane.loading}
        panelId={pane.side}
        scrollKey={`${pane.sftpSessionId ?? "local"}:${pane.path}`}
        onSelect={(entry) => pane.selected = entry}
        onNavigate={handleNavigate}
        onNavigateUp={handleNavigateUp}
        onTransfer={(entry) => void sftpStore.transfer(pane, entry)}
      />
    </div>
  {:else}
    <div class="flex min-h-0 flex-1 flex-col overflow-y-auto p-4">
      <h2 class="mb-3 text-sm font-medium text-white">Select a connection</h2>
      <div class="grid gap-2">
        <button
          type="button"
          class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.035] p-3 text-left transition hover:border-cyan-300/30 hover:bg-cyan-300/8 disabled:opacity-50"
          onclick={onUseLocal}
          disabled={pickerDisabled || otherMachine === LOCAL_MACHINE}
          title={otherMachine === LOCAL_MACHINE ? takenTitle : undefined}
        >
          <div class="grid size-8 shrink-0 place-items-center rounded-lg border border-cyan-300/20 bg-cyan-300/12 text-cyan-200">
            <Laptop class="size-4" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm font-medium text-white">Local</p>
            <p class="truncate text-xs text-slate-400">This machine</p>
          </div>
        </button>

        {#if sshSessions.length > 0}
          <h3 class="mt-2 text-xs font-medium uppercase tracking-wide text-slate-500">Open sessions</h3>
          {#each sshSessions as session (session.id)}
            <button
              type="button"
              class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.035] p-3 text-left transition hover:border-cyan-300/30 hover:bg-cyan-300/8 disabled:opacity-50"
              onclick={() => onOpenSession(session)}
              disabled={pickerDisabled || otherMachine === machineKey(session)}
              title={otherMachine === machineKey(session) ? takenTitle : undefined}
            >
              <div class="grid size-8 shrink-0 place-items-center rounded-lg border border-emerald-300/20 bg-emerald-300/12 text-emerald-200">
                <TerminalSquare class="size-4" />
              </div>
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium text-white">{session.name}</p>
                <p class="truncate text-xs text-slate-400">{formatHost(session)}</p>
              </div>
            </button>
          {/each}
        {/if}

        <h3 class="mt-2 text-xs font-medium uppercase tracking-wide text-slate-500">Saved connections</h3>
        {#each connections as connection (connection.id)}
          <button
            type="button"
            class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.035] p-3 text-left transition hover:border-cyan-300/30 hover:bg-cyan-300/8 disabled:opacity-50"
            onclick={() => onConnect(connection)}
            disabled={pickerDisabled || otherMachine === machineKey(connection)}
            title={otherMachine === machineKey(connection) ? takenTitle : undefined}
          >
            <div class="grid size-8 shrink-0 place-items-center rounded-lg border border-cyan-300/20 bg-cyan-300/12 text-cyan-200">
              <Server class="size-4" />
            </div>
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium text-white">{connection.name}</p>
              <p class="truncate text-xs text-slate-400">{formatHost(connection)}</p>
            </div>
          </button>
        {/each}
        {#if connections.length === 0}
          <p class="py-4 text-center text-sm text-slate-400">No saved connections. Add one in Connections.</p>
        {/if}
      </div>
      {#if pane.isClosing}
        <div class="mt-3 flex items-center gap-2 text-sm text-slate-400" role="status">
          <Loader2 class="size-4 animate-spin" />
          <span>Disconnecting...</span>
        </div>
      {/if}
    </div>
  {/if}
  </div>
  {#if connectionStatus}
    <ConnectionStatusOverlay
      status={connectionStatus}
      name={pane.connection?.name}
      protocol="sftp"
      error={pane.connectionError}
      trustPrompt={pane.trustPrompt}
      trustMismatch={pane.trustMismatch}
      trustError={pane.trustError}
      trustConfirming={pane.trustConfirming}
      canTrust={pane.connectionId !== null}
      {onRetry}
      {onTrust}
      {onReplaceTrust}
      onCancel={onDisconnect}
    />
  {/if}
</div>

{#if showCreateFolderDialog}
  <CreateFolderDialog
    open={true}
    onConfirm={handleCreateFolder}
    onCancel={() => showCreateFolderDialog = false}
  />
{/if}

{#if renameEntry}
  <RenameDialog
    open={true}
    currentName={renameEntry.name}
    onConfirm={handleRename}
    onCancel={() => renameEntry = null}
  />
{/if}

{#if deleteEntry}
  <DeleteConfirmDialog
    open={true}
    itemName={deleteEntry.name}
    onConfirm={handleDelete}
    onCancel={() => deleteEntry = null}
  />
{/if}
