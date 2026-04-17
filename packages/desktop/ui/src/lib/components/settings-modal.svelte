<script lang="ts">
  import { X, Sun, Moon, Type, MousePointer, ScrollText } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { TerminalConfig } from "$lib/config.js";

  let {
    open,
    config,
    onSave,
    onClose,
  }: {
    open: boolean;
    config: TerminalConfig;
    onSave: (config: TerminalConfig) => void;
    onClose: () => void;
  } = $props();

  const FONT_FAMILIES = [
    "JetBrains Mono, Fira Code, monospace",
    "Fira Code, monospace",
    "Cascadia Code, monospace",
    "Hack, monospace",
    "Source Code Pro, monospace",
    "monospace",
  ] as const;

  const CURSOR_STYLES = ["block", "underline", "bar"] as const;

  const DEFAULTS: TerminalConfig = {
    theme: "dark",
    fontSize: 14,
    fontFamily: "JetBrains Mono, Fira Code, monospace",
    cursorStyle: "block",
    cursorBlink: true,
    scrollback: 5000,
  };

  let theme = $state<"dark" | "light">("dark");
  let fontSize = $state(14);
  let fontFamily = $state<string>(FONT_FAMILIES[0]);
  let cursorStyle = $state<"block" | "underline" | "bar">("block");
  let cursorBlink = $state(true);
  let scrollback = $state(5000);

  $effect(() => {
    if (open && config) {
      theme = config.theme;
      fontSize = config.fontSize;
      fontFamily = config.fontFamily;
      cursorStyle = config.cursorStyle;
      cursorBlink = config.cursorBlink;
      scrollback = config.scrollback;
    }
  });

  $effect(() => {
    if (theme === "dark") {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  });

  const errors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (fontSize < 8 || fontSize > 32) errs.fontSize = "Font size must be 8-32";
    if (scrollback < 100 || scrollback > 50000) errs.scrollback = "Scrollback must be 100-50000";
    return errs;
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  function handleSave() {
    if (!isValid) return;
    onSave({
      theme,
      fontSize,
      fontFamily,
      cursorStyle,
      cursorBlink,
      scrollback,
    });
  }

  function handleReset() {
    theme = DEFAULTS.theme;
    fontSize = DEFAULTS.fontSize;
    fontFamily = DEFAULTS.fontFamily;
    cursorStyle = DEFAULTS.cursorStyle;
    cursorBlink = DEFAULTS.cursorBlink;
    scrollback = DEFAULTS.scrollback;
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-label="Terminal Settings"
    tabindex="-1"
  >
    <div class="w-full max-w-lg bg-card rounded-xl border border-border shadow-2xl">
      <div class="flex items-center justify-between p-4 border-b border-border">
        <h2 class="text-lg font-semibold">Terminal Settings</h2>
        <Button variant="ghost" size="icon-sm" onclick={onClose}>
          <X class="size-4" />
        </Button>
      </div>

      <div class="p-4 space-y-5">
        <div class="space-y-2">
          <p class="text-sm font-medium">Theme</p>
          <div class="flex gap-2">
            <Button
              type="button"
              variant={theme === "dark" ? "default" : "outline"}
              size="sm"
              class="flex-1 gap-2"
              onclick={() => (theme = "dark")}
            >
              <Moon class="size-4" />
              Dark
            </Button>
            <Button
              type="button"
              variant={theme === "light" ? "default" : "outline"}
              size="sm"
              class="flex-1 gap-2"
              onclick={() => (theme = "light")}
            >
              <Sun class="size-4" />
              Light
            </Button>
          </div>
        </div>

        <div class="space-y-2">
          <label for="settings-fontsize" class="text-sm font-medium">Font Size</label>
          <Input
            id="settings-fontsize"
            type="number"
            min={8}
            max={32}
            bind:value={fontSize}
            class={errors.fontSize ? 'border-destructive' : ''}
          />
          {#if errors.fontSize}
            <p class="text-xs text-destructive">{errors.fontSize}</p>
          {/if}
        </div>

        <div class="space-y-2">
          <label for="settings-fontfamily" class="text-sm font-medium">Font Family</label>
          <div class="relative">
            <select
              id="settings-fontfamily"
              bind:value={fontFamily}
              class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 appearance-none"
            >
              {#each FONT_FAMILIES as font}
                <option value={font}>{font}</option>
              {/each}
            </select>
            <Type class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
          </div>
        </div>

        <div class="space-y-2">
          <label for="settings-cursorstyle" class="text-sm font-medium">Cursor Style</label>
          <div class="relative">
            <select
              id="settings-cursorstyle"
              bind:value={cursorStyle}
              class="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 appearance-none"
            >
              {#each CURSOR_STYLES as style}
                <option value={style}>{style}</option>
              {/each}
            </select>
            <MousePointer class="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
          </div>
        </div>

        <div class="flex items-center justify-between">
          <label for="settings-cursorblink" class="text-sm font-medium">Cursor Blink</label>
          <input
            id="settings-cursorblink"
            type="checkbox"
            bind:checked={cursorBlink}
            class="h-4 w-4 rounded border-border bg-transparent text-primary focus:ring-ring focus:ring-offset-0 cursor-pointer"
          />
        </div>

        <div class="space-y-2">
          <label for="settings-scrollback" class="text-sm font-medium">Scrollback Buffer</label>
          <Input
            id="settings-scrollback"
            type="number"
            min={100}
            max={50000}
            bind:value={scrollback}
            class={errors.scrollback ? 'border-destructive' : ''}
          />
          {#if errors.scrollback}
            <p class="text-xs text-destructive">{errors.scrollback}</p>
          {/if}
        </div>
      </div>

      <div class="flex gap-2 p-4 pt-0">
        <Button type="button" variant="outline" class="flex-1" onclick={handleReset}>
          <ScrollText class="mr-2 size-4" />
          Reset to Defaults
        </Button>
        <Button type="button" class="flex-1" disabled={!isValid} onclick={handleSave}>
          Save
        </Button>
      </div>
    </div>
  </div>
{/if}
