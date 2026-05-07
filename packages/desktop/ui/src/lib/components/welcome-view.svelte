<script lang="ts">
  import {
    Cpu,
    HardDrive,
    RefreshCw,
    Server,
    ShieldAlert,
  } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";
  import type {
    HostInfoEntry,
    HostInfoStore,
  } from "$lib/stores/host-info.svelte.js";

  const HOME_SSH_REFRESH_INTERVAL_MS = 30_000;

  let {
    connections,
    hostInfoStore,
    onOpenConnectionManager,
    onConnectConnection,
  }: {
    connections: ConnectionConfig[];
    hostInfoStore: HostInfoStore;
    onOpenConnectionManager: () => void;
    onConnectConnection: (connection: ConnectionConfig) => Promise<void>;
  } = $props();

  let probeAllPromise: Promise<void> | null = null;

  function probeAllConnections(): void {
    if (connections.length === 0 || probeAllPromise !== null) {
      return;
    }

    probeAllPromise = hostInfoStore.probeMany(connections.slice()).finally(() => {
      probeAllPromise = null;
    });
  }

  $effect(() => {
    const connectionIds = connections.map((connection) => connection.id).join("\0");
    if (!connectionIds) {
      return;
    }

    probeAllConnections();

    const refreshTimer = setInterval(
      probeAllConnections,
      HOME_SSH_REFRESH_INTERVAL_MS,
    );

    return () => clearInterval(refreshTimer);
  });

  function formatMemory(bytes?: number | null): string {
    if (!bytes || bytes <= 0) {
      return "Unknown";
    }

    const gib = bytes / 1024 / 1024 / 1024;
    if (gib >= 1) {
      return `${gib.toFixed(gib >= 10 ? 0 : 1)} GiB`;
    }

    const mib = bytes / 1024 / 1024;
    return `${mib.toFixed(0)} MiB`;
  }

  function formatBytesPerSecond(bytes?: number | null): string {
    if (bytes === undefined || bytes === null || bytes < 0) {
      return "--";
    }

    const units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"] as const;
    let value = bytes;
    let unitIndex = 0;

    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }

    if (unitIndex === 0) {
      return `${Math.round(value)} ${units[unitIndex]}/s`;
    }

    return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unitIndex]}/s`;
  }

  function formatPercent(value?: number | null): string {
    if (value === undefined || value === null) {
      return "--";
    }

    return `${Math.round(value)}%`;
  }

  function checkedLabel(entry: HostInfoEntry): string {
    if (!entry.checkedAtMs) {
      return "Not checked yet";
    }

    const seconds = Math.max(
      1,
      Math.round((Date.now() - entry.checkedAtMs) / 1000),
    );
    if (seconds < 60) {
      return `${seconds}s ago`;
    }

    const minutes = Math.round(seconds / 60);
    return `${minutes}m ago`;
  }

  function statusLabel(entry: HostInfoEntry): string {
    switch (entry.status) {
      case "loading":
        return "Checking";
      case "success":
        return "Online";
      case "trust_required":
        return "Trust required";
      case "trust_mismatch":
        return "Trust mismatch";
      case "error":
        return "Unavailable";
      default:
        return "Unknown";
    }
  }

  function statusClass(entry: HostInfoEntry): string {
    switch (entry.status) {
      case "loading":
        return "border-amber-300/15 bg-amber-300/8 text-amber-200";
      case "success":
        return "border-emerald-300/15 bg-emerald-300/8 text-emerald-200";
      case "trust_required":
        return "border-cyan-300/15 bg-cyan-300/8 text-cyan-200";
      case "trust_mismatch":
      case "error":
        return "border-red-300/15 bg-red-300/8 text-red-200";
      default:
        return "border-white/10 bg-white/[0.04] text-slate-400";
    }
  }
</script>

<div
  class="workspace-canvas flex h-full flex-col overflow-y-auto px-5 py-6 lg:px-8"
>
  <div
    class="mb-5 flex flex-col gap-4 md:flex-row md:items-end md:justify-between"
  >
    <div>
      <p class="section-title text-cyan-200/70">Dashboard</p>
      <h1 class="mt-2 text-xl font-semibold tracking-tight text-white">
        Machines
      </h1>
      <p class="mt-1 max-w-2xl text-sm leading-6 text-slate-400">
        CPU, RAM, disk, and network activity from short SSH probes.
      </p>
    </div>
    <Button
      variant="outline"
      class="gap-2 rounded-2xl border-cyan-300/20 bg-cyan-300/8 text-cyan-100 hover:bg-cyan-300/12"
      onclick={probeAllConnections}
      disabled={connections.length === 0}
    >
      <RefreshCw class="size-3.5" />
      Refresh all
    </Button>
  </div>

  {#if connections.length > 0}
    <div class="grid gap-2 xl:grid-cols-2 2xl:grid-cols-3">
      {#each connections as connection (connection.id)}
        {@const entry = hostInfoStore.getEntry(connection.id)}
        <article
          class="rounded-2xl border border-white/10 bg-white/[0.035] p-3 shadow-lg shadow-black/15"
        >
          <div class="flex items-center justify-between gap-3">
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <Server class="size-4 shrink-0 text-cyan-200" />
                <h2 class="truncate text-sm font-semibold text-white">
                  {connection.name}
                </h2>
              </div>
              <p class="mt-1 truncate text-xs text-slate-500">
                {connection.username}@{connection.host}:{connection.port}
              </p>
            </div>
            <span
              class="shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-medium {statusClass(
                entry,
              )}"
            >
              {statusLabel(entry)}
            </span>
          </div>

          <div class="mt-3 grid grid-cols-2 gap-2">
            <div class="rounded-2xl border border-white/8 bg-black/15 p-3">
              <div class="flex items-center justify-between text-slate-500">
                <span
                  class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.18em]"
                >
                  <Cpu class="size-3.5" />CPU
                </span>
                <span class="text-sm font-semibold text-cyan-100">
                  {formatPercent(entry.info?.cpu_usage_percent)}
                </span>
              </div>
              <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-white/8">
                <div
                  class="h-full rounded-full bg-cyan-300 transition-[width]"
                  style={`width: ${Math.min(100, Math.max(0, entry.info?.cpu_usage_percent ?? 0))}%`}
                ></div>
              </div>
            </div>
            <div class="rounded-2xl border border-white/8 bg-black/15 p-3">
              <div class="flex items-center justify-between text-slate-500">
                <span
                  class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.18em]"
                >
                  <HardDrive class="size-3.5" />RAM
                </span>
                <span class="text-sm font-semibold text-emerald-100">
                  {formatPercent(entry.info?.memory_usage_percent)}
                </span>
              </div>
              <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-white/8">
                <div
                  class="h-full rounded-full bg-emerald-300 transition-[width]"
                  style={`width: ${Math.min(100, Math.max(0, entry.info?.memory_usage_percent ?? 0))}%`}
                ></div>
              </div>
              <p class="mt-1 truncate text-[10px] text-slate-500">
                {formatMemory(entry.info?.memory_used_bytes)} / {formatMemory(
                  entry.info?.memory_total_bytes,
                )}
              </p>
            </div>
            <div class="rounded-2xl border border-white/8 bg-black/15 p-3">
              <div class="flex items-center justify-between text-slate-500">
                <span
                  class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.18em]"
                >
                  <HardDrive class="size-3.5" />Disk I/O
                </span>
              </div>
              <div class="mt-2 grid grid-cols-2 gap-2 text-[10px]">
                <div class="min-w-0">
                  <p class="uppercase tracking-[0.16em] text-slate-600">Read</p>
                  <p class="truncate font-semibold text-blue-100">
                    {formatBytesPerSecond(
                      entry.info?.disk_read_bytes_per_second,
                    )}
                  </p>
                </div>
                <div class="min-w-0">
                  <p class="uppercase tracking-[0.16em] text-slate-600">Write</p>
                  <p class="truncate font-semibold text-violet-100">
                    {formatBytesPerSecond(
                      entry.info?.disk_write_bytes_per_second,
                    )}
                  </p>
                </div>
              </div>
            </div>
            <div class="rounded-2xl border border-white/8 bg-black/15 p-3">
              <div class="flex items-center justify-between text-slate-500">
                <span
                  class="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.18em]"
                >
                  <Server class="size-3.5" />Network I/O
                </span>
              </div>
              <div class="mt-2 grid grid-cols-2 gap-2 text-[10px]">
                <div class="min-w-0">
                  <p class="uppercase tracking-[0.16em] text-slate-600">RX</p>
                  <p class="truncate font-semibold text-sky-100">
                    {formatBytesPerSecond(
                      entry.info?.network_rx_bytes_per_second,
                    )}
                  </p>
                </div>
                <div class="min-w-0">
                  <p class="uppercase tracking-[0.16em] text-slate-600">TX</p>
                  <p class="truncate font-semibold text-fuchsia-100">
                    {formatBytesPerSecond(
                      entry.info?.network_tx_bytes_per_second,
                    )}
                  </p>
                </div>
              </div>
            </div>
          </div>

          {#if entry.status === "trust_required" && entry.prompt}
            <div
              class="mt-3 rounded-2xl border border-cyan-300/15 bg-cyan-300/8 p-3 text-sm text-cyan-100"
            >
              <div class="flex items-start gap-2">
                <ShieldAlert class="mt-0.5 size-4 shrink-0" />
                <p>
                  Trust {entry.prompt.algorithm} host key
                  <span class="font-mono text-xs"
                    >{entry.prompt.fingerprint}</span
                  >
                  before probing this machine.
                </p>
              </div>
              <Button
                size="sm"
                class="mt-3 rounded-xl bg-cyan-200 text-cyan-950 hover:bg-cyan-100"
                onclick={() => hostInfoStore.confirmTrustAndProbe(connection)}
              >
                Trust and refresh
              </Button>
            </div>
          {:else if entry.status === "trust_mismatch"}
            <p
              class="mt-3 rounded-2xl border border-red-300/15 bg-red-300/8 p-3 text-sm text-red-200"
            >
              Host key changed. Open a terminal connection to review this
              security warning before refreshing.
            </p>
          {:else if entry.status === "error" && entry.error}
            <p
              class="mt-3 rounded-2xl border border-red-300/15 bg-red-300/8 p-3 text-sm text-red-200"
            >
              {entry.error}
            </p>
          {/if}

          <div class="mt-3 flex items-center justify-between gap-2">
            <p class="truncate text-xs text-slate-600">
              {entry.info?.hostname ?? entry.info?.os ?? checkedLabel(entry)}
            </p>
            <div class="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                class="rounded-xl border-white/10 bg-white/[0.035] text-slate-200 hover:bg-white/8"
                onclick={() => hostInfoStore.probe(connection)}
                disabled={entry.status === "loading"}
              >
                <RefreshCw
                  class="size-3.5 {entry.status === 'loading'
                    ? 'animate-spin'
                    : ''}"
                />
              </Button>
              <Button
                size="sm"
                class="rounded-xl bg-cyan-300 text-cyan-950 hover:bg-cyan-200"
                onclick={() => onConnectConnection(connection)}
              >
                Connect
              </Button>
            </div>
          </div>
        </article>
      {/each}
    </div>
  {:else}
    <div
      class="rounded-3xl border border-dashed border-white/12 bg-white/[0.025] p-8 text-center"
    >
      <Server class="mx-auto size-8 text-cyan-200/80" />
      <h2 class="mt-4 text-lg font-semibold text-white">No machines yet</h2>
      <p class="mt-2 text-sm text-slate-500">
        Add a saved SSH connection to show machine details on Dashboard.
      </p>
      <Button class="mt-5 rounded-2xl" onclick={onOpenConnectionManager}>
        Add connection
      </Button>
    </div>
  {/if}
</div>
