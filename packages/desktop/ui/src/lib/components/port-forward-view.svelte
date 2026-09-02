<script lang="ts">
  import {
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

  function cardClass(state: PortForward["state"] | null): string {
    const base =
      "group cursor-pointer rounded-[1.35rem] border bg-white/[0.03] px-4 py-4 transition hover:bg-white/[0.055] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/40";

    switch (state) {
      case "listening":
        return `${base} forward-card--listening border-emerald-300/55`;
      case "connecting":
        return `${base} forward-card--connecting border-amber-300/55`;
      case "error":
        return `${base} forward-card--error border-red-300/55`;
      default:
        return `${base} border-white/8 hover:border-cyan-300/30`;
    }
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
          tone: "bg-emerald-400 shadow-[0_0_14px_rgb(52_211_153/0.65)]",
          label: "Listening",
          text: "text-emerald-300",
        };
      case "connecting":
        return {
          tone: "bg-amber-300 shadow-[0_0_14px_rgb(252_211_77/0.55)]",
          label: "Connecting",
          text: "text-amber-300",
        };
      case "error":
        return {
          tone: "bg-red-400 shadow-[0_0_14px_rgb(248_113_113/0.55)]",
          label: "Error",
          text: "text-red-300",
        };
      default:
        return {
          tone: "bg-slate-500",
          label: "Stopped",
          text: "text-slate-400",
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
  <section class="ide-panel flex min-h-0 flex-1 flex-col overflow-hidden p-5 text-white sm:p-6">
    <div
      class="flex flex-col gap-4 border-b border-white/10 pb-5 sm:flex-row sm:items-start sm:justify-between"
    >
      <div>
        <p class="section-title text-cyan-200/70">Network</p>
        <h1 class="mt-2 text-2xl font-semibold tracking-tight">
          Port Forwards
        </h1>
        <p class="mt-2 text-sm text-slate-500">
          Save reusable SSH tunnels, then start them with one click.
        </p>
      </div>

      <div class="flex items-center gap-2 self-start">
        <div class="relative">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-slate-500"
          />
          <Input
            type="search"
            bind:value={searchQuery}
            placeholder="Search forwards"
            aria-label="Search port forwards"
            class="h-8 w-48 rounded-2xl border-white/10 bg-white/[0.03] pl-8 text-sm text-white placeholder:text-slate-500 focus-visible:border-cyan-300/40 focus-visible:ring-cyan-300/20"
          />
        </div>

        <Button
          onclick={onNew}
          variant="default"
          size="sm"
          class="gap-2 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
        >
          <Plus class="size-3.5" />
          New Forward
        </Button>
      </div>
    </div>

    {#if error}
      <div
        class="mt-5 rounded-2xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive"
        role="alert"
      >
        {error}
      </div>
    {/if}

    <div class="mt-6 min-h-0 flex-1 overflow-y-auto pr-1">
      {#if sortedSavedForwards.length === 0}
        <div
          class="flex h-full min-h-[16rem] items-center justify-center rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground"
        >
          No saved forwards yet
        </div>
      {:else if visibleForwards.length === 0}
        <div
          class="rounded-[1.35rem] border border-dashed border-white/10 bg-white/[0.025] px-4 py-8 text-center text-sm text-muted-foreground"
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
                  class={cardClass(state)}
                >
                  <div class="flex items-start gap-3">
                    <div
                      class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200"
                    >
                      {#if isBusy(savedForward.id)}
                        <Loader2 class="size-5 animate-spin" />
                      {:else}
                        <Network class="size-5" />
                      {/if}
                    </div>

                    <div class="min-w-0 flex-1">
                      <div class="flex items-center gap-2">
                        <p class="truncate text-sm font-medium text-white">
                          {savedForward.name}
                        </p>
                        {#if badge}
                          <span
                            class="shrink-0 rounded-full border border-white/10 bg-black/20 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide {badge.text}"
                          >
                            <span
                              class="mr-1 inline-block size-1.5 shrink-0 rounded-full {badge.tone}"
                            ></span>
                            {badge.label}
                          </span>
                        {/if}
                      </div>
                      <p class="mt-1 truncate text-xs text-slate-400">
                        {#if connection}
                          {connection.username}@{connection.host}:{connection.port}
                        {:else}
                          Missing connection
                        {/if}
                      </p>
                    </div>

                    <div
                      class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
                    >
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        class="rounded-xl text-slate-400 hover:bg-white/8 hover:text-white"
                        aria-label={`Edit ${savedForward.name}`}
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
                        class="rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                        aria-label={`Delete ${savedForward.name}`}
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

                  <div
                    class="mt-3 space-y-1.5 rounded-xl border border-white/8 bg-black/20 px-3 py-2 text-xs"
                  >
                    {#each savedForward.mappings as mapping, index (index)}
                      {@const runtime = runtimes[index]}
                      <div>
                        <div class="flex items-center">
                          <span
                            class="mr-2 inline-block size-1.5 shrink-0 rounded-full {runtime
                              ? stateBadge(runtime.state).tone
                              : 'bg-slate-600'}"
                          ></span>
                          <span class="font-mono text-cyan-100"
                            >{mapping.bind_host}:{mapping.bind_port}</span
                          >
                          <span class="mx-2 text-slate-600">→</span>
                          <span class="font-mono text-slate-300"
                            >{mapping.target_host}:{mapping.target_port}</span
                          >
                        </div>
                        {#if runtime?.error}
                          <p class="mt-0.5 pl-3.5 text-red-300">
                            {runtime.error}
                          </p>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              </ContextMenu.Trigger>

              <ContextMenu.Content
                class="min-w-44 border-white/10 bg-slate-950/96 text-slate-100 shadow-2xl shadow-black/45"
              >
                <ContextMenu.Label class="max-w-56 truncate text-slate-400">
                  {savedForward.name}
                </ContextMenu.Label>
                <ContextMenu.Separator class="bg-white/10" />
                {#if state}
                  <ContextMenu.Item
                    class="cursor-pointer focus:bg-amber-400/10 focus:text-amber-200"
                    disabled={deletingRuntimeForwardIds.includes(savedForward.id)}
                    onclick={() => void handleStopGroup(savedForward, runtimes)}
                  >
                    <Square class="size-3.5" />
                    Stop
                  </ContextMenu.Item>
                {:else}
                  <ContextMenu.Item
                    class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                    disabled={!connection ||
                      forwardingPresetIds.includes(savedForward.id)}
                    onclick={() => void handleForward(savedForward)}
                  >
                    <Play class="size-3.5" />
                    Forward
                  </ContextMenu.Item>
                {/if}
                <ContextMenu.Item
                  class="cursor-pointer focus:bg-cyan-300/10 focus:text-white"
                  disabled={state !== null}
                  onclick={() => void onEdit(savedForward)}
                >
                  <Pencil class="size-3.5" />
                  Edit
                </ContextMenu.Item>
                <ContextMenu.Separator class="bg-white/10" />
                <ContextMenu.Item
                  class="cursor-pointer text-red-300 focus:bg-red-400/10 focus:text-red-200"
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

<style>
  @keyframes forward-border-listening {
    0%,
    100% {
      box-shadow: 0 0 18px rgb(52 211 153 / 0.12);
    }
    50% {
      box-shadow: 0 0 34px rgb(52 211 153 / 0.36);
    }
  }

  @keyframes forward-border-connecting {
    0%,
    100% {
      box-shadow: 0 0 18px rgb(252 211 77 / 0.1);
    }
    50% {
      box-shadow: 0 0 34px rgb(252 211 77 / 0.32);
    }
  }

  @keyframes forward-border-error {
    0%,
    100% {
      box-shadow: 0 0 18px rgb(248 113 113 / 0.1);
    }
    50% {
      box-shadow: 0 0 34px rgb(248 113 113 / 0.32);
    }
  }

  :global(.forward-card--listening) {
    animation: forward-border-listening 1.5s ease-in-out infinite;
  }

  :global(.forward-card--connecting) {
    animation: forward-border-connecting 1.5s ease-in-out infinite;
  }

  :global(.forward-card--error) {
    animation: forward-border-error 1.5s ease-in-out infinite;
  }
</style>
