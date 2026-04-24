<script lang="ts">
  import { onMount } from "svelte";
  import { Plus, KeyRound } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import TerminalView from "$lib/terminal/terminal.svelte";
  import type { SessionStatus } from "$lib/stores/session.svelte.js";
  import type { ConnectionConfig, TerminalConfig } from "$lib/stores/bootstrap.svelte.js";

  let {
    sessions,
    sessionStore,
    terminalConfig,
    onOpenConnectionManager,
    onManageKeys,
  }: {
    sessions: Map<string, { id: string; name: string; status: SessionStatus; connectionId?: string | null }>;
    sessionStore: {
      sessions: Map<string, { id: string; name: string; status: SessionStatus; connectionId?: string | null }>;
      activeSessionId: string | null;
      setActiveSession: (id: string) => void;
      connectLocal: (name: string) => Promise<string>;
    };
    terminalConfig: TerminalConfig;
    onOpenConnectionManager: () => void;
    onManageKeys: () => void;
  } = $props();

  const localSession = $derived(
    Array.from(sessions.values()).find((s) => s.name === "Local Terminal" && s.status === "connected"),
  );

  onMount(() => {
    if (localSession) return;
    (async () => {
      try {
        await sessionStore.connectLocal("Local Terminal");
      } catch {
        // ignore
      }
    })();
  });
</script>

<div class="flex h-full flex-col overflow-y-auto bg-background px-5 py-6 lg:px-8">
  {#if localSession}
    <div class="mb-6 flex items-center justify-between">
      <h2 class="text-sm font-medium text-slate-300">Local Terminal</h2>
    </div>
    <div class="flex-1 rounded-2xl border border-white/8 overflow-hidden min-h-0" style="height: 45vh;">
      <TerminalView
        sessionId={localSession.id}
        sessionType="local"
        active={localSession.id === sessionStore.activeSessionId}
        config={terminalConfig}
      />
    </div>
  {/if}

  <div class="mt-6 flex items-center gap-3">
    <Button onclick={onOpenConnectionManager} variant="default" size="sm" class="gap-2 rounded-xl">
      <Plus class="size-3.5" />
      New connection
    </Button>
    <Button onclick={onManageKeys} variant="outline" size="sm" class="gap-2 rounded-xl">
      <KeyRound class="size-3.5" />
      SSH Keys
    </Button>
  </div>
</div>
