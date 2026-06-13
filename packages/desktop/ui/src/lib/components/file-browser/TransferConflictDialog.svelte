<script lang="ts">
  import { AlertTriangle, Copy, FileWarning } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import type { TransferConflict } from "$lib/stores/sftp.svelte.js";

  interface Props {
    conflict: TransferConflict | null;
    onOverwrite: () => void;
    onRename: () => void;
    onCancel: () => void;
  }

  let { conflict, onOverwrite, onRename, onCancel }: Props = $props();

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    }
  }

  function handleBackdropClick(event: MouseEvent): void {
    if (event.target === event.currentTarget) {
      onCancel();
    }
  }
</script>

{#if conflict}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4 py-6 backdrop-blur-sm"
    role="presentation"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <div
      class="ide-panel w-full max-w-lg overflow-hidden text-white shadow-[0_32px_100px_rgb(0_0_0/0.62)]"
      role="dialog"
      aria-modal="true"
      aria-labelledby="transfer-conflict-dialog-title"
      aria-describedby="transfer-conflict-dialog-description"
      data-testid="transfer-conflict-dialog"
    >
      <div class="border-b border-white/10 px-6 py-5">
        <div class="flex items-start gap-4">
          <div class="grid size-11 shrink-0 place-items-center rounded-2xl border border-amber-300/20 bg-amber-300/10 text-amber-200 shadow-[0_0_24px_rgb(252_211_77/0.10)]">
            <AlertTriangle class="size-5" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="section-title text-amber-200/80">File already exists</p>
            <h2 id="transfer-conflict-dialog-title" class="mt-2 text-xl font-semibold tracking-tight text-white">
              Choose how to continue
            </h2>
            <p id="transfer-conflict-dialog-description" class="mt-2 text-sm leading-6 text-slate-400">
              A file named <span class="font-mono text-slate-200">{conflict.existingName}</span> already exists at the {conflict.direction === "Upload" ? "remote" : "local"} destination.
            </p>
          </div>
        </div>
      </div>

      <div class="space-y-3 px-6 py-4">
        <div class="rounded-2xl border border-white/10 bg-white/[0.035] px-4 py-3">
          <p class="text-xs font-medium uppercase tracking-[0.16em] text-slate-400">Original</p>
          <p class="mt-1 truncate font-mono text-sm text-slate-100" data-testid="transfer-conflict-original">
            {conflict.fileName}
          </p>
        </div>

        <div class="rounded-2xl border border-cyan-300/15 bg-cyan-300/[0.06] px-4 py-3">
          <p class="text-xs font-medium uppercase tracking-[0.16em] text-cyan-200/70">Rename option</p>
          <p class="mt-1 truncate font-mono text-sm text-cyan-50" data-testid="transfer-conflict-suggested">
            {conflict.suggestedName}
          </p>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-white/[0.025] px-6 py-4">
        <Button
          type="button"
          variant="outline"
          class="rounded-2xl border-white/10 bg-white/[0.04] px-4 text-white hover:bg-white/[0.08]"
          onclick={onCancel}
        >
          Cancel
        </Button>
        <Button
          type="button"
          variant="outline"
          class="gap-2 rounded-2xl border-amber-300/20 bg-amber-300/10 px-4 text-amber-100 hover:bg-amber-300/15"
          onclick={onOverwrite}
          data-testid="transfer-conflict-overwrite"
        >
          <FileWarning class="size-4" />
          Overwrite
        </Button>
        <Button
          type="button"
          class="gap-2 rounded-2xl bg-cyan-300 px-4 text-cyan-950 hover:bg-cyan-200"
          onclick={onRename}
          data-testid="transfer-conflict-rename"
        >
          <Copy class="size-4" />
          Rename to copy
        </Button>
      </div>
    </div>
  </div>
{/if}
