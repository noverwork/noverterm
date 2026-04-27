<script lang="ts">
  import { Loader2, Network, Play, Plus, Server, Square, Trash2 } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type {
    ConnectionConfig,
    SavePortForwardInput,
    SavedPortForwardConfig,
  } from "$lib/stores/bootstrap.svelte.js";
  import type { PortForward } from "$lib/stores/port-forward.svelte.js";

  interface Props {
    connections: ConnectionConfig[];
    savedForwards: SavedPortForwardConfig[];
    forwards: PortForward[];
    onSave: (input: SavePortForwardInput) => Promise<SavedPortForwardConfig>;
    onForward: (forward: SavedPortForwardConfig) => Promise<PortForward>;
    onStop: (forwardId: string) => Promise<PortForward>;
    onDeleteSaved: (forwardId: string) => Promise<void>;
    onDeleteRuntime: (forwardId: string) => Promise<void>;
  }

  let {
    connections,
    savedForwards,
    forwards,
    onSave,
    onForward,
    onStop,
    onDeleteSaved,
    onDeleteRuntime,
  }: Props = $props();

  let showForm = $state(false);
  let error = $state<string | null>(null);
  let isSaving = $state(false);
  let forwardingPresetIds = $state<string[]>([]);
  let stoppingForwardIds = $state<string[]>([]);
  let deletingSavedForwardIds = $state<string[]>([]);
  let deletingRuntimeForwardIds = $state<string[]>([]);

  let selectedConnectionId = $state<string | null>(null);
  let selectedConnection = $derived(
    selectedConnectionId
      ? connections.find((connection) => connection.id === selectedConnectionId) ?? null
      : null,
  );

  let formName = $state("");
  let formBindHost = $state("127.0.0.1");
  let formBindPort = $state("");
  let formTargetHost = $state("127.0.0.1");
  let formTargetPort = $state("");

  let sortedConnections = $derived(
    [...connections].sort((left, right) => left.name.localeCompare(right.name)),
  );

  let sortedSavedForwards = $derived(
    [...savedForwards].sort((left, right) => left.name.localeCompare(right.name)),
  );

  function getAuthLabel(connection: ConnectionConfig): string {
    switch (connection.auth?.kind) {
      case "public_key_and_password":
        return "Key + Password";
      case "public_key":
        return "SSH Key";
      default:
        return "Password";
    }
  }

  function connectionForForward(forward: SavedPortForwardConfig): ConnectionConfig | null {
    return connections.find((connection) => connection.id === forward.connectionId) ?? null;
  }

  function resetForm() {
    selectedConnectionId = null;
    formName = "";
    formBindHost = "127.0.0.1";
    formBindPort = "";
    formTargetHost = "127.0.0.1";
    formTargetPort = "";
    error = null;
  }

  function openNewForm() {
    resetForm();
    showForm = true;
  }

  function closeForm() {
    showForm = false;
    resetForm();
  }

  function parsePort(value: string, label: string): number {
    const trimmedValue = value.trim();
    if (!/^\d+$/.test(trimmedValue)) {
      throw new Error(`${label} must be a number from 1 to 65535`);
    }

    const port = Number.parseInt(trimmedValue, 10);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      throw new Error(`${label} must be a number from 1 to 65535`);
    }
    return port;
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    error = null;

    if (!selectedConnection) {
      error = "Select a saved connection to continue";
      return;
    }

    if (!formName.trim()) {
      error = "Name is required";
      return;
    }

    if (!formBindPort.trim()) {
      error = "Bind port is required";
      return;
    }

    if (!formTargetPort.trim()) {
      error = "Target port is required";
      return;
    }

    isSaving = true;

    try {
      const input: SavePortForwardInput = {
        name: formName.trim(),
        connectionId: selectedConnection.id,
        bind_host: formBindHost.trim(),
        bind_port: parsePort(formBindPort, "Bind port"),
        target_host: formTargetHost.trim(),
        target_port: parsePort(formTargetPort, "Target port"),
      };

      await onSave(input);
      closeForm();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to save port forward";
    } finally {
      isSaving = false;
    }
  }

  async function handleForward(savedForward: SavedPortForwardConfig) {
    error = null;
    forwardingPresetIds = [...forwardingPresetIds, savedForward.id];

    try {
      await onForward(savedForward);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to start port forward";
    } finally {
      forwardingPresetIds = forwardingPresetIds.filter((id) => id !== savedForward.id);
    }
  }

  async function handleStop(forward: PortForward) {
    error = null;
    stoppingForwardIds = [...stoppingForwardIds, forward.id];

    try {
      await onStop(forward.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to stop port forward";
    } finally {
      stoppingForwardIds = stoppingForwardIds.filter((id) => id !== forward.id);
    }
  }

  async function handleDeleteSaved(forward: SavedPortForwardConfig) {
    if (!window.confirm(`Delete saved port forward "${forward.name}"?`)) {
      return;
    }

    error = null;
    deletingSavedForwardIds = [...deletingSavedForwardIds, forward.id];

    try {
      await onDeleteSaved(forward.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to delete saved port forward";
    } finally {
      deletingSavedForwardIds = deletingSavedForwardIds.filter((id) => id !== forward.id);
    }
  }

  async function handleDeleteRuntime(forward: PortForward) {
    if (!window.confirm(`Remove stopped port forward "${forward.name}" from this list?`)) {
      return;
    }

    error = null;
    deletingRuntimeForwardIds = [...deletingRuntimeForwardIds, forward.id];

    try {
      await onDeleteRuntime(forward.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : "Failed to remove stopped port forward";
    } finally {
      deletingRuntimeForwardIds = deletingRuntimeForwardIds.filter((id) => id !== forward.id);
    }
  }

  function stateBadge(state: PortForward["state"]) {
    switch (state) {
      case "listening":
        return { tone: "bg-emerald-400 shadow-[0_0_14px_rgb(52_211_153/0.65)]", label: "Listening", text: "text-emerald-300" };
      case "connecting":
        return { tone: "bg-amber-300 shadow-[0_0_14px_rgb(252_211_77/0.55)] animate-pulse", label: "Connecting", text: "text-amber-300" };
      case "error":
        return { tone: "bg-red-400 shadow-[0_0_14px_rgb(248_113_113/0.55)]", label: "Error", text: "text-red-300" };
      default:
        return { tone: "bg-slate-500", label: "Stopped", text: "text-slate-400" };
    }
  }
</script>

<div class="workspace-canvas flex h-full flex-col overflow-y-auto px-5 py-6 lg:px-8">
  <section class="ide-panel flex min-h-full flex-col p-5 text-white sm:p-6">
    <div class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Network</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">Port Forwards</h1>
        <p class="mt-2 text-sm text-slate-500">Save reusable SSH tunnels, then start them with one click.</p>
      </div>

      {#if !showForm}
        <Button onclick={openNewForm} variant="default" size="sm" class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200">
          <Plus class="size-3.5" />
          Save Forward
        </Button>
      {/if}
    </div>

    {#if error}
      <div class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
        {error}
      </div>
    {/if}

    {#if showForm}
      <form class="mt-5 rounded-[1.35rem] border border-cyan-300/24 bg-cyan-300/8 p-5 shadow-[0_16px_42px_rgb(34_211_238/0.08)]" onsubmit={handleSubmit}>
        <div class="flex items-center justify-between gap-3">
          <h2 class="text-sm font-semibold text-cyan-100">Save Port Forward</h2>
          <button type="button" class="cursor-pointer text-xs text-slate-400 transition-colors hover:text-white" onclick={closeForm}>
            Cancel
          </button>
        </div>

        <div class="mt-4 space-y-3">
          <p class="text-sm font-medium text-slate-100">Select a Saved Connection</p>

          {#if sortedConnections.length === 0}
            <div class="rounded-2xl border border-dashed border-white/10 bg-white/[0.025] px-4 py-6 text-center">
              <Server class="mx-auto mb-3 size-8 text-slate-600" />
              <p class="text-sm text-slate-400">No saved connections yet.</p>
              <p class="mt-1 text-xs text-slate-500">Create a Host in <span class="text-cyan-300/80">Connections</span> first, then return here to save a tunnel.</p>
            </div>
          {:else}
            <div class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
              {#each sortedConnections as connection (connection.id)}
                <button
                  type="button"
                  class="cursor-pointer rounded-xl border px-3 py-3 text-left transition-all {selectedConnectionId === connection.id
                    ? 'border-cyan-300/30 bg-cyan-300/10 shadow-[0_0_20px_rgb(34_211_238/0.08)]'
                    : 'border-white/8 bg-white/[0.03] hover:border-white/14 hover:bg-white/[0.055]'}"
                  onclick={() => (selectedConnectionId = connection.id)}
                >
                  <div class="flex items-start gap-2.5">
                    <div class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                      <Server class="size-4" />
                    </div>
                    <div class="min-w-0 flex-1">
                      <p class="truncate text-sm font-medium text-white">{connection.name}</p>
                      <p class="mt-0.5 truncate font-mono text-[11px] text-slate-400">{connection.username}@{connection.host}:{connection.port}</p>
                      <p class="mt-0.5 text-[11px] text-slate-500">{getAuthLabel(connection)}</p>
                    </div>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>

        {#if selectedConnection}
          <div class="mt-5 grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
            <div class="space-y-4">
              <div class="space-y-2">
                <label for="pf-name" class="text-sm font-medium text-slate-100">Name</label>
                <Input
                  id="pf-name"
                  bind:value={formName}
                  placeholder="My tunnel"
                  class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                  disabled={isSaving}
                />
              </div>

              <div class="rounded-2xl border border-white/8 bg-white/[0.035] p-3">
                <p class="text-[11px] font-medium uppercase tracking-[0.16em] text-slate-500">Using Connection</p>
                <p class="mt-1 font-mono text-sm text-cyan-100">{selectedConnection.username}@{selectedConnection.host}:{selectedConnection.port}</p>
                <p class="mt-0.5 text-xs text-slate-400">{selectedConnection.name}</p>
              </div>
            </div>

            <div class="space-y-4">
              <div class="rounded-2xl border border-white/8 bg-white/[0.035] p-4">
                <h3 class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500">Forward Route</h3>

                <div class="mt-3 grid gap-3">
                  <div class="grid grid-cols-[1fr_6rem] gap-3">
                    <div class="space-y-2">
                      <label for="pf-bind-host" class="text-sm font-medium text-slate-100">Bind Host</label>
                      <Input
                        id="pf-bind-host"
                        bind:value={formBindHost}
                        placeholder="127.0.0.1"
                        class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                        disabled={isSaving}
                      />
                    </div>
                    <div class="space-y-2">
                      <label for="pf-bind-port" class="text-sm font-medium text-slate-100">Bind Port</label>
                      <Input
                        id="pf-bind-port"
                        bind:value={formBindPort}
                        type="number"
                        min="1"
                        max="65535"
                        class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                        disabled={isSaving}
                      />
                    </div>
                  </div>

                  <div class="flex items-center justify-center">
                    <span class="text-lg text-slate-600">↓</span>
                  </div>

                  <div class="grid grid-cols-[1fr_6rem] gap-3">
                    <div class="space-y-2">
                      <label for="pf-target-host" class="text-sm font-medium text-slate-100">Target Host</label>
                      <Input
                        id="pf-target-host"
                        bind:value={formTargetHost}
                        placeholder="127.0.0.1"
                        class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                        disabled={isSaving}
                      />
                    </div>
                    <div class="space-y-2">
                      <label for="pf-target-port" class="text-sm font-medium text-slate-100">Target Port</label>
                      <Input
                        id="pf-target-port"
                        bind:value={formTargetPort}
                        type="number"
                        min="1"
                        max="65535"
                        class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                        disabled={isSaving}
                      />
                    </div>
                  </div>
                </div>
              </div>

              <div class="flex flex-wrap items-center gap-2 pt-1">
                <Button type="submit" class="rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200" disabled={isSaving}>
                  {#if isSaving}
                    <Loader2 class="size-4 animate-spin" />
                  {/if}
                  {isSaving ? "Saving…" : "Save Forward"}
                </Button>
                <Button type="button" variant="ghost" size="sm" class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white" onclick={closeForm} disabled={isSaving}>
                  Cancel
                </Button>
              </div>
            </div>
          </div>
        {/if}
      </form>
    {/if}

    <div class="mt-6 flex-1 space-y-6">
      <section>
        <div class="flex items-center justify-between gap-3">
          <h2 class="text-sm font-semibold text-slate-100">Saved Forwards</h2>
          <span class="rounded-full border border-cyan-300/15 bg-cyan-300/8 px-2 py-0.5 text-[10px] font-medium text-cyan-200">{savedForwards.length}</span>
        </div>

        {#if sortedSavedForwards.length === 0 && !showForm}
          <div class="mt-3 flex min-h-[12rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground">
            No saved forwards yet. Click "Save Forward" to create one.
          </div>
        {:else if sortedSavedForwards.length > 0}
          <div class="mt-3 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {#each sortedSavedForwards as savedForward (savedForward.id)}
              {@const connection = connectionForForward(savedForward)}
              <article class="group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]">
                <div class="flex items-start gap-3">
                  <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                    <Network class="size-5" />
                  </div>

                  <div class="min-w-0 flex-1">
                    <p class="truncate text-sm font-medium text-white">{savedForward.name}</p>
                    <p class="mt-1 truncate text-xs text-slate-400">
                      {#if connection}
                        {connection.username}@{connection.host}:{connection.port}
                      {:else}
                        Missing connection
                      {/if}
                    </p>
                  </div>
                </div>

                <div class="mt-3 rounded-xl border border-white/8 bg-black/20 px-3 py-2 text-xs">
                  <span class="font-mono text-cyan-100">{savedForward.bind_host}:{savedForward.bind_port}</span>
                  <span class="mx-2 text-slate-600">→</span>
                  <span class="font-mono text-slate-300">{savedForward.target_host}:{savedForward.target_port}</span>
                </div>

                <div class="mt-4 flex items-center gap-2">
                  <Button
                    variant="ghost"
                    size="xs"
                    class="gap-1.5 rounded-xl bg-cyan-300/10 text-cyan-100 hover:bg-cyan-300/18 hover:text-white"
                    onclick={() => handleForward(savedForward)}
                    disabled={!connection || forwardingPresetIds.includes(savedForward.id)}
                  >
                    {#if forwardingPresetIds.includes(savedForward.id)}
                      <Loader2 class="size-3 animate-spin" />
                    {:else}
                      <Play class="size-3" />
                    {/if}
                    Forward
                  </Button>
                  <Button
                    variant="ghost"
                    size="xs"
                    class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                    onclick={() => handleDeleteSaved(savedForward)}
                    disabled={deletingSavedForwardIds.includes(savedForward.id)}
                  >
                    {#if deletingSavedForwardIds.includes(savedForward.id)}
                      <Loader2 class="size-3 animate-spin" />
                    {:else}
                      <Trash2 class="size-3" />
                    {/if}
                    Delete
                  </Button>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </section>

      {#if forwards.length > 0}
        <section>
          <div class="flex items-center justify-between gap-3">
            <h2 class="text-sm font-semibold text-slate-100">Runtime Forwards</h2>
            <span class="rounded-full border border-emerald-300/15 bg-emerald-300/8 px-2 py-0.5 text-[10px] font-medium text-emerald-200">{forwards.length}</span>
          </div>

          <div class="mt-3 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {#each forwards as forward (forward.id)}
              {@const badge = stateBadge(forward.state)}
              <article class="group rounded-[1.35rem] border border-white/8 bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]">
                <div class="flex items-start gap-3">
                  <div class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                    <Network class="size-5" />
                  </div>

                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <p class="truncate text-sm font-medium text-white">{forward.name}</p>
                      <span class="shrink-0 rounded-full border border-white/10 bg-black/20 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide {badge.text}">
                        <span class="mr-1 inline-block size-1.5 shrink-0 rounded-full {badge.tone}"></span>
                        {badge.label}
                      </span>
                    </div>
                    <p class="mt-1 text-xs text-slate-400">{forward.username}@{forward.host}:{forward.port}</p>
                  </div>
                </div>

                <div class="mt-3 rounded-xl border border-white/8 bg-black/20 px-3 py-2 text-xs">
                  <span class="font-mono text-cyan-100">{forward.bind_host}:{forward.bind_port}</span>
                  <span class="mx-2 text-slate-600">→</span>
                  <span class="font-mono text-slate-300">{forward.target_host}:{forward.target_port}</span>
                </div>

                {#if forward.error}
                  <p class="mt-2 text-xs text-red-300">{forward.error}</p>
                {/if}

                <div class="mt-4 flex items-center gap-2">
                  {#if forward.state !== "stopped"}
                    <Button variant="ghost" size="xs" class="gap-1.5 rounded-xl text-slate-400 hover:bg-amber-400/10 hover:text-amber-300" onclick={() => handleStop(forward)} disabled={stoppingForwardIds.includes(forward.id)}>
                      {#if stoppingForwardIds.includes(forward.id)}
                        <Loader2 class="size-3 animate-spin" />
                      {:else}
                        <Square class="size-3" />
                      {/if}
                      Stop
                    </Button>
                  {/if}
                  {#if forward.state === "stopped"}
                    <Button variant="ghost" size="xs" class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300" onclick={() => handleDeleteRuntime(forward)} disabled={deletingRuntimeForwardIds.includes(forward.id)}>
                      {#if deletingRuntimeForwardIds.includes(forward.id)}
                        <Loader2 class="size-3 animate-spin" />
                      {:else}
                        <Trash2 class="size-3" />
                      {/if}
                      Remove
                    </Button>
                  {/if}
                </div>
              </article>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  </section>
</div>
