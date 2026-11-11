<script lang="ts">
  import { Download, Loader2 } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { t } from "$lib/i18n/index.svelte.js";
  import { installUpdate, updater } from "$lib/updater/auto-update.svelte.js";

  let installing = $state(false);
  let error = $state<string | null>(null);

  const update = $derived(updater.available);
  const notes = $derived(update?.body?.trim());
  const progress = $derived(updater.progress);
  const percent = $derived(
    progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null,
  );

  function megabytes(bytes: number): string {
    return (bytes / 1024 / 1024).toFixed(1);
  }

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

      {#if notes || error || progress}
        <div class="space-y-3 px-6 py-5">
          {#if notes}
            <p class="max-h-48 overflow-auto whitespace-pre-wrap text-sm text-stencil-dim">{notes}</p>
          {/if}
          {#if progress}
            <div class="space-y-1.5">
              <div
                class="h-1.5 overflow-hidden rounded-[2px] bg-white/10"
                role="progressbar"
                aria-label={t("Downloading update")}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={percent ?? undefined}
              >
                <div
                  class="h-full bg-emerald-500 transition-[width] duration-150 motion-reduce:transition-none {percent ===
                  null
                    ? 'w-full motion-safe:animate-pulse'
                    : ''}"
                  style:width={percent === null ? undefined : `${percent}%`}
                ></div>
              </div>
              <p class="flex justify-between text-xs tabular-nums text-stencil-mute">
                <span>
                  {progress.finished ? t("Installing…") : t("Downloading update")}
                </span>
                <span>
                  {progress.total
                    ? `${megabytes(progress.downloaded)} / ${megabytes(progress.total)} MB · ${percent}%`
                    : `${megabytes(progress.downloaded)} MB`}
                </span>
              </p>
            </div>
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
          {installing
            ? progress && !progress.finished
              ? t("Downloading…")
              : t("Installing…")
            : t("Install")}
        </Button>
      </div>
    </div>
  </div>
{/if}
