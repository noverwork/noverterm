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
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4 py-6"
    role="presentation"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <div
      class="w-full max-w-md overflow-hidden rounded-[2px] border border-seal/60 bg-rail text-stencil"
      role="dialog"
      aria-modal="true"
      aria-labelledby="delete-confirm-dialog-title"
      aria-describedby="delete-confirm-dialog-description"
    >
      <div class="border-b border-white/10 px-6 py-5">
        <div class="flex items-start gap-4">
          <div class="grid size-11 shrink-0 place-items-center rounded-[2px] border border-seal/60 bg-transparent text-red-300">
            <AlertTriangle class="size-5" />
          </div>
          <div class="min-w-0 flex-1">
            <h2 id="delete-confirm-dialog-title" class="text-xl font-semibold tracking-tight text-stencil">
              Delete this item?
            </h2>
            <p id="delete-confirm-dialog-description" class="mt-2 text-sm leading-6 text-stencil-mute">
              This action cannot be undone. The item will be permanently removed.
            </p>
          </div>
        </div>
      </div>

      <div class="px-6 py-4">
        <div class="rounded-[2px] border border-seal/40 bg-rib px-4 py-3">
          <p class="section-title text-red-200/80">Target</p>
          <p class="mt-1 truncate font-mono text-sm text-stencil">{itemName}</p>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-port px-6 py-4">
        <Button
          type="button"
          variant="outline"
          class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
          onclick={onCancel}
        >
          Cancel
        </Button>
        <Button
          type="button"
          class="gap-2 rounded-[2px] bg-seal px-4 text-stencil hover:bg-[#b4432f]"
          onclick={onConfirm}
        >
          <Trash2 class="size-4" />
          Delete
        </Button>
      </div>
    </div>
  </div>
{/if}
