<script lang="ts">
  import { ChevronDown, Eye, EyeOff, KeyRound, Server } from "@lucide/svelte";
  import { superForm } from "sveltekit-superforms";
  import { zod4 } from "sveltekit-superforms/adapters";

  import { connectionSchema, type ConnectionForm } from "$lib/schemas/index.js";
  import type {
  ConnectionConfig,
  SaveConnectionInput,
} from "$lib/app-data-types.js";
  import type { SshKeyRecord } from "$lib/api/types.js";
  import FormSection from "$lib/components/form-section.svelte";
  import FormShell from "$lib/components/form-shell.svelte";
  import { fieldClass, SELECT_CLASS, textareaClass } from "$lib/components/form-styles.js";
  import { Input } from "$lib/components/ui/input/index.js";

  let {
    connection = null,
    keys = [],
    onSave,
    onCancel,
    error = null,
    isSaving = false,
  }: {
    connection?: ConnectionConfig | null;
    keys?: SshKeyRecord[];
    onSave: (connection: SaveConnectionInput) => void | Promise<void>;
    onCancel: () => void;
    error?: string | null;
    isSaving?: boolean;
  } = $props();

  const form = superForm<ConnectionForm>(
    {
      name: "",
      host: "",
      port: 22,
      username: "",
      password: "",
      privateKey: "",
      passphrase: "",
      useSshKey: false,
      keyMode: "saved",
      selectedKeyId: null,
      existingPassword: false,
    },
    { validators: zod4(connectionSchema) },
  );

  const { form: formData, errors } = form;

  let keyName = $state("");
  let initializedConnectionId = $state<string | null>(null);
  let showPassword = $state(false);

  function segmentClass(active: boolean) {
    return active
      ? "flex-1 cursor-pointer rounded-xl bg-cyan-300/12 px-3 py-2 text-sm font-medium text-cyan-100 transition-colors"
      : "flex-1 cursor-pointer rounded-xl px-3 py-2 text-sm text-slate-400 transition-colors hover:bg-white/6 hover:text-white";
  }

  $effect(() => {
    const connectionId = connection?.id ?? "new";
    if (initializedConnectionId === connectionId) {
      return;
    }

    initializedConnectionId = connectionId;
    keyName = "";
    showPassword = false;
    $formData = {
      name: connection?.name ?? "",
      host: connection?.host ?? "",
      port: connection?.port ?? 22,
      username: connection?.username ?? "",
      password: connection ? (savedPasswordFor(connection) ?? "") : "",
      privateKey: "",
      passphrase: "",
      useSshKey: Boolean(connection?.sshKeyId),
      keyMode: "saved",
      selectedKeyId: connection?.sshKeyId ?? null,
      existingPassword: Boolean(connection?.hasPassword),
    };
  });

  const formTitle = $derived(connection ? "Edit connection" : "New connection");
  const submitLabel = $derived.by(() => {
    if (isSaving) {
      return "Saving…";
    }

    return connection ? "Save changes" : "Save";
  });

  function savedPasswordFor(connection: ConnectionConfig): string | null {
    if (
      connection.auth?.kind === "password" ||
      connection.auth?.kind === "public_key_and_password"
    ) {
      return connection.auth.password;
    }

    return null;
  }

  const selectedKey = $derived(
    keys.find((key) => key.id === $formData.selectedKeyId) ?? null,
  );

  const selectedKeyWillBeUsed = $derived(
    $formData.useSshKey &&
      $formData.keyMode === "saved" &&
      !!$formData.selectedKeyId,
  );

  const pastingNewKey = $derived(
    $formData.useSshKey && $formData.keyMode === "new",
  );

  function toggleSshKey() {
    $formData.useSshKey = !$formData.useSshKey;
    if (!$formData.useSshKey) {
      $formData.privateKey = "";
      $formData.passphrase = "";
      $formData.keyMode = "saved";
      $formData.selectedKeyId = null;
      keyName = "";
    }
  }

  function handleKeyModeChange(mode: "saved" | "new") {
    $formData.keyMode = mode;
    if (mode === "new") {
      $formData.selectedKeyId = null;
    } else {
      keyName = "";
    }
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();

    const result = connectionSchema.safeParse($formData);
    if (!result.success || isSaving) {
      return;
    }

    const rawKeyId =
      $formData.keyMode === "saved" ? $formData.selectedKeyId : null;
    const effectiveKeyId = rawKeyId && rawKeyId.trim() ? rawKeyId : null;
    const preservedEncryptedPassword =
      connection?.hasPassword && !$formData.password.trim()
        ? connection.auth?.kind === "password" ||
          connection.auth?.kind === "public_key_and_password"
          ? connection.auth.password
          : null
        : null;

    await onSave({
      ...(connection?.id ? { id: connection.id } : {}),
      name: $formData.name.trim(),
      groupId: connection?.groupId ?? null,
      host: $formData.host.trim(),
      port: $formData.port,
      username: $formData.username.trim(),
      ...($formData.password.trim()
        ? { password: $formData.password.trim() }
        : {}),
      ...(preservedEncryptedPassword ? { preservedEncryptedPassword } : {}),
      ...($formData.useSshKey &&
      $formData.keyMode === "new" &&
      $formData.privateKey.trim()
        ? { privateKey: $formData.privateKey.trim() }
        : {}),
      ...($formData.useSshKey &&
      $formData.keyMode === "new" &&
      $formData.passphrase.trim()
        ? { passphrase: $formData.passphrase.trim() }
        : {}),
      ...(keyName.trim() ? { keyName: keyName.trim() } : {}),
      existingKeyId: effectiveKeyId,
    });
  }
