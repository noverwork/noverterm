<script lang="ts">
  import { Pencil } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    open: boolean;
    currentName: string;
    onConfirm: (newName: string) => void;
    onCancel: () => void;
  }

  let { open, currentName, onConfirm, onCancel }: Props = $props();

  let name = $state("");
  let error = $state<string | null>(null);
  let inputRef = $state<HTMLInputElement | null>(null);
  let initializedFor = $state<string | null>(null);

  const NAME_PATTERN = /^[^/\\:*?"<>|]+$/;

  $effect(() => {
    if (!open) {
      initializedFor = null;
      return;
    }

    if (initializedFor === currentName) {
      return;
    }

    initializedFor = currentName;
    name = currentName;
    error = null;

    queueMicrotask(() => {
      const el = inputRef;
      if (el) {
        el.focus();
        el.select();
      }
    });
  });

  function validate(value: string): string | null {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      return "Name is required";
    }

    if (trimmed === currentName) {
      return "Name is unchanged";
    }

    if (trimmed === "." || trimmed === "..") {
      return "Name cannot be '.' or '..'";
    }

    if (!NAME_PATTERN.test(trimmed)) {
      return "Name contains invalid characters";
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
      aria-labelledby="rename-dialog-title"
      aria-describedby="rename-dialog-description"
    >
      <div class="border-b border-white/10 px-6 py-5">
        <div class="flex items-start gap-4">
          <div class="grid size-11 shrink-0 place-items-center rounded-2xl border border-cyan-300/20 bg-cyan-300/10 text-cyan-200 shadow-[0_0_24px_rgb(34_211_238/0.12)]">
            <Pencil class="size-5" />
          </div>
          <div class="min-w-0 flex-1">
            <p class="section-title text-cyan-200/70">Rename</p>
            <h2 id="rename-dialog-title" class="mt-2 text-xl font-semibold tracking-tight text-white">
              Rename item
            </h2>
            <p id="rename-dialog-description" class="mt-2 text-sm leading-6 text-slate-400">
              Choose a new name for <span class="font-mono text-slate-200">{currentName}</span>.
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
        <label for="rename-input" class="section-title block text-slate-400">
          New name
        </label>
        <Input
          id="rename-input"
          bind:ref={inputRef}
          bind:value={name}
          type="text"
          autocomplete="off"
          spellcheck={false}
          class="mt-2 h-9 rounded-xl border-white/10 bg-white/[0.04] px-3 text-sm text-white"
          aria-invalid={error !== null}
          aria-describedby={error ? "rename-error" : undefined}
        />
        {#if error}
          <p id="rename-error" class="mt-2 text-xs font-medium text-red-300">
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
            Rename
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}
