<script lang="ts">
  import { Loader2, Network, Server } from "@lucide/svelte";

  import type {
    ConnectionConfig,
    SavePortForwardInput,
    SavedPortForwardConfig,
  } from "$lib/app-data-types.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    connections: ConnectionConfig[];
    forward?: SavedPortForwardConfig | null;
    onSave: (input: SavePortForwardInput) => void | Promise<void>;
    onCancel: () => void;
  }

  let { connections, forward = null, onSave, onCancel }: Props = $props();

  let error = $state<string | null>(null);
  let isSaving = $state(false);
  let initializedForwardId = $state<string | null>(null);
  let selectedConnectionId = $state<string | null>(null);
  let formName = $state("");
  let formBindHost = $state("127.0.0.1");
  let formBindPort = $state("");
  let formTargetHost = $state("127.0.0.1");
  let formTargetPort = $state("");
  let autoFilledTargetPort = $state(false);

  let sortedConnections = $derived(
    [...connections].sort((left, right) => left.name.localeCompare(right.name)),
  );

  let selectedConnection = $derived(
    selectedConnectionId
      ? (connections.find(
          (connection) => connection.id === selectedConnectionId,
        ) ?? null)
      : null,
  );

  const formTitle = $derived(forward ? "Edit port forward" : "New port forward");
  const submitLabel = $derived.by(() => {
    if (isSaving) {
      return "Saving…";
    }

    return forward ? "Save changes" : "Save forward";
  });

  $effect(() => {
    const forwardId = forward?.id ?? "new";
    if (initializedForwardId === forwardId) {
      return;
    }

    initializedForwardId = forwardId;
    selectedConnectionId = forward?.connectionId ?? null;
    formName = forward?.name ?? "";
    formBindHost = forward?.bind_host ?? "127.0.0.1";
    formBindPort = forward ? String(forward.bind_port) : "";
    formTargetHost = forward?.target_host ?? "127.0.0.1";
    formTargetPort = forward ? String(forward.target_port) : "";
    autoFilledTargetPort = false;
    error = null;
  });

  $effect(() => {
    if (formBindPort && autoFilledTargetPort) {
      formTargetPort = String(formBindPort);
    }
  });

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

  function onBindPortInput() {
    autoFilledTargetPort = true;
    formTargetPort = String(formBindPort);
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

  async function handleSubmit() {
    error = null;

    if (!selectedConnection) {
      error = "Select a saved connection to continue";
      return;
    }

    if (!formName.trim()) {
      error = "Name is required";
      return;
    }

    if (!String(formBindPort).trim()) {
      error = "Bind port is required";
      return;
    }

    if (!String(formTargetPort).trim()) {
      error = "Target port is required";
      return;
    }

    isSaving = true;

    try {
      await onSave({
        ...(forward?.id ? { id: forward.id } : {}),
        name: formName.trim(),
        connectionId: selectedConnection.id,
        bind_host: formBindHost.trim(),
        bind_port: parsePort(String(formBindPort), "Bind port"),
        target_host: formTargetHost.trim(),
        target_port: parsePort(String(formTargetPort), "Target port"),
      });
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Failed to save port forward";
    } finally {
      isSaving = false;
    }
  }
</script>

<div
  class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8"
>
  <section
    class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6"
  >
    <div
      class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between"
    >
      <div>
        <p class="section-title text-cyan-200/70">Network</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">{formTitle}</h1>
        <p class="mt-2 text-sm text-slate-500">
          Save a reusable SSH tunnel from a local bind port to a target reachable
          from the selected host.
        </p>
      </div>

      <Button
        type="button"
        variant="ghost"
        size="sm"
        class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white"
        onclick={onCancel}
        disabled={isSaving}
      >
        Cancel
      </Button>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto pr-1">
      {#if error}
        <div
          class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
          role="alert"
        >
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
          <h2 class="text-sm font-semibold text-cyan-100">Port Forward</h2>
          <button
            type="button"
            class="cursor-pointer text-xs text-slate-400 transition-colors hover:text-white"
            onclick={onCancel}
            disabled={isSaving}
          >
            Cancel
          </button>
        </div>

        <div class="mt-4 grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
          <div class="space-y-4">
            <div class="rounded-2xl border border-white/8 bg-white/[0.035] p-4">
              <div class="flex items-center gap-3">
                <div
                  class="flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200"
                >
                  <Server class="size-5" />
                </div>
                <div>
                  <h3
                    class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500"
                  >
                    Saved Connection
                  </h3>
                  <p class="mt-1 text-xs text-slate-400">
                    Target host is resolved from this SSH server, not your local
                    machine.
                  </p>
                </div>
              </div>

              {#if sortedConnections.length === 0}
                <div
                  class="mt-4 rounded-2xl border border-dashed border-white/10 bg-white/[0.025] px-4 py-6 text-center"
                >
                  <Server class="mx-auto mb-3 size-8 text-slate-600" />
                  <p class="text-sm text-slate-400">No saved connections yet.</p>
                  <p class="mt-1 text-xs text-slate-500">
                    Create a Host in <span class="text-cyan-300/80"
                      >Connections</span
                    > first, then return here to save a tunnel.
                  </p>
                </div>
              {:else}
                <div
                  class="mt-4 grid max-h-80 gap-2 overflow-y-auto pr-2 sm:grid-cols-2"
                  style="scrollbar-gutter: stable;"
                >
                  {#each sortedConnections as connection (connection.id)}
                    <button
                      type="button"
                      class="cursor-pointer rounded-xl border px-3 py-3 text-left transition-all {selectedConnectionId ===
                      connection.id
                        ? 'border-cyan-300/30 bg-cyan-300/10 shadow-[0_0_20px_rgb(34_211_238/0.08)]'
                        : 'border-white/8 bg-white/[0.03] hover:border-white/14 hover:bg-white/[0.055]'}"
                      onclick={() => (selectedConnectionId = connection.id)}
                    >
                      <div class="flex items-start gap-2.5">
                        <div
                          class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border border-cyan-300/14 bg-cyan-300/8 text-cyan-200"
                        >
                          <Server class="size-4" />
                        </div>
                        <div class="min-w-0 flex-1">
                          <p class="truncate text-sm font-medium text-white">
                            {connection.name}
                          </p>
                          <p
                            class="mt-0.5 truncate font-mono text-[11px] text-slate-400"
                          >
                            {connection.username}@{connection.host}:{connection.port}
                          </p>
                          <p class="mt-0.5 text-[11px] text-slate-500">
                            {getAuthLabel(connection)}
                          </p>
                        </div>
                      </div>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>

            <div class="space-y-2">
              <label for="pf-name" class="text-sm font-medium text-slate-100"
                >Name</label
              >
              <Input
                id="pf-name"
                bind:value={formName}
                placeholder="My tunnel"
                class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                disabled={isSaving}
              />
            </div>
          </div>

          <div class="space-y-4">
            <div class="rounded-2xl border border-white/8 bg-white/[0.035] p-4">
              <div class="flex items-center gap-3">
                <div
                  class="flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200"
                >
                  <Network class="size-5" />
                </div>
                <div>
                  <h3
                    class="text-xs font-medium uppercase tracking-[0.16em] text-slate-500"
                  >
                    Forward Route
                  </h3>
                  <p class="mt-1 text-xs text-slate-400">
                    Bind locally, then connect to the target from the SSH host.
                  </p>
                </div>
              </div>

              <div class="mt-4 grid gap-3">
                <div class="grid grid-cols-[1fr_6rem] gap-3">
                  <div class="space-y-2">
                    <label
                      for="pf-bind-host"
                      class="text-sm font-medium text-slate-100">Bind Host</label
                    >
                    <Input
                      id="pf-bind-host"
                      bind:value={formBindHost}
                      placeholder="127.0.0.1"
                      class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                      disabled={isSaving}
                    />
                  </div>
                  <div class="space-y-2">
                    <label
                      for="pf-bind-port"
                      class="text-sm font-medium text-slate-100">Bind Port</label
                    >
                    <Input
                      id="pf-bind-port"
                      bind:value={formBindPort}
                      type="number"
                      min="1"
                      max="65535"
                      class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                      disabled={isSaving}
                      oninput={onBindPortInput}
                    />
                  </div>
                </div>

                <div class="flex items-center justify-center">
                  <span class="text-lg text-slate-600">↓</span>
                </div>

                <div class="grid grid-cols-[1fr_6rem] gap-3">
                  <div class="space-y-2">
                    <label
                      for="pf-target-host"
                      class="text-sm font-medium text-slate-100"
                      >Target Host</label
                    >
                    <Input
                      id="pf-target-host"
                      bind:value={formTargetHost}
                      placeholder="127.0.0.1"
                      class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                      disabled={isSaving}
                    />
                  </div>
                  <div class="space-y-2">
                    <label
                      for="pf-target-port"
                      class="text-sm font-medium text-slate-100"
                      >Target Port</label
                    >
                    <Input
                      id="pf-target-port"
                      bind:value={formTargetPort}
                      type="number"
                      min="1"
                      max="65535"
                      class="border-white/10 bg-black/20 text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40"
                      disabled={isSaving}
                      oninput={() => (autoFilledTargetPort = false)}
                    />
                  </div>
                </div>
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-2 pt-1">
              <Button
                type="submit"
                class="rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
                disabled={isSaving}
              >
                {#if isSaving}
                  <Loader2 class="size-4 animate-spin" />
                {/if}
                {submitLabel}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                class="rounded-2xl text-slate-300 hover:bg-white/8 hover:text-white"
                onclick={onCancel}
                disabled={isSaving}
              >
                Cancel
              </Button>
            </div>
          </div>
        </div>
      </form>
    </div>
  </section>
</div>
