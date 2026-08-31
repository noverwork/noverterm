<script lang="ts">
  import { ArrowDown, ChevronDown, Network, Plus, Server, Trash2 } from "@lucide/svelte";

  import type {
    PortForwardRecord,
    PortForwardWriteRequest,
  } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import FormSection from "$lib/components/form-section.svelte";
  import FormShell from "$lib/components/form-shell.svelte";
  import { fieldClass, SELECT_CLASS } from "$lib/components/form-styles.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";

  interface Props {
    connections: ConnectionConfig[];
    forward?: PortForwardRecord | null;
    onSave: (
      input: { id?: string } & PortForwardWriteRequest,
    ) => void | Promise<void>;
    onCancel: () => void;
  }

  let { connections, forward = null, onSave, onCancel }: Props = $props();

  interface MappingRow {
    bind_host: string;
    bind_port: string | number;
    target_host: string;
    target_port: string | number;
  }

  function emptyRow(): MappingRow {
    return {
      bind_host: "127.0.0.1",
      bind_port: "",
      target_host: "127.0.0.1",
      target_port: "",
    };
  }

  let error = $state<string | null>(null);
  let isSaving = $state(false);
  let initializedForwardId = $state<string | null>(null);
  let selectedConnectionId = $state("");
  let formName = $state("");
  let rows = $state<MappingRow[]>([emptyRow()]);

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
    selectedConnectionId = forward?.host_id ?? "";
    formName = forward?.name ?? "";
    rows = forward
      ? forward.mappings.map((mapping) => ({
          bind_host: mapping.bind_host,
          bind_port: String(mapping.bind_port),
          target_host: mapping.target_host,
          target_port: String(mapping.target_port),
        }))
      : [emptyRow()];
    error = null;
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

  function onBindPortInput(row: MappingRow) {
    row.target_port = row.bind_port;
  }

  function addRow() {
    rows = [...rows, emptyRow()];
  }

  function removeRow(index: number) {
    rows = rows.filter((_, candidate) => candidate !== index);
  }

  // `type="number"` inputs bind a number, so this takes whatever the row holds.
  function parsePort(value: string | number, label: string): number {
    const trimmedValue = String(value ?? "").trim();
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

    if (rows.length === 0) {
      error = "Add at least one port mapping";
      return;
    }

    isSaving = true;

    try {
      const seenBindAddresses: string[] = [];
      const mappings = rows.map((row, index) => {
        const label = `Row ${index + 1}`;
        const bindHost = row.bind_host.trim();
        const bindPort = parsePort(row.bind_port, `${label}: Bind port`);
        const bindAddress = `${bindHost}:${bindPort}`;
        if (seenBindAddresses.includes(bindAddress)) {
          throw new Error(`${label}: ${bindAddress} is already used above`);
        }
        seenBindAddresses.push(bindAddress);

        return {
          bind_host: bindHost,
          bind_port: bindPort,
          target_host: row.target_host.trim(),
          target_port: parsePort(row.target_port, `${label}: Target port`),
        };
      });

      await onSave({
        ...(forward?.id ? { id: forward.id } : {}),
        name: formName.trim(),
        host_id: selectedConnection.id,
        mappings,
      });
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Failed to save port forward";
    } finally {
      isSaving = false;
    }
  }
</script>

<FormShell
  eyebrow="Network"
  title={formTitle}
  description="Save a reusable SSH tunnel from a local bind port to a target reachable from the selected host."
  formId="port-forward-form"
  {submitLabel}
  {error}
  busy={isSaving}
  onsubmit={handleSubmit}
  {onCancel}
