<script lang="ts">
  import { AlertCircle, Files, Loader2, Terminal } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  import FileBrowser from "$lib/components/file-browser/FileBrowser.svelte";
  import { sftpStore } from "$lib/stores/sftp.svelte.js";
  import type { Session } from "$lib/stores/session.svelte.js";

  interface Props {
    activeSession?: Session;
    children?: Snippet;
  }

  type SessionView = "terminal" | "files";

  let { activeSession, children }: Props = $props();

  let activeView = $state<SessionView>("terminal");
  let openingSftp = $state(false);
  let sftpError = $state<string | null>(null);
  let attemptedSftpSessionId = $state<string | null>(null);

  const canBrowseFiles = $derived(
    activeSession?.type === "ssh" && activeSession.status === "connected",
  );

  function viewTabClass(view: SessionView): string {
    const active = activeView === view;
    return active
      ? "inline-flex items-center gap-2 rounded-xl border border-cyan-300/35 bg-cyan-300/12 px-3 py-1.5 text-xs font-medium text-cyan-100 shadow-[0_8px_24px_rgb(34_211_238/0.10)]"
      : "inline-flex items-center gap-2 rounded-xl border border-transparent px-3 py-1.5 text-xs font-medium text-slate-400 transition hover:border-white/10 hover:bg-white/[0.055] hover:text-white";
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  async function openSftpForSession(sessionId: string): Promise<void> {
    if (openingSftp) {
      return;
    }

    if (sftpStore.sftpSessionId && sftpStore.sshSessionId === sessionId) {
      sftpError = null;
      return;
    }

    openingSftp = true;
    sftpError = null;

    try {
      if (sftpStore.sftpSessionId && sftpStore.sshSessionId !== sessionId) {
        await sftpStore.closeSftp();
      }

      if (!sftpStore.sftpSessionId || sftpStore.sshSessionId !== sessionId) {
        await sftpStore.openSftp(sessionId);
      }
    } catch (error) {
      sftpError = errorMessage(error);
    } finally {
      openingSftp = false;
    }
  }

  function selectView(view: SessionView): void {
    activeView = view;

    if (view === "files") {
      attemptedSftpSessionId = null;
    }
  }

  $effect(() => {
    const sessionId = activeSession?.id ?? null;
    if (attemptedSftpSessionId && attemptedSftpSessionId !== sessionId) {
      attemptedSftpSessionId = null;
      sftpError = null;
    }
  });

  $effect(() => {
    const session = activeSession;
    if (activeView !== "files" || !canBrowseFiles || !session) {
      return;
    }

    const isOpenForSession =
      sftpStore.sftpSessionId !== null && sftpStore.sshSessionId === session.id;
    if (isOpenForSession || openingSftp || attemptedSftpSessionId === session.id) {
      return;
    }

    attemptedSftpSessionId = session.id;
    void openSftpForSession(session.id);
  });
</script>

<div class="relative h-full min-h-0 flex-1 overflow-hidden" data-testid="session-view-switcher">
  <div
    class="absolute right-6 top-5 z-40 flex items-center gap-1 rounded-2xl border border-white/10 bg-[#080c13]/88 p-1 shadow-2xl shadow-black/30 backdrop-blur-xl"
    role="tablist"
    aria-label="Session view"
    data-testid="session-view-tabs"
  >
    <button
      type="button"
      role="tab"
      aria-selected={activeView === "terminal"}
      class={viewTabClass("terminal")}
      onclick={() => selectView("terminal")}
      data-testid="terminal-view-tab"
    >
      <Terminal class="size-3.5" />
      Terminal
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={activeView === "files"}
      class={viewTabClass("files")}
      onclick={() => selectView("files")}
      data-testid="files-view-tab"
    >
      <Files class="size-3.5" />
      Files
    </button>
  </div>

  <div
    class={activeView === "terminal"
      ? "absolute inset-0 z-10 min-h-0 overflow-hidden opacity-100"
      : "pointer-events-none absolute inset-0 z-0 min-h-0 overflow-hidden opacity-0"}
    aria-hidden={activeView !== "terminal"}
    data-testid="terminal-view-panel"
  >
    {#if children}
      {@render children()}
    {:else}
      <div class="flex h-full items-center justify-center text-sm text-slate-400" data-testid="terminal-view-placeholder">
        Terminal view
      </div>
    {/if}
  </div>

  {#if activeView === "files"}
    <div
      class="absolute inset-0 z-20 flex h-full min-h-0 flex-col overflow-hidden p-3 pt-[4.5rem]"
      data-testid="files-view-panel"
    >
      {#if !canBrowseFiles}
        <div class="flex h-full items-center justify-center p-8 text-center text-white">
          <div class="max-w-md rounded-[2rem] border border-white/10 bg-white/[0.035] p-8 shadow-2xl shadow-black/30">
            <div class="mx-auto grid size-14 place-items-center rounded-2xl bg-cyan-300/10 text-cyan-200">
              <Files class="size-7" />
            </div>
            <h2 class="mt-5 text-xl font-semibold">Connect to a server first</h2>
            <p class="mt-2 text-sm leading-6 text-slate-400">
              The SFTP file browser is available for connected SSH sessions.
            </p>
          </div>
        </div>
      {:else if sftpError}
        <div class="flex h-full items-center justify-center p-8 text-center text-white" role="alert">
          <div class="max-w-md rounded-[2rem] border border-red-300/20 bg-red-400/8 p-8 shadow-2xl shadow-black/30">
            <div class="mx-auto grid size-14 place-items-center rounded-2xl bg-red-400/12 text-red-300">
              <AlertCircle class="size-7" />
            </div>
            <h2 class="mt-5 text-xl font-semibold">Unable to open SFTP</h2>
            <p class="mt-2 text-sm leading-6 text-slate-400">{sftpError}</p>
          </div>
        </div>
      {:else}
        <div class="relative min-h-0 flex-1 overflow-hidden">
          <FileBrowser />
          {#if openingSftp}
            <div class="absolute inset-0 z-10 flex flex-col items-center justify-center rounded-[1.35rem] bg-[#080c13]/82 text-white backdrop-blur-sm">
              <Loader2 class="mb-4 size-8 animate-spin text-cyan-200" />
              <p class="text-sm font-semibold">Opening SFTP session…</p>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>
