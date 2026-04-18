<script lang="ts">
  import { KeyRound, LockKeyhole, Server, X } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { ConnectionConfig, SaveConnectionInput } from "$lib/stores/bootstrap.svelte.js";

  type AuthMode = "password" | "publickey" | "publickey_password";

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
  let authMode = $state<AuthMode>("password");
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
    authMode = (connection?.authMode as AuthMode | undefined) ?? "password";
    submitted = false;
    touched = {};
  });

  const existingKeyWillBeKept = $derived(Boolean(connection?.sshKeyId) && !privateKey.trim());

  const errors = $derived.by(() => {
    const nextErrors: Record<string, string> = {};
    const hasPassword = password.trim().length > 0;
    const hasPrivateKey = privateKey.trim().length > 0 || existingKeyWillBeKept;

    if (!name.trim()) nextErrors.name = "Connection name is required";
    if (!host.trim()) nextErrors.host = "Host is required";
    if (port < 1 || port > 65535) nextErrors.port = "Port must be between 1 and 65535";
    if (!username.trim()) nextErrors.username = "Username is required";

    if (authMode === "password" && !hasPassword) {
      nextErrors.password = connection?.hasPassword
        ? "Re-enter the password to keep password authentication"
        : "Password is required";
    }

    if (authMode === "publickey" && !hasPrivateKey) {
      nextErrors.privateKey = "Paste a private key or keep the existing saved key";
    }

    if (authMode === "publickey_password") {
      if (!hasPrivateKey) {
        nextErrors.privateKey = "A private key is required for this auth mode";
      }
      if (!hasPassword) {
        nextErrors.password = connection?.hasPassword
          ? "Re-enter the password to keep hybrid authentication"
          : "Password is required for this auth mode";
      }
    }

    return nextErrors;
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  const authOptions: Array<{ mode: AuthMode; title: string; body: string; icon: typeof LockKeyhole }> = [
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
      existingKeyId: authMode === "password" ? null : connection?.sshKeyId ?? null,
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
            <Input id="conn-name" bind:value={name} onblur={() => markTouched("name")} placeholder="Production API" class={showError("name") ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} disabled={isSaving} />
            {#if showError("name")}
              <p class="text-xs text-destructive" role="alert">{showError("name")}</p>
            {/if}
          </div>

          <div class="space-y-2 sm:col-span-2">
            <label for="conn-host" class="text-sm font-medium text-slate-100">Host</label>
            <Input id="conn-host" bind:value={host} onblur={() => markTouched("host")} placeholder="prod.example.com" class={showError("host") ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} disabled={isSaving} />
            {#if showError("host")}
              <p class="text-xs text-destructive" role="alert">{showError("host")}</p>
            {/if}
          </div>

          <div class="space-y-2">
            <label for="conn-port" class="text-sm font-medium text-slate-100">Port</label>
            <Input id="conn-port" type="number" bind:value={port} onblur={() => markTouched("port")} class={showError("port") ? "border-destructive bg-white/5 text-white" : "border-white/10 bg-white/5 text-white"} disabled={isSaving} />
            {#if showError("port")}
              <p class="text-xs text-destructive" role="alert">{showError("port")}</p>
            {/if}
          </div>

          <div class="space-y-2">
            <label for="conn-username" class="text-sm font-medium text-slate-100">Username</label>
            <Input id="conn-username" bind:value={username} onblur={() => markTouched("username")} placeholder="deploy" class={showError("username") ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} disabled={isSaving} />
            {#if showError("username")}
              <p class="text-xs text-destructive" role="alert">{showError("username")}</p>
            {/if}
          </div>
        </div>

        <div class="rounded-2xl border border-white/8 bg-white/[0.04] p-5">
          <p class="section-title text-slate-400">Authentication mode</p>
          <div class="mt-4 grid gap-3 md:grid-cols-3">
            {#each authOptions as option}
              <button
                type="button"
                class={authMode === option.mode
                  ? "rounded-2xl border border-primary/40 bg-primary/10 p-4 text-left transition-colors"
                  : "rounded-2xl border border-white/8 bg-white/[0.03] p-4 text-left transition-colors hover:bg-white/[0.06]"}
                onclick={() => (authMode = option.mode)}
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

        {#if authMode === "password" || authMode === "publickey_password"}
          <div class="space-y-2">
            <label for="conn-password" class="text-sm font-medium text-slate-100">Password</label>
            <Input id="conn-password" type="password" bind:value={password} onblur={() => markTouched("password")} placeholder={connection?.hasPassword ? "Re-enter saved password" : "Enter password"} class={showError("password") ? "border-destructive bg-white/5 text-white placeholder:text-slate-500" : "border-white/10 bg-white/5 text-white placeholder:text-slate-500"} disabled={isSaving} />
            {#if showError("password")}
              <p class="text-xs text-destructive" role="alert">{showError("password")}</p>
            {/if}
          </div>
        {/if}

        {#if authMode === "publickey" || authMode === "publickey_password"}
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
              bind:value={privateKey}
              onblur={() => markTouched("privateKey")}
              placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
              rows="8"
              class={showError("privateKey") ? "flex w-full rounded-xl border border-destructive bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring" : "flex w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder:text-slate-500 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"}
              disabled={isSaving}
            ></textarea>
            {#if showError("privateKey")}
              <p class="text-xs text-destructive" role="alert">{showError("privateKey")}</p>
            {/if}
            <p class="text-xs leading-5 text-slate-400">Paste a new key only if you want to add or replace the stored SSH key material.</p>
          </div>

          <div class="space-y-2">
            <label for="conn-passphrase" class="text-sm font-medium text-slate-100">Key passphrase</label>
            <Input id="conn-passphrase" type="password" bind:value={passphrase} placeholder="Optional passphrase for the private key" class="border-white/10 bg-white/5 text-white placeholder:text-slate-500" disabled={isSaving} />
          </div>
        {/if}

        <div class="rounded-2xl border border-white/8 bg-slate-950/45 p-4 text-sm text-slate-300">
          {#if authMode === "password"}
            Use password auth for environments where SSH key distribution is not available yet.
          {:else if authMode === "publickey"}
            Key-based auth is recommended when you want stronger, reusable access control.
          {:else}
            Hybrid auth is ideal for bastions or hardened hosts that require both a private key and a password.
          {/if}
        </div>

        <div class="flex gap-3 pt-2">
          <Button type="button" variant="outline" class="flex-1 border-white/10 bg-white/4 text-white hover:bg-white/8" onclick={onCancel} disabled={isSaving}>
            Cancel
          </Button>
          <Button type="submit" class="flex-1" disabled={isSaving || !isValid}>
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
