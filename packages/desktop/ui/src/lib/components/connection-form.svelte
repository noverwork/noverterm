<script lang="ts">
  import { X } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type {
    ConnectionConfig,
    SaveConnectionInput,
  } from "$lib/stores/bootstrap.svelte.js";

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

  let name = $state("");
  let host = $state("");
  let port = $state(22);
  let username = $state("");
  let password = $state("");
  let privateKey = $state("");
  let passphrase = $state("");
  let submitted = $state(false);
  let touched = $state<Record<string, boolean>>({});

  $effect(() => {
    name = connection?.name ?? "";
    host = connection?.host ?? "";
    port = connection?.port ?? 22;
    username = connection?.username ?? "";
    password = "";
    privateKey = "";
    passphrase = "";
    submitted = false;
    touched = {};
  });

  const errors = $derived.by(() => {
    const nextErrors: Record<string, string> = {};
    const hasPassword = password.trim().length > 0;
    const hasPrivateKey = privateKey.trim().length > 0;
    const preservesExistingKey = Boolean(connection?.sshKeyId) && !hasPrivateKey;
    const requiresPassword = connection?.hasPassword ?? false;

    if (!name.trim()) nextErrors.name = "Name is required";
    if (!host.trim()) nextErrors.host = "Host is required";
    if (port < 1 || port > 65535) nextErrors.port = "Port must be 1-65535";
    if (!username.trim()) nextErrors.username = "Username is required";
    if (!hasPassword && !hasPrivateKey && !preservesExistingKey) {
      nextErrors.auth = "Password or private key is required";
    }
    if (requiresPassword && !hasPassword) {
      nextErrors.password = "Re-enter the password to keep password-based auth";
    }
    return nextErrors;
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  function markTouched(field: string) {
    touched[field] = true;
  }

  function showError(field: string) {
    return (submitted || touched[field]) && errors[field];
  }

  async function handleSubmit() {
    submitted = true;
    if (!isValid || isSaving) return;

    await onSave({
      ...(connection?.id ? { id: connection.id } : {}),
      name: name.trim(),
      host: host.trim(),
      port,
      username: username.trim(),
      ...(password.trim() ? { password: password.trim() } : {}),
      ...(privateKey.trim() ? { privateKey: privateKey.trim() } : {}),
      ...(passphrase.trim() ? { passphrase: passphrase.trim() } : {}),
      existingKeyId: connection?.sshKeyId ?? null,
    });
  }
</script>

<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
  onclick={(event) => event.target === event.currentTarget && onCancel()}
  onkeydown={(event) => event.key === "Escape" && onCancel()}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <div class="w-full max-w-xl bg-card rounded-xl border border-border shadow-2xl">
    <div class="flex items-center justify-between p-4 border-b border-border">
      <h2 class="text-lg font-semibold">
        {connection ? "Edit Connection" : "New Connection"}
      </h2>
      <Button variant="ghost" size="icon-sm" onclick={onCancel}>
        <X class="size-4" />
      </Button>
    </div>

    <form
      class="p-4 space-y-4"
      onsubmit={(event) => {
        event.preventDefault();
        void handleSubmit();
      }}
    >
      {#if error}
        <div class="rounded-lg border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      {/if}

      <div class="space-y-2">
        <label for="conn-name" class="text-sm font-medium">Name</label>
        <Input
          id="conn-name"
          bind:value={name}
          onblur={() => markTouched("name")}
          placeholder="Production"
          class={showError("name") ? "border-destructive" : ""}
          disabled={isSaving}
        />
        {#if showError("name")}
          <p class="text-xs text-destructive">{showError("name")}</p>
        {/if}
      </div>

      <div class="grid grid-cols-3 gap-3">
        <div class="col-span-2 space-y-2">
          <label for="conn-host" class="text-sm font-medium">Host</label>
          <Input
            id="conn-host"
            bind:value={host}
            onblur={() => markTouched("host")}
            placeholder="prod.example.com"
            class={showError("host") ? "border-destructive" : ""}
            disabled={isSaving}
          />
          {#if showError("host")}
            <p class="text-xs text-destructive">{showError("host")}</p>
          {/if}
        </div>
        <div class="space-y-2">
          <label for="conn-port" class="text-sm font-medium">Port</label>
          <Input
            id="conn-port"
            type="number"
            bind:value={port}
            onblur={() => markTouched("port")}
            class={showError("port") ? "border-destructive" : ""}
            disabled={isSaving}
          />
          {#if showError("port")}
            <p class="text-xs text-destructive">{showError("port")}</p>
          {/if}
        </div>
      </div>

      <div class="space-y-2">
        <label for="conn-username" class="text-sm font-medium">Username</label>
        <Input
          id="conn-username"
          bind:value={username}
          onblur={() => markTouched("username")}
          placeholder="deploy"
          class={showError("username") ? "border-destructive" : ""}
          disabled={isSaving}
        />
        {#if showError("username")}
          <p class="text-xs text-destructive">{showError("username")}</p>
        {/if}
      </div>

      <div class="space-y-2">
        <label for="conn-password" class="text-sm font-medium">
          Password <span class="text-muted-foreground font-normal">(optional if private key is provided)</span>
        </label>
        <Input
          id="conn-password"
          type="password"
          bind:value={password}
          onblur={() => markTouched("password")}
          placeholder={connection?.hasPassword ? "Re-enter saved password" : "Enter password"}
          class={showError("password") || showError("auth") ? "border-destructive" : ""}
          disabled={isSaving}
        />
        {#if showError("password")}
          <p class="text-xs text-destructive">{showError("password")}</p>
        {:else if showError("auth")}
          <p class="text-xs text-destructive">{showError("auth")}</p>
        {/if}
      </div>

      <div class="space-y-2">
        <label for="conn-private-key" class="text-sm font-medium">
          SSH Private Key <span class="text-muted-foreground font-normal">(paste the key text to replace or add key auth)</span>
        </label>
        <textarea
          id="conn-private-key"
          bind:value={privateKey}
          onblur={() => markTouched("privateKey")}
          placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
          rows="6"
          class="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
          disabled={isSaving}
        ></textarea>
      </div>

      <div class="space-y-2">
        <label for="conn-passphrase" class="text-sm font-medium">
          Key Passphrase <span class="text-muted-foreground font-normal">(optional)</span>
        </label>
        <Input
          id="conn-passphrase"
          type="password"
          bind:value={passphrase}
          placeholder="Passphrase for the private key"
          disabled={isSaving}
        />
      </div>

      <div class="flex gap-2 pt-2">
        <Button type="button" variant="outline" class="flex-1" onclick={onCancel} disabled={isSaving}>
          Cancel
        </Button>
        <Button type="submit" class="flex-1" disabled={isSaving || !isValid}>
          {#if isSaving}
            Saving...
          {:else if connection}
            Save Changes
          {:else}
            Save Connection
          {/if}
        </Button>
      </div>
    </form>
  </div>
</div>
