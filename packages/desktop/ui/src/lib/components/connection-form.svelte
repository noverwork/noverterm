<script lang="ts">
  import { KeyRound, X } from "@lucide/svelte";
  import { superForm } from "sveltekit-superforms";
  import { zod4 } from "sveltekit-superforms/adapters";

  import { connectionSchema, type ConnectionForm } from "$lib/schemas/index.js";
  import type { ConnectionConfig, SaveConnectionInput } from "$lib/stores/bootstrap.svelte.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  let {
    connection = null,
    onSave,
    onCancel,
    error = null,
    isSaving = false,
  }: {
    connection?: ConnectionConfig | null;
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
      hasExistingKey: false,
    },
    { validators: zod4(connectionSchema) },
  );

  const { form: formData, errors } = form;

  $effect(() => {
    $formData.name = connection?.name ?? "";
    $formData.host = connection?.host ?? "";
    $formData.port = connection?.port ?? 22;
    $formData.username = connection?.username ?? "";
    $formData.password = "";
    $formData.privateKey = "";
    $formData.passphrase = "";
    $formData.useSshKey = Boolean(connection?.sshKeyId);
    $formData.hasExistingKey = Boolean(connection?.sshKeyId);
    form.reset();
  });

  const existingKeyWillBeKept = $derived(
    $formData.useSshKey && $formData.hasExistingKey && !$formData.privateKey.trim(),
  );

  function toggleSshKey() {
    $formData.useSshKey = !$formData.useSshKey;
    if (!$formData.useSshKey) {
      $formData.privateKey = "";
      $formData.passphrase = "";
    }
  }

  async function handleSubmit() {
    const result = connectionSchema.safeParse($formData);
    if (!result.success || isSaving) return;

    await onSave({
      ...(connection?.id ? { id: connection.id } : {}),
      name: $formData.name.trim(),
      host: $formData.host.trim(),
      port: $formData.port,
      username: $formData.username.trim(),
      ...($formData.password.trim() ? { password: $formData.password.trim() } : {}),
      ...($formData.useSshKey && $formData.privateKey.trim()
        ? { privateKey: $formData.privateKey.trim() }
        : {}),
      ...($formData.useSshKey && $formData.passphrase.trim()
        ? { passphrase: $formData.passphrase.trim() }
        : {}),
      existingKeyId: $formData.useSshKey ? connection?.sshKeyId ?? null : null,
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
        <p class="section-title text-slate-400">Saved connection</p>
        <h2 class="mt-2 text-xl font-semibold tracking-tight">
          {connection ? "Edit connection" : "Create connection"}
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
            <p class="text-sm leading-6 text-slate-400">
              Keep it simple: save where to connect, then add only the credentials this host actually
              needs.
            </p>
          </div>

          <div class="mt-5 grid gap-4 sm:grid-cols-2">
            <div class="space-y-2 sm:col-span-2">
              <label for="conn-name" class="text-sm font-medium text-slate-100">Connection name</label>
              <Input
                id="conn-name"
                bind:value={$formData.name}
                placeholder="Production API"
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
            <h3 class="text-lg font-semibold text-white">Password first, SSH key only when needed</h3>
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
                      Keep the saved key, replace it, or turn this off if the host should use password only.
                    {:else}
                      Turn this on only when the host needs key-based SSH auth.
                    {/if}
                  </p>
                </div>
              </button>

              {#if $formData.useSshKey}
                <div class="mt-4 space-y-4">
                  <div class="space-y-2">
                    <div class="flex items-center justify-between gap-3">
                      <label for="conn-private-key" class="text-sm font-medium text-slate-100">Private key</label>
                      {#if connection?.sshKeyId}
                        <span class="rounded-full border border-white/10 bg-white/5 px-2.5 py-1 text-[11px] font-medium text-slate-300">
                          {existingKeyWillBeKept ? "Keeping saved key" : "Replacing saved key"}
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