</script>

<FormShell
  eyebrow="Inventory"
  title={formTitle}
  description="Save host details and encrypted credentials for terminal and forwarding workflows."
  formId="connection-form"
  {submitLabel}
  {error}
  busy={isSaving}
  onsubmit={handleSubmit}
  {onCancel}
>
  <FormSection
    icon={Server}
    title="Endpoint"
    hint="Name the host and define where the SSH session connects."
  >
    <div class="space-y-2">
      <label for="conn-name" class="text-sm font-medium text-slate-100">Name</label>
      <Input
        id="conn-name"
        bind:value={$formData.name}
        placeholder="Awesome host"
        autocapitalize="none"
        autocomplete="off"
        autocorrect="off"
        spellcheck="false"
        aria-invalid={$errors.name ? "true" : undefined}
        aria-describedby={$errors.name ? "conn-name-error" : undefined}
        class={fieldClass($errors.name)}
        disabled={isSaving}
      />
      {#if $errors.name}
        <p id="conn-name-error" class="text-xs text-destructive" role="alert">
          {$errors.name}
        </p>
      {/if}
    </div>

    <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_7rem]">
      <div class="space-y-2">
        <label for="conn-host" class="text-sm font-medium text-slate-100">Hostname</label>
        <Input
          id="conn-host"
          bind:value={$formData.host}
          placeholder="prod.example.com"
          autocapitalize="none"
          autocomplete="off"
          autocorrect="off"
          spellcheck="false"
          aria-invalid={$errors.host ? "true" : undefined}
          aria-describedby={$errors.host ? "conn-host-error" : undefined}
          class={fieldClass($errors.host, "font-mono")}
          disabled={isSaving}
        />
        {#if $errors.host}
          <p id="conn-host-error" class="text-xs text-destructive" role="alert">
            {$errors.host}
          </p>
        {/if}
      </div>

      <div class="space-y-2">
        <label for="conn-port" class="text-sm font-medium text-slate-100">Port</label>
        <Input
          id="conn-port"
          type="number"
          inputmode="numeric"
          min="1"
          max="65535"
          bind:value={$formData.port}
          aria-invalid={$errors.port ? "true" : undefined}
          aria-describedby={$errors.port ? "conn-port-error" : undefined}
          class={fieldClass($errors.port, "font-mono")}
          disabled={isSaving}
        />
        {#if $errors.port}
          <p id="conn-port-error" class="text-xs text-destructive" role="alert">
            {$errors.port}
          </p>
        {/if}
      </div>
    </div>

    <div class="space-y-2">
      <label for="conn-username" class="text-sm font-medium text-slate-100">Username</label>
      <Input
        id="conn-username"
        bind:value={$formData.username}
        placeholder="deploy"
        autocapitalize="none"
        autocomplete="username"
        autocorrect="off"
        spellcheck="false"
        aria-invalid={$errors.username ? "true" : undefined}
        aria-describedby={$errors.username ? "conn-username-error" : undefined}
        class={fieldClass($errors.username, "font-mono")}
        disabled={isSaving}
      />
      {#if $errors.username}
        <p id="conn-username-error" class="text-xs text-destructive" role="alert">
          {$errors.username}
        </p>
      {/if}
    </div>
  </FormSection>

  <FormSection
    icon={KeyRound}
    title="Authentication"
    hint="Credentials are encrypted at rest. Use a password, an SSH key, or both."
  >
    <div class="space-y-2">
      <label for="conn-password" class="text-sm font-medium text-slate-100">Password</label>
      <div class="relative">
        <Input
          id="conn-password"
          type={showPassword ? "text" : "password"}
          bind:value={$formData.password}
          placeholder={connection?.hasPassword
            ? "Saved password"
            : "Leave blank for key-only auth"}
          aria-invalid={$errors.password ? "true" : undefined}
          aria-describedby={$errors.password ? "conn-password-error" : undefined}
          class={fieldClass($errors.password, "pr-11")}
          disabled={isSaving}
        />
        <button
          type="button"
          class="absolute right-3 top-1/2 flex size-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-lg text-slate-400 transition-colors hover:bg-white/8 hover:text-white disabled:cursor-not-allowed disabled:opacity-50"
          onclick={() => {
            showPassword = !showPassword;
          }}
          disabled={isSaving || !$formData.password}
          aria-label={showPassword ? "Hide password" : "Show password"}
          title={showPassword ? "Hide password" : "Show password"}
        >
          {#if showPassword}
            <EyeOff class="size-4" />
          {:else}
            <Eye class="size-4" />
          {/if}
        </button>
      </div>
      {#if $errors.password}
        <p id="conn-password-error" class="text-xs text-destructive" role="alert">
          {$errors.password}
        </p>
      {/if}
    </div>

    <div class="border-t border-white/8 pt-3">
      <div class="flex items-center justify-between gap-4">
        <div>
          <p class="text-sm font-medium text-slate-100">SSH key</p>
          <p class="mt-1 text-xs text-slate-400">
            {#if connection?.sshKeyId}
              Select a saved key, paste a replacement, or disable key auth.
            {:else}
              Select a saved key or paste a new SSH private key.
            {/if}
          </p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={$formData.useSshKey}
          aria-label="Use SSH key"
          onclick={toggleSshKey}
          disabled={isSaving}
          class={$formData.useSshKey
            ? "relative h-6 w-11 shrink-0 cursor-pointer rounded-full bg-cyan-300/80 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
            : "relative h-6 w-11 shrink-0 cursor-pointer rounded-full border border-white/10 bg-white/8 transition-colors hover:bg-white/12 disabled:cursor-not-allowed disabled:opacity-50"}
        >
          <span
            class={$formData.useSshKey
              ? "absolute left-0.5 top-0.5 size-5 translate-x-5 rounded-full bg-slate-950 transition-transform"
              : "absolute left-0.5 top-0.5 size-5 rounded-full bg-slate-300 transition-transform"}
          ></span>
        </button>
      </div>

      {#if $formData.useSshKey}
        <div class="mt-4 space-y-3">
          {#if keys.length > 0}
            <div
              class="flex gap-1 rounded-2xl border border-white/10 bg-black/20 p-1"
              role="group"
              aria-label="SSH key source"
            >
              <button
                type="button"
                class={segmentClass($formData.keyMode === "saved")}
                aria-pressed={$formData.keyMode === "saved"}
                onclick={() => handleKeyModeChange("saved")}
                disabled={isSaving}
              >
                Saved key
              </button>
              <button
                type="button"
                class={segmentClass($formData.keyMode === "new")}
                aria-pressed={$formData.keyMode === "new"}
                onclick={() => handleKeyModeChange("new")}
                disabled={isSaving}
              >
                Paste new key
              </button>
            </div>
          {/if}

          {#if $formData.keyMode === "saved" && keys.length > 0}
            <div class="space-y-2">
              <label for="conn-key-select" class="text-sm font-medium text-slate-100"
                >Saved key</label
              >
              <div class="relative">
                <select
                  id="conn-key-select"
                  bind:value={$formData.selectedKeyId}
                  class={SELECT_CLASS}
                  disabled={isSaving}
                >
                  <option value="" class="bg-slate-900">— Select a saved key —</option>
                  {#each keys as key (key.id)}
                    <option value={key.id} class="bg-slate-900">
                      {key.name} ({key.kind}){key.fingerprint
                        ? " — " + key.fingerprint.slice(0, 16) + "…"
                        : ""}
                    </option>
                  {/each}
                </select>
                <ChevronDown
                  class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-400"
                />
              </div>
              {#if selectedKeyWillBeUsed}
                <p class="text-xs text-emerald-400">
                  Using saved key: {selectedKey?.name ?? ""}
                </p>
              {/if}
            </div>
          {/if}

          {#if pastingNewKey || keys.length === 0}
            <div class="space-y-2">
              <label for="conn-key-name" class="text-sm font-medium text-slate-100"
                >Key name <span class="text-slate-500">(optional)</span></label
              >
              <Input
                id="conn-key-name"
                bind:value={keyName}
                placeholder="my-github-key"
                autocapitalize="none"
                autocomplete="off"
                autocorrect="off"
                spellcheck="false"
                class={fieldClass()}
                disabled={isSaving}
              />
            </div>

            <div class="space-y-2">
              <label for="conn-private-key" class="text-sm font-medium text-slate-100"
                >Private key</label
              >
              <textarea
                id="conn-private-key"
                bind:value={$formData.privateKey}
                placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                rows="8"
                aria-invalid={$errors.privateKey ? "true" : undefined}
                aria-describedby={$errors.privateKey
                  ? "conn-private-key-error"
                  : undefined}
                class={textareaClass($errors.privateKey)}
                disabled={isSaving}
              ></textarea>
              {#if $errors.privateKey}
                <p
                  id="conn-private-key-error"
                  class="text-xs text-destructive"
                  role="alert"
                >
                  {$errors.privateKey}
                </p>
              {/if}
            </div>

            <div class="space-y-2">
              <label for="conn-passphrase" class="text-sm font-medium text-slate-100"
                >Key passphrase <span class="text-slate-500">(optional)</span></label
              >
              <Input
                id="conn-passphrase"
                type="password"
                bind:value={$formData.passphrase}
                placeholder="Optional passphrase"
                class={fieldClass()}
                disabled={isSaving}
              />
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </FormSection>
</FormShell>
