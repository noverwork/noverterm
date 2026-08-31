<script lang="ts">
  import { KeyRound } from "@lucide/svelte";

  import type { SshKeyRecord, SshKeySecret } from "$lib/api/types.js";
  import FormSection from "$lib/components/form-section.svelte";
  import FormShell from "$lib/components/form-shell.svelte";
  import { fieldClass, textareaClass } from "$lib/components/form-styles.js";
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
  const formDescription = $derived(
    isEditing
      ? "The saved private key is loaded automatically so you can inspect or replace it."
      : "Save an SSH private key for connection profiles.",
  );
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

<FormShell
  eyebrow="Credentials"
  title={formTitle}
  description={formDescription}
  formId="ssh-key-form"
  {submitLabel}
  {error}
  busy={isSaving}
  submitDisabled={isLoadingSecret}
  onsubmit={handleSubmit}
  {onCancel}
>
  <FormSection
    icon={KeyRound}
    title="Key identity"
    hint="Name the key and unlock it if it is passphrase-protected."
  >
    {#if keyRecord}
      <div class="rounded-2xl border border-white/8 bg-black/15 px-3 py-2.5">
        <p class="text-[11px] font-medium uppercase tracking-[0.16em] text-slate-500">
          Current fingerprint
        </p>
        <p class="mt-1 truncate font-mono text-sm text-slate-300">
          {keyRecord.fingerprint ?? "—"}
        </p>
      </div>
    {/if}

    <div class="space-y-2">
      <label for="ssh-key-name" class="text-sm font-medium text-slate-100">Key name</label>
      <Input
        id="ssh-key-name"
        bind:value={keyName}
        placeholder="My GitHub key"
        autocapitalize="none"
        autocomplete="off"
        autocorrect="off"
        spellcheck="false"
        class={fieldClass()}
        disabled={isSaving || isLoadingSecret}
      />
    </div>

    <div class="space-y-2">
      <label for="ssh-key-passphrase" class="text-sm font-medium text-slate-100"
        >Passphrase <span class="text-slate-500">(optional)</span></label
      >
      <Input
        id="ssh-key-passphrase"
        type="password"
        bind:value={passphrase}
        placeholder={isEditing ? "Loading saved passphrase if present" : "Optional passphrase"}
        class={fieldClass()}
        disabled={isSaving || isLoadingSecret}
      />
    </div>
  </FormSection>

  <FormSection
    icon={KeyRound}
    title="Private key"
    hint="Paste the full PEM block, including the BEGIN and END lines."
  >
    <div class="space-y-2">
      <label for="ssh-key-private" class="text-sm font-medium text-slate-100">Private key</label>
      <textarea
        id="ssh-key-private"
        bind:value={privateKey}
        placeholder={isEditing ? "Loading saved private key…" : "-----BEGIN OPENSSH PRIVATE KEY-----"}
        rows="10"
        aria-describedby={isEditing ? "ssh-key-private-hint" : undefined}
        class={textareaClass()}
        disabled={isSaving || isLoadingSecret}
      ></textarea>
      {#if isEditing}
        <p id="ssh-key-private-hint" class="text-xs leading-5 text-slate-500">
          This field shows the saved raw private key after vault decryption. Editing it
          will rotate the stored key on update.
        </p>
      {/if}
    </div>
  </FormSection>
</FormShell>
