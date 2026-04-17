<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { createTerminal } from "./xterm.js";
  import type { TerminalConfig } from "$lib/config.js";

  let {
    sessionId,
    config,
    onOutput,
    onClose,
    controller = $bindable(null),
  }: {
    sessionId: string;
    config: TerminalConfig;
    onOutput?: (data: string) => void;
    onClose?: () => void;
    controller?: ReturnType<typeof createTerminal> | null;
  } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let term: ReturnType<typeof createTerminal> | null = null;
  let resizeObserver: ResizeObserver | null = null;

  $effect(() => {
    term?.updateConfig(config);
  });

  onMount(() => {
    if (!container) return;

    term = createTerminal({ sessionId, config, onOutput, onClose });
    term.init(container);

    if (controller !== undefined) {
      controller = term;
    }

    resizeObserver = new ResizeObserver(() => {
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
