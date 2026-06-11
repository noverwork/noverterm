<script lang="ts">
  import { AlertTriangle, Trash2 } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";

  interface Props {
    open: boolean;
    itemName: string;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { open, itemName, onConfirm, onCancel }: Props = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onCancel();
    }
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4 py-6 backdrop-blur-sm"
    role="presentation"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <div
      class="w-full max-w-md overflow-hidden rounded-[1.75rem] border border-red-300/20 bg-slate-950/95 text-white shadow-2xl shadow-black/50 ring-1 ring-red-300/10"
      role="dialog"
      aria-modal="true"
      aria-labelledby="delete-confirm-dialog-title"
      aria-describedby="delete-confirm-dialog-description"
    >
      <div class="border-b border-white/10 px-6 py-5">
        <div class="flex items-start gap-4">
          <div class="grid size-11 shrink-0 place-items-center rounded-2xl border border-red-300/20 bg-red-300/10 text-red-200 shadow-[0_0_24px_rgb(248_113_113/0.10)]">
            <AlertTriangle class="size-5" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="section-title text-red-200/80">Confirm delete</p>
            <h2 id="delete-confirm-dialog-title" class="mt-2 text-xl font-semibold tracking-tight text-white">
              Delete this item?
            </h2>
            <p id="delete-confirm-dialog-description" class="mt-2 text-sm leading-6 text-slate-400">
              This action cannot be undone. The item will be permanently removed.
            </p>
          </div>
        </div>
      </div>

      <div class="px-6 py-4">
        <div class="rounded-2xl border border-red-300/12 bg-red-300/[0.06] px-4 py-3">
          <p class="text-xs font-medium uppercase tracking-[0.16em] text-red-200/70">Target</p>
          <p class="mt-1 truncate font-mono text-sm text-red-50">{itemName}</p>
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
          class="gap-2 rounded-2xl bg-red-300 px-4 text-red-950 hover:bg-red-200"
          onclick={onConfirm}
        >
          <Trash2 class="size-4" />
          Delete
        </Button>
      </div>
    </div>
  </div>
{/if}
