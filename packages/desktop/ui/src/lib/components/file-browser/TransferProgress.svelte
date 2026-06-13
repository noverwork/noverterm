<script lang="ts">
  import { ArrowDown, ArrowUp, X } from "@lucide/svelte";

  import type { TransferProgress as TransferProgressType } from "$lib/types/sftp.js";

  interface Props {
    transfers: Map<string, TransferProgressType>;
    onCancel: (transferId: string) => void;
  }

  let { transfers, onCancel }: Props = $props();

  const transferList = $derived([...transfers.values()]);
  const transferCount = $derived(transferList.length);

  function percent(transferred: number, total: number): number {
    if (total <= 0) {
      return 0;
    }
    const ratio = (transferred / total) * 100;
    if (ratio < 0) {
      return 0;
    }
    if (ratio > 100) {
      return 100;
    }
    return ratio;
  }

  function formatSpeed(bytesPerSecond: number): string {
    if (!Number.isFinite(bytesPerSecond) || bytesPerSecond < 0) {
      return "0 B/s";
    }
    if (bytesPerSecond < 1024) {
      return `${bytesPerSecond.toFixed(0)} B/s`;
    }
    const kilobytes = bytesPerSecond / 1024;
    if (kilobytes < 1024) {
      return `${kilobytes.toFixed(1)} KB/s`;
    }
    const megabytes = kilobytes / 1024;
    return `${megabytes.toFixed(1)} MB/s`;
  }

  function formatBytes(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) {
      return "0 B";
    }
    if (bytes < 1024) {
      return `${bytes.toFixed(0)} B`;
    }
    const kilobytes = bytes / 1024;
    if (kilobytes < 1024) {
      return `${kilobytes.toFixed(1)} KB`;
    }
    const megabytes = kilobytes / 1024;
    if (megabytes < 1024) {
      return `${megabytes.toFixed(1)} MB`;
    }
    const gigabytes = megabytes / 1024;
    return `${gigabytes.toFixed(1)} GB`;
  }

  function shortId(id: string): string {
    if (id.length <= 8) {
      return id;
    }
    return id.slice(0, 8);
  }
</script>

{#if transferList.length > 0}
  <div
    class="pointer-events-none fixed inset-x-0 bottom-0 z-50 px-3 pb-3 sm:px-5 sm:pb-4"
    role="region"
    aria-label="Active file transfers"
    data-testid="transfer-status-bar"
  >
    <div class="pointer-events-auto mx-auto w-full max-w-5xl overflow-hidden rounded-2xl border border-cyan-300/20 bg-slate-950/96 text-white shadow-2xl shadow-black/70 ring-1 ring-white/10 backdrop-blur-xl">
      <div class="flex items-center justify-between border-b border-white/10 px-3 py-2 sm:px-4">
        <div class="flex min-w-0 items-center gap-2">
          <span class="size-2 shrink-0 rounded-full bg-cyan-300 shadow-[0_0_12px_rgb(103_232_249/0.75)]" aria-hidden="true"></span>
          <p class="truncate text-xs font-semibold uppercase tracking-[0.14em] text-cyan-100">
            Transfers
          </p>
        </div>
        <span class="shrink-0 text-xs text-slate-400" data-testid="transfer-count">
          {transferCount} active
        </span>
      </div>

      <div class="max-h-44 overflow-y-auto px-2 py-2 sm:px-3">
      {#each transferList as transfer (transfer.transfer_id)}
        {@const progress = percent(transfer.bytes_transferred, transfer.total_bytes)}
        <div
          class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 rounded-xl px-2 py-2 transition-colors hover:bg-white/[0.04] sm:grid-cols-[auto_minmax(0,1fr)_auto_auto] sm:px-3"
          data-testid="transfer-row"
          data-transfer-id={transfer.transfer_id}
        >
          <div
            class="grid size-8 shrink-0 place-items-center rounded-xl border border-cyan-300/20 bg-cyan-300/10 text-cyan-200"
            aria-hidden="true"
          >
            {#if transfer.direction === "Upload"}
              <ArrowUp class="size-4" />
            {:else}
              <ArrowDown class="size-4" />
            {/if}
          </div>

          <div class="min-w-0">
            <div class="flex items-center justify-between gap-2">
              <p class="truncate text-sm font-medium text-white" title={transfer.transfer_id}>
                {transfer.direction === "Upload" ? "Uploading" : "Downloading"} {shortId(transfer.transfer_id)}
              </p>
              <span class="shrink-0 font-mono text-xs tabular-nums text-slate-300">
                {progress.toFixed(0)}%
              </span>
            </div>

            <div class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-slate-800">
              <div
                class="h-full rounded-full bg-gradient-to-r from-cyan-400 to-blue-500 transition-[width] duration-200 ease-out"
                style:width="{progress}%"
                role="progressbar"
                aria-valuemin="0"
                aria-valuemax="100"
                aria-valuenow={progress}
                data-testid="transfer-progress-bar"
              ></div>
            </div>

            <div class="mt-1 flex items-center justify-between gap-2 text-xs text-slate-400">
              <span class="truncate" data-testid="transfer-size">
                {formatBytes(transfer.bytes_transferred)} / {formatBytes(transfer.total_bytes)}
              </span>
              <span class="shrink-0 font-mono tabular-nums sm:hidden">
                {formatSpeed(transfer.speed_bps)}
              </span>
            </div>
          </div>

          <span class="hidden shrink-0 font-mono text-xs tabular-nums text-slate-300 sm:block" data-testid="transfer-speed">
            {formatSpeed(transfer.speed_bps)}
          </span>

          <button
            type="button"
            class="grid size-7 shrink-0 cursor-pointer place-items-center rounded-lg border border-white/10 bg-white/5 text-slate-300 transition-colors duration-150 hover:border-red-300/30 hover:bg-red-300/10 hover:text-red-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-300/40"
            onclick={() => onCancel(transfer.transfer_id)}
            aria-label="Cancel transfer {shortId(transfer.transfer_id)}"
            data-testid="transfer-cancel"
          >
            <X class="size-3.5" />
          </button>
        </div>
      {/each}
      </div>
    </div>
  </div>
{/if}
