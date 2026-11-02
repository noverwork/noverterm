<script lang="ts">
  import { t, tParts } from "$lib/i18n/index.svelte.js";
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

  const descParts = $derived(
    tParts("A {kind} named {name} already exists in {destination}.", ["name", "destination"], {
      kind: conflict?.isDirectory ? t("folder") : t("file"),
    }),
  );

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
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4 py-6"
    role="presentation"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <div
      class="ide-panel w-full max-w-lg overflow-hidden rounded-[2px] text-stencil"
      role="dialog"
      aria-modal="true"
      aria-labelledby="transfer-conflict-dialog-title"
      aria-describedby="transfer-conflict-dialog-description"
      data-testid="transfer-conflict-dialog"
    >
      <div class="border-b border-white/10 px-6 py-5">
        <div class="flex items-start gap-4">
          <div class="grid size-11 shrink-0 place-items-center rounded-[2px] border border-[#c9952e]/60 bg-transparent text-[#e6c27a]">
            <AlertTriangle class="size-5" />
          </div>
          <div class="min-w-0 flex-1">
            <h2 id="transfer-conflict-dialog-title" class="text-xl font-semibold tracking-tight text-stencil">
              {t("{kind} already exists", { kind: conflict.isDirectory ? t("Folder") : t("File") })}
            </h2>
            <p id="transfer-conflict-dialog-description" class="mt-2 text-sm leading-6 text-stencil-mute">
              {#each descParts as part, index (index)}{#if index % 2 === 0}{part}{:else if part === "name"}<span class="font-mono text-stencil">{conflict.existingName}</span>{:else}<span class="text-stencil">{conflict.destination}</span>{/if}{/each}
            </p>
          </div>
        </div>
      </div>

      <div class="space-y-3 px-6 py-4">
        <div class="rounded-[2px] border border-white/10 bg-rib px-4 py-3">
          <p class="section-title">{t("Original")}</p>
          <p class="mt-1 truncate font-mono text-sm text-stencil" data-testid="transfer-conflict-original">
            {conflict.fileName}
          </p>
        </div>

        {#if conflict.isDirectory}
          <div class="rounded-[2px] border border-[#c9952e]/40 bg-rib px-4 py-3" data-testid="transfer-conflict-inner">
            {#if conflict.conflictingFiles === null}
              <p class="section-title text-[#e6c27a]">{t("Checking folder contents…")}</p>
            {:else if conflict.conflictingFiles.length === 0}
              <p class="section-title text-[#e6c27a]">{t("No files inside would be overwritten")}</p>
            {:else}
              <p class="section-title text-[#e6c27a]">
                {conflict.conflictingFiles.length === 1 ? t("Overwrite replaces {count} existing file", { count: 1 }) : t("Overwrite replaces {count} existing files", { count: conflict.conflictingFiles.length })}
              </p>
              <ul class="mt-2 max-h-40 space-y-1 overflow-y-auto font-mono text-sm text-stencil">
                {#each conflict.conflictingFiles as path (path)}
                  <li class="truncate">{path}</li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}

        <div class="rounded-[2px] border border-white/10 bg-rib px-4 py-3">
          <p class="section-title">{t("Rename option")}</p>
          <p class="mt-1 truncate font-mono text-sm text-stencil" data-testid="transfer-conflict-suggested">
            {conflict.suggestedName}
          </p>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-port px-6 py-4">
        <Button
          type="button"
          variant="outline"
          class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
          onclick={onCancel}
        >
          {t("Cancel")}
        </Button>
        <Button
          type="button"
          variant="outline"
          class="gap-2 rounded-[2px] border-[#c9952e]/60 bg-transparent px-4 text-[#e6c27a] hover:bg-[#c9952e]/10"
          onclick={onOverwrite}
          data-testid="transfer-conflict-overwrite"
        >
          <FileWarning class="size-4" />
          {t("Overwrite")}
        </Button>
        <Button
          type="button"
          class="gap-2 rounded-[2px] bg-stencil px-4 text-port hover:bg-white"
          onclick={onRename}
          data-testid="transfer-conflict-rename"
        >
          <Copy class="size-4" />
          {t("Rename to copy")}
        </Button>
      </div>
    </div>
  </div>
{/if}
