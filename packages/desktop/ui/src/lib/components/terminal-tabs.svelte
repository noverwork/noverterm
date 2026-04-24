<script lang="ts">
  import { X } from "@lucide/svelte";
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
        return "bg-emerald-400";
      case "connecting":
        return "bg-amber-400 animate-pulse";
      case "error":
        return "bg-red-400";
      default:
        return "bg-slate-500";
    }
  }
</script>

<div class="terminal-tabs relative z-20 flex h-14 shrink-0 items-center gap-2 border-b border-border/70 bg-background/90 px-2 backdrop-blur-sm">
  <div class="flex min-w-0 flex-1 items-center gap-2 overflow-x-auto pb-1 pt-1">
    {#each sessions as session (session.id)}
      <button
        class={session.id === activeSessionId
          ? "group flex min-w-0 max-w-56 cursor-pointer items-center gap-3 rounded-2xl border border-primary/25 bg-primary/10 px-3 py-2 text-left text-foreground shadow-sm"
          : "group flex min-w-0 max-w-56 cursor-pointer items-center gap-3 rounded-2xl border border-transparent bg-transparent px-3 py-2 text-left text-muted-foreground transition-colors hover:border-border/70 hover:bg-muted/60 hover:text-foreground"}
        onclick={() => onActivate(session.id)}
      >
        <span class="size-2.5 shrink-0 rounded-full {sessionTone(session.status)}"></span>
        <span class="truncate text-sm font-medium">{session.name}</span>
        <Button
          variant="ghost"
          size="icon-xs"
          class="size-6 shrink-0 rounded-full text-muted-foreground opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 hover:text-destructive"
          onclick={(event) => {
            event.stopPropagation();
            onClose(session.id);
          }}
        >
          <X class="size-3" />
        </Button>
      </button>
    {/each}
  </div>
</div>
