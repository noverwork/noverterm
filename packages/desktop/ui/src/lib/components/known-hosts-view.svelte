<script lang="ts">
  import { onMount } from "svelte";
  import { Fingerprint, Server, Trash2 } from "@lucide/svelte";
  import { invoke } from "@tauri-apps/api/core";

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
      error = cause instanceof Error ? cause.message : "Failed to load Known Hosts";
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
      error = cause instanceof Error ? cause.message : "Failed to remove Known Host";
    } finally {
      deletingHostKey = null;
    }
  }

  onMount(() => {
    void loadHosts();
  });
</script>

<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">SSH Trust Store</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">Known Hosts</h1>
        <p class="mt-2 text-sm text-slate-500">
          Hosts you trusted after confirming their SSH fingerprint.
        </p>
      </div>

    </div>

    {#if error}
      <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    <div class="mt-6 min-h-0 flex-1 overflow-y-auto pr-1">
      {#if isLoading}
        <div class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
          Loading...
        </div>
      {:else if hosts.length === 0}
        <div class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
          No Known Hosts yet. Confirm a host fingerprint during SSH connection to add it here.
        </div>
      {:else}
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each hosts as host (hostKey(host))}
            <article class="group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]">
              <div class="flex items-start gap-3">
                <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                  <Server class="size-5" />
                </div>

                <div class="min-w-0 flex-1">
                  <p class="break-all text-sm font-mono font-medium text-white">
                    {host.host}:{host.port}
                  </p>
                  <p class="mt-1 text-xs text-slate-500">{host.algorithm}</p>
                </div>
              </div>

              <div class="mt-4 rounded-xl border border-white/8 bg-black/20 px-3 py-2.5">
                <div class="flex items-center gap-2 text-xs font-medium uppercase tracking-[0.16em] text-slate-500">
                  <Fingerprint class="size-3.5" />
                  Fingerprint
                </div>
                <p class="mt-2 break-all font-mono text-xs leading-5 text-slate-200">
                  {host.fingerprint}
                </p>
              </div>

              <div class="mt-4 flex items-center gap-2">
                <Button
                  variant="ghost"
                  size="xs"
                  class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                  onclick={() => requestDelete(host)}
                  disabled={deletingHostKey === hostKey(host)}
                >
                  <Trash2 class="size-3" />
                  Remove
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
  title="Remove Known Host?"
  description="This removes the saved SSH fingerprint. The next connection to this host will ask you to trust it again."
  itemName={pendingDelete ? hostKey(pendingDelete) : undefined}
  confirmLabel="Remove host"
  isDeleting={deletingHostKey !== null}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDelete = null)}
/>
