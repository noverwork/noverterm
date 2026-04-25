<script lang="ts">
  import { Loader2, Network, Square } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type {
    LocalPortForward,
    Session,
    StartLocalPortForwardInput,
  } from "$lib/stores/session.svelte.js";

  interface Props {
    session: Session;
    forwards: LocalPortForward[];
    onStart: (input: StartLocalPortForwardInput) => Promise<LocalPortForward>;
    onStop: (sessionId: string, forwardId: string) => Promise<LocalPortForward>;
  }

  let { session, forwards, onStart, onStop }: Props = $props();

  let bindHost = $state("127.0.0.1");
  let bindPort = $state("");
  let targetHost = $state("127.0.0.1");
  let targetPort = $state("");
  let error = $state<string | null>(null);
  let isStarting = $state(false);
  let stoppingForwardIds = $state<string[]>([]);

  const activeForwards = $derived(
    forwards.filter((forward) => forward.status !== "stopped"),
  );

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

    try {
      const parsedBindPort = parsePort(bindPort, "Bind port");
      const parsedTargetPort = parsePort(targetPort, "Target port");
      isStarting = true;
      await onStart({
        sessionId: session.id,
        bindHost: bindHost.trim(),
        bindPort: parsedBindPort,
        targetHost: targetHost.trim(),
        targetPort: parsedTargetPort,
      });
      bindPort = "";
      targetPort = "";
    } catch (startError) {
      error = startError instanceof Error ? startError.message : String(startError);
    } finally {
      isStarting = false;
    }
  }

  async function handleStop(forward: LocalPortForward) {
    error = null;
    stoppingForwardIds = [...stoppingForwardIds, forward.forward_id];

    try {
      await onStop(forward.session_id, forward.forward_id);
    } catch (stopError) {
      error = stopError instanceof Error ? stopError.message : String(stopError);
    } finally {
      stoppingForwardIds = stoppingForwardIds.filter(
        (forwardId) => forwardId !== forward.forward_id,
      );
    }
  }
</script>

<section class="mb-3 rounded-[1.15rem] border border-cyan-300/15 bg-cyan-300/[0.04] p-3 shadow-lg shadow-black/20">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <div class="flex items-center gap-2">
      <div class="grid size-8 place-items-center rounded-xl bg-cyan-300/10 text-cyan-200">
        <Network class="size-4" />
      </div>
      <div>
        <h2 class="text-sm font-semibold text-white">Local port forwarding</h2>
        <p class="text-xs text-slate-400">OpenSSH <span class="font-mono">-L</span> only for {session.name}</p>
      </div>
    </div>
    <p class="text-xs text-slate-500">{activeForwards.length} active</p>
  </div>

  <form class="mt-3 grid gap-2 md:grid-cols-[1fr_7rem_1fr_7rem_auto]" onsubmit={handleSubmit}>
    <label class="space-y-1 text-xs text-slate-400">
      Bind host
      <Input bind:value={bindHost} class="bg-black/30 text-white" autocomplete="off" />
    </label>
    <label class="space-y-1 text-xs text-slate-400">
      Bind port
      <Input bind:value={bindPort} type="number" min="1" max="65535" class="bg-black/30 text-white" />
    </label>
    <label class="space-y-1 text-xs text-slate-400">
      Target host
      <Input bind:value={targetHost} class="bg-black/30 text-white" autocomplete="off" />
    </label>
    <label class="space-y-1 text-xs text-slate-400">
      Target port
      <Input bind:value={targetPort} type="number" min="1" max="65535" class="bg-black/30 text-white" />
    </label>
    <div class="flex items-end">
      <Button type="submit" disabled={isStarting} class="w-full gap-2 bg-cyan-300 text-cyan-950 hover:bg-cyan-200">
        {#if isStarting}
          <Loader2 class="size-3 animate-spin" />
        {/if}
        Start
      </Button>
    </div>
  </form>

  {#if error}
    <p class="mt-2 text-xs text-red-300">{error}</p>
  {/if}

  {#if forwards.length > 0}
    <div class="mt-3 grid gap-2">
      {#each forwards as forward (forward.forward_id)}
        <div class="flex flex-wrap items-center justify-between gap-2 rounded-xl border border-white/10 bg-black/20 px-3 py-2 text-xs">
          <div class="min-w-0 text-slate-300">
            <span class="font-mono text-cyan-100">{forward.bind_host}:{forward.bind_port}</span>
            <span class="mx-2 text-slate-600">→</span>
            <span class="font-mono">{forward.target_host}:{forward.target_port}</span>
            <span class="ml-2 rounded-full bg-white/8 px-2 py-0.5 text-[0.68rem] uppercase tracking-wide text-slate-400">{forward.status}</span>
            {#if forward.error}
              <span class="ml-2 text-red-300">{forward.error}</span>
            {/if}
          </div>
          <Button
            variant="outline"
            size="xs"
            disabled={forward.status === "stopped" || stoppingForwardIds.includes(forward.forward_id)}
            onclick={() => handleStop(forward)}
            class="gap-1 border-white/10 bg-white/4 text-white hover:bg-white/8"
          >
            {#if stoppingForwardIds.includes(forward.forward_id)}
              <Loader2 class="size-3 animate-spin" />
            {:else}
              <Square class="size-3" />
            {/if}
            Stop
          </Button>
        </div>
      {/each}
    </div>
  {/if}
</section>
