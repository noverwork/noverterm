<script lang="ts">
  import { Settings, X, Download, CheckCircle, DatabaseBackup, Languages } from "@lucide/svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";

  import AiSettingsSection from "$lib/components/ai-settings-section.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { t } from "$lib/i18n/index.svelte.js";
  import { getLocale, LOCALES, setLocale, type Locale } from "$lib/i18n/index.svelte.js";
  import { checkForAppUpdate } from "$lib/updater/auto-update.js";

  let {
    open,
    onClose,
  }: {
    open: boolean;
    onClose: () => void;
  } = $props();

  let appVersion = $state("");
  let checking = $state(false);
  let updateStatus = $state<"idle" | "checking" | "up-to-date" | "error">("idle");
  let backingUp = $state(false);
  let backupNotice = $state<string | null>(null);
  let backupError = $state<string | null>(null);

  $effect(() => {
    if (open) {
      getVersion().then((v) => (appVersion = v));
    }
  });

  async function handleBackup() {
    backingUp = true;
    backupNotice = null;
    backupError = null;

    try {
      const fileName = await invoke<string>("db_backup");
      backupNotice = t("Saved to Downloads/{fileName}", { fileName });
    } catch (cause) {
      backupError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      backingUp = false;
    }
  }

  async function handleCheckUpdate() {
    checking = true;
    updateStatus = "checking";
    try {
      await checkForAppUpdate();
      updateStatus = "up-to-date";
      setTimeout(() => (updateStatus = "idle"), 3000);
    } catch {
      updateStatus = "error";
      setTimeout(() => (updateStatus = "idle"), 3000);
    } finally {
      checking = false;
    }
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4"
    onclick={(event) => event.target === event.currentTarget && onClose()}
    onkeydown={(event) => event.key === "Escape" && onClose()}
    role="dialog"
    aria-modal="true"
    aria-label={t("Settings")}
    tabindex="-1"
  >
    <div
      class="ide-panel flex max-h-[90vh] w-full max-w-lg flex-col overflow-hidden rounded-[2px] text-stencil"
    >
      <div
        class="flex shrink-0 items-center justify-between border-b border-white/10 px-6 py-4"
      >
        <div>
          <h2 class="text-xl font-semibold tracking-tight">{t("Settings")}</h2>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          class="rounded-[2px] text-stencil-mute hover:bg-white/8 hover:text-stencil"
          onclick={onClose}
          aria-label={t("Close settings")}
          title={t("Close settings")}
        >
          <X class="size-4" />
        </Button>
      </div>

      <div class="min-h-0 space-y-6 overflow-y-auto px-6 py-8">
        <div
          class="rounded-[2px] border border-white/10 bg-transparent px-5 py-5"
        >
          <div class="flex items-center gap-3">
            <div
              class="grid size-10 place-items-center rounded-[2px] border border-white/10 bg-transparent text-stencil-dim"
            >
              <Languages class="size-4" />
            </div>
            <label for="settings-language" class="flex-1 text-sm font-semibold text-stencil">
              {t("Language")}
            </label>
            <select
              id="settings-language"
              class="h-8 rounded-[2px] border border-white/10 bg-rail px-2 text-sm text-stencil focus:border-stencil focus:outline-none"
              value={getLocale()}
              onchange={(event) => setLocale(event.currentTarget.value as Locale)}
            >
              {#each LOCALES as option (option.value)}
                <option value={option.value}>{option.label}</option>
              {/each}
            </select>
          </div>
        </div>

        <AiSettingsSection />

        <div
          class="rounded-[2px] border border-white/10 bg-transparent px-5 py-5"
        >
          <div class="flex items-center gap-3">
            <div
              class="grid size-10 place-items-center rounded-[2px] border border-white/10 bg-transparent text-stencil-dim"
            >
              <Settings class="size-4" />
            </div>
            <div class="flex-1">
              <p class="text-sm font-semibold text-stencil">{t("About Noverterm")}</p>
              <p class="text-xs text-stencil-mute">{t("Version {version}", { version: appVersion })}</p>
            </div>
          </div>

          <div class="mt-4 flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              class="flex-1 rounded-[2px] border-white/10 bg-transparent text-sm text-stencil hover:border-white/20 hover:bg-white/5 disabled:opacity-50"
              onclick={handleCheckUpdate}
              disabled={checking}
            >
              {#if checking}
                <span class="animate-pulse">{t("Checking...")}</span>
              {:else if updateStatus === "up-to-date"}
                <CheckCircle class="mr-1.5 size-3.5 text-emerald-400" />
                <span class="text-emerald-400">{t("Up to date")}</span>
              {:else}
                <Download class="mr-1.5 size-3.5" />
                {t("Check for updates")}
              {/if}
            </Button>
          </div>

          {#if updateStatus === "error"}
            <p class="mt-2 text-xs text-red-300">{t("Failed to check for updates")}</p>
          {/if}
        </div>

        <div
          class="rounded-[2px] border border-white/10 bg-transparent px-5 py-5"
        >
          <div class="flex items-center gap-3">
            <div
              class="grid size-10 place-items-center rounded-[2px] border border-white/10 bg-transparent text-stencil-dim"
            >
              <DatabaseBackup class="size-4" />
            </div>
            <div class="flex-1">
              <p class="text-sm font-semibold text-stencil">{t("Backup")}</p>
              <p class="text-xs text-stencil-mute">
                {t("Connections, keys and snippets are stored in plain text — keep the file somewhere private.")}
              </p>
            </div>
          </div>

          <div class="mt-4 flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              class="flex-1 rounded-[2px] border-white/10 bg-transparent text-sm text-stencil hover:border-white/20 hover:bg-white/5 disabled:opacity-50"
              onclick={handleBackup}
              disabled={backingUp}
            >
              {#if backingUp}
                <span class="animate-pulse">{t("Backing up...")}</span>
              {:else}
                <DatabaseBackup class="mr-1.5 size-3.5" />
                {t("Back up to Downloads")}
              {/if}
            </Button>
          </div>

          {#if backupNotice}
            <p class="mt-2 text-xs text-emerald-400">{backupNotice}</p>
          {/if}
          {#if backupError}
            <p class="mt-2 text-xs text-red-300">{backupError}</p>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
