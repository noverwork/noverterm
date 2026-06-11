<script lang="ts">
  import { ArrowDown, ArrowUp, X } from "@lucide/svelte";

  import type { TransferProgress as TransferProgressType } from "$lib/types/sftp.js";

  interface Props {
    transfers: Map<string, TransferProgressType>;
    onCancel: (transferId: string) => void;
  }

  let { transfers, onCancel }: Props = $props();

  const transferList = $derived([...transfers.values()]);

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

  function shortId(id: string): string {
    if (id.length <= 8) {
      return id;
    }
    return id.slice(0, 8);
  }
</script>

{#if transferList.length > 0}
  <div
    class="pointer-events-none fixed inset-x-0 bottom-4 z-40 flex justify-center px-4 sm:bottom-6 sm:px-6"
    role="region"
    aria-label="Active file transfers"
  >
    <div class="pointer-events-auto w-full max-w-md space-y-2">
      {#each transferList as transfer (transfer.transfer_id)}
        {@const progress = percent(transfer.bytes_transferred, transfer.total_bytes)}
        <div
          class="rounded-2xl border border-white/10 bg-slate-950/95 px-4 py-3 text-white shadow-2xl shadow-black/60 ring-1 ring-white/5 backdrop-blur-sm"
          data-testid="transfer-row"
          data-transfer-id={transfer.transfer_id}
        >
          <div class="flex items-center gap-3">
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

            <div class="min-w-0 flex-1">
              <div class="flex items-center justify-between gap-2">
                <p class="truncate text-sm font-medium text-white" title={transfer.transfer_id}>
                  Transfer {shortId(transfer.transfer_id)}
                </p>
                <span class="shrink-0 font-mono text-xs tabular-nums text-slate-300">
                  {progress.toFixed(0)}%
                </span>
              </div>

              <div class="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-gray-700">
                <div
                  class="h-full rounded-full bg-blue-500 transition-[width] duration-200 ease-out"
                  style:width="{progress}%"
                  role="progressbar"
                  aria-valuemin="0"
                  aria-valuemax="100"
                  aria-valuenow={progress}
                  data-testid="transfer-progress-bar"
                ></div>
              </div>

              <div class="mt-1.5 flex items-center justify-between gap-2 text-xs text-slate-400">
                <span class="truncate">
                  {transfer.direction === "Upload" ? "Uploading" : "Downloading"}
                </span>
                <span class="shrink-0 font-mono tabular-nums" data-testid="transfer-speed">
                  {formatSpeed(transfer.speed_bps)}
                </span>
              </div>
            </div>

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
        </div>
      {/each}
    </div>
  </div>
{/if}
