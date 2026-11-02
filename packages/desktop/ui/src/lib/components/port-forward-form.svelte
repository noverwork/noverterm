<script lang="ts">
  import { ArrowDown, ChevronDown, Network, Plus, Server, Trash2 } from "@lucide/svelte";

  import type {
    PortForwardRecord,
    PortForwardWriteRequest,
  } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import { t, tParts } from "$lib/i18n/index.svelte.js";
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

  const formTitle = $derived(forward ? t("Edit port forward") : t("New port forward"));
  const submitLabel = $derived.by(() => {
    if (isSaving) {
      return t("Saving…");
    }

    return forward ? t("Save changes") : t("Save forward");
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
        return t("Key + Password");
      case "public_key":
        return t("SSH Key");
      default:
        return t("Password");
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
      throw new Error(t("{label} must be a number from 1 to 65535", { label }));
    }

    const port = Number.parseInt(trimmedValue, 10);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      throw new Error(t("{label} must be a number from 1 to 65535", { label }));
    }
    return port;
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    error = null;

    if (!selectedConnection) {
      error = t("Select a saved connection to continue");
      return;
    }

    if (!formName.trim()) {
      error = t("Name is required");
      return;
    }

    if (rows.length === 0) {
      error = t("Add at least one port mapping");
      return;
    }

    isSaving = true;

    try {
      const seenBindAddresses: string[] = [];
      const mappings = rows.map((row, index) => {
        const label = t("Row {number}", { number: index + 1 });
        const bindHost = row.bind_host.trim();
        const bindPort = parsePort(row.bind_port, t("{label}: Bind port", { label }));
        const bindAddress = `${bindHost}:${bindPort}`;
        if (seenBindAddresses.includes(bindAddress)) {
          throw new Error(t("{label}: {address} is already used above", { label, address: bindAddress }));
        }
        seenBindAddresses.push(bindAddress);

        return {
          bind_host: bindHost,
          bind_port: bindPort,
          target_host: row.target_host.trim(),
          target_port: parsePort(row.target_port, t("{label}: Target port", { label })),
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
        cause instanceof Error ? cause.message : t("Failed to save port forward");
    } finally {
      isSaving = false;
    }
  }
</script>

<FormShell
  title={formTitle}
  description={t("Save a reusable SSH tunnel from a local bind port to a target reachable from the selected host.")}
  formId="port-forward-form"
  {submitLabel}
  {error}
  busy={isSaving}
  onsubmit={handleSubmit}
  {onCancel}
>
  <FormSection
    icon={Server}
    title={t("Saved connection")}
    hint={t("Target host is resolved from this SSH server, not your local machine.")}
  >
    {#if sortedConnections.length === 0}
      <div
        class="rounded-[2px] border border-dashed border-white/10 bg-transparent px-4 py-6 text-center"
      >
        <Server class="mx-auto mb-3 size-8 text-stencil-faint" />
        <p class="text-sm text-stencil-mute">{t("No saved connections yet.")}</p>
        <p class="mt-1 text-xs text-stencil-mute">
          {#each tParts("Create a Host in {page} first, then return here to save a tunnel.", ["page"]) as part, index (index)}{#if index % 2 === 0}{part}{:else}<span class="text-stencil">{t("Connections")}</span>{/if}{/each}
        </p>
      </div>
    {:else}
      <div class="space-y-2">
        <label for="pf-connection" class="text-sm font-medium text-stencil"
          >{t("Connection")}</label
        >
        <div class="relative">
          <select
            id="pf-connection"
            bind:value={selectedConnectionId}
            class={SELECT_CLASS}
            disabled={isSaving}
          >
            <option value="" class="bg-rail">{t("— Select a saved connection —")}</option>
            {#each sortedConnections as connection (connection.id)}
              <option value={connection.id} class="bg-rail">
                {connection.name} ({connection.username}@{connection.host}:{connection.port})
              </option>
            {/each}
          </select>
          <ChevronDown
            class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-stencil-mute"
          />
        </div>
      </div>

      {#if selectedConnection}
        <div class="rounded-[2px] border border-white/10 bg-rail px-3 py-2.5">
          <p class="truncate text-sm font-medium text-stencil">
            {selectedConnection.name}
          </p>
          <p class="mt-0.5 truncate font-mono text-[11px] text-stencil-mute">
            {selectedConnection.username}@{selectedConnection.host}:{selectedConnection.port}
          </p>
          <p class="mt-0.5 text-[11px] text-stencil-mute">
            {getAuthLabel(selectedConnection)}
          </p>
        </div>
      {/if}

      <div class="space-y-2">
        <label for="pf-name" class="text-sm font-medium text-stencil">{t("Name")}</label>
        <Input
          id="pf-name"
          bind:value={formName}
          placeholder={t("e.g. {example}", { example: "Staging Postgres" })}
          class={fieldClass()}
          disabled={isSaving}
        />
      </div>
    {/if}
  </FormSection>

  <FormSection
    icon={Network}
    title={t("Forward route")}
    hint={t("Bind locally, then connect to the target from the SSH host. All ports in this forward start and stop together.")}
  >
    {#each rows as row, index (index)}
      <div class="rounded-[2px] border border-white/10 bg-rail p-3">
        <div class="flex items-center justify-between">
          <span
            class="section-title"
          >
            {t("Port {number}", { number: index + 1 })}
          </span>
          <Button
            type="button"
            variant="ghost"
            size="xs"
            class="rounded-[2px] text-stencil-mute hover:bg-destructive/10 hover:text-destructive"
            onclick={() => removeRow(index)}
            disabled={isSaving || rows.length === 1}
            aria-label={t("Remove port {number}", { number: index + 1 })}
          >
            <Trash2 class="size-3" />
          </Button>
        </div>

        <div class="mt-3 grid gap-2">
          <div class="grid grid-cols-[1fr_7rem] gap-3">
            <div class="space-y-2">
              <label
                for={`pf-bind-host-${index}`}
                class="text-sm font-medium text-stencil">{t("Bind host")}</label
              >
              <Input
                id={`pf-bind-host-${index}`}
                bind:value={row.bind_host}
                placeholder={t("e.g. {example}", { example: "127.0.0.1" })}
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
                class="text-sm font-medium text-stencil">{t("Bind port")}</label
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
            <ArrowDown class="size-4 text-stencil-faint" />
          </div>

          <div class="grid grid-cols-[1fr_7rem] gap-3">
            <div class="space-y-2">
              <label
                for={`pf-target-host-${index}`}
                class="text-sm font-medium text-stencil">{t("Target host")}</label
              >
              <Input
                id={`pf-target-host-${index}`}
                bind:value={row.target_host}
                placeholder={t("e.g. {example}", { example: "127.0.0.1" })}
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
                class="text-sm font-medium text-stencil">{t("Target port")}</label
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
      class="gap-1.5 rounded-[2px] border border-white/10 bg-transparent text-stencil hover:border-white/20 hover:bg-white/5"
      onclick={addRow}
      disabled={isSaving}
    >
      <Plus class="size-3.5" />
      {t("Add port")}
    </Button>
  </FormSection>
</FormShell>
