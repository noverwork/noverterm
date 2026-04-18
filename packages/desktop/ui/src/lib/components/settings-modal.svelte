<script lang="ts">
  import { Moon, MousePointer, ScrollText, Sun, Type, X } from "@lucide/svelte";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { TerminalConfig } from "$lib/stores/bootstrap.svelte.js";

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

  let theme = $state<TerminalConfig["theme"]>("dark");
  let fontSize = $state(14);
  let fontFamily = $state<string>(FONT_FAMILIES[0]);
  let cursorStyle = $state<TerminalConfig["cursorStyle"]>("block");
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

  const errors = $derived.by(() => {
    const errs: Record<string, string> = {};
    if (fontSize < 8 || fontSize > 32) errs.fontSize = "Font size must stay between 8 and 32";
    if (scrollback < 100 || scrollback > 50000) errs.scrollback = "Scrollback must stay between 100 and 50000";
    return errs;
  });

  const isValid = $derived(Object.keys(errors).length === 0);

  function handleSave() {
    if (!isValid) return;
    onSave({ theme, fontSize, fontFamily, cursorStyle, cursorBlink, scrollback });
  }

  function handleReset() {
    theme = DEFAULTS.theme;
    fontSize = DEFAULTS.fontSize;
    fontFamily = DEFAULTS.fontFamily;
    cursorStyle = DEFAULTS.cursorStyle;
    cursorBlink = DEFAULTS.cursorBlink;
    scrollback = DEFAULTS.scrollback;
  }
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70 px-4 backdrop-blur-md"
    onclick={(event) => event.target === event.currentTarget && onClose()}
    onkeydown={(event) => event.key === "Escape" && onClose()}
    role="dialog"
    aria-modal="true"
    aria-label="Terminal settings"
    tabindex="-1"
  >
    <div class="w-full max-w-3xl rounded-[1.75rem] border border-white/10 bg-slate-950/92 text-white shadow-2xl">
      <div class="flex items-center justify-between border-b border-white/10 px-6 py-5">
        <div>
          <p class="section-title text-slate-400">Workspace preferences</p>
          <h2 class="mt-2 text-xl font-semibold tracking-tight">Terminal settings</h2>
          <p class="mt-2 text-sm leading-6 text-slate-400">Tune appearance and terminal behavior without leaving your current workspace.</p>
        </div>
        <Button variant="ghost" size="icon-sm" class="text-slate-300 hover:text-white" onclick={onClose}>
          <X class="size-4" />
        </Button>
      </div>

      <div class="grid gap-6 px-6 py-6 lg:grid-cols-[0.95fr_1.05fr]">
        <div class="space-y-5 rounded-2xl border border-white/8 bg-white/[0.04] p-5">
          <div>
            <p class="section-title text-slate-400">Appearance</p>
            <h3 class="mt-2 text-lg font-semibold text-white">Visual tone of your terminal workspace</h3>
          </div>

          <div class="grid gap-3 sm:grid-cols-2">
            <button type="button" class={theme === "dark" ? "rounded-2xl border border-primary/35 bg-primary/10 p-4 text-left" : "rounded-2xl border border-white/8 bg-white/[0.03] p-4 text-left hover:bg-white/[0.06]"} onclick={() => (theme = "dark")}>
              <Moon class="size-5 text-primary" />
              <p class="mt-4 font-medium text-white">Dark workspace</p>
              <p class="mt-2 text-sm leading-6 text-slate-400">Best for sustained terminal work and low-eye-strain environments.</p>
            </button>
            <button type="button" class={theme === "light" ? "rounded-2xl border border-primary/35 bg-primary/10 p-4 text-left" : "rounded-2xl border border-white/8 bg-white/[0.03] p-4 text-left hover:bg-white/[0.06]"} onclick={() => (theme = "light")}>
              <Sun class="size-5 text-primary" />
              <p class="mt-4 font-medium text-white">Light workspace</p>
              <p class="mt-2 text-sm leading-6 text-slate-400">Useful when you want higher contrast against the desktop and brighter shell output.</p>
            </button>
          </div>

          <div class="space-y-2">
            <label for="settings-fontfamily" class="text-sm font-medium text-slate-100">Font family</label>
            <div class="relative">
              <select
                id="settings-fontfamily"
                bind:value={fontFamily}
                class="flex h-11 w-full rounded-xl border border-white/10 bg-white/5 px-3 py-1 text-sm text-white shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring appearance-none"
              >
                {#each FONT_FAMILIES as font}
                  <option value={font}>{font}</option>
                {/each}
              </select>
              <Type class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
            </div>
            <p class="text-xs text-slate-400">JetBrains Mono stays the default, but you can switch to a lighter monospace tone if needed.</p>
          </div>

          <div class="space-y-2">
            <label for="settings-fontsize" class="text-sm font-medium text-slate-100">Font size</label>
            <Input id="settings-fontsize" type="number" min={8} max={32} bind:value={fontSize} class={errors.fontSize ? "border-destructive bg-white/5 text-white" : "border-white/10 bg-white/5 text-white"} />
            {#if errors.fontSize}
              <p class="text-xs text-destructive" role="alert">{errors.fontSize}</p>
            {:else}
              <p class="text-xs text-slate-400">Balance density and readability for how many panes or logs you usually keep open.</p>
            {/if}
          </div>
        </div>

        <div class="space-y-5 rounded-2xl border border-white/8 bg-white/[0.04] p-5">
          <div>
            <p class="section-title text-slate-400">Terminal behavior</p>
            <h3 class="mt-2 text-lg font-semibold text-white">Cursor and history management</h3>
          </div>

          <div class="space-y-2">
            <label for="settings-cursorstyle" class="text-sm font-medium text-slate-100">Cursor style</label>
            <div class="relative">
              <select
                id="settings-cursorstyle"
                bind:value={cursorStyle}
                class="flex h-11 w-full rounded-xl border border-white/10 bg-white/5 px-3 py-1 text-sm text-white shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring appearance-none"
              >
                {#each CURSOR_STYLES as style}
                  <option value={style}>{style}</option>
                {/each}
              </select>
              <MousePointer class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
            </div>
            <p class="text-xs text-slate-400">Choose the cursor that feels most legible for your shell font and color scheme.</p>
          </div>

          <div class="rounded-2xl border border-white/8 bg-slate-950/45 p-4">
            <div class="flex items-center justify-between gap-4">
              <div>
                <p class="text-sm font-medium text-slate-100">Cursor blink</p>
                <p class="mt-1 text-xs leading-5 text-slate-400">Disable this if you prefer a steady caret while reading dense logs.</p>
              </div>
              <button
                type="button"
                class={cursorBlink ? "relative inline-flex h-7 w-12 items-center rounded-full bg-primary transition" : "relative inline-flex h-7 w-12 items-center rounded-full bg-white/10 transition"}
                onclick={() => (cursorBlink = !cursorBlink)}
                aria-pressed={cursorBlink}
                aria-label="Toggle cursor blink"
                title="Toggle cursor blink"
              >
                <span class={cursorBlink ? "inline-block size-5 translate-x-6 rounded-full bg-slate-950 transition" : "inline-block size-5 translate-x-1 rounded-full bg-white transition"}></span>
              </button>
            </div>
          </div>

          <div class="space-y-2">
            <label for="settings-scrollback" class="text-sm font-medium text-slate-100">Scrollback buffer</label>
            <Input id="settings-scrollback" type="number" min={100} max={50000} bind:value={scrollback} class={errors.scrollback ? "border-destructive bg-white/5 text-white" : "border-white/10 bg-white/5 text-white"} />
            {#if errors.scrollback}
              <p class="text-xs text-destructive" role="alert">{errors.scrollback}</p>
            {:else}
              <p class="text-xs text-slate-400">Higher values preserve more command history, but can add memory overhead during long-running sessions.</p>
            {/if}
          </div>

          <div class="rounded-2xl border border-white/8 bg-slate-950/45 p-4 text-sm text-slate-300">
            Reset returns you to the default developer-focused profile: dark theme, JetBrains Mono, block cursor,
            blinking caret, and a 5000-line scrollback buffer.
          </div>
        </div>
      </div>

      <div class="flex gap-3 border-t border-white/10 px-6 py-5">
        <Button type="button" variant="outline" class="flex-1 border-white/10 bg-white/4 text-white hover:bg-white/8" onclick={handleReset}>
          <ScrollText class="mr-2 size-4" />
          Reset to defaults
        </Button>
        <Button type="button" class="flex-1" disabled={!isValid} onclick={handleSave}>
          Save settings
        </Button>
      </div>
    </div>
  </div>
{/if}
