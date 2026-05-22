<script lang="ts">
  import { CirclePlus, X } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import type { Session, SessionStatus } from "$lib/stores/session.svelte.js";

  let {
    sessions,
    activeSessionId,
    onActivate,
    onClose,
  }: {
    sessions: Session[];
    activeSessionId: string | null;
    onActivate: (id: string) => void;
    onClose: (id: string) => void;
  } = $props();

  function sessionTone(status: SessionStatus) {
    switch (status) {
      case "connected":
        return "bg-emerald-400 shadow-[0_0_12px_rgb(52_211_153/0.7)]";
      case "connecting":
        return "bg-amber-300 shadow-[0_0_12px_rgb(252_211_77/0.55)] animate-pulse";
      case "trust_required":
        return "bg-amber-300 shadow-[0_0_12px_rgb(252_211_77/0.55)]";
      case "error":
        return "bg-red-400 shadow-[0_0_12px_rgb(248_113_113/0.55)]";
      case "disconnected":
        return "bg-red-400 shadow-[0_0_12px_rgb(248_113_113/0.55)]";
      default:
        return "bg-slate-500";
    }
  }
</script>

<div class="terminal-tabs relative z-20 flex h-[3.75rem] shrink-0 items-center gap-3 border-b border-white/10 bg-[#080c13]/88 px-3 backdrop-blur-2xl">
  <div class="pointer-events-none absolute inset-x-0 bottom-0 h-px bg-gradient-to-r from-transparent via-cyan-300/20 to-transparent"></div>
  <div class="flex min-w-0 flex-1 items-center gap-2 overflow-x-auto py-2" data-tab-scroller>
    {#each sessions as session (session.id)}
      <div
        class={session.id === activeSessionId
          ? "group relative flex min-w-36 max-w-64 items-center gap-2 rounded-2xl border border-cyan-300/24 bg-cyan-300/10 px-2.5 py-1.5 text-white shadow-[0_10px_30px_rgb(34_211_238/0.10)]"
          : "group relative flex min-w-36 max-w-64 items-center gap-2 rounded-2xl border border-white/6 bg-white/[0.025] px-2.5 py-1.5 text-slate-400 transition hover:border-white/12 hover:bg-white/[0.055] hover:text-white"}
      >
        <button
          type="button"
          class="absolute inset-0 cursor-pointer rounded-2xl focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-300/50"
          onclick={() => onActivate(session.id)}
          aria-label={`Switch to ${session.name}`}
          aria-current={session.id === activeSessionId ? "page" : undefined}
        ></button>
        <div class="pointer-events-none relative z-10 flex min-w-0 flex-1 items-center gap-2 text-left">
          <span class="size-2.5 shrink-0 rounded-full {sessionTone(session.status)}"></span>
          <span class="truncate text-sm font-medium">{session.name}</span>
        </div>
        <Button
          variant="ghost"
          size="icon-xs"
          class="relative z-10 size-6 shrink-0 rounded-full text-slate-500 opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-red-400/10 hover:text-red-300"
          onclick={() => onClose(session.id)}
          aria-label={`Close ${session.name}`}
        >
          <X class="size-3" />
        </Button>
      </div>
    {/each}

    {#if sessions.length === 0}
      <div class="flex items-center gap-2 rounded-2xl border border-dashed border-white/10 bg-white/[0.025] px-3 py-2 text-xs text-slate-500">
        <CirclePlus class="size-3.5" />
        No open terminals
      </div>
    {/if}
  </div>
</div>
