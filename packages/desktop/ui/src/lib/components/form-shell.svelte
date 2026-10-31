<script lang="ts">
  import { Loader2 } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import { Button } from "$lib/components/ui/button/index.js";

  let {
    title,
    description,
    formId,
    submitLabel,
    error = null,
    busy = false,
    submitDisabled = false,
    onsubmit,
    onCancel,
    children,
  }: {
    title: string;
    description: string;
    formId: string;
    submitLabel: string;
    error?: string | null;
    busy?: boolean;
    submitDisabled?: boolean;
    onsubmit: (event: SubmitEvent) => void;
    onCancel: () => void;
    children: Snippet;
  } = $props();
</script>

<div
  class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8"
>
  <section
    class="flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-stencil sm:p-6"
  >
    <div class="border-b border-white/10 pb-4">
      <h1 class="text-2xl font-semibold tracking-tight">{title}</h1>
      <p class="mt-2 text-sm text-stencil-mute">{description}</p>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto pr-1">
      <form id={formId} class="mx-auto max-w-3xl space-y-4 py-4" {onsubmit}>
        {#if error}
          <div
            class="rounded-[2px] border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
            role="alert"
          >
            {error}
          </div>
        {/if}

        {@render children()}
      </form>
    </div>

    <div
      class="flex shrink-0 items-center justify-end gap-2 border-t border-white/10 pt-4"
    >
      <Button
        type="button"
        variant="ghost"
        class="rounded-[2px] text-stencil-dim hover:bg-white/8 hover:text-stencil"
        onclick={onCancel}
        disabled={busy}
      >
        Cancel
      </Button>
      <Button
        type="submit"
        form={formId}
        class="rounded-[2px] bg-stencil text-port hover:bg-white"
        disabled={busy || submitDisabled}
      >
        {#if busy}
          <Loader2 class="size-4 animate-spin" />
        {/if}
        {submitLabel}
      </Button>
    </div>
  </section>
</div>
