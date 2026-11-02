<script lang="ts">
  import { KeyRound } from "@lucide/svelte";

  import type { SshKeyRecord, SshKeySecret } from "$lib/api/types.js";
  import { t } from "$lib/i18n/index.svelte.js";
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
  const formTitle = $derived(isEditing ? t("Edit SSH key") : t("New SSH key"));
  const submitLabel = $derived.by(() => {
    if (isLoadingSecret) {
      return t("Loading key…");
    }

    if (isSaving) {
      return isEditing ? t("Updating…") : t("Saving…");
    }

    return isEditing ? t("Update key") : t("Save key");
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();

    if (!keyName.trim()) {
      error = t("Key name is required");
      return;
    }

    if (!isEditing && !privateKey.trim()) {
      error = t("Private key is required");
      return;
    }

    if (isEditing && !privateKey.trim() && passphrase.trim()) {
      error = t("Paste a replacement private key before changing the passphrase");
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
      error = cause instanceof Error ? cause.message : t("Failed to save key");
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
        error = cause instanceof Error ? cause.message : t("Failed to load key secret");
      }
    } finally {
      if (initializedKeyId === keyId) {
        isLoadingSecret = false;
      }
    }
  }
</script>

<FormShell
  title={formTitle}
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
    title={t("Key identity")}
  >
    {#if keyRecord}
      <div class="rounded-[2px] border border-white/10 bg-rail px-3 py-2.5">
        <p class="section-title">
          {t("Current fingerprint")}
        </p>
        <p class="mt-1 truncate font-mono text-sm text-stencil-dim">
          {keyRecord.fingerprint ?? "—"}
        </p>
      </div>
    {/if}

    <div class="space-y-2">
      <label for="ssh-key-name" class="text-sm font-medium text-stencil">{t("Key name")}</label>
      <Input
        id="ssh-key-name"
        bind:value={keyName}
        placeholder={t("e.g. {example}", { example: "GitHub key" })}
        autocapitalize="none"
        autocomplete="off"
        autocorrect="off"
        spellcheck="false"
        class={fieldClass()}
        disabled={isSaving || isLoadingSecret}
      />
    </div>

    <div class="space-y-2">
      <label for="ssh-key-passphrase" class="text-sm font-medium text-stencil"
        >{t("Passphrase")} <span class="text-stencil-mute">{t("(optional)")}</span></label
      >
      <Input
        id="ssh-key-passphrase"
        type="password"
        bind:value={passphrase}
        placeholder={isLoadingSecret ? t("Loading saved passphrase…") : undefined}
        class={fieldClass()}
        disabled={isSaving || isLoadingSecret}
      />
    </div>
  </FormSection>

  <FormSection
    icon={KeyRound}
    title={t("Private key")}
  >
    <div class="space-y-2">
      <label for="ssh-key-private" class="text-sm font-medium text-stencil">{t("Private key")}</label>
      <textarea
        id="ssh-key-private"
        bind:value={privateKey}
        placeholder={isLoadingSecret ? t("Loading saved private key…") : "-----BEGIN OPENSSH PRIVATE KEY-----"}
        rows="10"
        class={textareaClass()}
        disabled={isSaving || isLoadingSecret}
      ></textarea>
    </div>
  </FormSection>
</FormShell>
