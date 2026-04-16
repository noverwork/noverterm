<script lang="ts">
  import type { TerminalConfig } from "$lib/config.js";
  import { createTerminal } from "./xterm.js";
  import { onDestroy, untrack } from "svelte";

  type TerminalController = ReturnType<typeof createTerminal>;

  type Props = {
    sessionId: string;
    config: TerminalConfig;
    controller?: TerminalController | null;
    onOutput?: (data: string) => void;
    onClose?: () => void;
  };

  let {
    sessionId,
    config,
    controller = $bindable<TerminalController | null>(null),
    onOutput,
    onClose,
  }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let term = $state<TerminalController | null>(null);

  $effect(() => {
    term?.updateConfig(config);
  });

  $effect(() => {
    if (!container) return;

    const currentTerm = createTerminal({
      sessionId,
      config: untrack(() => config),
      onOutput,
      onClose,
    });

    currentTerm.init(container);
    term = currentTerm;
    controller = currentTerm;

    const observer = new ResizeObserver(() => {
      currentTerm.fit();
    });
    observer.observe(container);

    return () => {
      observer.disconnect();
      currentTerm.dispose();

      if (term === currentTerm) {
        term = null;
      }

      if (controller === currentTerm) {
        controller = null;
      }
    };
  });

  onDestroy(() => {
    term?.dispose();
  });
</script>

<div bind:this={container} class="terminal-container w-full h-full"></div>
