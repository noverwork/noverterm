<script lang="ts">
  import { KeyRound, Pencil, Plus, Search, Trash2 } from "@lucide/svelte";

  import type { SshKeyRecord } from "$lib/api/types.js";
  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

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
  let searchQuery = $state("");

  let visibleKeys = $derived.by(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return keys;
    }
    return keys.filter(
      (key) =>
        key.name.toLowerCase().includes(query) ||
        (key.fingerprint ?? "").toLowerCase().includes(query),
    );
  });

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

<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Credentials</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">SSH Keys</h1>
        <p class="mt-2 text-sm text-slate-500">Encrypted private keys ready for saved host profiles.</p>
      </div>

      <div class="flex items-center gap-2 self-start">
        <div class="relative">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-slate-500"
          />
          <Input
            type="search"
            bind:value={searchQuery}
            placeholder="Search keys"
            aria-label="Search SSH keys"
            class="h-8 w-48 rounded-2xl border-white/10 bg-white/[0.03] pl-8 text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:ring-cyan-300/20"
          />
        </div>

        <Button onclick={onNew} variant="default" size="sm" class="gap-2 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
          <Plus class="size-3.5" />
          Add key
        </Button>
      </div>
    </div>

    {#if error}
      <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    <div class="mt-6 min-h-0 flex-1 overflow-y-auto pr-1">
      {#if keys.length === 0}
        <div class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
          No SSH keys saved yet
        </div>
      {:else if visibleKeys.length === 0}
        <div class="rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
          No keys match your search.
        </div>
      {:else}
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each visibleKeys as key (key.id)}
            <ContextMenu.Root>
              <ContextMenu.Trigger class="contents">
                <div
                  role="button"
                  tabindex="0"
                  aria-label={`Edit ${key.name}`}
                  onclick={() => onEdit(key)}
                  onkeydown={(event) => {
                    if (event.key === "Enter") {
                      onEdit(key);
                    }
                  }}
                  class="group cursor-pointer rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-cyan-300/30 hover:bg-white/[0.055] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40"
                >
                  <div class="flex items-start gap-3">
                    <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                      <KeyRound class="size-5" />
                    </div>

                    <div class="min-w-0 flex-1">
                      <p class="truncate text-sm font-medium text-white">{key.name}</p>
                      <p class="mt-1 text-xs text-slate-400">{formatFingerprint(key.fingerprint)}</p>
                    </div>

                    <div
                      class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
                    >
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        class="rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                        aria-label={`Delete ${key.name}`}
                        disabled={deletingKeyId === key.id}
                        onclick={(event) => {
                          event.stopPropagation();
                          requestDelete(key);
                        }}
                      >
                        <Trash2 class="size-3.5" />
                      </Button>
                    </div>
                  </div>
                </div>
              </ContextMenu.Trigger>

              <ContextMenu.Content
                class="min-w-44 border-white/10 bg-slate-950/96 text-slate-100 shadow-2xl shadow-black/45"
              >
                <ContextMenu.Label class="max-w-56 truncate text-slate-400">
                  {key.name}
                </ContextMenu.Label>
                <ContextMenu.Separator class="bg-white/10" />
                <ContextMenu.Item
                  class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                  onclick={() => onEdit(key)}
                >
                  <Pencil class="size-3.5" />
                  Edit
                </ContextMenu.Item>
                <ContextMenu.Separator class="bg-white/10" />
                <ContextMenu.Item
                  class="cursor-pointer text-red-300 focus:bg-red-400/10 focus:text-red-200"
                  disabled={deletingKeyId === key.id}
                  onclick={() => requestDelete(key)}
                >
                  <Trash2 class="size-3.5" />
                  Delete
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Root>
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
