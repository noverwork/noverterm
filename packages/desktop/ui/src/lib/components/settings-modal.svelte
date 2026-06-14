<script lang="ts">
  import { Settings, X, Download, CheckCircle } from "@lucide/svelte";
  import { getVersion } from "@tauri-apps/api/app";

  import { Button } from "$lib/components/ui/button/index.js";
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

  $effect(() => {
    if (open) {
      getVersion().then((v) => (appVersion = v));
    }
  });

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
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/76 px-4 backdrop-blur-xl"
    onclick={(event) => event.target === event.currentTarget && onClose()}
    onkeydown={(event) => event.key === "Escape" && onClose()}
    role="dialog"
    aria-modal="true"
    aria-label="Settings"
    tabindex="-1"
  >
    <div
      class="ide-panel flex w-full max-w-lg flex-col overflow-hidden text-white shadow-[0_32px_100px_rgb(0_0_0/0.62)]"
    >
      <div
        class="flex shrink-0 items-center justify-between border-b border-white/10 px-6 py-4"
      >
        <div>
          <p class="section-title text-cyan-200/70">Workspace preferences</p>
          <h2 class="mt-1 text-xl font-semibold tracking-tight">Settings</h2>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          class="rounded-2xl text-slate-400 hover:bg-white/8 hover:text-white"
          onclick={onClose}
          aria-label="Close settings"
          title="Close settings"
        >
          <X class="size-4" />
        </Button>
      </div>

      <div class="space-y-6 px-6 py-8">
        <div
          class="rounded-[1.35rem] border border-white/10 bg-white/[0.025] px-5 py-5"
        >
          <div class="flex items-center gap-3">
            <div
              class="grid size-10 place-items-center rounded-xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200"
            >
              <Settings class="size-4" />
            </div>
            <div class="flex-1">
              <p class="text-sm font-semibold text-white">About Noverterm</p>
              <p class="text-xs text-slate-500">Version {appVersion}</p>
            </div>
          </div>

          <div class="mt-4 flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              class="flex-1 rounded-xl border-white/10 bg-white/5 text-sm text-white hover:bg-white/10 disabled:opacity-50"
              onclick={handleCheckUpdate}
              disabled={checking}
            >
              {#if checking}
                <span class="animate-pulse">Checking...</span>
              {:else if updateStatus === "up-to-date"}
                <CheckCircle class="mr-1.5 size-3.5 text-emerald-400" />
                <span class="text-emerald-400">Up to date</span>
              {:else}
                <Download class="mr-1.5 size-3.5" />
                Check for updates
              {/if}
            </Button>
          </div>

          {#if updateStatus === "error"}
            <p class="mt-2 text-xs text-rose-400">Failed to check for updates</p>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
