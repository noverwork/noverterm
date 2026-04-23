<script lang="ts">
  import { KeyRound, Pencil, Plus, Trash2, X } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { SshKeyRecord } from "$lib/api/types.js";

  let {
    keys = [],
    onSave,
    onUpdate,
    onDelete,
    onClose,
  }: {
    keys?: SshKeyRecord[];
    onSave: (name: string, privateKey: string, passphrase: string) => void | Promise<void>;
    onUpdate: (keyId: string, name: string, privateKey: string, passphrase: string) => void | Promise<void>;
    onDelete: (key: SshKeyRecord) => void | Promise<void>;
    onClose: () => void;
  } = $props();

  let keyName = $state("");
  let privateKey = $state("");
  let passphrase = $state("");
  let error = $state<string | null>(null);
  let isSaving = $state(false);
  let showForm = $state(false);
  let editingKey = $state<SshKeyRecord | null>(null);

  function resetForm() {
    keyName = "";
    privateKey = "";
    passphrase = "";
    error = null;
    editingKey = null;
  }

  async function handleSave() {
    if (!keyName.trim() || !privateKey.trim()) {
      error = "Key name and private key are required";
      return;
    }
    isSaving = true;
    error = null;
    try {
      if (editingKey) {
        await onUpdate(editingKey.id, keyName.trim(), privateKey.trim(), passphrase.trim());
      } else {
        await onSave(keyName.trim(), privateKey.trim(), passphrase.trim());
      }
      resetForm();
      showForm = false;
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to save key";
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete(key: SshKeyRecord) {
    const confirmed = window.confirm(`Delete SSH key "${key.name}"?`);
    if (!confirmed) return;
    await onDelete(key);
  }

  function handleEdit(key: SshKeyRecord) {
    editingKey = key;
    keyName = key.name;
    privateKey = "";
    passphrase = "";
    error = null;
    showForm = true;
  }

  function formatFingerprint(fp: string | null): string {
    if (!fp) return "—";
    return fp.length > 24 ? fp.slice(0, 24) + "…" : fp;
  }
</script>

<div
  class="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-sm"
  onclick={(event) => event.target === event.currentTarget && onClose()}
  onkeydown={(event) => event.key === "Escape" && !showForm && onClose()}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <div class="flex h-full w-full max-w-[32rem] flex-col border-l border-white/10 bg-slate-950/96 text-white shadow-2xl">
    <div class="flex items-center justify-between border-b border-white/10 px-5 py-4 sm:px-6 sm:py-5">
      <div>
        <p class="section-title text-slate-400">SSH Keys</p>
        <h2 class="mt-2 text-xl font-semibold tracking-tight">Manage saved keys</h2>
      </div>
      <Button variant="ghost" size="icon-sm" class="text-slate-300 hover:text-white" onclick={onClose}>
        <X class="size-4" />
      </Button>
    </div>

    <div class="flex min-h-0 flex-1 flex-col overflow-y-auto px-5 py-5 sm:px-6 sm:py-6">
      {#if error}
        <div class="mb-4 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
          {error}
        </div>
      {/if}

      {#if !showForm}
        <Button onclick={() => { resetForm(); showForm = true; }} variant="default" size="sm" class="mb-5 w-full justify-start gap-2 rounded-xl">
          <Plus class="size-3.5" />
          Add SSH key
        </Button>
      {:else}
        <div class="mb-5 space-y-4 rounded-2xl border border-primary/30 bg-primary/5 p-5">
          <div class="flex items-center justify-between">
            <h3 class="text-sm font-semibold text-primary">{editingKey ? "Edit SSH key" : "New SSH key"}</h3>
            <button type="button" class="text-xs text-slate-400 hover:text-white" onclick={() => { showForm = false; resetForm(); }}>Cancel</button>
          </div>

          <div class="space-y-2">
            <label for="key-name" class="text-sm font-medium text-slate-100">Key name</label>
            <Input
              id="key-name"
              bind:value={keyName}
              placeholder="My GitHub key"
              class="border-white/10 bg-white/5 text-white placeholder:text-slate-500"
              disabled={isSaving}
            />
          </div>

          <div class="space-y-2">
            <label for="key-private" class="text-sm font-medium text-slate-100">Private key</label>
            <textarea
              id="key-private"
              bind:value={privateKey}
              placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
              rows="6"
              class="flex w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              disabled={isSaving}
            ></textarea>
          </div>

          <div class="space-y-2">
            <label for="key-passphrase" class="text-sm font-medium text-slate-100">Passphrase <span class="text-slate-500">(optional)</span></label>
            <Input
              id="key-passphrase"
              type="password"
              bind:value={passphrase}
              placeholder="Optional passphrase"
              class="border-white/10 bg-white/5 text-white placeholder:text-slate-500"
              disabled={isSaving}
            />
          </div>

          <Button onclick={handleSave} class="w-full" disabled={isSaving}>
            {#if isSaving}
              {editingKey ? "Updating…" : "Saving…"}
            {:else}
              {editingKey ? "Update key" : "Save key"}
            {/if}
          </Button>
        </div>
      {/if}

      <div class="space-y-2">
        <p class="section-title px-1">Saved keys ({keys.length})</p>

        {#if keys.length === 0}
          <div class="rounded-2xl border border-dashed border-white/10 bg-white/[0.03] px-4 py-8 text-center text-sm text-muted-foreground">
            No SSH keys saved yet. Add one to reuse across connections.
          </div>
        {:else}
          {#each keys as key (key.id)}
            <div class="group flex items-start gap-3 rounded-2xl border border-white/8 bg-white/[0.03] px-4 py-3 transition-colors hover:border-white/12 hover:bg-white/[0.06]">
              <div class="mt-0.5 flex size-9 shrink-0 items-center justify-center rounded-xl bg-white/6 text-primary">
                <KeyRound class="size-4" />
              </div>
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium">{key.name}</p>
                <p class="mt-0.5 text-xs text-slate-400">
                  {key.kind.toUpperCase()}
                  {#if key.fingerprint} · {formatFingerprint(key.fingerprint)}{/if}
                </p>
              </div>
              <div class="hidden shrink-0 items-center gap-1 group-hover:flex">
                <Button
                  variant="ghost"
                  size="icon-xs"
                  class="size-7 text-muted-foreground hover:text-white"
                  onclick={() => handleEdit(key)}
                >
                  <Pencil class="size-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  class="size-7 text-muted-foreground hover:text-destructive"
                  onclick={() => handleDelete(key)}
                >
                  <Trash2 class="size-3" />
                </Button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
</div>
