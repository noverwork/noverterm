<script lang="ts">
  import { Download, Loader2 } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { t } from "$lib/i18n/index.svelte.js";
  import { installUpdate, updater } from "$lib/updater/auto-update.svelte.js";

  let installing = $state(false);
  let error = $state<string | null>(null);

  const update = $derived(updater.available);
  const notes = $derived(update?.body?.trim());

  function later() {
    if (!installing) {
      updater.available = null;
      error = null;
    }
  }

  async function install() {
    if (!update) return;
    installing = true;
    error = null;
    try {
      await installUpdate(update);
    } catch (cause) {
      error = String(cause);
      installing = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (update && event.key === "Escape") {
      event.preventDefault();
      later();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if update}
  <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/70 px-4 py-6">
    <div
      class="w-full max-w-md overflow-hidden rounded-[2px] border border-white/10 bg-rail text-stencil"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="update-dialog-title"
    >
      <div class="flex items-center gap-3 border-b border-white/10 px-6 py-5">
        <Download class="size-5 shrink-0 text-emerald-400" />
        <h2 id="update-dialog-title" class="text-lg font-semibold tracking-tight">
          {t("Noverterm {version} is available. Install it now?", { version: update.version })}
        </h2>
      </div>

      {#if notes || error}
        <div class="space-y-3 px-6 py-5">
          {#if notes}
            <p class="max-h-48 overflow-auto whitespace-pre-wrap text-sm text-stencil-dim">{notes}</p>
          {/if}
          {#if error}
            <p class="text-xs text-red-300" role="alert">{error}</p>
          {/if}
        </div>
      {/if}

      <div class="flex flex-wrap items-center justify-end gap-3 border-t border-white/10 bg-port px-6 py-4">
        <Button
          variant="outline"
          class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
          onclick={later}
          disabled={installing}
        >
          {t("Later")}
        </Button>
        <Button
          class="gap-2 rounded-[2px] bg-emerald-600 px-4 text-stencil hover:bg-emerald-500"
          onclick={() => void install()}
          disabled={installing}
        >
          {#if installing}<Loader2 class="size-4 animate-spin" />{/if}
          {installing ? t("Installing…") : t("Install")}
        </Button>
      </div>
    </div>
  </div>
{/if}
