<script lang="ts">
  import { KeyRound, Pencil, Plus, Trash2 } from "@lucide/svelte";

  import type { SshKeyRecord } from "$lib/api/types.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    keys: SshKeyRecord[];
    onSave: (name: string, privateKey: string, passphrase: string) => Promise<void>;
    onUpdate: (keyId: string, name: string, privateKey: string, passphrase: string) => Promise<void>;
    onDelete: (key: SshKeyRecord) => Promise<void>;
  }

  let { keys, onSave, onUpdate, onDelete }: Props = $props();

  let keyName = $state("");
  let privateKey = $state("");
  let passphrase = $state("");
  let error = $state<string | null>(null);
  let isSaving = $state(false);
  let showForm = $state(false);
  let editingKey = $state<SshKeyRecord | null>(null);

  let formTitle = $derived(editingKey ? "Edit SSH key" : "New SSH key");
  let submitLabel = $derived.by(() => {
    if (isSaving) {
      return editingKey ? "Updating…" : "Saving…";
    }

    return editingKey ? "Update key" : "Save key";
  });

  function resetForm() {
    keyName = "";
    privateKey = "";
    passphrase = "";
    error = null;
    editingKey = null;
  }

  function openNewKeyForm() {
    resetForm();
    showForm = true;
  }

  function closeForm() {
    showForm = false;
    resetForm();
  }

  function handleEdit(key: SshKeyRecord) {
    editingKey = key;
    keyName = key.name;
    privateKey = "";
    passphrase = "";
    error = null;
    showForm = true;
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();

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

      closeForm();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to save key";
    } finally {
      isSaving = false;
    }
  }

  async function handleDelete(key: SshKeyRecord) {
    if (!window.confirm(`Delete SSH key "${key.name}"?`)) {
      return;
    }

    error = null;

    try {
      await onDelete(key);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to delete key";
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

      {#if !showForm}
        <Button onclick={openNewKeyForm} variant="default" size="sm" class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
          <Plus class="size-3.5" />
          Add key
        </Button>
      {/if}
    </div>

    {#if error}
      <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    {#if showForm}
      <form class="mt-5 rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8 p-5 shadow-[0_16px_42px_rgb(34_211_238/0.08)]" onsubmit={handleSubmit}>
        <div class="flex items-center justify-between gap-3">
          <h2 class="text-sm font-semibold text-cyan-100">{formTitle}</h2>
          <button type="button" class="cursor-pointer text-xs text-slate-400 transition-colors hover:text-white" onclick={closeForm}>
            Cancel
          </button>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]">
          <div class="space-y-4">
            <div class="space-y-2">
              <label for="ssh-key-name" class="text-sm font-medium text-slate-100">Key name</label>
              <Input
                id="ssh-key-name"
                bind:value={keyName}
                placeholder="My GitHub key"
                class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                disabled={isSaving}
              />
            </div>

            <div class="space-y-2">
              <label for="ssh-key-passphrase" class="text-sm font-medium text-slate-100">Passphrase <span class="text-slate-500">(optional)</span></label>
              <Input
                id="ssh-key-passphrase"
                type="password"
                bind:value={passphrase}
                placeholder="Optional passphrase"
                class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                disabled={isSaving}
              />
            </div>

            <div class="flex flex-wrap items-center gap-2 pt-1">
              <Button type="submit" class="rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200" disabled={isSaving}>
                {submitLabel}
              </Button>
              <Button type="button" variant="ghost" size="sm" class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white" onclick={closeForm} disabled={isSaving}>
                Cancel
              </Button>
            </div>
          </div>

          <div class="space-y-2">
            <label for="ssh-key-private" class="text-sm font-medium text-slate-100">Private key</label>
            <textarea
              id="ssh-key-private"
              bind:value={privateKey}
              placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
              rows="10"
              class="flex min-h-[15rem] w-full rounded-2xl border border-white/10 bg-black/20 px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20"
              disabled={isSaving}
            ></textarea>
          </div>
        </div>
      </form>
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
                <Button variant="ghost" size="xs" class="gap-1.5 rounded-xl bg-white/[0.035] text-slate-200 hover:bg-cyan-300/10 hover:text-white" onclick={() => handleEdit(key)}>
                  <Pencil class="size-3" />
                  Edit
                </Button>
                <Button variant="ghost" size="xs" class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300" onclick={() => handleDelete(key)}>
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
