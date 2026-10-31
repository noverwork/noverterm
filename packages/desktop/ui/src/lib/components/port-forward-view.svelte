<script lang="ts">
  import {
    ArrowRight,
    CircleAlert,
    Loader2,
    Network,
    Pencil,
    Play,
    Plus,
    Search,
    Square,
    Trash2,
  } from "@lucide/svelte";

  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { PortForwardRecord } from "$lib/api/types.js";
  import type { ConnectionConfig } from "$lib/app-data-types.js";
  import { groupState, runtimeForwardsFor } from "$lib/port-forward-group.js";
  import type { PortForward } from "$lib/stores/port-forward.svelte.js";

  interface Props {
    connections: ConnectionConfig[];
    savedForwards: PortForwardRecord[];
    forwards: PortForward[];
    onNew: () => void | Promise<void>;
    onEdit: (forward: PortForwardRecord) => void | Promise<void>;
    onForward: (forward: PortForwardRecord) => Promise<PortForward[]>;
    onStop: (forwardId: string) => Promise<PortForward>;
    onDeleteSaved: (forwardId: string) => Promise<void>;
    onDeleteRuntime: (forwardId: string) => Promise<void>;
  }

  let {
    connections,
    savedForwards,
    forwards,
    onNew,
    onEdit,
    onForward,
    onStop,
    onDeleteSaved,
    onDeleteRuntime,
  }: Props = $props();

  let error = $state<string | null>(null);
  let forwardingPresetIds = $state<string[]>([]);
  let deletingSavedForwardIds = $state<string[]>([]);
  let deletingRuntimeForwardIds = $state<string[]>([]);
  let pendingDeleteTarget = $state<PortForwardRecord | null>(null);
  let searchQuery = $state("");

  let sortedSavedForwards = $derived(
    [...savedForwards].sort((left, right) =>
      left.name.localeCompare(right.name),
    ),
  );

  let visibleForwards = $derived.by(() => {
    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return sortedSavedForwards;
    }
    return sortedSavedForwards.filter((forward) => {
      if (forward.name.toLowerCase().includes(query)) {
        return true;
      }
      const connection = connectionForForward(forward);
      if (
        connection &&
        `${connection.username}@${connection.host}`.toLowerCase().includes(query)
      ) {
        return true;
      }
      return forward.mappings.some(
        (mapping) =>
          `${mapping.bind_host}:${mapping.bind_port}`.toLowerCase().includes(query) ||
          `${mapping.target_host}:${mapping.target_port}`.toLowerCase().includes(query),
      );
    });
  });

  function connectionForForward(
    forward: PortForwardRecord,
  ): ConnectionConfig | null {
    return (
      connections.find((connection) => connection.id === forward.host_id) ?? null
    );
  }

  function runtimesFor(
    savedForward: PortForwardRecord,
    connection: ConnectionConfig | null,
  ): (PortForward | null)[] {
    return runtimeForwardsFor(savedForward, connection, forwards);
  }

  function isBusy(id: string): boolean {
    return (
      forwardingPresetIds.includes(id) ||
      deletingRuntimeForwardIds.includes(id) ||
      deletingSavedForwardIds.includes(id)
    );
  }

  function handleCardActivate(
    savedForward: PortForwardRecord,
    connection: ConnectionConfig | null,
    runtimes: (PortForward | null)[],
    state: PortForward["state"] | null,
  ) {
    if (isBusy(savedForward.id)) {
      return;
    }
    if (state) {
      void handleStopGroup(savedForward, runtimes);
    } else if (connection) {
      void handleForward(savedForward);
    }
  }

  async function handleForward(savedForward: PortForwardRecord) {
    error = null;
    forwardingPresetIds = [...forwardingPresetIds, savedForward.id];

    try {
      await onForward(savedForward);
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Failed to start port forward";
    } finally {
      forwardingPresetIds = forwardingPresetIds.filter(
        (id) => id !== savedForward.id,
      );
    }
  }

  async function handleStopGroup(
    savedForward: PortForwardRecord,
    runtimes: (PortForward | null)[],
  ) {
    error = null;
    deletingRuntimeForwardIds = [
      ...deletingRuntimeForwardIds,
      savedForward.id,
    ];

    try {
      for (const forward of runtimes) {
        if (!forward) {
          continue;
        }
        if (forward.state === "connecting" || forward.state === "listening") {
          await onStop(forward.id);
        }
        await onDeleteRuntime(forward.id);
      }
    } catch (cause) {
      error =
        cause instanceof Error ? cause.message : "Failed to stop port forward";
    } finally {
      deletingRuntimeForwardIds = deletingRuntimeForwardIds.filter(
        (id) => id !== savedForward.id,
      );
    }
  }

  function requestDeleteSaved(forward: PortForwardRecord) {
    pendingDeleteTarget = forward;
    error = null;
  }

  async function confirmDelete() {
    if (!pendingDeleteTarget) {
      return;
    }

    await confirmDeleteSaved(pendingDeleteTarget);
  }

  async function confirmDeleteSaved(forward: PortForwardRecord) {
    deletingSavedForwardIds = [...deletingSavedForwardIds, forward.id];

    try {
      await onDeleteSaved(forward.id);
      pendingDeleteTarget = null;
    } catch (cause) {
      error =
        cause instanceof Error
          ? cause.message
          : "Failed to delete saved port forward";
    } finally {
      deletingSavedForwardIds = deletingSavedForwardIds.filter(
        (id) => id !== forward.id,
      );
    }
  }

  function stateBadge(state: PortForward["state"]) {
    switch (state) {
      case "listening":
        return {
          tone: "bg-emerald-400",
          label: "Listening",
          text: "text-emerald-300",
          border: "border-emerald-300/30",
        };
      case "connecting":
        return {
          tone: "bg-amber-300 motion-safe:animate-pulse",
          label: "Connecting",
          text: "text-amber-300",
          border: "border-amber-300/30",
        };
      case "error":
        return {
          tone: "bg-red-400",
          label: "Error",
          text: "text-red-300",
          border: "border-red-300/35",
        };
      default:
        return {
          tone: "bg-stencil-faint",
          label: "Stopped",
          text: "text-stencil-mute",
          border: "border-white/10",
        };
    }
  }

  function deleteDialogTitle(): string {
    return "Delete saved forward?";
  }

  function deleteDialogDescription(): string {
    return "This removes the saved forwarding preset. Existing runtime forwards are not stopped automatically.";
  }

  function deleteDialogItemName(): string | undefined {
    return pendingDeleteTarget?.name;
  }

  function deleteDialogIsDeleting(): boolean {
    if (!pendingDeleteTarget) {
      return false;
    }

    return deletingSavedForwardIds.includes(pendingDeleteTarget.id);
  }
