<script lang="ts">
  import { ChevronDown, KeyRound, X } from "@lucide/svelte";
  import { superForm } from "sveltekit-superforms";
  import { zod4 } from "sveltekit-superforms/adapters";

  import { connectionSchema, type ConnectionForm } from "$lib/schemas/index.js";
  import type { ConnectionConfig, SaveConnectionInput } from "$lib/stores/bootstrap.svelte.js";
  import type { SshKeyRecord } from "$lib/api/types.js";
  import { Button } from "$lib/components/ui/button/index.js";
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

  $effect(() => {
    const connectionId = connection?.id ?? "new";
    if (initializedConnectionId === connectionId) {
      return;
    }

    initializedConnectionId = connectionId;
    keyName = "";
    $formData = {
      name: connection?.name ?? "",
      host: connection?.host ?? "",
      port: connection?.port ?? 22,
      username: connection?.username ?? "",
      password: "",
      privateKey: "",
      passphrase: "",
      useSshKey: Boolean(connection?.sshKeyId),
      keyMode: "saved",
      selectedKeyId: connection?.sshKeyId ?? null,
      existingPassword: Boolean(connection?.hasPassword),
    };
  });

  const selectedKeyWillBeUsed = $derived(
    $formData.useSshKey && $formData.keyMode === "saved" && !!$formData.selectedKeyId,
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

  async function handleSubmit() {
    const result = connectionSchema.safeParse($formData);
    if (!result.success || isSaving) return;

    const effectiveKeyId = $formData.keyMode === "saved" ? $formData.selectedKeyId : null;
    const preservedEncryptedPassword = connection?.hasPassword && !$formData.password.trim()
      ? connection.auth?.kind === "password" || connection.auth?.kind === "public_key_and_password"
        ? connection.auth.password
        : null
      : null;

    await onSave({
      ...(connection?.id ? { id: connection.id } : {}),
      name: $formData.name.trim(),
      host: $formData.host.trim(),
      port: $formData.port,
      username: $formData.username.trim(),
      ...($formData.password.trim() ? { password: $formData.password.trim() } : {}),
      ...(preservedEncryptedPassword ? { preservedEncryptedPassword } : {}),
      ...($formData.useSshKey && $formData.keyMode === "new" && $formData.privateKey.trim()
        ? { privateKey: $formData.privateKey.trim() }
        : {}),
      ...($formData.useSshKey && $formData.keyMode === "new" && $formData.passphrase.trim()
        ? { passphrase: $formData.passphrase.trim() }
        : {}),
      ...(keyName.trim() ? { keyName: keyName.trim() } : {}),
      existingKeyId: effectiveKeyId,
    });
  }
</script>

<div
  class="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-sm"
  onclick={(event) => event.target === event.currentTarget && onCancel()}
  onkeydown={(event) => event.key === "Escape" && onCancel()}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <div class="flex h-full w-full max-w-[40rem] flex-col border-l border-white/10 bg-slate-950/96 text-white shadow-2xl xl:max-w-[44rem]">
    <div class="flex items-center justify-between border-b border-white/10 px-5 py-4 sm:px-6 sm:py-5">
      <div>
        <p class="section-title text-slate-400">SSH connection</p>
        <h2 class="mt-2 text-xl font-semibold tracking-tight">
          {connection ? "Edit connection" : "New connection"}
        </h2>
      </div>
      <Button variant="ghost" size="icon-sm" class="text-slate-300 hover:text-white" onclick={onCancel}>
        <X class="size-4" />
      </Button>
    </div>

    <form
      class="flex min-h-0 flex-1 flex-col"
      onsubmit={(event) => {
        event.preventDefault();
        void handleSubmit();
      }}
    >
      <div class="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-5 sm:px-6 sm:py-6">
        {#if error}
          <div class="rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
            {error}
          </div>
        {/if}

        <div class="rounded-2xl border border-white/8 bg-white/[0.04] p-5">
          <div class="space-y-2">
            <p class="section-title text-slate-400">Connection details</p>
          </div>

          <div class="mt-5 grid gap-4 sm:grid-cols-2">
            <div class="space-y-2 sm:col-span-2">
              <label for="conn-name" class="text-sm font-medium text-slate-100">Connection name</label>
              <Input
                id="conn-name"
                bind:value={$formData.name}
                placeholder="Production API"
                autocapitalize="none"
                autocomplete="off"
                autocorrect="off"
                spellcheck="false"
                class={$errors.name
                  ? "border-destructive bg-white/5 text-white placeholder:text-slate-500"
                  : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
                disabled={isSaving}
              />
              {#if $errors.name}
                <p class="text-xs text-destructive" role="alert">{$errors.name}</p>
              {/if}
            </div>

            <div class="space-y-2 sm:col-span-2">
              <label for="conn-host" class="text-sm font-medium text-slate-100">Host</label>
              <Input
                id="conn-host"
                bind:value={$formData.host}
                placeholder="prod.example.com"
                autocapitalize="none"
                autocomplete="off"
                autocorrect="off"
                spellcheck="false"
                class={$errors.host
                  ? "border-destructive bg-white/5 text-white placeholder:text-slate-500"
                  : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
                disabled={isSaving}
              />
              {#if $errors.host}
                <p class="text-xs text-destructive" role="alert">{$errors.host}</p>
              {/if}
            </div>

            <div class="space-y-2 sm:col-span-2">
              <label for="conn-username" class="text-sm font-medium text-slate-100">Username</label>
              <Input
                id="conn-username"
                bind:value={$formData.username}
                placeholder="deploy"
                autocapitalize="none"
                autocomplete="username"
                autocorrect="off"
                spellcheck="false"
                class={$errors.username
                  ? "border-destructive bg-white/5 text-white placeholder:text-slate-500"
                  : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
                disabled={isSaving}
              />
              {#if $errors.username}
                <p class="text-xs text-destructive" role="alert">{$errors.username}</p>
              {/if}
            </div>

            <div class="space-y-2 sm:w-32">
              <label for="conn-port" class="text-sm font-medium text-slate-100">Port</label>
              <Input
                id="conn-port"
                type="number"
                bind:value={$formData.port}
                class={$errors.port
                  ? "border-destructive bg-white/5 text-white"
                  : "border-white/10 bg-white/5 text-white"}
                disabled={isSaving}
              />
              {#if $errors.port}
                <p class="text-xs text-destructive" role="alert">{$errors.port}</p>
              {/if}
            </div>
          </div>
        </div>

        <div class="rounded-2xl border border-white/8 bg-white/[0.04] p-5">
          <div class="space-y-2">
            <p class="section-title text-slate-400">Credentials</p>
          </div>

          <div class="mt-5 space-y-5">
            <div class="space-y-2">
              <label for="conn-password" class="text-sm font-medium text-slate-100">Password</label>
              <Input
                id="conn-password"
                type="password"
                bind:value={$formData.password}
                placeholder={connection?.hasPassword
                  ? "Re-enter password if this host still needs it"
                  : "Leave blank if this host only uses an SSH key"}
                class={$errors.password
                  ? "border-destructive bg-white/5 text-white placeholder:text-slate-500"
                  : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"}
                disabled={isSaving}
              />
              {#if $errors.password}
                <p class="text-xs text-destructive" role="alert">{$errors.password}</p>
              {/if}
              <p class="text-xs leading-5 text-slate-400">
                {#if connection?.sshKeyId && connection?.hasPassword}
                  Re-enter the password only if this host still requires both key and password.
                {:else if connection?.hasPassword}
                  Re-enter the password if you want to keep password auth on this saved connection.
                {:else}
                  Leave this blank if the host only uses SSH key auth.
                {/if}
              </p>
            </div>

            <div class="rounded-2xl border border-white/8 bg-slate-950/45 p-4">
              <button
                type="button"
                class={$formData.useSshKey
                  ? "flex w-full items-start gap-3 rounded-2xl border border-primary/40 bg-primary/10 p-4 text-left transition-colors"
                  : "flex w-full items-start gap-3 rounded-2xl border border-white/8 bg-white/[0.03] p-4 text-left transition-colors hover:bg-white/[0.06]"}
                onclick={toggleSshKey}
                aria-pressed={$formData.useSshKey}
              >
                <div class="mt-0.5 flex size-10 items-center justify-center rounded-2xl bg-white/6 text-primary">
                  <KeyRound class="size-5" />
                </div>
                <div class="space-y-1">
                  <p class="font-medium text-white">Use SSH key</p>
                  <p class="text-sm leading-6 text-slate-400">
                    {#if connection?.sshKeyId}
                      Select a saved key from the backend, paste a new one, or turn this off.
                    {:else}
                      Select a saved key from the backend or paste a new SSH private key.
                    {/if}
                  </p>
                </div>
              </button>

              {#if $formData.useSshKey}
                <div class="mt-4 space-y-4">
                  {#if keys.length > 0}
                    <div class="space-y-2">
                      <label for="conn-key-select" class="text-sm font-medium text-slate-100">Saved key</label>
                      <div class="relative">
                        <select
                          id="conn-key-select"
                          bind:value={$formData.selectedKeyId}
                          class="flex h-11 w-full appearance-none rounded-2xl border border-white/10 bg-black/20 px-3 py-1 pr-10 text-sm text-white shadow-sm transition-colors hover:bg-white/[0.06] focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20 disabled:cursor-not-allowed disabled:opacity-50"
                          disabled={isSaving}
                        >
                          <option value="" class="bg-slate-900">— Select a saved key —</option>
                          {#each keys as key (key.id)}
                            <option value={key.id} class="bg-slate-900">
                              {key.name} ({key.kind}){key.fingerprint ? " — " + key.fingerprint.slice(0, 16) + "…" : ""}
                            </option>
                          {/each}
                        </select>
                        <ChevronDown class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
                      </div>
                      {#if selectedKeyWillBeUsed}
                        <p class="text-xs text-emerald-400">
                          Using saved key: {keys.find(k => k.id === $formData.selectedKeyId)?.name ?? ""}
                        </p>
                      {/if}
                    </div>

                    <div class="flex items-center gap-3 text-xs text-slate-400">
                      <span>or</span>
                      <button
                        type="button"
                        class="text-primary hover:text-primary/80 underline"
                        onclick={() => handleKeyModeChange($formData.keyMode === "new" ? "saved" : "new")}
                      >
                        {$formData.keyMode === "new" ? "select a saved key instead" : "paste a new key"}
                      </button>
                    </div>
                  {/if}

                  {#if pastingNewKey || keys.length === 0}
                    <div class="space-y-2">
                      <label for="conn-key-name" class="text-sm font-medium text-slate-100">Key name</label>
                      <Input
                        id="conn-key-name"
                        bind:value={keyName}
                        placeholder="my-github-key (optional)"
                        class="border-white/10 bg-white/5 text-white placeholder:text-slate-500"
                        disabled={isSaving}
                      />
                    </div>

                    <div class="space-y-2">
                      <div class="flex items-center justify-between gap-3">
                        <label for="conn-private-key" class="text-sm font-medium text-slate-100">Private key</label>
                        {#if connection?.sshKeyId && $formData.keyMode === "saved"}
                          <span class="rounded-full border border-white/10 bg-white/5 px-2.5 py-1 text-[11px] font-medium text-slate-300">
                            Keeping saved key
                          </span>
                        {/if}
                      </div>
                      <textarea
                        id="conn-private-key"
                        bind:value={$formData.privateKey}
                        placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                        rows="7"
                        class={$errors.privateKey
                          ? "flex w-full rounded-xl border border-destructive bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                          : "flex w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"}
                        disabled={isSaving}
                      ></textarea>
                      {#if $errors.privateKey}
                        <p class="text-xs text-destructive" role="alert">{$errors.privateKey}</p>
                      {/if}
                      <p class="text-xs leading-5 text-slate-400">
                        Paste a key only when you want to add or replace SSH key material for this connection.
                      </p>
                    </div>

                    <div class="space-y-2">
                      <label for="conn-passphrase" class="text-sm font-medium text-slate-100">Key passphrase</label>
                      <Input
                        id="conn-passphrase"
                        type="password"
                        bind:value={$formData.passphrase}
                        placeholder="Optional passphrase"
                        class="border-white/10 bg-white/5 text-white placeholder:text-slate-500"
                        disabled={isSaving}
                      />
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          </div>
        </div>
      </div>

      <div class="border-t border-white/10 px-5 py-4 sm:px-6">
        <div class="flex gap-3">
          <Button
            type="button"
            variant="outline"
            class="flex-1 border-white/10 bg-white/4 text-white hover:bg-white/8"
            onclick={onCancel}
          disabled={isSaving}
        >
          Cancel
        </Button>
        <Button type="submit" class="flex-1" disabled={isSaving}>
          {#if isSaving}
            Saving…
          {:else if connection}
            Save changes
          {:else}
            Create connection
          {/if}
        </Button>
        </div>
      </div>
    </form>
  </div>
</div>
