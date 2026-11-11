<script lang="ts">
  import { ArrowRight, Square } from "@lucide/svelte";

  import { t } from "$lib/i18n/index.svelte.js";
  import {
    activity,
    activityLabel,
    activityTone,
    approvals,
    live,
    stopCommand,
    stopForward,
  } from "$lib/mcp/mcp-events.svelte.js";

  let {
    onOpenSettings,
    onOpenHosts,
  }: { onOpenSettings: () => void; onOpenHosts: () => void } = $props();

  /** How long a finished call stays in the status line. */
  const FLASH_MS = 4000;

  let open = $state(false);
  let error = $state<string | null>(null);
  let flash = $state(false);
  let root = $state<HTMLDivElement | null>(null);

  const current = $derived(live.running.at(-1) ?? null);
  const latest = $derived(activity[0] ?? null);
  const runningCommands = $derived(live.commands.filter((command) => !command.finished).length);
  const held = $derived(runningCommands + live.forwards.length);

  $effect(() => {
    if (!latest) return;
    flash = true;
    const timer = setTimeout(() => (flash = false), FLASH_MS);
    return () => clearTimeout(timer);
  });

  async function act(action: () => Promise<void>) {
    error = null;
    try {
      await action();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function time(value: string | number): string {
    return new Date(value).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }

  function handlePointerDown(event: PointerEvent) {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }
</script>

<svelte:window
  onpointerdown={handlePointerDown}
  onkeydown={(event) => {
    if (open && event.key === "Escape") open = false;
  }}
/>

<div bind:this={root} class="relative flex h-full min-w-0 items-center">
  <button
    type="button"
    class="flex h-full min-w-0 items-center gap-2 px-2 uppercase tracking-[0.14em] transition-colors duration-[120ms] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil motion-reduce:transition-none {approvals.length >
      0 || current
      ? 'bg-[#c9952e]/20 text-stencil hover:bg-[#c9952e]/30'
      : held > 0
        ? 'bg-emerald-400/15 text-stencil hover:bg-emerald-400/25'
        : 'text-stencil-mute hover:bg-white/5'}"
    aria-expanded={open}
    aria-haspopup="dialog"
    aria-label={t("MCP activity")}
    onclick={() => (open = !open)}
  >
    {#if approvals.length > 0}
      <span class="size-2 shrink-0 rounded-full bg-[#c9952e] motion-safe:animate-pulse" aria-hidden="true"></span>
      <span class="shrink-0">MCP</span>
      <span class="truncate text-[#e6c27a]">{t("Waiting for approval")}</span>
    {:else if current}
      <span class="size-2 shrink-0 rounded-full bg-[#c9952e] motion-safe:animate-pulse" aria-hidden="true"></span>
      <span class="shrink-0">MCP{current.host ? `·${current.host}` : ""}</span>
      <span class="truncate font-mono normal-case tracking-normal text-stencil-dim"
        >{current.detail ?? current.tool}</span
      >
    {:else if flash && latest}
      <span class="size-2 shrink-0 rounded-full bg-current {activityTone(latest)}" aria-hidden="true"></span>
      <span class="shrink-0">MCP·{latest.host_name}</span>
      <span class="shrink-0 {activityTone(latest)}">{activityLabel(latest)}</span>
      <span class="truncate font-mono normal-case tracking-normal text-stencil-dim">{latest.command}</span>
    {:else if held > 0}
      <span class="size-2 shrink-0 rounded-full bg-emerald-400" aria-hidden="true"></span>
      <span class="shrink-0">MCP</span>
      <span class="shrink-0 tabular-nums">{t("{count} active", { count: held })}</span>
    {:else}
      <span class="size-2 shrink-0 rounded-full border-2 border-stencil-faint" aria-hidden="true"></span>
      <span class="shrink-0">MCP</span>
    {/if}
  </button>

  {#if open}
    <div
      role="dialog"
      aria-label={t("MCP activity")}
      class="absolute bottom-full right-0 z-50 mb-px flex max-h-[60vh] w-[28rem] flex-col rounded-[2px] border border-white/10 bg-rail text-xs normal-case tracking-normal text-stencil"
    >
      <div class="min-h-0 flex-1 space-y-4 overflow-y-auto p-3">
        {#if error}
          <p class="text-red-300" role="alert">{error}</p>
        {/if}

        {#if live.running.length > 0}
          <section>
            <h3 class="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.14em] text-stencil-mute">
              {t("Working now")}
            </h3>
            <ul class="space-y-1">
              {#each live.running as call (call.id)}
                <li class="flex items-center gap-2">
                  <span class="size-1.5 shrink-0 rounded-full bg-[#c9952e] motion-safe:animate-pulse" aria-hidden="true"></span>
                  <span class="shrink-0 font-mono text-stencil-dim">{call.tool}</span>
                  {#if call.host}<span class="shrink-0 text-stencil-mute">{call.host}</span>{/if}
                  {#if call.detail}<span class="truncate font-mono" title={call.detail}>{call.detail}</span>{/if}
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if live.commands.length > 0}
          <section>
            <h3 class="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.14em] text-stencil-mute">
              {t("Background commands")}
            </h3>
            <ul class="space-y-1">
              {#each live.commands as command (command.id)}
                <li class="flex items-center gap-2">
                  <span
                    class="size-1.5 shrink-0 rounded-full {command.finished ? 'bg-stencil-faint' : 'bg-emerald-400'}"
                    aria-label={command.finished ? t("Finished") : t("Running")}
                  ></span>
                  <span class="shrink-0 text-stencil-mute">{command.host_name}</span>
                  <span class="truncate font-mono" title={command.command}>{command.command}</span>
                  <span class="ml-auto shrink-0 tabular-nums text-stencil-faint">{time(command.started_at)}</span>
                  <button
                    type="button"
                    class="flex size-5 shrink-0 items-center justify-center rounded-[2px] text-stencil-mute hover:bg-seal/20 hover:text-red-300 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil"
                    aria-label={t("Stop {name}", { name: command.command })}
                    title={t("Stop")}
                    onclick={() => void act(() => stopCommand(command.id))}
                  >
                    <Square class="size-3" />
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if live.forwards.length > 0}
          <section>
            <h3 class="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.14em] text-stencil-mute">
              {t("Port forwards")}
            </h3>
            <ul class="space-y-1">
              {#each live.forwards as forward (forward.id)}
                <li class="flex items-center gap-2">
                  <span
                    class="size-1.5 shrink-0 rounded-full {forward.state === 'listening'
                      ? 'bg-emerald-400'
                      : 'bg-[#c9952e] motion-safe:animate-pulse'}"
                    aria-hidden="true"
                  ></span>
                  <span class="shrink-0 text-stencil-mute">{forward.name.replace(/^MCP: /, "")}</span>
                  <span class="shrink-0 font-mono">{forward.bind_host}:{forward.bind_port}</span>
                  <ArrowRight class="size-3 shrink-0 text-stencil-faint" aria-label={t("forwards to")} />
                  <span class="truncate font-mono text-stencil-dim">{forward.target_host}:{forward.target_port}</span>
                  <button
                    type="button"
                    class="ml-auto flex size-5 shrink-0 items-center justify-center rounded-[2px] text-stencil-mute hover:bg-seal/20 hover:text-red-300 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil"
                    aria-label={t("Stop {name}", { name: `${forward.bind_host}:${forward.bind_port}` })}
                    title={t("Stop")}
                    onclick={() => void act(() => stopForward(forward.id))}
                  >
                    <Square class="size-3" />
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        <section>
          <h3 class="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.14em] text-stencil-mute">
            {t("Recent MCP commands")}
          </h3>
          {#if activity.length === 0}
            <p class="text-stencil-mute">{t("No agent has used noverterm yet.")}</p>
          {:else}
            <ul class="space-y-1">
              {#each activity.slice(0, 8) as entry, index (index)}
                <li class="flex items-center gap-2">
                  <span class="w-16 shrink-0 {activityTone(entry)}">{activityLabel(entry)}</span>
                  <span class="shrink-0 text-stencil-mute">{entry.host_name}</span>
                  <span class="truncate font-mono" title={entry.command}>{entry.command}</span>
                  <span class="ml-auto shrink-0 tabular-nums text-stencil-faint">{time(entry.at)}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </div>

      <div class="flex justify-between border-t border-white/10 bg-port px-3 py-1.5">
        <button
          type="button"
          class="rounded-[2px] px-1.5 py-0.5 text-stencil-mute hover:bg-white/5 hover:text-stencil focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil"
          title={t("Right-click a host in Connections to change its AI access")}
          onclick={() => {
            open = false;
            onOpenHosts();
          }}
        >
          {t("Host AI access")}
        </button>
        <button
          type="button"
          class="rounded-[2px] px-1.5 py-0.5 text-stencil-mute hover:bg-white/5 hover:text-stencil focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-stencil"
          onclick={() => {
            open = false;
            onOpenSettings();
          }}
        >
          {t("MCP settings")}
        </button>
      </div>
    </div>
  {/if}
</div>
