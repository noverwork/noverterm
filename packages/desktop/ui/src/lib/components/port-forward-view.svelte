<script lang="ts">
  import {
    Loader2,
    Network,
    Pencil,
    Play,
    Plus,
    Square,
    Trash2,
  } from "@lucide/svelte";

  import DeleteConfirmDialog from "$lib/components/delete-confirm-dialog.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
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

  let sortedSavedForwards = $derived(
    [...savedForwards].sort((left, right) =>
      left.name.localeCompare(right.name),
    ),
  );

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

  function cardClass(state: PortForward["state"] | null): string {
    const base =
      "group rounded-[1.35rem] border bg-white/[0.03] px-4 py-4 transition hover:border-white/14 hover:bg-white/[0.055]";

    switch (state) {
      case "listening":
        return `${base} forward-card--listening border-emerald-300/55`;
      case "connecting":
        return `${base} forward-card--connecting border-amber-300/55`;
      case "error":
        return `${base} forward-card--error border-red-300/55`;
      default:
        return `${base} border-white/8`;
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

      <Button
        onclick={onNew}
        variant="default"
        size="sm"
        class="gap-2 self-start rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200"
      >
        <Plus class="size-3.5" />
        New Forward
      </Button>
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
      {:else}
        <div class="space-y-6">
          <section>
              <div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                {#each sortedSavedForwards as savedForward (savedForward.id)}
                  {@const connection = connectionForForward(savedForward)}
                  {@const runtimes = runtimesFor(savedForward, connection)}
                  {@const state = groupState(runtimes)}
                  {@const badge = state ? stateBadge(state) : null}
                  <article class={cardClass(state)}>
                    <div class="flex items-start gap-3">
                      <div
                        class="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200"
                      >
                        <Network class="size-5" />
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

                    <div class="mt-4 flex items-center gap-2">
                      {#if state}
                        <Button
                          variant="ghost"
                          size="xs"
                          class="gap-1.5 rounded-xl text-slate-400 hover:bg-amber-400/10 hover:text-amber-300"
                          onclick={() => handleStopGroup(savedForward, runtimes)}
                          disabled={deletingRuntimeForwardIds.includes(
                            savedForward.id,
                          )}
                        >
                          {#if deletingRuntimeForwardIds.includes(savedForward.id)}
                            <Loader2 class="size-3 animate-spin" />
                          {:else}
                            <Square class="size-3" />
                          {/if}
                          Stop
                        </Button>
                      {:else}
                        <Button
                          variant="ghost"
                          size="xs"
                          class="gap-1.5 rounded-xl bg-cyan-300/10 text-cyan-100 hover:bg-cyan-300/18 hover:text-white"
                          onclick={() => handleForward(savedForward)}
                          disabled={!connection ||
                            forwardingPresetIds.includes(savedForward.id)}
                        >
                          {#if forwardingPresetIds.includes(savedForward.id)}
                            <Loader2 class="size-3 animate-spin" />
                          {:else}
                            <Play class="size-3" />
                          {/if}
                          Forward
                        </Button>
                      {/if}
                      <Button
                        variant="ghost"
                        size="xs"
                        class="gap-1.5 rounded-xl text-slate-400 hover:bg-white/8 hover:text-white"
                        onclick={() => onEdit(savedForward)}
                        disabled={state !== null}
                      >
                        <Pencil class="size-3" />
                        Edit
                      </Button>
                      <Button
                        variant="ghost"
                        size="xs"
                        class="gap-1.5 rounded-xl text-slate-400 hover:bg-red-400/10 hover:text-red-300"
                        onclick={() => requestDeleteSaved(savedForward)}
                        disabled={state !== null ||
                          deletingSavedForwardIds.includes(savedForward.id)}
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
            </section>

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
