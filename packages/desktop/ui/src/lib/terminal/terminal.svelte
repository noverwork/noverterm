<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { createTerminal } from "./xterm.js";
  import type { TerminalConfig } from "$lib/stores/bootstrap.svelte.js";
  import type { SessionType } from "$lib/stores/session.svelte.js";

  let {
    sessionId,
    sessionType = "ssh",
    active = false,
    config,
    onOutput,
    onClose,
    onSelectionChange,
    controller = $bindable(null),
  }: {
    sessionId: string;
    sessionType?: SessionType;
    active?: boolean;
    config: TerminalConfig;
    onOutput?: (data: string) => void;
    onClose?: () => void;
    onSelectionChange?: () => void;
    controller?: ReturnType<typeof createTerminal> | null;
  } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let term: ReturnType<typeof createTerminal> | null = null;
  let resizeObserver: ResizeObserver | null = null;

  $effect(() => {
    term?.updateConfig(config);
  });

  $effect(() => {
    if (!active || !term) return;

    requestAnimationFrame(() => {
      term?.reveal();
      requestAnimationFrame(() => {
        term?.reveal();
      });
    });
  });

  onMount(() => {
    if (!container) return;

    term = createTerminal({ sessionId, sessionType, config, onOutput, onClose });
    term.init(container);

    if (controller !== undefined) {
      controller = term;
    }

    if (onSelectionChange) {
      term.onSelectionChange(onSelectionChange);
    }

    resizeObserver = new ResizeObserver(() => {
      if (!active) return;

      term?.fit();
    });
    resizeObserver.observe(container);

    return () => {
      resizeObserver?.disconnect();
      term?.dispose();
    };
  });

  onDestroy(() => {
    term?.dispose();
  });
</script>

<div bind:this={container} class="terminal-container w-full h-full"></div>
