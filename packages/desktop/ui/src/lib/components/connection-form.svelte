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
  let password = $state("");
  let keyPath = $state("");
  let submitted = $state(false);
  let touched = $state<Record<string, boolean>>({});

  $effect(() => {
    name = connection?.name ?? "";
    host = connection?.host ?? "";
    port = connection?.port ?? 22;
    username = connection?.username ?? "";
    password = connection?.password ?? "";
    keyPath = connection?.keyPath ?? "";
    submitted = false;
    touched = {};
  });

  const errors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (!name.trim()) errs.name = "Name is required";
    if (!host.trim()) errs.host = "Host is required";
    if (port < 1 || port > 65535) errs.port = "Port must be 1-65535";
    if (!username.trim()) errs.username = "Username is required";
    if (!password && !keyPath.trim()) errs.auth = "Password or SSH key is required";
    return errs;
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  function markTouched(field: string) {
    touched[field] = true;
  }

  function showError(field: string) {
    return (submitted || touched[field]) && errors[field];
  }

  function handleSubmit() {
    submitted = true;
    if (!isValid) return;
    onSave({
      ...(connection?.id ? { id: connection.id } : {}),
      name: name.trim(),
      host: host.trim(),
      port,
      username: username.trim(),
      ...(password ? { password } : {}),
      ...(keyPath.trim() ? { keyPath: keyPath.trim() } : {}),
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
        <Input id="conn-name" bind:value={name} onblur={() => markTouched("name")} placeholder="My Server" class={showError('name') ? 'border-destructive' : ''} />
        {#if showError("name")}
          <p class="text-xs text-destructive">{showError("name")}</p>
        {/if}
      </div>

      <div class="grid grid-cols-3 gap-3">
        <div class="col-span-2 space-y-2">
          <label for="conn-host" class="text-sm font-medium">Host</label>
          <Input id="conn-host" bind:value={host} onblur={() => markTouched("host")} placeholder="192.168.1.1" class={showError('host') ? 'border-destructive' : ''} />
          {#if showError("host")}
            <p class="text-xs text-destructive">{showError("host")}</p>
          {/if}
        </div>
        <div class="space-y-2">
          <label for="conn-port" class="text-sm font-medium">Port</label>
          <Input id="conn-port" type="number" bind:value={port} onblur={() => markTouched("port")} class={showError('port') ? 'border-destructive' : ''} />
          {#if showError("port")}
            <p class="text-xs text-destructive">{showError("port")}</p>
          {/if}
        </div>
      </div>

      <div class="space-y-2">
        <label for="conn-username" class="text-sm font-medium">Username</label>
        <Input id="conn-username" bind:value={username} onblur={() => markTouched("username")} placeholder="root" class={showError('username') ? 'border-destructive' : ''} />
        {#if showError("username")}
          <p class="text-xs text-destructive">{showError("username")}</p>
        {/if}
      </div>

      <div class="space-y-2">
        <label for="conn-password" class="text-sm font-medium">Password <span class="text-muted-foreground font-normal">(optional if key provided)</span></label>
        <Input
          id="conn-password"
          type="password"
          bind:value={password}
          onblur={() => markTouched("password")}
          placeholder="Enter password"
        />
      </div>

      <div class="space-y-2">
        <label for="conn-keypath" class="text-sm font-medium">SSH Key Path <span class="text-muted-foreground font-normal">(optional if password provided)</span></label>
        <Input
          id="conn-keypath"
          bind:value={keyPath}
          onblur={() => markTouched("keyPath")}
          placeholder="~/.ssh/id_ed25519"
          class={showError('auth') ? 'border-destructive' : ''}
        />
        {#if showError("auth")}
          <p class="text-xs text-destructive">{showError("auth")}</p>
        {/if}
      </div>

      <div class="flex gap-2 pt-2">
        <Button type="button" variant="outline" class="flex-1" onclick={onCancel}>
          Cancel
        </Button>
        <Button type="submit" class="flex-1">
          {connection ? "Save Changes" : "Connect"}
        </Button>
      </div>
    </form>
  </div>
</div>
