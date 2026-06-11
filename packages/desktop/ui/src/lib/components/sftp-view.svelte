<script lang="ts">
  import { FolderOpen, Loader2, X } from "@lucide/svelte";
  import FileList from "./file-browser/FileList.svelte";
  import TransferProgress from "./file-browser/TransferProgress.svelte";
  import CreateFolderDialog from "./file-browser/CreateFolderDialog.svelte";
  import RenameDialog from "./file-browser/RenameDialog.svelte";
  import DeleteConfirmDialog from "./file-browser/DeleteConfirmDialog.svelte";
  import { sftpStore } from "$lib/stores/sftp.svelte.js";
  import type { FileEntry } from "$lib/types/sftp.js";

  interface Props {
    onConnect: (options: {
      host: string;
      port: number;
      username: string;
      password?: string;
      privateKeyPath?: string;
      passphrase?: string;
    }) => Promise<void>;
    onDisconnect: () => Promise<void>;
  }

  let { onConnect, onDisconnect }: Props = $props();

  let connecting = $state(false);
  let connectError = $state<string | null>(null);

  let host = $state("");
  let port = $state(22);
  let username = $state("");
  let password = $state("");
  let privateKeyPath = $state("");
  let passphrase = $state("");
  let authMethod = $state<"password" | "key">("password");

  let showCreateFolderDialog = $state<"local" | "remote" | null>(null);
  let showRenameDialog = $state<{ panel: "local" | "remote"; entry: FileEntry } | null>(null);
  let showDeleteDialog = $state<{ panel: "local" | "remote"; entry: FileEntry } | null>(null);

  async function handleConnect() {
    connecting = true;
    connectError = null;
    try {
      await onConnect({
        host,
        port,
        username,
        password: authMethod === "password" ? password : undefined,
        privateKeyPath: authMethod === "key" ? privateKeyPath : undefined,
        passphrase: authMethod === "key" ? passphrase : undefined,
      });
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
      <div class="flex-1 overflow-auto p-2">
        <FileList
          files={sftpStore.localFiles}
          selected={sftpStore.selectedLocal}
          loading={sftpStore.localLoading}
          onSelect={handleLocalSelect}
          onNavigate={handleLocalNavigate}
        />
      </div>
    </div>

    <div class="flex w-1/2 flex-col">
      {#if !sftpStore.isConnected}
        <div class="flex flex-col border-b border-white/8 p-6">
          <h2 class="mb-4 text-sm font-medium text-white">Connect to a remote machine</h2>
          <form class="grid gap-4" onsubmit={(e) => { e.preventDefault(); handleConnect(); }}>
            <div class="grid grid-cols-3 gap-3">
              <div class="col-span-2">
                <label class="mb-1 block text-xs text-slate-400">Host</label>
                <input
                  type="text"
                  class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder-slate-500 focus:border-cyan-300/30 focus:outline-none"
                  placeholder="example.com"
                  bind:value={host}
                  required
                />
              </div>
              <div>
                <label class="mb-1 block text-xs text-slate-400">Port</label>
                <input
                  type="number"
                  class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder-slate-500 focus:border-cyan-300/30 focus:outline-none"
                  bind:value={port}
                  required
                />
              </div>
            </div>
            <div>
              <label class="mb-1 block text-xs text-slate-400">Username</label>
              <input
                type="text"
                class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder-slate-500 focus:border-cyan-300/30 focus:outline-none"
                placeholder="root"
                bind:value={username}
                required
              />
            </div>
            <div>
              <label class="mb-2 block text-xs text-slate-400">Authentication</label>
              <div class="flex gap-4">
                <label class="flex items-center gap-2 text-sm text-white">
                  <input type="radio" bind:group={authMethod} value="password" class="text-cyan-300" />
                  Password
                </label>
                <label class="flex items-center gap-2 text-sm text-white">
                  <input type="radio" bind:group={authMethod} value="key" class="text-cyan-300" />
                  Private Key
                </label>
              </div>
            </div>
            {#if authMethod === "password"}
              <div>
                <label class="mb-1 block text-xs text-slate-400">Password</label>
                <input
                  type="password"
                  class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder-slate-500 focus:border-cyan-300/30 focus:outline-none"
                  bind:value={password}
                />
              </div>
            {:else}
              <div>
                <label class="mb-1 block text-xs text-slate-400">Private Key Path</label>
                <input
                  type="text"
                  class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder-slate-500 focus:border-cyan-300/30 focus:outline-none"
                  placeholder="~/.ssh/id_rsa"
                  bind:value={privateKeyPath}
                />
              </div>
              <div>
                <label class="mb-1 block text-xs text-slate-400">Passphrase (optional)</label>
                <input
                  type="password"
                  class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder-slate-500 focus:border-cyan-300/30 focus:outline-none"
                  bind:value={passphrase}
                />
              </div>
            {/if}
            <button
              type="submit"
              class="flex items-center justify-center gap-2 rounded-xl border border-cyan-300/30 bg-cyan-300/10 px-4 py-2 text-sm text-cyan-100 transition hover:bg-cyan-300/15 disabled:opacity-30"
              disabled={connecting}
            >
              {#if connecting}
                <Loader2 class="size-4 animate-spin" />
              {/if}
              Connect
            </button>
          </form>
          {#if connectError}
            <div class="mt-4 rounded-xl border border-red-300/20 bg-red-400/8 p-3">
              <p class="text-sm text-red-200">{connectError}</p>
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
        <div class="flex-1 overflow-auto p-2">
          <FileList
            files={sftpStore.remoteFiles}
            selected={sftpStore.selectedRemote}
            loading={sftpStore.remoteLoading}
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
