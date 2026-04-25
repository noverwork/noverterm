<script lang="ts">
  import { MousePointer, ScrollText, Type, X } from "@lucide/svelte";
  import { superForm } from "sveltekit-superforms";
  import { zod4 } from "sveltekit-superforms/adapters";

  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import type { TerminalConfig } from "$lib/stores/bootstrap.svelte.js";
  import { settingsSchema, type SettingsForm } from "$lib/schemas/index.js";

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

  const DEFAULTS: SettingsForm = {
    fontSize: 14,
    fontFamily: "JetBrains Mono, Fira Code, monospace",
    cursorStyle: "block",
    cursorBlink: true,
    scrollback: 5000,
  };

  const form = superForm<SettingsForm>(
    { ...DEFAULTS },
    { validators: zod4(settingsSchema) },
  );

  const { form: formData, errors } = form;

  $effect(() => {
    if (open && config) {
      $formData.fontSize = config.fontSize;
      $formData.fontFamily = config.fontFamily;
      $formData.cursorStyle = config.cursorStyle;
      $formData.cursorBlink = config.cursorBlink;
      $formData.scrollback = config.scrollback;
      form.reset();
    }
  });

  async function handleSave() {
    const result = settingsSchema.safeParse($formData);
    if (!result.success) return;
    onSave({
      fontSize: $formData.fontSize,
      fontFamily: $formData.fontFamily,
      cursorStyle: $formData.cursorStyle,
      cursorBlink: $formData.cursorBlink,
      scrollback: $formData.scrollback,
    });
  }

  function handleReset() {
    $formData.fontSize = DEFAULTS.fontSize;
    $formData.fontFamily = DEFAULTS.fontFamily;
    $formData.cursorStyle = DEFAULTS.cursorStyle;
    $formData.cursorBlink = DEFAULTS.cursorBlink;
    $formData.scrollback = DEFAULTS.scrollback;
    form.reset();
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
    <div class="flex h-full max-h-[90vh] w-full max-w-3xl flex-col rounded-[1.75rem] border border-white/10 bg-slate-950/92 text-white shadow-2xl">
      <div class="flex items-center justify-between border-b border-white/10 px-6 py-4">
        <div>
          <p class="section-title text-slate-400">Workspace preferences</p>
          <h2 class="mt-1 text-lg font-semibold tracking-tight">Terminal settings</h2>
        </div>
        <Button variant="ghost" size="icon-sm" class="text-slate-300 hover:text-white" onclick={onClose}>
          <X class="size-4" />
        </Button>
      </div>

      <div class="flex-1 overflow-y-auto px-6 py-5">
        <div class="grid gap-5 lg:grid-cols-[0.95fr_1.05fr]">
          <div class="space-y-5 rounded-2xl border border-white/8 bg-white/[0.04] p-5">
          <div>
            <p class="section-title text-slate-400">Appearance</p>
            <h3 class="mt-2 text-lg font-semibold text-white">Visual tone of your terminal workspace</h3>
          </div>

          <div class="space-y-2">
            <label for="settings-fontfamily" class="text-sm font-medium text-slate-100">Font family</label>
            <div class="relative">
              <select
                id="settings-fontfamily"
                bind:value={$formData.fontFamily}
                class="flex h-11 w-full rounded-xl border border-white/10 bg-white/5 px-3 py-1 text-sm text-white shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring appearance-none"
              >
                {#each FONT_FAMILIES as font}
                  <option value={font}>{font}</option>
                {/each}
              </select>
              <Type class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
            </div>
            <p class="text-xs text-slate-400">JetBrains Mono stays the default, with other monospace options available if needed.</p>
          </div>

          <div class="space-y-2">
            <label for="settings-fontsize" class="text-sm font-medium text-slate-100">Font size</label>
            <Input id="settings-fontsize" type="number" min={8} max={32} bind:value={$formData.fontSize} class={$errors.fontSize ? "border-destructive bg-white/5 text-white" : "border-white/10 bg-white/5 text-white"} />
            {#if $errors.fontSize}
              <p class="text-xs text-destructive" role="alert">{$errors.fontSize}</p>
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
                bind:value={$formData.cursorStyle}
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
                class={$formData.cursorBlink ? "relative inline-flex h-7 w-12 items-center rounded-full bg-primary transition" : "relative inline-flex h-7 w-12 items-center rounded-full bg-white/10 transition"}
                onclick={() => ($formData.cursorBlink = !$formData.cursorBlink)}
                aria-pressed={$formData.cursorBlink}
                aria-label="Toggle cursor blink"
                title="Toggle cursor blink"
              >
                <span class={$formData.cursorBlink ? "inline-block size-5 translate-x-6 rounded-full bg-slate-950 transition" : "inline-block size-5 translate-x-1 rounded-full bg-white transition"}></span>
              </button>
            </div>
          </div>

          <div class="space-y-2">
            <label for="settings-scrollback" class="text-sm font-medium text-slate-100">Scrollback buffer</label>
            <Input id="settings-scrollback" type="number" min={100} max={50000} bind:value={$formData.scrollback} class={$errors.scrollback ? "border-destructive bg-white/5 text-white" : "border-white/10 bg-white/5 text-white"} />
            {#if $errors.scrollback}
              <p class="text-xs text-destructive" role="alert">{$errors.scrollback}</p>
            {:else}
              <p class="text-xs text-slate-400">Higher values preserve more command history, but can add memory overhead during long-running sessions.</p>
            {/if}
          </div>

          <div class="rounded-2xl border border-white/8 bg-slate-950/45 p-4 text-sm text-slate-300">
            Reset returns you to the default developer-focused profile: JetBrains Mono, block cursor,
            blinking caret, and a 5000-line scrollback buffer.
          </div>
          </div>
        </div>
      </div>

      <div class="flex shrink-0 gap-3 border-t border-white/10 px-6 py-4">
        <Button type="button" variant="outline" class="flex-1 border-white/10 bg-white/4 text-white hover:bg-white/8" onclick={handleReset}>
          <ScrollText class="mr-2 size-4" />
          Reset to defaults
        </Button>
        <Button type="button" class="flex-1" disabled={!!Object.keys($errors).length} onclick={handleSave}>
          Save settings
        </Button>
      </div>
    </div>
  </div>
{/if}
