<script lang="ts">
  import { FileInput, Loader2 } from "@lucide/svelte";

  import type { SshConfigHost } from "../../bindings.js";
  import { importSshConfig, scanSshConfig } from "$lib/api/connections-api.js";
  import { t } from "$lib/i18n/index.svelte.js";
  import { Button } from "$lib/components/ui/button/index.js";

  interface Props {
    open: boolean;
    onImported: () => Promise<unknown>;
    onClose: () => void;
  }

  let { open, onImported, onClose }: Props = $props();

  let hosts = $state<SshConfigHost[]>([]);
  let selected = $state<string[]>([]);
  let isScanning = $state(false);
  let isImporting = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (open) void scan();
  });

  async function scan() {
    isScanning = true;
    error = null;
    try {
      hosts = await scanSshConfig();
      selected = hosts.filter((host) => !host.exists).map((host) => host.alias);
    } catch (caught) {
      hosts = [];
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      isScanning = false;
    }
  }

  async function handleImport() {
    isImporting = true;
    error = null;
    try {
      await importSshConfig(selected);
      await onImported();
      onClose();
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      isImporting = false;
    }
  }

  function toggle(alias: string) {
    selected = selected.includes(alias)
      ? selected.filter((value) => value !== alias)
      : [...selected, alias];
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget && !isImporting) onClose();
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4 py-6"
    role="presentation"
    onclick={handleBackdropClick}
  >
    <div
      class="flex max-h-full w-full max-w-lg flex-col overflow-hidden rounded-[2px] border border-white/10 bg-rail text-stencil"
      role="dialog"
      aria-modal="true"
      aria-labelledby="ssh-config-import-title"
    >
      <div class="border-b border-white/10 px-6 py-5">
        <h2 id="ssh-config-import-title" class="text-xl font-semibold tracking-tight">
          {t("Import from {path}", { path: "~/.ssh/config" })}
        </h2>
        <p class="mt-2 text-sm leading-6 text-stencil-mute">
          {t("Key files are copied into Noverterm. Hosts already saved are skipped.")}
        </p>
      </div>

      <div class="min-h-0 flex-1 overflow-y-auto px-6 py-4">
        {#if isScanning}
          <p class="flex items-center gap-2 text-sm text-stencil-mute">
            <Loader2 class="size-4 animate-spin" /> {t("Reading config…")}
          </p>
        {:else if error}
          <p class="text-sm text-destructive" role="alert">{error}</p>
        {:else if hosts.length === 0}
          <p class="text-sm text-stencil-mute">{t("No hosts found in {path}.", { path: "~/.ssh/config" })}</p>
        {:else}
          <ul class="space-y-1">
            {#each hosts as host (host.alias)}
              <li>
                <label
                  class="flex cursor-pointer items-center gap-3 rounded-[2px] px-2 py-2 hover:bg-white/5 has-[:disabled]:cursor-default has-[:disabled]:opacity-50"
                >
                  <input
                    type="checkbox"
                    checked={selected.includes(host.alias)}
                    disabled={host.exists || isImporting}
                    onchange={() => toggle(host.alias)}
                  />
                  <span class="min-w-0 flex-1">
                    <span class="block truncate text-sm font-medium">{host.alias}</span>
                    <span class="block truncate font-mono text-xs text-stencil-mute">
                      {host.username}@{host.host}:{host.port}{host.jump_alias
                        ? t(" via {alias}", { alias: host.jump_alias })
                        : ""}
                    </span>
                  </span>
                  {#if host.exists}
                    <span class="text-xs text-stencil-mute">{t("Already saved")}</span>
                  {/if}
                </label>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="flex items-center justify-end gap-3 border-t border-white/10 bg-port px-6 py-4">
        <Button
          type="button"
          variant="outline"
          class="rounded-[2px] border-white/10 bg-transparent px-4 text-stencil hover:border-white/20 hover:bg-white/5"
          onclick={onClose}
          disabled={isImporting}
        >
          {t("Cancel")}
        </Button>
        <Button
          type="button"
          class="gap-2 rounded-[2px] bg-stencil px-4 text-port hover:bg-white"
          onclick={handleImport}
          disabled={isImporting || isScanning || selected.length === 0}
        >
          {#if isImporting}
            <Loader2 class="size-4 animate-spin" />
          {:else}
            <FileInput class="size-4" />
          {/if}
          {t("Import {count}", { count: selected.length || "" })}
        </Button>
      </div>
    </div>
  </div>
{/if}
