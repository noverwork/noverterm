<script lang="ts">
  import { ChevronDown, ChevronUp, Copy, Search, X } from "@lucide/svelte";
  import { onMount, tick } from "svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as ContextMenu from "$lib/components/ui/context-menu/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
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
    onRequestClose,
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
    onRequestClose?: () => void;
    subscribeOutput?: (callback: TerminalOutputCallback) => () => void;
    onSelectionChange?: () => void;
    controller?: ReturnType<typeof createTerminal> | null;
  } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let term: ReturnType<typeof createTerminal> | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let revealFrame: number | null = null;
  let revealGeneration = 0;
  let searchOpen = $state(false);
  let searchTerm = $state("");
  let searchMatched = $state<boolean | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);
  let hasSelection = $state(false);

  function updateSelectionState() {
    hasSelection = Boolean(term?.copySelection());
    onSelectionChange?.();
  }

  async function copySelection() {
    const selection = term?.copySelection();
    if (!selection) return;

    await navigator.clipboard.writeText(selection).catch(() => undefined);
    term?.focus();
  }

  async function openSearch() {
    searchOpen = true;
    await tick();
    searchInput?.focus();
    searchInput?.select();
  }

  function closeSearch() {
    searchOpen = false;
    searchMatched = null;
    term?.clearSearch();
    term?.focus();
  }

  function findNext() {
    if (searchTerm.length === 0) {
      searchMatched = null;
      term?.clearSearch();
      return;
    }

    searchMatched = term?.findNext(searchTerm) ?? false;
  }

  function findPrevious() {
    if (searchTerm.length === 0) {
      searchMatched = null;
      term?.clearSearch();
      return;
    }

    searchMatched = term?.findPrevious(searchTerm) ?? false;
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    event.stopPropagation();

    if (event.key === "Escape") {
      event.preventDefault();
      closeSearch();
      return;
    }

    if (event.key.toLowerCase() === "f" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      searchInput?.select();
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      if (event.shiftKey) {
        findPrevious();
      } else {
        findNext();
      }
    }
  }

  $effect(() => {
    if (!searchOpen) return;

    if (searchTerm.length === 0) {
      searchMatched = null;
      term?.clearSearch();
      return;
    }

    searchMatched = term?.findNext(searchTerm) ?? false;
  });

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

    term?.focus();
    void scheduleReveal();
  });

  onMount(() => {
    if (!container) return;

    term = createTerminal({
      sessionId,
      sessionType,
      config,
      onOutput,
      onClose,
      onRequestClose,
      onSearchRequest: () => {
        void openSearch();
      },
      subscribeOutput,
    });
    term.init(container);

    if (controller !== undefined) {
      controller = term;
    }

    term.onSelectionChange(updateSelectionState);

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

<div class="relative h-full w-full">
  <ContextMenu.Root>
    <ContextMenu.Trigger class="contents">
      <div bind:this={container} class="terminal-container h-full w-full"></div>
    </ContextMenu.Trigger>
    <ContextMenu.Content class="min-w-36 border-white/10 bg-slate-950/96 text-slate-100 shadow-2xl shadow-black/45">
      <ContextMenu.Item
        class="cursor-pointer gap-2 focus:bg-cyan-300/10 focus:text-white"
        disabled={!hasSelection}
        onclick={() => void copySelection()}
      >
        <Copy class="size-3.5" />
        Copy
      </ContextMenu.Item>
    </ContextMenu.Content>
  </ContextMenu.Root>

  {#if searchOpen}
    <div
      class="absolute right-2 top-2 z-30 flex h-8 items-center gap-1 rounded-xl border border-white/10 bg-[#080c13]/95 px-2 py-1 text-slate-100 shadow-lg shadow-black/40 backdrop-blur-sm"
      role="search"
      aria-label="Terminal search"
    >
      <Search class="size-3.5 text-slate-400" aria-hidden="true" />
      <Input
        bind:ref={searchInput}
        bind:value={searchTerm}
        aria-label="Search terminal buffer"
        placeholder="Search"
        class="h-6 w-44 border-white/10 bg-white/[0.04] px-2 py-0 text-xs text-slate-100 placeholder:text-slate-500 focus-visible:ring-1"
        onkeydown={handleSearchKeydown}
      />
      {#if searchMatched === false && searchTerm.length > 0}
        <span class="px-1 text-[0.68rem] text-amber-300" aria-live="polite">
          No match
        </span>
      {/if}
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 cursor-pointer text-slate-300 hover:bg-white/10 hover:text-white"
        disabled={searchTerm.length === 0}
        title="Previous result"
        aria-label="Previous search result"
        onclick={findPrevious}
      >
        <ChevronUp class="size-3" />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 cursor-pointer text-slate-300 hover:bg-white/10 hover:text-white"
        disabled={searchTerm.length === 0}
        title="Next result"
        aria-label="Next search result"
        onclick={findNext}
      >
        <ChevronDown class="size-3" />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 cursor-pointer text-slate-400 hover:bg-white/10 hover:text-white"
        title="Close search"
        aria-label="Close terminal search"
        onclick={closeSearch}
      >
        <X class="size-3" />
      </Button>
    </div>
  {/if}
</div>
