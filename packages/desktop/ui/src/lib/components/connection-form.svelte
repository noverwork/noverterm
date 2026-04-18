<script lang="ts">
  import { KeyRound, LockKeyhole, Server, X } from "@lucide/svelte";
  import { superForm } from "sveltekit-superforms";
  import { zod4 } from "sveltekit-superforms/adapters";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { ConnectionConfig, SaveConnectionInput } from "$lib/stores/bootstrap.svelte.js";
  import { connectionSchema, type ConnectionForm } from "$lib/schemas/index.js";

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
      authMode: "password",
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
    $formData.authMode = (connection?.authMode as ConnectionForm["authMode"] | undefined) ?? "password";
    $formData.hasExistingKey = Boolean(connection?.sshKeyId);
    form.reset();
  });

  const existingKeyWillBeKept = $derived($formData.hasExistingKey && !$formData.privateKey.trim());

  const authOptions: Array<{ mode: ConnectionForm["authMode"]; title: string; body: string; icon: typeof LockKeyhole }> = [
    {
      mode: "password",
      title: "Password",
      body: "Fastest setup when password auth is already enabled on the host.",
      icon: LockKeyhole,
    },
    {
      mode: "publickey",
      title: "SSH key",
      body: "Use a saved or pasted private key for stronger host authentication.",
      icon: KeyRound,
    },
    {
      mode: "publickey_password",
      title: "Key + password",
      body: "Use both factors when your environment requires hybrid SSH authentication.",
      icon: Server,
    },
  ];

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
      ...($formData.privateKey.trim() ? { privateKey: $formData.privateKey.trim() } : {}),
      ...($formData.passphrase.trim() ? { passphrase: $formData.passphrase.trim() } : {}),
      existingKeyId: $formData.authMode === "password" ? null : connection?.sshKeyId ?? null,
    });
  }
</script>

