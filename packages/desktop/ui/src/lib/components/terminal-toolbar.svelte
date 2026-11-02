<script lang="ts">
  import {
    Clipboard,
    ClipboardPaste,
    Trash2,
    Maximize,
    Minimize,
    Settings,
  } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { t } from "$lib/i18n/index.svelte.js";

  let {
    hasSelection = false,
    isFullscreen = false,
    onCopy,
    onPaste,
    onClear,
    onFullscreen,
    onSettings,
  }: {
    hasSelection?: boolean;
    isFullscreen?: boolean;
    onCopy: () => void;
    onPaste: () => void;
    onClear: () => void;
    onFullscreen: () => void;
    onSettings: () => void;
  } = $props();
</script>

<div class="flex items-center gap-1 px-2 py-1 bg-card border-b border-border h-8">
  <Button
    variant="ghost"
    size="icon-xs"
    disabled={!hasSelection}
    onclick={onCopy}
    title={t("Copy")}
    class="cursor-pointer transition-colors duration-150"
  >
    <Clipboard class="size-3" />
  </Button>

  <Button
    variant="ghost"
    size="icon-xs"
    onclick={onPaste}
    title={t("Paste")}
    class="cursor-pointer transition-colors duration-150"
  >
    <ClipboardPaste class="size-3" />
  </Button>

  <Button
    variant="ghost"
    size="icon-xs"
    onclick={onClear}
    title={t("Clear")}
    class="cursor-pointer transition-colors duration-150"
  >
    <Trash2 class="size-3" />
  </Button>

  <Button
    variant="ghost"
    size="icon-xs"
    onclick={onFullscreen}
    title={t("Toggle Fullscreen")}
    class="cursor-pointer transition-colors duration-150 {isFullscreen ? 'bg-accent text-accent-foreground' : ''}"
  >
    {#if isFullscreen}
      <Minimize class="size-3" />
    {:else}
      <Maximize class="size-3" />
    {/if}
  </Button>

  <div class="flex-1"></div>

  <Button
    variant="ghost"
    size="icon-xs"
    onclick={onSettings}
    title={t("Settings")}
    class="cursor-pointer transition-colors duration-150"
  >
    <Settings class="size-3" />
  </Button>
</div>
