<script lang="ts">
  import { FolderOpen } from "@lucide/svelte";
  import { untrack } from "svelte";

  import SftpPaneView from "./sftp-pane.svelte";
  import TransferProgress from "./file-browser/TransferProgress.svelte";
  import TransferConflictDialog from "./file-browser/TransferConflictDialog.svelte";
  import { sftpStore, type SftpPane } from "$lib/stores/sftp.svelte.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import type { Session } from "$lib/stores/session.svelte.js";

  interface Props {
    connections: ConnectionConfig[];
    sshSessions?: Session[];
    onConnect: (pane: SftpPane, connection: ConnectionConfig) => Promise<void>;
    onOpenSession?: (pane: SftpPane, session: Session) => Promise<void>;
    onUseLocal?: (pane: SftpPane) => Promise<void>;
    onDisconnect: (pane: SftpPane) => Promise<void>;
    onRetry?: (pane: SftpPane) => Promise<void>;
    onTrust?: (pane: SftpPane) => Promise<void>;
    onReplaceTrust?: (pane: SftpPane) => Promise<void>;
  }

  let {
    connections,
    sshSessions = [],
    onConnect,
    onOpenSession = (pane, session) => pane.openSftp(session.id, session),
    onUseLocal = (pane) => pane.useLocal(),
    onDisconnect,
    onRetry,
    onTrust,
    onReplaceTrust,
  }: Props = $props();

  // Reload local listings once per mount; remote panes keep their listing.
  $effect(() => untrack(() => {
    for (const pane of [sftpStore.left, sftpStore.right]) {
      if (pane.isLocal) {
        void pane.navigate(pane.path);
      }
    }
  }));
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-[#080c13]/72">
  <div class="flex items-center gap-4 border-b border-white/10 px-6 py-4">
    <div class="flex items-center gap-3">
      <div class="grid size-10 place-items-center rounded-2xl border border-cyan-300/20 bg-cyan-300/12 text-cyan-100">
        <FolderOpen class="size-5" />
      </div>
      <div>
        <h1 class="text-lg font-semibold text-white">SFTP File Browser</h1>
        <p class="text-xs text-slate-400">Pick a machine on each side, then drag files across</p>
      </div>
    </div>
  </div>

  <div class="flex min-h-0 flex-1 overflow-hidden">
    {#each [sftpStore.left, sftpStore.right] as pane (pane.side)}
      <SftpPaneView
        {pane}
        {connections}
        {sshSessions}
        onConnect={(connection) => onConnect(pane, connection)}
        onOpenSession={(session) => onOpenSession(pane, session)}
        onUseLocal={() => onUseLocal(pane)}
        onDisconnect={() => onDisconnect(pane)}
        onRetry={onRetry && (() => onRetry(pane))}
        onTrust={onTrust && (() => onTrust(pane))}
        onReplaceTrust={onReplaceTrust && (() => onReplaceTrust(pane))}
      />
    {/each}
  </div>

  <TransferProgress
    transfers={sftpStore.activeTransfers}
    onCancel={(id) => sftpStore.cancelTransfer(id)}
  />
</div>

<TransferConflictDialog
  conflict={sftpStore.transferConflict}
  onOverwrite={() => sftpStore.resolveTransferConflict("overwrite")}
  onRename={() => sftpStore.resolveTransferConflict("rename")}
  onCancel={() => sftpStore.cancelTransferConflict()}
/>
