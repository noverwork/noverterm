<script lang="ts">
  import { FolderPlus } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    open: boolean;
    onConfirm: (name: string) => void;
    onCancel: () => void;
  }

  let { open, onConfirm, onCancel }: Props = $props();

  let name = $state("");
  let error = $state<string | null>(null);
  let inputRef = $state<HTMLInputElement | null>(null);
  let lastOpen = $state(false);

  const NAME_PATTERN = /^[^/\\:*?"<>|]+$/;

  $effect(() => {
    if (open && !lastOpen) {
      name = "";
      error = null;
      queueMicrotask(() => inputRef?.focus());
    }

    lastOpen = open;
  });

  function validate(value: string): string | null {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      return "Folder name is required";
    }

    if (trimmed === "." || trimmed === "..") {
      return "Folder name cannot be '.' or '..'";
    }

    if (!NAME_PATTERN.test(trimmed)) {
      return "Folder name contains invalid characters";
    }

    return null;
  }

  function handleConfirm() {
    const validationError = validate(name);
    if (validationError) {
      error = validationError;
      return;
    }

    onConfirm(name.trim());
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
      return;
    }

    if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      handleConfirm();
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
      class="ide-panel w-full max-w-md overflow-hidden text-white shadow-[0_32px_100px_rgb(0_0_0/0.62)]"
      role="dialog"
      aria-modal="true"
      aria-labelledby="create-folder-dialog-title"
      aria-describedby="create-folder-dialog-description"
    >
      <div class="border-b border-white/10 px-6 py-5">
        <div class="flex items-start gap-4">
          <div class="grid size-11 shrink-0 place-items-center rounded-2xl border border-cyan-300/20 bg-cyan-300/10 text-cyan-200 shadow-[0_0_24px_rgb(34_211_238/0.12)]">
            <FolderPlus class="size-5" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="section-title text-cyan-200/70">New folder</p>
            <h2 id="create-folder-dialog-title" class="mt-2 text-xl font-semibold tracking-tight text-white">
              Create folder
            </h2>
            <p id="create-folder-dialog-description" class="mt-2 text-sm leading-6 text-slate-400">
              Enter a name for the new folder.
            </p>
          </div>
        </div>
      </div>

      <form
        class="px-6 py-5"
        onsubmit={(event) => {
          event.preventDefault();
          handleConfirm();
        }}
      >
        <label for="create-folder-name" class="section-title block text-slate-400">
          Folder name
        </label>
        <Input
          id="create-folder-name"
          bind:ref={inputRef}
          bind:value={name}
          type="text"
          placeholder="my-folder"
          autocomplete="off"
          spellcheck={false}
          class="mt-2 h-9 rounded-xl border-white/10 bg-white/[0.04] px-3 text-sm text-white placeholder:text-slate-500"
          aria-invalid={error !== null}
          aria-describedby={error ? "create-folder-error" : undefined}
        />
        {#if error}
          <p id="create-folder-error" class="mt-2 text-xs font-medium text-red-300">
            {error}
          </p>
        {/if}

        <div class="mt-6 flex flex-wrap items-center justify-end gap-3">
          <Button
            type="button"
            variant="outline"
            class="rounded-2xl border-white/10 bg-white/[0.04] px-4 text-white hover:bg-white/[0.08]"
            onclick={onCancel}
          >
            Cancel
          </Button>
          <Button
            type="submit"
            class="rounded-2xl bg-cyan-300 px-4 text-cyan-950 hover:bg-cyan-200"
          >
            Create
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}
