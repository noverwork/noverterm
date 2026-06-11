<script lang="ts">
  import { FolderOpen, Loader2, Server, X } from "@lucide/svelte";
  import FileList from "./file-browser/FileList.svelte";
  import TransferProgress from "./file-browser/TransferProgress.svelte";
  import CreateFolderDialog from "./file-browser/CreateFolderDialog.svelte";
  import RenameDialog from "./file-browser/RenameDialog.svelte";
  import DeleteConfirmDialog from "./file-browser/DeleteConfirmDialog.svelte";
  import { sftpStore } from "$lib/stores/sftp.svelte.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import type { FileEntry } from "$lib/types/sftp.js";

  interface Props {
    connections: ConnectionConfig[];
    onConnect: (connection: ConnectionConfig) => Promise<void>;
    onDisconnect: () => Promise<void>;
  }

  let { connections, onConnect, onDisconnect }: Props = $props();

  let connecting = $state(false);
  let connectError = $state<string | null>(null);

  let showCreateFolderDialog = $state<"local" | "remote" | null>(null);
  let showRenameDialog = $state<{ panel: "local" | "remote"; entry: FileEntry } | null>(null);
  let showDeleteDialog = $state<{ panel: "local" | "remote"; entry: FileEntry } | null>(null);

  let hasLoadedLocal = $state(false);
  let dragOverPanel = $state<"local" | "remote" | null>(null);

  $effect(() => {
    if (!hasLoadedLocal) {
      hasLoadedLocal = true;
      void sftpStore.navigateLocal(sftpStore.localPath);
    }
  });

  async function handleConnect(connection: ConnectionConfig) {
    if (connecting) return;
    connecting = true;
    connectError = null;
    try {
      await onConnect(connection);
    } catch (e) {
      connectError = e instanceof Error ? e.message : String(e);
    } finally {
      connecting = false;
    }
  }

  async function handleDisconnect() {
    await onDisconnect();
  }

  function handleLocalSelect(entry: FileEntry) {
    sftpStore.selectedLocal = entry;
  }

  function handleRemoteSelect(entry: FileEntry) {
    sftpStore.selectedRemote = entry;
  }

  function handleLocalNavigate(entry: FileEntry) {
    if (entry.file_type === "Dir") {
      const newPath = sftpStore.localPath ? `${sftpStore.localPath}/${entry.name}` : entry.name;
      sftpStore.navigateLocal(newPath);
    }
  }

  function handleRemoteNavigate(entry: FileEntry) {
    if (entry.file_type === "Dir") {
      const newPath = sftpStore.remotePath ? `${sftpStore.remotePath}/${entry.name}` : entry.name;
      sftpStore.navigateRemote(newPath);
    }
  }

  async function handleUpload() {
    if (sftpStore.selectedLocal && sftpStore.isConnected) {
      await sftpStore.startUpload(sftpStore.selectedLocal);
    }
  }

  async function handleDownload() {
    if (sftpStore.selectedRemote && sftpStore.isConnected) {
      await sftpStore.startDownload(sftpStore.selectedRemote);
    }
  }

  async function handleCreateFolder(name: string) {
    if (showCreateFolderDialog === "local") {
      await sftpStore.localMkdir(name);
    } else if (showCreateFolderDialog === "remote") {
      await sftpStore.remoteMkdir(name);
    }
    showCreateFolderDialog = null;
  }

  async function handleRename(newName: string) {
    if (!showRenameDialog) return;
    const { panel, entry } = showRenameDialog;
    if (panel === "local") {
      await sftpStore.localRename(entry, newName);
    } else {
      await sftpStore.remoteRename(entry, newName);
    }
    showRenameDialog = null;
  }

  async function handleDelete() {
    if (!showDeleteDialog) return;
    const { panel, entry } = showDeleteDialog;
    if (panel === "local") {
      await sftpStore.localRemove(entry);
    } else {
      await sftpStore.remoteRemove(entry);
    }
    showDeleteDialog = null;
  }

  function handleDragOver(panel: "local" | "remote", event: DragEvent): void {
    if (!event.dataTransfer) return;
    if (panel === "remote" && !sftpStore.isConnected) return;
    if (!event.dataTransfer.types.includes("application/x-sftp-entry")) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    if (dragOverPanel !== panel) {
      dragOverPanel = panel;
    }
  }

  function handleDragLeave(panel: "local" | "remote", event: DragEvent): void {
    if (event.currentTarget instanceof HTMLElement) {
      const related = event.relatedTarget as Node | null;
      if (related && event.currentTarget.contains(related)) {
        return;
      }
    }
    if (dragOverPanel === panel) {
      dragOverPanel = null;
    }
  }

  function handleDrop(panel: "local" | "remote", event: DragEvent): void {
    event.preventDefault();
    dragOverPanel = null;
    if (!event.dataTransfer) return;
    const raw = event.dataTransfer.getData("application/x-sftp-entry");
    if (!raw) return;
    let payload: { panel: "local" | "remote"; entry: FileEntry };
    try {
      payload = JSON.parse(raw);
    } catch {
      return;
    }
    if (payload.panel === panel) return;
    if (panel === "remote" && !sftpStore.isConnected) {
      connectError = "Connect to a server before dragging files to Remote";
      return;
    }
    void sftpStore.dropTransfer(payload.panel, panel, payload.entry);
  }
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-[#080c13]/72">
  <div class="flex items-center gap-4 border-b border-white/10 px-6 py-4">
    <div class="flex items-center gap-3">
      <div class="grid size-10 place-items-center rounded-2xl border border-cyan-300/20 bg-cyan-300/12 text-cyan-100">
        <FolderOpen class="size-5" />
      </div>
      <div>
        <h1 class="text-lg font-semibold text-white">SFTP File Browser</h1>
        <p class="text-xs text-slate-400">Transfer files between local and remote machines</p>
      </div>
    </div>
  </div>

  <div class="flex min-h-0 flex-1 overflow-hidden">
    <div class="flex w-1/2 flex-col border-r border-white/10">
      <div class="flex items-center justify-between border-b border-white/8 px-4 py-3">
        <div class="flex items-center gap-2">
          <span class="text-sm font-medium text-white">Local</span>
          <span class="truncate text-xs text-slate-400">{sftpStore.localPath || "~"}</span>
        </div>
        <div class="flex items-center gap-1">
          <button
            type="button"
            class="rounded-lg p-1.5 text-slate-400 hover:bg-white/10 hover:text-white"
            onclick={() => sftpStore.refreshLocal()}
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
            onclick={() => showCreateFolderDialog = "local"}
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
            onclick={() => sftpStore.selectedLocal && (showRenameDialog = { panel: "local", entry: sftpStore.selectedLocal })}
            disabled={!sftpStore.selectedLocal}
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
            onclick={() => sftpStore.selectedLocal && (showDeleteDialog = { panel: "local", entry: sftpStore.selectedLocal })}
            disabled={!sftpStore.selectedLocal}
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
        class="flex-1 overflow-auto p-2 {dragOverPanel === 'local' ? 'bg-cyan-300/10 ring-2 ring-cyan-300/40 ring-inset rounded-lg' : ''}"
        ondragover={(event) => handleDragOver("local", event)}
        ondragleave={(event) => handleDragLeave("local", event)}
        ondrop={(event) => handleDrop("local", event)}
        data-testid="local-drop-zone"
      >
        <FileList
          files={sftpStore.localFiles}
          selected={sftpStore.selectedLocal}
          loading={sftpStore.localLoading}
          panelId="local"
          onSelect={handleLocalSelect}
          onNavigate={handleLocalNavigate}
        />
      </div>
    </div>

    <div class="flex w-1/2 flex-col">
      {#if !sftpStore.isConnected}
        <div class="flex flex-col border-b border-white/8 p-4">
          <h2 class="mb-3 text-sm font-medium text-white">Select a connection</h2>
          <div class="grid gap-2">
            {#each connections as connection}
              <button
                type="button"
                class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.035] p-3 text-left transition hover:border-cyan-300/30 hover:bg-cyan-300/8 disabled:opacity-50"
                onclick={() => handleConnect(connection)}
                disabled={connecting}
              >
                <div class="grid size-8 shrink-0 place-items-center rounded-lg border border-cyan-300/20 bg-cyan-300/12 text-cyan-200">
                  <Server class="size-4" />
                </div>
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-medium text-white">{connection.name}</p>
                  <p class="truncate text-xs text-slate-400">{connection.username}@{connection.host}:{connection.port}</p>
                </div>
              </button>
            {/each}
            {#if connections.length === 0}
              <p class="py-4 text-center text-sm text-slate-400">No saved connections. Add one in Connections.</p>
            {/if}
          </div>
          {#if connectError}
            <div class="mt-3 rounded-xl border border-red-300/20 bg-red-400/8 p-3">
              <p class="text-sm text-red-200">{connectError}</p>
            </div>
          {/if}
          {#if connecting}
            <div class="mt-3 flex items-center gap-2 text-sm text-slate-400">
              <Loader2 class="size-4 animate-spin" />
              <span>Connecting...</span>
            </div>
          {/if}
        </div>
      {:else}
        <div class="flex items-center justify-between border-b border-white/8 px-4 py-3">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium text-white">Remote</span>
            <span class="truncate text-xs text-slate-400">{sftpStore.remotePath || "/"}</span>
          </div>
          <div class="flex items-center gap-1">
            <button
              type="button"
              class="rounded-lg p-1.5 text-slate-400 hover:bg-white/10 hover:text-white"
              onclick={() => sftpStore.refreshRemote()}
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
              onclick={() => showCreateFolderDialog = "remote"}
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
              onclick={() => sftpStore.selectedRemote && (showRenameDialog = { panel: "remote", entry: sftpStore.selectedRemote })}
              disabled={!sftpStore.selectedRemote}
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
              onclick={() => sftpStore.selectedRemote && (showDeleteDialog = { panel: "remote", entry: sftpStore.selectedRemote })}
              disabled={!sftpStore.selectedRemote}
              title="Delete"
            >
              <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="3 6 5 6 21 6" />
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              </svg>
            </button>
            <button
              type="button"
              class="ml-2 rounded-lg p-1.5 text-slate-400 hover:bg-white/10 hover:text-white"
              onclick={handleDisconnect}
              title="Disconnect"
            >
              <X class="size-4" />
            </button>
          </div>
        </div>
        <div
          class="flex-1 overflow-auto p-2 {dragOverPanel === 'remote' ? 'bg-cyan-300/10 ring-2 ring-cyan-300/40 ring-inset rounded-lg' : ''}"
          ondragover={(event) => handleDragOver("remote", event)}
          ondragleave={(event) => handleDragLeave("remote", event)}
          ondrop={(event) => handleDrop("remote", event)}
          data-testid="remote-drop-zone"
        >
          <FileList
            files={sftpStore.remoteFiles}
            selected={sftpStore.selectedRemote}
            loading={sftpStore.remoteLoading}
            panelId="remote"
            onSelect={handleRemoteSelect}
            onNavigate={handleRemoteNavigate}
          />
        </div>
      {/if}
    </div>
  </div>

  {#if sftpStore.isConnected}
    <div class="flex items-center justify-between border-t border-white/10 px-6 py-3">
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="flex items-center gap-2 rounded-xl border border-cyan-300/30 bg-cyan-300/10 px-4 py-2 text-sm text-cyan-100 transition hover:bg-cyan-300/15 disabled:opacity-30"
          onclick={handleUpload}
          disabled={!sftpStore.selectedLocal}
        >
          <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
          Upload
        </button>
        <button
          type="button"
          class="flex items-center gap-2 rounded-xl border border-cyan-300/30 bg-cyan-300/10 px-4 py-2 text-sm text-cyan-100 transition hover:bg-cyan-300/15 disabled:opacity-30"
          onclick={handleDownload}
          disabled={!sftpStore.selectedRemote}
        >
          <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
          Download
        </button>
      </div>
      <TransferProgress
        transfers={sftpStore.activeTransfers}
        onCancel={(id) => sftpStore.cancelTransfer(id)}
      />
    </div>
  {/if}
</div>

{#if showCreateFolderDialog}
  <CreateFolderDialog
    open={true}
    onConfirm={handleCreateFolder}
    onCancel={() => showCreateFolderDialog = null}
  />
{/if}

{#if showRenameDialog}
  <RenameDialog
    open={true}
    currentName={showRenameDialog.entry.name}
    onConfirm={handleRename}
    onCancel={() => showRenameDialog = null}
  />
{/if}

{#if showDeleteDialog}
  <DeleteConfirmDialog
    open={true}
    itemName={showDeleteDialog.entry.name}
    onConfirm={handleDelete}
    onCancel={() => showDeleteDialog = null}
  />
{/if}
