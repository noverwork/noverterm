<script lang="ts">
  import { onMount } from "svelte";
  import { Fingerprint, Server, Trash2 } from "@lucide/svelte";
  import { invoke } from "@tauri-apps/api/core";

  import { t } from "$lib/i18n/index.svelte.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";

  interface TrustedSshHost {
    host: string;
    port: number;
    algorithm: string;
    fingerprint: string;
  }

  let hosts = $state<TrustedSshHost[]>([]);
  let error = $state<string | null>(null);
  let isLoading = $state(true);
  let pendingDelete = $state<TrustedSshHost | null>(null);
  let deletingHostKey = $state<string | null>(null);

  function hostKey(host: TrustedSshHost): string {
    return `${host.host}:${host.port}`;
  }

  async function loadHosts() {
    isLoading = true;
    error = null;
    try {
      const result = await invoke<{ hosts: TrustedSshHost[] }>("known_hosts_get");
      hosts = result.hosts;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : t("Failed to load Known Hosts");
    } finally {
      isLoading = false;
    }
  }

  function requestDelete(host: TrustedSshHost) {
    pendingDelete = host;
    error = null;
  }

  async function confirmDelete() {
    if (!pendingDelete) return;

    const host = pendingDelete;
    const key = hostKey(host);
    error = null;
    deletingHostKey = key;
    try {
      const result = await invoke<{ hosts: TrustedSshHost[] }>("known_hosts_remove", {
        host: host.host,
        port: host.port,
      });
      hosts = result.hosts;
      pendingDelete = null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : t("Failed to remove Known Host");
    } finally {
      deletingHostKey = null;
    }
  }

  onMount(() => {
    void loadHosts();
  });
</script>

<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-stencil sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <h1 class="text-2xl font-semibold tracking-tight">{t("Known Hosts")}</h1>
      </div>

    </div>

    {#if error}
      <div class="mt-5 rounded-[2px] border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    <div class="mt-6 min-h-0 flex-1 overflow-y-auto pr-1">
      {#if isLoading}
        <div class="flex h-full min-h-[16rem] items-center justify-center rounded-[2px] border border-dashed border-white/10 bg-transparent px-4 py-8 text-center text-sm text-muted-foreground">
          {t("Loading...")}
        </div>
      {:else if hosts.length === 0}
        <div class="flex h-full min-h-[16rem] items-center justify-center rounded-[2px] border border-dashed border-white/10 bg-transparent px-4 py-8 text-center text-sm text-muted-foreground">
          {t("No Known Hosts yet. Confirm a host fingerprint during SSH connection to add it here.")}
        </div>
      {:else}
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each hosts as host (hostKey(host))}
            <article class="group rounded-[2px] border border-white/10 bg-rib px-4 py-4 transition hover:border-white/20 hover:bg-[#1f252c] ">
              <div class="flex items-start gap-3">
                <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-[2px] border border-white/10 bg-transparent text-stencil-dim">
                  <Server class="size-5" />
                </div>

                <div class="min-w-0 flex-1">
                  <p class="break-all text-sm font-mono font-medium text-stencil">
                    {host.host}:{host.port}
                  </p>
                  <p class="mt-1 text-xs text-stencil-mute">{host.algorithm}</p>
                </div>
              </div>

              <div class="mt-4 rounded-[2px] border border-white/10 bg-rail px-3 py-2.5">
                <div class="flex items-center gap-2 section-title">
                  <Fingerprint class="size-3.5" />
                  {t("Fingerprint")}
                </div>
                <p class="mt-2 break-all font-mono text-xs leading-5 text-stencil">
                  {host.fingerprint}
                </p>
              </div>

              <div class="mt-4 flex items-center gap-2">
                <Button
                  variant="ghost"
                  size="xs"
                  class="gap-1.5 rounded-[2px] text-stencil-mute hover:bg-seal/20 hover:text-red-300"
                  onclick={() => requestDelete(host)}
                  disabled={deletingHostKey === hostKey(host)}
                >
                  <Trash2 class="size-3" />
                  {t("Remove")}
                </Button>
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</div>

<DeleteConfirmDialog
  open={pendingDelete !== null}
  title={t("Remove Known Host?")}
  description={t("This removes the saved SSH fingerprint. The next connection to this host will ask you to trust it again.")}
  itemName={pendingDelete ? hostKey(pendingDelete) : undefined}
  confirmLabel={t("Remove host")}
  isDeleting={deletingHostKey !== null}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDelete = null)}
/>
