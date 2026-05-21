<script lang="ts">
  import { KeyRound } from "@lucide/svelte";

  import type { SshKeyRecord, SshKeySecret } from "$lib/api/types.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    keyRecord?: SshKeyRecord | null;
    onSave: (name: string, privateKey: string, passphrase: string) => Promise<void>;
    onUpdate: (keyId: string, name: string, privateKey?: string, passphrase?: string) => Promise<void>;
    onReveal?: (keyId: string) => Promise<SshKeySecret>;
    onCancel: () => void;
  }

  let { keyRecord = null, onSave, onUpdate, onReveal, onCancel }: Props = $props();

  let keyName = $state("");
  let privateKey = $state("");
  let passphrase = $state("");
  let error = $state<string | null>(null);
  let isSaving = $state(false);
  let isLoadingSecret = $state(false);
  let initializedKeyId = $state<string | null>(null);

  $effect(() => {
    const keyId = keyRecord?.id ?? "new";
    if (initializedKeyId === keyId) {
      return;
    }

    initializedKeyId = keyId;
    keyName = keyRecord?.name ?? "";
    privateKey = "";
    passphrase = "";
    error = null;

    if (keyRecord && onReveal) {
      void loadSavedSecret(keyRecord.id);
    }
  });

  const isEditing = $derived(keyRecord !== null);
  const formTitle = $derived(isEditing ? "Edit SSH key" : "New SSH key");
  const submitLabel = $derived.by(() => {
    if (isLoadingSecret) {
      return "Loading key…";
    }

    if (isSaving) {
      return isEditing ? "Updating…" : "Saving…";
    }

    return isEditing ? "Update key" : "Save key";
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();

    if (!keyName.trim()) {
      error = "Key name is required";
      return;
    }

    if (!isEditing && !privateKey.trim()) {
      error = "Private key is required";
      return;
    }

    if (isEditing && !privateKey.trim() && passphrase.trim()) {
      error = "Paste a replacement private key before changing the passphrase";
      return;
    }

    isSaving = true;
    error = null;

    try {
      if (keyRecord) {
        await onUpdate(
          keyRecord.id,
          keyName.trim(),
          privateKey.trim() || undefined,
          passphrase.trim() || undefined,
        );
      } else {
        await onSave(keyName.trim(), privateKey.trim(), passphrase.trim());
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to save key";
    } finally {
      isSaving = false;
    }
  }

  async function loadSavedSecret(keyId: string) {
    if (!onReveal) {
      return;
    }

    isLoadingSecret = true;
    error = null;

    try {
      const secret = await onReveal(keyId);
      if (initializedKeyId !== keyId) {
        return;
      }
      privateKey = secret.private_key;
      passphrase = secret.passphrase ?? "";
    } catch (cause) {
      if (initializedKeyId === keyId) {
        error = cause instanceof Error ? cause.message : "Failed to load key secret";
      }
    } finally {
      if (initializedKeyId === keyId) {
        isLoadingSecret = false;
      }
    }
  }
</script>

<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Credentials</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">{formTitle}</h1>
        <p class="mt-2 text-sm text-slate-500">
          {#if isEditing}
            The saved private key is loaded automatically so you can inspect or replace it.
          {:else}
            Save an SSH private key for connection profiles.
          {/if}
        </p>
      </div>

      <Button type="button" variant="ghost" size="sm" class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white" onclick={onCancel} disabled={isSaving}>
        Cancel
      </Button>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto pr-1">
      {#if error}
        <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
          {error}
        </div>
      {/if}

      <form class="mt-5 rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8 p-5 shadow-[0_16px_42px_rgb(34_211_238/0.08)]" onsubmit={handleSubmit}>
        <div class="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
          <div class="space-y-4">
            {#if keyRecord}
              <div class="rounded-2xl border border-white/8 bg-white/[0.035] p-4">
                <div class="flex items-center gap-3">
                  <div class="flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                    <KeyRound class="size-5" />
                  </div>
                  <div class="min-w-0 flex-1">
                    <p class="text-[11px] font-medium uppercase tracking-[0.16em] text-slate-500">Current fingerprint</p>
                    <p class="mt-1 truncate font-mono text-sm text-slate-300">{keyRecord.fingerprint ?? "—"}</p>
                  </div>
                </div>
              </div>
            {/if}

            <div class="space-y-2">
              <label for="ssh-key-name" class="text-sm font-medium text-slate-100">Key name</label>
              <Input
                id="ssh-key-name"
                bind:value={keyName}
                placeholder="My GitHub key"
                class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                disabled={isSaving || isLoadingSecret}
              />
            </div>

            <div class="space-y-2">
              <label for="ssh-key-passphrase" class="text-sm font-medium text-slate-100">Passphrase <span class="text-slate-500">(optional)</span></label>
              <Input
                id="ssh-key-passphrase"
                type="password"
                bind:value={passphrase}
                placeholder={isEditing ? "Loading saved passphrase if present" : "Optional passphrase"}
                class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                disabled={isSaving || isLoadingSecret}
              />
            </div>
          </div>

          <div class="space-y-4">
            <label for="ssh-key-private" class="text-sm font-medium text-slate-100">Private key</label>
            <textarea
              id="ssh-key-private"
              bind:value={privateKey}
              placeholder={isEditing ? "Loading saved private key…" : "-----BEGIN OPENSSH PRIVATE KEY-----"}
              rows="10"
              class="flex min-h-[15rem] w-full rounded-2xl border border-white/10 bg-black/20 px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20"
              disabled={isSaving || isLoadingSecret}
            ></textarea>
            {#if isEditing}
              <p class="text-xs leading-5 text-slate-500">This field shows the saved raw private key after vault decryption. Editing it will rotate the stored key on update.</p>
            {/if}

            <div class="flex flex-wrap items-center gap-2 pt-1">
              <Button type="submit" class="rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200" disabled={isSaving || isLoadingSecret}>
                {submitLabel}
              </Button>
              <Button type="button" variant="ghost" size="sm" class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white" onclick={onCancel} disabled={isSaving}>
                Cancel
              </Button>
            </div>
          </div>
        </div>
      </form>
    </div>
  </section>
</div>
