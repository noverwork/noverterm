<script lang="ts">
  import { KeyRound, Pencil, Plus, Trash2 } from "@lucide/svelte";

  import type { SshKeyRecord } from "$lib/api/types.js";
  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";

  interface Props {
    keys: SshKeyRecord[];
    onNew: () => void;
    onEdit: (key: SshKeyRecord) => void;
    onDelete: (key: SshKeyRecord) => Promise<void>;
  }

  let { keys, onNew, onEdit, onDelete }: Props = $props();

  let error = $state<string | null>(null);
  let pendingDeleteKey = $state<SshKeyRecord | null>(null);
  let deletingKeyId = $state<string | null>(null);

  function requestDelete(key: SshKeyRecord) {
    pendingDeleteKey = key;
    error = null;
  }

  async function confirmDelete() {
    if (!pendingDeleteKey) {
      return;
    }

    const key = pendingDeleteKey;
    error = null;
    deletingKeyId = key.id;

    try {
      await onDelete(key);
      pendingDeleteKey = null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to delete key";
    } finally {
      deletingKeyId = null;
    }
  }

  function formatFingerprint(fingerprint: string | null): string {
    if (!fingerprint) {
      return "—";
    }

    return fingerprint.length > 24 ? `${fingerprint.slice(0, 24)}…` : fingerprint;
  }
</script>

<div class="workspace-canvas flex h-full flex-col overflow-y-auto px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-full flex-col p-5 text-white sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Credentials</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">SSH Keys</h1>
        <p class="mt-2 text-sm text-slate-500">Encrypted private keys ready for saved host profiles.</p>
      </div>

      <Button onclick={onNew} variant="default" size="sm" class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
        <Plus class="size-3.5" />
        Add key
      </Button>
    </div>

    {#if error}
      <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    <div class="mt-6 flex-1">
      {#if keys.length === 0}
        <div class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
          No SSH keys saved yet
        </div>
      {:else}
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each keys as key (key.id)}
            <article class="group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]">
              <div class="flex items-start gap-3">
                <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                  <KeyRound class="size-5" />
                </div>

                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-medium text-white">{key.name}</p>
                  <p class="mt-1 text-xs text-slate-400">{formatFingerprint(key.fingerprint)}</p>
                </div>
              </div>

              <div class="mt-4 flex items-center gap-2">
                <Button variant="ghost" size="xs" class="gap-1.5 rounded-xl bg-white/[0.035] text-slate-200 hover:bg-cyan-300/10 hover:text-white" onclick={() => onEdit(key)}>
                  <Pencil class="size-3" />
                  Edit
                </Button>
                <Button
                  variant="ghost"
                  size="xs"
                  class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                  onclick={() => requestDelete(key)}
                  disabled={deletingKeyId === key.id}
                >
                  <Trash2 class="size-3" />
                  Delete
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
  open={pendingDeleteKey !== null}
  title="Delete SSH key?"
  description="Saved connections using this key may no longer be able to authenticate. This action cannot be undone."
  itemName={pendingDeleteKey?.name}
  confirmLabel="Delete key"
  isDeleting={deletingKeyId !== null}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDeleteKey = null)}
/>
