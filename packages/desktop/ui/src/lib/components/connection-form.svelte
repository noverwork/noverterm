<script lang="ts">
  import { ChevronDown, KeyRound, Loader2, Server } from "@lucide/svelte";
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

  const formTitle = $derived(connection ? "Edit connection" : "New connection");
  const submitLabel = $derived.by(() => {
    if (isSaving) {
      return connection ? "Saving…" : "Creating…";
    }

    return connection ? "Save changes" : "Create connection";
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
    if (!result.success || isSaving) {
      return;
    }

    const rawKeyId = $formData.keyMode === "saved" ? $formData.selectedKeyId : null;
    const effectiveKeyId = rawKeyId && rawKeyId.trim() ? rawKeyId : null;
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

<div class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Inventory</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">{formTitle}</h1>
        <p class="mt-2 text-sm text-slate-500">Save host details and encrypted credentials for terminal and forwarding workflows.</p>
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

      <form
        class="mt-5 rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8 p-5 shadow-[0_16px_42px_rgb(34_211_238/0.08)]"
        onsubmit={(event) => {
          event.preventDefault();
          void handleSubmit();
        }}
      >
      <div class="flex items-center justify-between gap-3">
        <h2 class="text-sm font-semibold text-cyan-100">SSH Connection</h2>
        <button type="button" class="cursor-pointer text-xs text-slate-400 transition-colors hover:text-white" onclick={onCancel} disabled={isSaving}>
          Cancel
        </button>
      </div>

      <div class="mt-4 grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
        <div class="space-y-4">
          <div class="rounded-2xl border border-white/8 bg-white/[0.035] p-4">
            <div class="flex items-center gap-3">
              <div class="flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                <Server class="size-5" />
              </div>
              <div>
                <h3 class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500">Connection Details</h3>
                <p class="mt-1 text-xs text-slate-400">Name the host and define the SSH endpoint.</p>
              </div>
            </div>

            <div class="mt-4 grid gap-4 sm:grid-cols-[minmax(0,1fr)_7rem]">
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
                    ? "border-destructive bg-black/20 text-white placeholder:text-slate-500"
                    : "border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"}
                  disabled={isSaving}
                />
                {#if $errors.name}
                  <p class="text-xs text-destructive" role="alert">{$errors.name}</p>
                {/if}
              </div>

              <div class="space-y-2">
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
                    ? "border-destructive bg-black/20 text-white placeholder:text-slate-500"
                    : "border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"}
                  disabled={isSaving}
                />
                {#if $errors.host}
                  <p class="text-xs text-destructive" role="alert">{$errors.host}</p>
                {/if}
              </div>

              <div class="space-y-2">
                <label for="conn-port" class="text-sm font-medium text-slate-100">Port</label>
                <Input
                  id="conn-port"
                  type="number"
                  bind:value={$formData.port}
                  class={$errors.port
                    ? "border-destructive bg-black/20 text-white"
                    : "border-white/10 bg-black/20 text-white focus-visible:border-cyan-300/40"}
                  disabled={isSaving}
                />
                {#if $errors.port}
                  <p class="text-xs text-destructive" role="alert">{$errors.port}</p>
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
                    ? "border-destructive bg-black/20 text-white placeholder:text-slate-500"
                    : "border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"}
                  disabled={isSaving}
                />
                {#if $errors.username}
                  <p class="text-xs text-destructive" role="alert">{$errors.username}</p>
                {/if}
              </div>
            </div>
          </div>

          <div class="space-y-2">
            <label for="conn-password" class="text-sm font-medium text-slate-100">Password</label>
            <Input
              id="conn-password"
              type="password"
              bind:value={$formData.password}
              placeholder={connection?.hasPassword
                ? "Re-enter password only if changing it"
                : "Leave blank for key-only auth"}
              class={$errors.password
                ? "border-destructive bg-black/20 text-white placeholder:text-slate-500"
                : "border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"}
              disabled={isSaving}
            />
            {#if $errors.password}
              <p class="text-xs text-destructive" role="alert">{$errors.password}</p>
            {/if}
          </div>
        </div>

        <div class="space-y-4">
          <div class="rounded-2xl border border-white/8 bg-white/[0.035] p-4">
            <button
              type="button"
              class={$formData.useSshKey
                ? "flex w-full cursor-pointer items-start gap-3 rounded-2xl border border-cyan-300/30 bg-cyan-300/10 p-4 text-left transition-colors"
                : "flex w-full cursor-pointer items-start gap-3 rounded-2xl border border-white/8 bg-white/[0.03] p-4 text-left transition-colors hover:bg-white/[0.06]"}
              onclick={toggleSshKey}
              aria-pressed={$formData.useSshKey}
              disabled={isSaving}
            >
              <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl bg-white/6 text-cyan-200">
                <KeyRound class="size-5" />
              </div>
              <div class="space-y-1">
                <p class="font-medium text-white">Use SSH key</p>
                <p class="text-sm leading-6 text-slate-400">
                  {#if connection?.sshKeyId}
                    Select a saved key, paste a replacement, or disable key auth.
                  {:else}
                    Select a saved key or paste a new SSH private key.
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
                        Using saved key: {keys.find((key) => key.id === $formData.selectedKeyId)?.name ?? ""}
                      </p>
                    {/if}
                  </div>

                  <div class="flex items-center gap-3 text-xs text-slate-400">
                    <span>or</span>
                    <button
                      type="button"
                      class="cursor-pointer text-cyan-300 underline hover:text-cyan-200"
                      onclick={() => handleKeyModeChange($formData.keyMode === "new" ? "saved" : "new")}
                      disabled={isSaving}
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
                      class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                      disabled={isSaving}
                    />
                  </div>

                  <div class="space-y-2">
                    <label for="conn-private-key" class="text-sm font-medium text-slate-100">Private key</label>
                    <textarea
                      id="conn-private-key"
                      bind:value={$formData.privateKey}
                      placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                      rows="8"
                      class={$errors.privateKey
                        ? "flex min-h-[12rem] w-full rounded-2xl border border-destructive bg-black/20 px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                        : "flex min-h-[12rem] w-full rounded-2xl border border-white/10 bg-black/20 px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20"}
                      disabled={isSaving}
                    ></textarea>
                    {#if $errors.privateKey}
                      <p class="text-xs text-destructive" role="alert">{$errors.privateKey}</p>
                    {/if}
                  </div>

                  <div class="space-y-2">
                    <label for="conn-passphrase" class="text-sm font-medium text-slate-100">Key passphrase <span class="text-slate-500">(optional)</span></label>
                    <Input
                      id="conn-passphrase"
                      type="password"
                      bind:value={$formData.passphrase}
                      placeholder="Optional passphrase"
                      class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                      disabled={isSaving}
                    />
                  </div>
                {/if}
              </div>
            {/if}
          </div>

          <div class="flex flex-wrap items-center gap-2 pt-1">
            <Button type="submit" class="rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200" disabled={isSaving}>
              {#if isSaving}
                <Loader2 class="size-4 animate-spin" />
              {/if}
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
