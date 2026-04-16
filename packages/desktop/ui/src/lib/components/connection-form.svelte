<script lang="ts">
  import { X } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { ConnectionConfig } from "$lib/config.js";

  let {
    connection,
    onSave,
    onCancel,
  }: {
    connection?: ConnectionConfig | null;
    onSave: (conn: Omit<ConnectionConfig, "id"> & { id?: string }) => void;
    onCancel: () => void;
  } = $props();

  let name = $state("");
  let host = $state("");
  let port = $state(22);
  let username = $state("");
  let authType = $state<"password" | "key">("password");
  let password = $state("");
  let keyPath = $state("");

  $effect(() => {
    name = connection?.name ?? "";
    host = connection?.host ?? "";
    port = connection?.port ?? 22;
    username = connection?.username ?? "";
    authType = connection?.authType ?? "password";
    password = connection?.password ?? "";
    keyPath = connection?.keyPath ?? "";
  });

  const errors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (!name.trim()) errs.name = "Name is required";
    if (!host.trim()) errs.host = "Host is required";
    if (port < 1 || port > 65535) errs.port = "Port must be 1-65535";
    if (!username.trim()) errs.username = "Username is required";
    if (authType === "password" && !password) errs.password = "Password is required";
    if (authType === "key" && !keyPath.trim()) errs.keyPath = "Key path is required";
    return errs;
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  function handleSubmit() {
    if (!isValid) return;
    onSave({
      ...(connection?.id ? { id: connection.id } : {}),
      name: name.trim(),
      host: host.trim(),
      port,
      username: username.trim(),
      authType,
      ...(authType === "password" ? { password } : { keyPath: keyPath.trim() }),
    });
  }
</script>

<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
  onclick={(e) => e.target === e.currentTarget && onCancel()}
  onkeydown={(e) => e.key === "Escape" && onCancel()}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
>
  <div class="w-full max-w-md bg-card rounded-xl border border-border shadow-2xl">
    <div class="flex items-center justify-between p-4 border-b border-border">
      <h2 class="text-lg font-semibold">
        {connection ? "Edit Connection" : "New Connection"}
      </h2>
      <Button variant="ghost" size="icon-sm" onclick={onCancel}>
        <X class="size-4" />
      </Button>
    </div>

    <form class="p-4 space-y-4" onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}>
      <div class="space-y-2">
        <label for="conn-name" class="text-sm font-medium">Name</label>
        <Input id="conn-name" bind:value={name} placeholder="My Server" class={errors.name ? 'border-destructive' : ''} />
        {#if errors.name}
          <p class="text-xs text-destructive">{errors.name}</p>
        {/if}
      </div>

      <div class="grid grid-cols-3 gap-3">
        <div class="col-span-2 space-y-2">
          <label for="conn-host" class="text-sm font-medium">Host</label>
          <Input id="conn-host" bind:value={host} placeholder="192.168.1.1" class={errors.host ? 'border-destructive' : ''} />
          {#if errors.host}
            <p class="text-xs text-destructive">{errors.host}</p>
          {/if}
        </div>
        <div class="space-y-2">
          <label for="conn-port" class="text-sm font-medium">Port</label>
          <Input id="conn-port" type="number" bind:value={port} class={errors.port ? 'border-destructive' : ''} />
          {#if errors.port}
            <p class="text-xs text-destructive">{errors.port}</p>
          {/if}
        </div>
      </div>

      <div class="space-y-2">
        <label for="conn-username" class="text-sm font-medium">Username</label>
        <Input id="conn-username" bind:value={username} placeholder="root" class={errors.username ? 'border-destructive' : ''} />
        {#if errors.username}
          <p class="text-xs text-destructive">{errors.username}</p>
        {/if}
      </div>

      <div class="space-y-2">
        <label class="text-sm font-medium">Authentication</label>
        <div class="flex gap-2">
          <Button
            type="button"
            variant={authType === "password" ? "default" : "outline"}
            size="sm"
            class="flex-1"
            onclick={() => (authType = "password")}
          >
            Password
          </Button>
          <Button
            type="button"
            variant={authType === "key" ? "default" : "outline"}
            size="sm"
            class="flex-1"
            onclick={() => (authType = "key")}
          >
            SSH Key
          </Button>
        </div>
      </div>

      {#if authType === "password"}
        <div class="space-y-2">
          <label for="conn-password" class="text-sm font-medium">Password</label>
          <Input
            id="conn-password"
            type="password"
            bind:value={password}
            placeholder="Enter password"
            class={errors.password ? 'border-destructive' : ''}
          />
          {#if errors.password}
            <p class="text-xs text-destructive">{errors.password}</p>
          {/if}
        </div>
      {:else}
        <div class="space-y-2">
          <label for="conn-keypath" class="text-sm font-medium">Key Path</label>
          <Input
            id="conn-keypath"
            bind:value={keyPath}
            placeholder="~/.ssh/id_ed25519"
            class={errors.keyPath ? 'border-destructive' : ''}
          />
          {#if errors.keyPath}
            <p class="text-xs text-destructive">{errors.keyPath}</p>
          {/if}
        </div>
      {/if}

      <div class="flex gap-2 pt-2">
        <Button type="button" variant="outline" class="flex-1" onclick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" class="flex-1" disabled={!isValid}>
          {connection ? "Save Changes" : "Connect"}
        </Button>
      </div>
    </form>
  </div>
</div>
