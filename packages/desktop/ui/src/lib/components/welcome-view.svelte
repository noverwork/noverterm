<script lang="ts">
  import { KeyRound, Plus, Sparkles } from "@lucide/svelte";

  import TerminalView from "$lib/terminal/terminal.svelte";
  import type {
    SessionStatus,
    TerminalOutputCallback,
  } from "$lib/stores/session.svelte.js";
  import type { TerminalConfig } from "$lib/stores/bootstrap.svelte.js";

  let {
    sessions,
    sessionStore,
    terminalConfig,
    onOpenConnectionManager,
    onManageKeys,
  }: {
    sessions: Map<
      string,
      {
        id: string;
        name: string;
        status: SessionStatus;
        connectionId?: string | null;
      }
    >;
    sessionStore: {
      sessions: Map<
        string,
        {
          id: string;
          name: string;
          status: SessionStatus;
          connectionId?: string | null;
        }
      >;
      activeSessionId: string | null;
      setActiveSession: (id: string) => void;
      connectLocal: (name: string) => Promise<string>;
      subscribeSessionOutput: (
        sessionId: string,
        callback: TerminalOutputCallback,
      ) => () => void;
    };
    terminalConfig: TerminalConfig;
    onOpenConnectionManager: () => void;
    onManageKeys: () => void;
  } = $props();

  const localSession = $derived(
    Array.from(sessions.values()).find(
      (s) => s.name === "Local Terminal" && s.status === "connected",
    ),
  );
</script>

<div
  class="workspace-canvas flex h-full flex-col overflow-y-auto px-5 py-6 lg:px-8"
>
  {#if localSession}
    <div class="mb-4 flex items-center justify-between">
      <div>
        <p class="section-title text-cyan-200/70">Ready</p>
        <h2 class="mt-1 text-xl font-semibold tracking-tight text-white">
          Local Terminal
        </h2>
      </div>
      <div
        class="hidden items-center gap-2 rounded-full border border-emerald-300/15 bg-emerald-300/8 px-3 py-1 text-xs font-medium text-emerald-200 sm:flex"
      >
        <span
          class="size-2 rounded-full bg-emerald-400 shadow-[0_0_12px_rgb(52_211_153/0.7)]"
        ></span>
        Online
      </div>
    </div>
    <div
      class="terminal-frame relative flex-1 overflow-hidden rounded-[1.35rem] border border-white/10 bg-[#080c13]/72 shadow-2xl shadow-black/40"
      style="height: 52vh;"
    >
      <TerminalView
        sessionId={localSession.id}
        sessionType="local"
        active={localSession.id === sessionStore.activeSessionId}
        config={terminalConfig}
        subscribeOutput={(callback) =>
          sessionStore.subscribeSessionOutput(localSession.id, callback)}
      />
    </div>
  {/if}

  <div class="mt-6 grid gap-3 lg:grid-cols-[1.4fr_0.8fr_0.8fr]">
    <div
      class="rounded-3xl border border-white/10 bg-white/[0.035] p-5 shadow-xl shadow-black/20"
    >
      <div class="flex items-start gap-3">
        <div
          class="grid size-10 shrink-0 place-items-center rounded-2xl bg-cyan-300/10 text-cyan-200"
        >
          <Sparkles class="size-4" />
        </div>
        <div>
          <p class="text-sm font-semibold text-white">
            Terminal-first workspace
          </p>
          <p class="mt-1 text-sm leading-6 text-slate-400">
            Open a local shell instantly or connect saved SSH hosts without
            leaving the command center.
          </p>
        </div>
      </div>
    </div>
    <button
      class="group rounded-3xl border border-white/10 bg-white/[0.035] p-5 text-left transition hover:border-cyan-300/30 hover:bg-cyan-300/8"
      onclick={onOpenConnectionManager}
    >
      <Plus class="size-4 text-cyan-200" />
      <p class="mt-4 text-sm font-semibold text-white">New connection</p>
      <p class="mt-1 text-xs leading-5 text-slate-500">Save an SSH target</p>
    </button>
    <button
      class="group rounded-3xl border border-white/10 bg-white/[0.035] p-5 text-left transition hover:border-cyan-300/30 hover:bg-cyan-300/8"
      onclick={onManageKeys}
    >
      <KeyRound class="size-4 text-cyan-200" />
      <p class="mt-4 text-sm font-semibold text-white">SSH keys</p>
      <p class="mt-1 text-xs leading-5 text-slate-500">Manage credentials</p>
    </button>
  </div>
</div>
