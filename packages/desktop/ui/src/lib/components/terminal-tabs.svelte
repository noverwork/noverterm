<script lang="ts">
  import { X, Plus } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import type { Session, SessionStatus } from "$lib/stores/session.svelte.js";

  let {
    sessions,
    activeSessionId,
    onActivate,
    onClose,
    onNew,
  }: {
    sessions: Session[];
    activeSessionId: string | null;
    onActivate: (id: string) => void;
    onClose: (id: string) => void;
    onNew: () => void;
  } = $props();

  function statusIcon(status: SessionStatus): string {
    switch (status) {
      case "connected":
        return "●";
      case "connecting":
        return "◌";
      case "error":
        return "✕";
      default:
        return "○";
    }
  }

  function statusClass(status: SessionStatus): string {
    switch (status) {
      case "connected":
        return "text-green-500";
      case "connecting":
        return "text-yellow-500 animate-pulse";
      case "error":
        return "text-red-500";
      default:
        return "text-muted-foreground";
    }
  }
</script>

<div class="terminal-tabs flex items-center bg-card border-b border-border overflow-x-auto">
  {#each sessions as session (session.id)}
    <button
      class="tab flex items-center gap-2 px-4 py-2 text-sm border-r border-border min-w-0 max-w-48 transition-colors group {session.id === activeSessionId ? 'bg-background text-foreground border-b-2 border-b-primary' : 'text-muted-foreground hover:text-foreground hover:bg-muted'}"
      onclick={() => onActivate(session.id)}
    >
      <span class="text-xs {statusClass(session.status)}">{statusIcon(session.status)}</span>
      <span class="truncate">{session.name}</span>
      <Button
        variant="ghost"
        size="icon-xs"
        class="close-btn shrink-0 opacity-0 group-hover:opacity-100 focus-within:opacity-100 hover:text-destructive cursor-pointer"
        onclick={(e) => {
          e.stopPropagation();
          onClose(session.id);
        }}
      >
        <X class="size-3" />
      </Button>
    </button>
  {/each}

  <Button
    variant="ghost"
    size="icon-sm"
    class="shrink-0 ml-1"
    onclick={onNew}
  >
    <Plus class="size-4" />
  </Button>
</div>

<style>
  .tab {
    position: relative;
  }
  .tab:hover .close-btn,
  .tab:focus-within .close-btn {
    opacity: 1;
  }
</style>
