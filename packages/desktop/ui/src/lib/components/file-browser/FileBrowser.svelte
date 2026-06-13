<script lang="ts">
  import {
    ChevronRight,
    FileEdit,
    FolderPlus,
    HardDrive,
    RefreshCw,
    Server,
    Trash2,
  } from "@lucide/svelte";

  import { sftpStore } from "$lib/stores/sftp.svelte.js";
  import ErrorToast from "$lib/components/toast/ErrorToast.svelte";
  import type { FileEntry } from "$lib/types/sftp.js";

  import CreateFolderDialog from "./CreateFolderDialog.svelte";
  import DeleteConfirmDialog from "./DeleteConfirmDialog.svelte";
  import FileList from "./FileList.svelte";
  import RenameDialog from "./RenameDialog.svelte";
  import TransferConflictDialog from "./TransferConflictDialog.svelte";
  import TransferProgress from "./TransferProgress.svelte";

  interface Props {
    showTransferProgress?: boolean;
  }

  let { showTransferProgress = true }: Props = $props();

  type Side = "local" | "remote";

  let createDialogOpen = $state(false);
  let renameDialogOpen = $state(false);
  let deleteDialogOpen = $state(false);
  let activeSide = $state<Side>("local");
  let dragOverPanel = $state<Side | null>(null);
  let entryToRename = $state<FileEntry | null>(null);
  let entryToDelete = $state<FileEntry | null>(null);

  function joinChildPath(parent: string, child: string): string {
    if (parent === "" || parent === "/") {
      return `/${child}`;
    }
    return `${parent.replace(/\/+$/, "")}/${child}`;
  }

  function joinParentPath(parent: string, segmentIndex: number): string {
    const parts = breadcrumbSegments(parent);
    if (parts[0] === "/") {
      return "/";
    }
    return "/" + parts.slice(0, segmentIndex + 1).join("/");
  }

  function breadcrumbSegments(path: string): string[] {
    if (!path) {
      return [];
    }
    if (path === "/") {
      return ["/"];
    }
    const trimmed = path.startsWith("/") ? path.slice(1) : path;
    const parts = trimmed.split("/").filter((segment) => segment.length > 0);
    if (parts.length === 0) {
      return ["/"];
    }
    return parts;
  }

  function navigateTo(side: Side, path: string): void {
    if (side === "local") {
      void sftpStore.navigateLocal(path);
    } else {
      void sftpStore.navigateRemote(path);
    }
  }

  function handleNavigate(side: Side, entry: FileEntry): void {
    if (entry.file_type !== "Dir") {
      return;
    }
    const current = side === "local" ? sftpStore.localPath : sftpStore.remotePath;
    navigateTo(side, joinChildPath(current, entry.name));
  }

  function handleBreadcrumbClick(side: Side, segmentIndex: number): void {
    const current = side === "local" ? sftpStore.localPath : sftpStore.remotePath;
    if (!current || current === "/") {
      return;
    }
    navigateTo(side, joinParentPath(current, segmentIndex));
  }

  function handleRefresh(side: Side): void {
    if (side === "local") {
      void sftpStore.refreshLocal();
    } else {
      void sftpStore.refreshRemote();
    }
  }

  function handleDomDragOver(side: Side, event: DragEvent): void {
    if (!event.dataTransfer) {
      return;
    }

    if (!event.dataTransfer.types.includes("application/x-sftp-entry")) {
      return;
    }

    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    if (side === "remote" && sftpStore.sftpSessionId) {
      dragOverPanel = "remote";
    }
  }

  function handleDomDragLeave(side: Side, event: DragEvent): void {
    if (event.currentTarget instanceof HTMLElement) {
      const related = event.relatedTarget as Node | null;
      if (related && event.currentTarget.contains(related)) {
        return;
      }
    }

    if (dragOverPanel === side) {
      dragOverPanel = null;
    }
  }

  function handleDomDrop(side: Side, event: DragEvent): void {
    event.preventDefault();
    dragOverPanel = null;
    if (!event.dataTransfer) {
      console.warn("[SFTP][FileBrowser] DOM drop without dataTransfer", { side });
      return;
    }

    const types = Array.from(event.dataTransfer.types);
    const raw = event.dataTransfer.getData("application/x-sftp-entry");
    if (!raw) {
      console.warn("[SFTP][FileBrowser] DOM drop without SFTP payload", {
        side,
        types,
      });
      return;
    }

    let payload: { panel: Side; entry: FileEntry };
    try {
      payload = JSON.parse(raw);
    } catch {
      console.warn("[SFTP][FileBrowser] DOM drop payload parse failed", {
        side,
        raw,
        types,
      });
      return;
    }

    console.info("[SFTP][FileBrowser] DOM drop payload", {
      sourcePanel: payload.panel,
      targetPanel: side,
      entry: payload.entry,
      types,
      sftpSessionId: sftpStore.sftpSessionId,
      remotePath: sftpStore.remotePath,
    });

    if (payload.panel === side) {
      console.info("[SFTP][FileBrowser] ignored DOM drop within same panel", {
        side,
        entry: payload.entry,
      });
      return;
    }

    void sftpStore.dropTransfer(payload.panel, side, payload.entry);
  }

  function openCreate(side: Side): void {
    activeSide = side;
    createDialogOpen = true;
  }

  function openRename(side: Side, entry: FileEntry): void {
    activeSide = side;
    entryToRename = entry;
    renameDialogOpen = true;
  }

  function openDelete(side: Side, entry: FileEntry): void {
    activeSide = side;
    entryToDelete = entry;
    deleteDialogOpen = true;
  }

  function closeCreate(): void {
    createDialogOpen = false;
  }

  function closeRename(): void {
    renameDialogOpen = false;
    entryToRename = null;
  }

  function closeDelete(): void {
    deleteDialogOpen = false;
    entryToDelete = null;
  }

  async function handleCreate(name: string): Promise<void> {
    try {
      if (activeSide === "local") {
        await sftpStore.localMkdir(name);
      } else {
        await sftpStore.remoteMkdir(name);
      }
    } finally {
      createDialogOpen = false;
    }
  }

  async function handleRename(newName: string): Promise<void> {
    const target = entryToRename;
    if (!target) {
      renameDialogOpen = false;
      return;
    }
    try {
      if (activeSide === "local") {
        await sftpStore.localRename(target, newName);
      } else {
        await sftpStore.remoteRename(target, newName);
      }
    } finally {
      renameDialogOpen = false;
      entryToRename = null;
    }
  }

  async function handleDelete(): Promise<void> {
    const target = entryToDelete;
    if (!target) {
      deleteDialogOpen = false;
      return;
    }
    try {
      if (activeSide === "local") {
        await sftpStore.localRemove(target);
      } else {
        await sftpStore.remoteRemove(target);
      }
    } finally {
      deleteDialogOpen = false;
      entryToDelete = null;
    }
  }

  async function handleCancelTransfer(transferId: string): Promise<void> {
    await sftpStore.cancelTransfer(transferId);
  }

  async function handleTransferConflictOverwrite(): Promise<void> {
    await sftpStore.resolveTransferConflict("overwrite");
  }

  async function handleTransferConflictRename(): Promise<void> {
    await sftpStore.resolveTransferConflict("rename");
  }

  const localSegments = $derived(breadcrumbSegments(sftpStore.localPath));
  const remoteSegments = $derived(breadcrumbSegments(sftpStore.remotePath));