<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4 backdrop-blur-md"
  onclick={(event) => event.target === event.currentTarget && onCancel()}
  onkeydown={(event) => event.key === "Escape" && onCancel()}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <div class="w-full max-w-4xl rounded-[1.75rem] border border-white/10 bg-slate-950/92 text-white shadow-2xl">
    <div class="flex items-center justify-between border-b border-white/10 px-6 py-5">
      <div>
        <p class="section-title text-slate-400">Connection manager</p>
        <h2 class="mt-2 text-xl font-semibold tracking-tight">
          {connection ? "Edit saved connection" : "Create a new saved connection"}
        </h2>
      </div>
      <Button variant="ghost" size="icon-sm" class="text-slate-300 hover:text-white" onclick={onCancel}>
        <X class="size-4" />
      </Button>
    </div>

    <form
      class="grid gap-6 px-6 py-6 lg:grid-cols-[1.05fr_0.95fr]"
      onsubmit={(event) => {
        event.preventDefault();
        void handleSubmit();
      }}
    >
      <div class="space-y-5">
        {#if error}
          <div class="rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
            {error}
          </div>
        {/if}

        <div class="grid gap-4 sm:grid-cols-2">
          <div class="space-y-2 sm:col-span-2">
            <label for="conn-name" class="text-sm font-medium text-slate-100">Connection name</label>
            <Input id="conn-name" bind:value={$formData.name} placeholder="Production API" class={$errors.name ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} disabled={isSaving} />
            {#if $errors.name}
              <p class="text-xs text-destructive" role="alert">{$errors.name}</p>
            {/if}
          </div>

          <div class="space-y-2 sm:col-span-2">
            <label for="conn-host" class="text-sm font-medium text-slate-100">Host</label>
            <Input id="conn-host" bind:value={$formData.host} placeholder="prod.example.com" class={$errors.host ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} disabled={isSaving} />
            {#if $errors.host}
              <p class="text-xs text-destructive" role="alert">{$errors.host}</p>
            {/if}
          </div>

          <div class="space-y-2">
            <label for="conn-port" class="text-sm font-medium text-slate-100">Port</label>
            <Input id="conn-port" type="number" bind:value={$formData.port} class={$errors.port ? "border-destructive bg-white/5 text-white" : "border-white/10 bg-white/5 text-white"} disabled={isSaving} />
            {#if $errors.port}
              <p class="text-xs text-destructive" role="alert">{$errors.port}</p>
            {/if}
          </div>

          <div class="space-y-2">
            <label for="conn-username" class="text-sm font-medium text-slate-100">Username</label>
            <Input id="conn-username" bind:value={$formData.username} placeholder="deploy" class={$errors.username ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} disabled={isSaving} />
            {#if $errors.username}
              <p class="text-xs text-destructive" role="alert">{$errors.username}</p>
            {/if}
          </div>
        </div>

        <div class="rounded-2xl border border-white/8 bg-white/[0.04] p-5">
          <p class="section-title text-slate-400">Authentication mode</p>
          <div class="mt-4 grid gap-3 md:grid-cols-3">
            {#each authOptions as option}
              <button
                type="button"
                class={$formData.authMode === option.mode
                  ? "rounded-2xl border border-primary/40 bg-primary/10 p-4 text-left transition-colors"
                  : "rounded-2xl border border-white/8 bg-white/[0.03] p-4 text-left transition-colors hover:bg-white/[0.06]"}
                onclick={() => ($formData.authMode = option.mode)}
              >
                <div class="flex size-10 items-center justify-center rounded-2xl bg-white/6 text-primary">
                  <option.icon class="size-5" />
                </div>
                <p class="mt-4 font-medium text-white">{option.title}</p>
                <p class="mt-2 text-sm leading-6 text-slate-400">{option.body}</p>
              </button>
            {/each}
          </div>
        </div>
      </div>

      <div class="space-y-5 rounded-2xl border border-white/8 bg-white/[0.04] p-5">
        <div>
          <p class="section-title text-slate-400">Credentials</p>
          <h3 class="mt-2 text-lg font-semibold text-white">Only show the fields this host needs</h3>
          <p class="mt-2 text-sm leading-6 text-slate-400">
            Choose the auth mode first, then provide just the required credentials. Existing saved keys are
            preserved unless you paste a replacement.
          </p>
        </div>

        {#if $formData.authMode === "password" || $formData.authMode === "publickey_password"}
          <div class="space-y-2">
            <label for="conn-password" class="text-sm font-medium text-slate-100">Password</label>
            <Input id="conn-password" type="password" bind:value={$formData.password} placeholder={connection?.hasPassword ? "Re-enter saved password" : "Enter password"} class={$errors.password ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} disabled={isSaving} />
            {#if $errors.password}
              <p class="text-xs text-destructive" role="alert">{$errors.password}</p>
            {/if}
          </div>
        {/if}

        {#if $formData.authMode === "publickey" || $formData.authMode === "publickey_password"}
          <div class="space-y-2">
            <div class="flex items-center justify-between gap-3">
              <label for="conn-private-key" class="text-sm font-medium text-slate-100">Private key</label>
              {#if connection?.sshKeyId}
                <span class="rounded-full border border-white/10 bg-white/5 px-2.5 py-1 text-[11px] font-medium text-slate-300">
                  {existingKeyWillBeKept ? "Existing saved key will be kept" : "Replacing saved key"}
                </span>
              {/if}
            </div>
            <textarea
              id="conn-private-key"
              bind:value={$formData.privateKey}
              placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
              rows="8"
              class={$errors.privateKey ? "flex w-full rounded-xl border border-destructive bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring" : "flex w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"}
              disabled={isSaving}
            ></textarea>
            {#if $errors.privateKey}
              <p class="text-xs text-destructive" role="alert">{$errors.privateKey}</p>
            {/if}
            <p class="text-xs leading-5 text-slate-400">Paste a new key only if you want to add or replace the stored SSH key material.</p>
          </div>

          <div class="space-y-2">
            <label for="conn-passphrase" class="text-sm font-medium text-slate-100">Key passphrase</label>
            <Input id="conn-passphrase" type="password" bind:value={$formData.passphrase} placeholder="Optional passphrase for the private key" class="border-white/10 bg-white/5 text-white placeholder:text-slate-500" disabled={isSaving} />
          </div>
        {/if}

        <div class="rounded-2xl border border-white/8 bg-slate-950/45 p-4 text-sm text-slate-300">
          {#if $formData.authMode === "password"}
            Use password auth for environments where SSH key distribution is not available yet.
          {:else if $formData.authMode === "publickey"}
            Key-based auth is recommended when you want stronger, reusable access control.
          {:else}
            Hybrid auth is ideal for bastions or hardened hosts that require both a private key and a password.
          {/if}
        </div>

        <div class="flex gap-3 pt-2">
          <Button type="button" variant="outline" class="flex-1 border-white/10 bg-white/4 text-white hover:bg-white/8" onclick={onCancel} disabled={isSaving}>
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
