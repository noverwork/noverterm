<script lang="ts">
  import { t } from "$lib/i18n/index.svelte.js";
  import { ChevronDown, ChevronUp, Search, X } from "@lucide/svelte";
  import { onMount, tick } from "svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import AiAssistantPanel from "./ai-assistant-panel.svelte";
  import { createTerminal } from "./xterm.js";
  import type { TerminalConfig } from "$lib/app-data-types.js";
  import type {
    SessionType,
    TerminalOutputCallback,
  } from "$lib/stores/session.svelte.js";

  let {
    sessionId,
    sessionType = "ssh",
    active = false,
    config,
    onOutput,
    onClose,
    onError,
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
    onError?: (message: string) => void;
    onRequestClose?: () => void;
    subscribeOutput?: (callback: TerminalOutputCallback) => () => void;
    onSelectionChange?: () => void;
    controller?: ReturnType<typeof createTerminal> | null;
  } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let term: ReturnType<typeof createTerminal> | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let revealGeneration = 0;
  let searchOpen = $state(false);
  let searchTerm = $state("");
  let searchMatched = $state<boolean | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);
  /** Rows sent to the AI as context: enough for the prompt, OS and recent errors. */
  const ASSISTANT_CONTEXT_LINES = 40;
  /** `undefined` while closed; `null` when open with no selection to explain. */
  let assistantSelection = $state<string | null | undefined>(undefined);

  function openAssistant() {
    assistantSelection = term?.copySelection() ?? null;
  }

  function closeAssistant() {
    assistantSelection = undefined;
    term?.focus();
  }
  function updateSelectionState() {
    onSelectionChange?.();
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
  }

  async function scheduleReveal() {
    const generation = ++revealGeneration;

    await tick();

    if (generation !== revealGeneration || !active || !term || !container)
      return;
    if (container.clientWidth === 0 || container.clientHeight === 0) return;

    // Reveal synchronously: deferring to a frame lets the browser paint the
    // newly visible tab with a stale canvas first, which reads as a flicker.
    term.reveal();
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
      onError,
      onRequestClose,
      onSearchRequest: () => {
        void openSearch();
      },
      onAssistantRequest: openAssistant,
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
      const [entry] = entries;
      if (
        entry &&
        (entry.contentRect.width === 0 || entry.contentRect.height === 0)
      )
        return;

      // Hidden tabs stay fitted so that switching to one never resizes the
      // buffer, which would reflow the whole screen in front of the user.
      if (!active) {
        term?.fit();
        return;
      }

      void scheduleReveal();
    });
    resizeObserver.observe(container);

    // Returning to an idle window can leave a stale glyph atlas behind. The
    // window can also be occluded without ever losing focus (Space switch,
    // display sleep), which only visibilitychange reports.
    const handleRepaint = () => {
      if (active && document.visibilityState === "visible") void scheduleReveal();
    };
    window.addEventListener("focus", handleRepaint);
    document.addEventListener("visibilitychange", handleRepaint);

    return () => {
      window.removeEventListener("focus", handleRepaint);
      document.removeEventListener("visibilitychange", handleRepaint);
      cancelScheduledReveal();
      resizeObserver?.disconnect();
      term?.dispose();
      term = null;
    };
  });
</script>

<div class="relative h-full w-full">
  <div bind:this={container} class="terminal-container h-full w-full"></div>

  {#if assistantSelection !== undefined}
    {#key assistantSelection}
      <AiAssistantPanel
        selection={assistantSelection}
        getContext={() => term?.recentText(ASSISTANT_CONTEXT_LINES) ?? ""}
        onInsert={(command) => {
          term?.paste(command);
          closeAssistant();
        }}
        onClose={closeAssistant}
      />
    {/key}
  {/if}

  {#if searchOpen}
    <div
      class="absolute right-2 top-2 z-30 flex h-8 items-center gap-1 rounded-[2px] border border-white/10 bg-rail px-2 py-1 text-stencil"
      role="search"
      aria-label={t("Terminal search")}
    >
      <Search class="size-3.5 text-stencil-mute" aria-hidden="true" />
      <Input
        bind:ref={searchInput}
        bind:value={searchTerm}
        aria-label={t("Search terminal buffer")}
        placeholder={t("Search")}
        class="h-6 w-44 rounded-[2px] border-white/10 bg-port px-2 py-0 text-xs text-stencil placeholder:text-stencil-faint focus-visible:ring-1 focus-visible:ring-stencil"
        onkeydown={handleSearchKeydown}
      />
      {#if searchMatched === false && searchTerm.length > 0}
        <span class="px-1 text-[0.68rem] text-[#e6c27a]" aria-live="polite">
          {t("No match")}
        </span>
      {/if}
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 cursor-pointer text-stencil-dim hover:bg-white/10 hover:text-stencil"
        disabled={searchTerm.length === 0}
        title={t("Previous result")}
        aria-label={t("Previous search result")}
        onclick={findPrevious}
      >
        <ChevronUp class="size-3" />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 cursor-pointer text-stencil-dim hover:bg-white/10 hover:text-stencil"
        disabled={searchTerm.length === 0}
        title={t("Next result")}
        aria-label={t("Next search result")}
        onclick={findNext}
      >
        <ChevronDown class="size-3" />
      </Button>
      <Button
        variant="ghost"
        size="icon-xs"
        class="size-6 cursor-pointer text-stencil-mute hover:bg-white/10 hover:text-stencil"
        title={t("Close search")}
        aria-label={t("Close terminal search")}
        onclick={closeSearch}
      >
        <X class="size-3" />
      </Button>
    </div>
  {/if}
</div>
