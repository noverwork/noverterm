<script lang="ts">
  import { onMount, tick } from "svelte";
  import { createTerminal } from "./xterm.js";
import type { TerminalConfig } from "$lib/app-data-types.js";
  import type { SessionType, TerminalOutputCallback } from "$lib/stores/session.svelte.js";

  let {
    sessionId,
    sessionType = "ssh",
    active = false,
    config,
    onOutput,
    onClose,
    subscribeOutput,
    onSelectionChange,
    controller = $bindable(null),
  }: {
    sessionId: string;
    sessionType?: SessionType;
    active?: boolean;
    config: TerminalConfig;
    onOutput?: (data: string) => void;
    onClose?: () => void;
    subscribeOutput?: (callback: TerminalOutputCallback) => () => void;
    onSelectionChange?: () => void;
    controller?: ReturnType<typeof createTerminal> | null;
  } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let term: ReturnType<typeof createTerminal> | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let revealFrame: number | null = null;
  let revealGeneration = 0;

  function cancelScheduledReveal() {
    revealGeneration += 1;

    if (revealFrame !== null) {
      cancelAnimationFrame(revealFrame);
      revealFrame = null;
    }
  }

  async function scheduleReveal() {
    const generation = ++revealGeneration;

    await tick();

    if (generation !== revealGeneration || !active || !term || !container) return;
    if (container.clientWidth === 0 || container.clientHeight === 0) return;

    if (revealFrame !== null) {
      cancelAnimationFrame(revealFrame);
    }

    revealFrame = requestAnimationFrame(() => {
      revealFrame = null;

      if (!active || !term || !container) return;
      if (container.clientWidth === 0 || container.clientHeight === 0) return;

      term.reveal();
    });
  }

  $effect(() => {
    term?.updateConfig(config);
  });

  $effect(() => {
    if (!active) {
      cancelScheduledReveal();
      return;
    }

    void scheduleReveal();
  });

  onMount(() => {
    if (!container) return;

    term = createTerminal({ sessionId, sessionType, config, onOutput, onClose, subscribeOutput });
    term.init(container);

    if (controller !== undefined) {
      controller = term;
    }

    if (onSelectionChange) {
      term.onSelectionChange(onSelectionChange);
    }

    if (active) {
      void scheduleReveal();
    }

    resizeObserver = new ResizeObserver((entries) => {
      if (!active) return;
      const [entry] = entries;
      if (entry && (entry.contentRect.width === 0 || entry.contentRect.height === 0)) return;

      void scheduleReveal();
    });
    resizeObserver.observe(container);

    return () => {
      cancelScheduledReveal();
      resizeObserver?.disconnect();
      term?.dispose();
      term = null;
    };
  });
</script>

<div bind:this={container} class="terminal-container w-full h-full"></div>