</script>

<div
  class="flex h-full min-h-0 flex-col gap-4 text-white"
  data-testid="file-browser"
>
  <div
    class="grid min-h-0 flex-1 grid-cols-1 gap-4 lg:grid-cols-2"
    data-testid="file-browser-panels"
  >
    <section
      class="ide-panel flex min-h-0 flex-col overflow-hidden"
      data-testid="file-browser-local-panel"
      data-side="local"
      aria-label="Local file drop zone"
      ondragover={(event) => handleDomDragOver("local", event)}
      ondragleave={(event) => handleDomDragLeave("local", event)}
      ondrop={(event) => handleDomDrop("local", event)}
    >
      <header
        class="flex items-center gap-2 border-b border-white/10 px-4 py-3"
      >
        <span
          class="grid size-8 shrink-0 place-items-center rounded-xl border border-cyan-300/20 bg-cyan-300/10 text-cyan-200"
          aria-hidden="true"
        >
          <HardDrive class="size-4" />
        </span>
        <div class="min-w-0 flex-1">
          <p class="section-title text-cyan-200/70">Local</p>
          <nav
            class="mt-0.5 flex min-w-0 flex-wrap items-center gap-1 text-xs text-slate-300"
            aria-label="Local path breadcrumb"
            data-testid="local-breadcrumb"
          >
            {#each localSegments as segment, index (segment + ":" + index)}
              {#if index > 0}
                <ChevronRight class="size-3 shrink-0 text-slate-500" aria-hidden="true" />
              {/if}
              <button
                type="button"
                class="cursor-pointer truncate rounded-md px-1.5 py-0.5 font-mono transition-colors hover:bg-white/5 hover:text-cyan-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
                data-testid="local-breadcrumb-segment"
                data-segment-index={index}
                onclick={() => handleBreadcrumbClick("local", index)}
                title={segment === "/" ? "/" : "/" + segment}
              >
                {segment === "/" ? "/" : segment}
              </button>
            {/each}
          </nav>
        </div>
        <button
          type="button"
          class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-xl border border-white/10 bg-white/5 text-slate-300 transition-colors hover:border-cyan-300/30 hover:bg-cyan-300/10 hover:text-cyan-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
          onclick={() => handleRefresh("local")}
          aria-label="Refresh local directory"
          data-testid="local-refresh"
        >
          <RefreshCw class="size-3.5" />
        </button>
      </header>

      <div class="min-h-0 flex-1 p-3 sm:p-4">
        <FileList
          files={sftpStore.localFiles}
          selected={sftpStore.selectedLocal}
          loading={sftpStore.localLoading}
          panelId="local"
          scrollKey={sftpStore.localPath}
          onSelect={(entry) => {
            sftpStore.selectedLocal = entry;
          }}
          onNavigate={(entry) => handleNavigate("local", entry)}
        />
      </div>

      <footer
        class="flex flex-wrap items-center gap-2 border-t border-white/10 bg-white/[0.02] px-3 py-2.5"
        data-testid="local-toolbar"
      >
        <button
          type="button"
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-medium text-slate-200 transition-colors hover:border-cyan-300/30 hover:bg-cyan-300/10 hover:text-cyan-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
          onclick={() => openCreate("local")}
          data-testid="local-new-folder"
        >
          <FolderPlus class="size-3.5" />
          New Folder
        </button>
        <button
          type="button"
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-medium text-slate-200 transition-colors hover:border-cyan-300/30 hover:bg-cyan-300/10 hover:text-cyan-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-white/10 disabled:hover:bg-white/5 disabled:hover:text-slate-200"
          onclick={() => {
            const selected = sftpStore.selectedLocal;
            if (selected) {
              openRename("local", selected);
            }
          }}
          disabled={!sftpStore.selectedLocal}
          data-testid="local-rename"
        >
          <FileEdit class="size-3.5" />
          Rename
        </button>
        <button
          type="button"
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-medium text-slate-200 transition-colors hover:border-red-300/30 hover:bg-red-300/10 hover:text-red-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-300/40 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-white/10 disabled:hover:bg-white/5 disabled:hover:text-slate-200"
          onclick={() => {
            const selected = sftpStore.selectedLocal;
            if (selected) {
              openDelete("local", selected);
            }
          }}
          disabled={!sftpStore.selectedLocal}
          data-testid="local-delete"
        >
          <Trash2 class="size-3.5" />
          Delete
        </button>
      </footer>
    </section>

    <section
      class="ide-panel flex min-h-0 flex-col overflow-hidden {dragOverPanel === 'remote' ? 'ring-2 ring-cyan-300/40' : ''}"
      data-testid="file-browser-remote-panel"
      data-side="remote"
      aria-label="Remote file drop zone"
      ondragover={(event) => handleDomDragOver("remote", event)}
      ondragleave={(event) => handleDomDragLeave("remote", event)}
      ondrop={(event) => handleDomDrop("remote", event)}
    >
      <header
        class="flex items-center gap-2 border-b border-white/10 px-4 py-3"
      >
        <span
          class="grid size-8 shrink-0 place-items-center rounded-xl border border-cyan-300/20 bg-cyan-300/10 text-cyan-200"
          aria-hidden="true"
        >
          <Server class="size-4" />
        </span>
        <div class="min-w-0 flex-1">
          <p class="section-title text-cyan-200/70">Remote</p>
          <nav
            class="mt-0.5 flex min-w-0 flex-wrap items-center gap-1 text-xs text-slate-300"
            aria-label="Remote path breadcrumb"
            data-testid="remote-breadcrumb"
          >
            {#if remoteSegments.length === 0}
              <span
                class="font-mono text-slate-500"
                data-testid="remote-breadcrumb-empty"
              >
                Not connected
              </span>
            {:else}
              {#each remoteSegments as segment, index (segment + ":" + index)}
                {#if index > 0}
                  <ChevronRight class="size-3 shrink-0 text-slate-500" aria-hidden="true" />
                {/if}
                <button
                  type="button"
                  class="cursor-pointer truncate rounded-md px-1.5 py-0.5 font-mono transition-colors hover:bg-white/5 hover:text-cyan-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
                  data-testid="remote-breadcrumb-segment"
                  data-segment-index={index}
                  onclick={() => handleBreadcrumbClick("remote", index)}
                  title={segment === "/" ? "/" : "/" + segment}
                >
                  {segment === "/" ? "/" : segment}
                </button>
              {/each}
            {/if}
          </nav>
        </div>
        <button
          type="button"
          class="grid size-8 shrink-0 cursor-pointer place-items-center rounded-xl border border-white/10 bg-white/5 text-slate-300 transition-colors hover:border-cyan-300/30 hover:bg-cyan-300/10 hover:text-cyan-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
          onclick={() => handleRefresh("remote")}
          aria-label="Refresh remote directory"
          data-testid="remote-refresh"
        >
          <RefreshCw class="size-3.5" />
        </button>
      </header>

      <div class="min-h-0 flex-1 p-3 sm:p-4">
        <FileList
          files={sftpStore.remoteFiles}
          selected={sftpStore.selectedRemote}
          loading={sftpStore.remoteLoading}
          panelId="remote"
          scrollKey={sftpStore.remotePath}
          onSelect={(entry) => {
            sftpStore.selectedRemote = entry;
          }}
          onNavigate={(entry) => handleNavigate("remote", entry)}
        />
      </div>

      <footer
        class="flex flex-wrap items-center gap-2 border-t border-white/10 bg-white/[0.02] px-3 py-2.5"
        data-testid="remote-toolbar"
      >
        <button
          type="button"
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-medium text-slate-200 transition-colors hover:border-cyan-300/30 hover:bg-cyan-300/10 hover:text-cyan-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
          onclick={() => openCreate("remote")}
          data-testid="remote-new-folder"
        >
          <FolderPlus class="size-3.5" />
          New Folder
        </button>
        <button
          type="button"
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-medium text-slate-200 transition-colors hover:border-cyan-300/30 hover:bg-cyan-300/10 hover:text-cyan-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-white/10 disabled:hover:bg-white/5 disabled:hover:text-slate-200"
          onclick={() => {
            const selected = sftpStore.selectedRemote;
            if (selected) {
              openRename("remote", selected);
            }
          }}
          disabled={!sftpStore.selectedRemote}
          data-testid="remote-rename"
        >
          <FileEdit class="size-3.5" />
          Rename
        </button>
        <button
          type="button"
          class="inline-flex cursor-pointer items-center gap-1.5 rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-xs font-medium text-slate-200 transition-colors hover:border-red-300/30 hover:bg-red-300/10 hover:text-red-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-300/40 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:border-white/10 disabled:hover:bg-white/5 disabled:hover:text-slate-200"
          onclick={() => {
            const selected = sftpStore.selectedRemote;
            if (selected) {
              openDelete("remote", selected);
            }
          }}
          disabled={!sftpStore.selectedRemote}
          data-testid="remote-delete"
        >
          <Trash2 class="size-3.5" />
          Delete
        </button>
      </footer>
    </section>
  </div>
</div>

{#if showTransferProgress}
  <TransferProgress
    transfers={sftpStore.activeTransfers}
    onCancel={handleCancelTransfer}
  />
{/if}

{#if sftpStore.errorQueue.length > 0}
  <div
    class="fixed bottom-4 right-4 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2"
    data-testid="error-toast-stack"
  >
    {#each sftpStore.errorQueue as error (error.id)}
      <ErrorToast
        message={error.message}
        type={error.type}
        onDismiss={() => sftpStore.dismissError(error.id)}
      />
    {/each}
  </div>
{/if}

<CreateFolderDialog
  open={createDialogOpen}
  onConfirm={handleCreate}
  onCancel={closeCreate}
/>

<RenameDialog
  open={renameDialogOpen}
  currentName={entryToRename?.name ?? ""}
  onConfirm={handleRename}
  onCancel={closeRename}
/>

<DeleteConfirmDialog
  open={deleteDialogOpen}
  itemName={entryToDelete?.name ?? ""}
  onConfirm={handleDelete}
  onCancel={closeDelete}
/>

<TransferConflictDialog
  conflict={sftpStore.transferConflict}
  onOverwrite={handleTransferConflictOverwrite}
  onRename={handleTransferConflictRename}
  onCancel={() => sftpStore.cancelTransferConflict()}
/>