</script>

<div
  class="workspace-canvas flex h-full min-h-0 flex-col overflow-hidden px-5 py-6 lg:px-8"
>
  <section class="flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-stencil sm:p-6">
    <div
      class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between"
    >
      <div>
        <h1 class="text-2xl font-semibold tracking-tight">
          Port Forwards
        </h1>
        <p class="mt-2 text-sm text-stencil-mute">
          Save reusable SSH tunnels, then start them with one click.
        </p>
      </div>

      <div class="flex items-center gap-2 self-start">
        <div class="relative">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-stencil-faint"
          />
          <Input
            type="search"
            bind:value={searchQuery}
            placeholder="Search forwards"
            aria-label="Search port forwards"
            class="h-8 w-48 rounded-[2px] border border-white/10 bg-rail pl-8 text-sm text-stencil placeholder:text-stencil-faint focus-visible:border-stencil"
          />
        </div>

        <Button
          onclick={onNew}
          variant="default"
          size="sm"
          class="gap-2 rounded-[2px] bg-stencil text-port hover:bg-white"
        >
          <Plus class="size-3.5" />
          New Forward
        </Button>
      </div>
    </div>

    {#if error}
      <div
        class="mt-5 rounded-[2px] border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        role="alert"
      >
        {error}
      </div>
    {/if}

    <div class="mt-6 min-h-0 flex-1 overflow-y-auto pr-1">
      {#if sortedSavedForwards.length === 0}
        <div
          class="flex h-full min-h-[16rem] items-center justify-center rounded-[2px] border border-dashed border-white/10 bg-transparent px-4 py-8 text-center text-sm text-muted-foreground"
        >
          No saved forwards yet
        </div>
      {:else if visibleForwards.length === 0}
        <div
          class="rounded-[2px] border border-dashed border-white/10 bg-transparent px-4 py-8 text-center text-sm text-muted-foreground"
        >
          No forwards match your search.
        </div>
      {:else}
        <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each visibleForwards as savedForward (savedForward.id)}
            {@const connection = connectionForForward(savedForward)}
            {@const runtimes = runtimesFor(savedForward, connection)}
            {@const state = groupState(runtimes)}
            {@const badge = state ? stateBadge(state) : null}
            <ContextMenu.Root>
              <ContextMenu.Trigger class="contents">
                <div
                  role="button"
                  tabindex="0"
                  aria-label={state
                    ? `Stop ${savedForward.name}`
                    : `Start ${savedForward.name}`}
                  onclick={() =>
                    handleCardActivate(savedForward, connection, runtimes, state)}
                  onkeydown={(event) => {
                    if (event.key === "Enter") {
                      handleCardActivate(savedForward, connection, runtimes, state);
                    }
                  }}
                  class="group cursor-pointer rounded-[2px] border bg-rib px-4 py-4 transition hover:bg-[#1f252c] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil {badge
                    ? badge.border
                    : 'border-white/10 hover:border-white/20'}"
                >
                  <div class="flex items-start gap-3">
                    <div
                      class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-[2px] border border-white/10 bg-transparent text-stencil-dim"
                    >
                      {#if isBusy(savedForward.id)}
                        <Loader2 class="size-5 animate-spin" />
                      {:else}
                        <Network class="size-5" />
                      {/if}
                    </div>

                    <div class="min-w-0 flex-1">
                      <div class="flex items-center gap-2">
                        <p class="truncate text-sm font-medium text-stencil">
                          {savedForward.name}
                        </p>
                        <span
                          class="flex shrink-0 items-center gap-1.5 rounded-[2px] border border-white/10 bg-rib px-1.5 text-[11px] uppercase tracking-[0.14em] {badge
                            ? badge.text
                            : 'text-stencil-mute'}"
                        >
                          <span
                            class="inline-block size-1.5 rounded-full {badge
                              ? badge.tone
                              : 'bg-stencil-faint'}"
                          ></span>
                          {badge ? badge.label : "Idle"}
                        </span>
                      </div>
                      {#if connection}
                        <p class="mt-1 truncate font-mono text-xs text-stencil-mute">
                          {connection.username}@{connection.host}:{connection.port}
                        </p>
                      {:else}
                        <p class="mt-1 flex items-center gap-1.5 text-xs text-amber-300">
                          <CircleAlert class="size-3 shrink-0" />
                          Connection was deleted
                        </p>
                      {/if}
                    </div>

                    <div
                      class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
                    >
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        class="rounded-[2px] text-stencil-mute hover:bg-white/8 hover:text-stencil"
                        aria-label={`Edit ${savedForward.name}`}
                        title={state ? "Stop the forward to edit it" : "Edit"}
                        disabled={state !== null}
                        onclick={(event) => {
                          event.stopPropagation();
                          void onEdit(savedForward);
                        }}
                      >
                        <Pencil class="size-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        class="rounded-[2px] text-stencil-mute hover:bg-seal/20 hover:text-red-300"
                        aria-label={`Delete ${savedForward.name}`}
                        title={state ? "Stop the forward to delete it" : "Delete"}
                        disabled={state !== null ||
                          deletingSavedForwardIds.includes(savedForward.id)}
                        onclick={(event) => {
                          event.stopPropagation();
                          requestDeleteSaved(savedForward);
                        }}
                      >
                        <Trash2 class="size-3.5" />
                      </Button>
                    </div>
                  </div>

                  <ul
                    class="mt-3 divide-y divide-white/6 rounded-[2px] border border-white/10 bg-rail text-xs"
                  >
                    {#each savedForward.mappings as mapping, index (index)}
                      {@const runtime = runtimes[index]}
                      <li class="px-3 py-2">
                        <div class="flex items-center gap-2">
                          <span
                            class="inline-block size-1.5 shrink-0 rounded-full {runtime
                              ? stateBadge(runtime.state).tone
                              : 'bg-stencil-faint'}"
                          ></span>
                          <span class="truncate font-mono text-stencil"
                            >{mapping.bind_host}:{mapping.bind_port}</span
                          >
                          <ArrowRight
                            class="size-3 shrink-0 text-stencil-faint"
                            aria-label="forwards to"
                          />
                          <span class="truncate font-mono text-stencil-dim"
                            >{mapping.target_host}:{mapping.target_port}</span
                          >
                        </div>
                        {#if runtime?.error}
                          <p
                            class="mt-1 flex items-start gap-1.5 pl-3.5 text-red-300"
                            role="alert"
                          >
                            <CircleAlert class="mt-px size-3 shrink-0" />
                            <span class="break-words">{runtime.error}</span>
                          </p>
                        {/if}
                      </li>
                    {/each}
                  </ul>
                </div>
              </ContextMenu.Trigger>

              <ContextMenu.Content
                class="min-w-44 rounded-[2px] border-white/10 bg-rail text-stencil"
              >
                <ContextMenu.Label class="max-w-56 truncate text-stencil-mute">
                  {savedForward.name}
                </ContextMenu.Label>
                <ContextMenu.Separator class="bg-white/10" />
                {#if state}
                  <ContextMenu.Item
                    class="cursor-pointer"
                    disabled={deletingRuntimeForwardIds.includes(savedForward.id)}
                    onclick={() => void handleStopGroup(savedForward, runtimes)}
                  >
                    <Square class="size-3.5" />
                    Stop
                  </ContextMenu.Item>
                {:else}
                  <ContextMenu.Item
                    class="cursor-pointer rounded-[2px] focus:bg-white/8 focus:text-stencil"
                    disabled={!connection ||
                      forwardingPresetIds.includes(savedForward.id)}
                    onclick={() => void handleForward(savedForward)}
                  >
                    <Play class="size-3.5" />
                    Forward
                  </ContextMenu.Item>
                {/if}
                <ContextMenu.Item
                  class="cursor-pointer rounded-[2px] focus:bg-white/8 focus:text-stencil"
                  disabled={state !== null}
                  onclick={() => void onEdit(savedForward)}
                >
                  <Pencil class="size-3.5" />
                  Edit
                </ContextMenu.Item>
                <ContextMenu.Separator class="bg-white/10" />
                <ContextMenu.Item
                  class="cursor-pointer rounded-[2px] text-red-300 focus:bg-seal/20 focus:text-red-200"
                  disabled={state !== null ||
                    deletingSavedForwardIds.includes(savedForward.id)}
                  onclick={() => requestDeleteSaved(savedForward)}
                >
                  <Trash2 class="size-3.5" />
                  Delete
                </ContextMenu.Item>
              </ContextMenu.Content>
            </ContextMenu.Root>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</div>

<DeleteConfirmDialog
  open={pendingDeleteTarget !== null}
  title={deleteDialogTitle()}
  description={deleteDialogDescription()}
  itemName={deleteDialogItemName()}
  confirmLabel="Delete forward"
  isDeleting={deleteDialogIsDeleting()}
  onConfirm={confirmDelete}
  onCancel={() => (pendingDeleteTarget = null)}
/>