>
  <FormSection
    icon={Server}
    title="Saved connection"
    hint="Target host is resolved from this SSH server, not your local machine."
  >
    {#if sortedConnections.length === 0}
      <div
        class="rounded-2xl border border-dashed border-white/10 bg-white/[0.025] px-4 py-6 text-center"
      >
        <Server class="mx-auto mb-3 size-8 text-slate-600" />
        <p class="text-sm text-slate-400">No saved connections yet.</p>
        <p class="mt-1 text-xs text-slate-500">
          Create a Host in <span class="text-cyan-300/80">Connections</span> first,
          then return here to save a tunnel.
        </p>
      </div>
    {:else}
      <div class="space-y-2">
        <label for="pf-connection" class="text-sm font-medium text-slate-100"
          >Connection</label
        >
        <div class="relative">
          <select
            id="pf-connection"
            bind:value={selectedConnectionId}
            class={SELECT_CLASS}
            disabled={isSaving}
          >
            <option value="" class="bg-slate-900">— Select a saved connection —</option>
            {#each sortedConnections as connection (connection.id)}
              <option value={connection.id} class="bg-slate-900">
                {connection.name} ({connection.username}@{connection.host}:{connection.port})
              </option>
            {/each}
          </select>
          <ChevronDown
            class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-400"
          />
        </div>
      </div>

      {#if selectedConnection}
        <div class="rounded-2xl border border-white/8 bg-black/15 px-3 py-2.5">
          <p class="truncate text-sm font-medium text-white">
            {selectedConnection.name}
          </p>
          <p class="mt-0.5 truncate font-mono text-[11px] text-slate-400">
            {selectedConnection.username}@{selectedConnection.host}:{selectedConnection.port}
          </p>
          <p class="mt-0.5 text-[11px] text-slate-500">
            {getAuthLabel(selectedConnection)}
          </p>
        </div>
      {/if}

      <div class="space-y-2">
        <label for="pf-name" class="text-sm font-medium text-slate-100">Name</label>
        <Input
          id="pf-name"
          bind:value={formName}
          placeholder="My tunnel"
          class={fieldClass()}
          disabled={isSaving}
        />
      </div>
    {/if}
  </FormSection>

  <FormSection
    icon={Network}
    title="Forward route"
    hint="Bind locally, then connect to the target from the SSH host. All ports in this forward start and stop together."
  >
    {#each rows as row, index (index)}
      <div class="rounded-2xl border border-white/8 bg-black/15 p-3">
        <div class="flex items-center justify-between">
          <span
            class="text-[11px] font-medium uppercase tracking-[0.16em] text-slate-500"
          >
            Port {index + 1}
          </span>
          <Button
            type="button"
            variant="ghost"
            size="xs"
            class="rounded-xl text-slate-400 hover:bg-destructive/10 hover:text-destructive"
            onclick={() => removeRow(index)}
            disabled={isSaving || rows.length === 1}
            aria-label={`Remove port ${index + 1}`}
          >
            <Trash2 class="size-3" />
          </Button>
        </div>

        <div class="mt-3 grid gap-2">
          <div class="grid grid-cols-[1fr_7rem] gap-3">
            <div class="space-y-2">
              <label
                for={`pf-bind-host-${index}`}
                class="text-sm font-medium text-slate-100">Bind host</label
              >
              <Input
                id={`pf-bind-host-${index}`}
                bind:value={row.bind_host}
                placeholder="127.0.0.1"
                autocapitalize="none"
                autocomplete="off"
                autocorrect="off"
                spellcheck="false"
                class={fieldClass(false, "font-mono")}
                disabled={isSaving}
              />
            </div>
            <div class="space-y-2">
              <label
                for={`pf-bind-port-${index}`}
                class="text-sm font-medium text-slate-100">Bind port</label
              >
              <Input
                id={`pf-bind-port-${index}`}
                bind:value={row.bind_port}
                type="number"
                inputmode="numeric"
                min="1"
                max="65535"
                class={fieldClass(false, "font-mono")}
                disabled={isSaving}
                oninput={() => onBindPortInput(row)}
              />
            </div>
          </div>

          <div class="flex items-center justify-center py-1" aria-hidden="true">
            <ArrowDown class="size-4 text-slate-600" />
          </div>

          <div class="grid grid-cols-[1fr_7rem] gap-3">
            <div class="space-y-2">
              <label
                for={`pf-target-host-${index}`}
                class="text-sm font-medium text-slate-100">Target host</label
              >
              <Input
                id={`pf-target-host-${index}`}
                bind:value={row.target_host}
                placeholder="127.0.0.1"
                autocapitalize="none"
                autocomplete="off"
                autocorrect="off"
                spellcheck="false"
                class={fieldClass(false, "font-mono")}
                disabled={isSaving}
              />
            </div>
            <div class="space-y-2">
              <label
                for={`pf-target-port-${index}`}
                class="text-sm font-medium text-slate-100">Target port</label
              >
              <Input
                id={`pf-target-port-${index}`}
                bind:value={row.target_port}
                type="number"
                inputmode="numeric"
                min="1"
                max="65535"
                class={fieldClass(false, "font-mono")}
                disabled={isSaving}
              />
            </div>
          </div>
        </div>
      </div>
    {/each}

    <Button
      type="button"
      variant="ghost"
      size="sm"
      class="gap-1.5 rounded-2xl bg-cyan-300/10 text-cyan-100 hover:bg-cyan-300/18 hover:text-white"
      onclick={addRow}
      disabled={isSaving}
    >
      <Plus class="size-3.5" />
      Add port
    </Button>
  </FormSection>
</FormShell>
