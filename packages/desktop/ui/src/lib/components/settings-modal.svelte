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
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/76 px-4 backdrop-blur-xl"
    onclick={(event) => event.target === event.currentTarget && onClose()}
    onkeydown={(event) => event.key === "Escape" && onClose()}
    role="dialog"
    aria-modal="true"
    aria-label="Terminal settings"
    tabindex="-1"
  >
    <div class="ide-panel flex h-full max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden text-white shadow-[0_32px_100px_rgb(0_0_0/0.62)]">
      <div class="flex shrink-0 items-center justify-between border-b border-white/10 px-6 py-4">
        <div>
          <p class="section-title text-cyan-200/70">Workspace preferences</p>
          <h2 class="mt-1 text-xl font-semibold tracking-tight">Terminal settings</h2>
        </div>
        <Button variant="ghost" size="icon-sm" class="rounded-2xl text-slate-400 hover:bg-white/8 hover:text-white" onclick={onClose} aria-label="Close settings" title="Close settings">
          <X class="size-4" />
        </Button>
      </div>

      <div class="flex-1 overflow-y-auto px-6 py-5">
        <div class="grid gap-5 lg:grid-cols-[0.95fr_1.05fr]">
          <section class="rounded-[1.35rem] border border-white/10 bg-white/[0.035] p-5">
            <div class="mb-5 flex items-start gap-3">
              <div class="grid size-10 shrink-0 place-items-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                <Type class="size-4" />
              </div>
              <div>
                <p class="section-title text-slate-500">Appearance</p>
                <h3 class="mt-1 text-base font-semibold text-white">Typography density</h3>
              </div>
            </div>

            <div class="space-y-5">
              <div class="space-y-2">
                <label for="settings-fontfamily" class="text-sm font-medium text-slate-100">Font family</label>
                <div class="relative">
                  <select
                    id="settings-fontfamily"
                    bind:value={$formData.fontFamily}
                    class="flex h-11 w-full appearance-none rounded-2xl border border-white/10 bg-black/20 px-3 py-1 text-sm text-white shadow-sm transition-colors focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20"
                  >
                    {#each FONT_FAMILIES as font}
                      <option value={font}>{font}</option>
                    {/each}
                  </select>
                  <Type class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
                </div>
                <p class="text-xs leading-5 text-slate-500">JetBrains Mono stays the tuned default, with other monospace options available.</p>
              </div>

              <div class="space-y-2">
                <label for="settings-fontsize" class="text-sm font-medium text-slate-100">Font size</label>
                <Input id="settings-fontsize" type="number" min={8} max={32} bind:value={$formData.fontSize} class={$errors.fontSize ? "border-destructive bg-black/20 text-white" : "border-white/10 bg-black/20 text-white focus-visible:border-cyan-300/40"} />
                {#if $errors.fontSize}
                  <p class="text-xs text-destructive" role="alert">{$errors.fontSize}</p>
                {:else}
                  <p class="text-xs leading-5 text-slate-500">Balance density and readability for logs, panes, and long-running output.</p>
                {/if}
              </div>
            </div>
          </section>

          <section class="rounded-[1.35rem] border border-white/10 bg-white/[0.035] p-5">
            <div class="mb-5 flex items-start gap-3">
              <div class="grid size-10 shrink-0 place-items-center rounded-2xl border border-cyan-300/14 bg-cyan-300/8 text-cyan-200">
                <MousePointer class="size-4" />
              </div>
              <div>
                <p class="section-title text-slate-500">Terminal behavior</p>
                <h3 class="mt-1 text-base font-semibold text-white">Cursor and history</h3>
              </div>
            </div>

            <div class="space-y-5">
              <div class="space-y-2">
                <label for="settings-cursorstyle" class="text-sm font-medium text-slate-100">Cursor style</label>
                <div class="relative">
                  <select
                    id="settings-cursorstyle"
                    bind:value={$formData.cursorStyle}
                    class="flex h-11 w-full appearance-none rounded-2xl border border-white/10 bg-black/20 px-3 py-1 text-sm text-white shadow-sm transition-colors focus-visible:border-cyan-300/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-cyan-300/20"
                  >
                    {#each CURSOR_STYLES as style}
                      <option value={style}>{style}</option>
                    {/each}
                  </select>
                  <MousePointer class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-slate-500" />
                </div>
                <p class="text-xs leading-5 text-slate-500">Choose the caret shape that stays most legible while scanning output.</p>
              </div>

              <div class="rounded-2xl border border-white/8 bg-black/20 p-4">
                <div class="flex items-center justify-between gap-4">
                  <div>
                    <p class="text-sm font-medium text-slate-100">Cursor blink</p>
                    <p class="mt-1 text-xs leading-5 text-slate-500">Disable if you prefer a steady caret while reading dense logs.</p>
                  </div>
                  <button
                    type="button"
                    class={$formData.cursorBlink ? "relative inline-flex h-7 w-12 cursor-pointer items-center rounded-full bg-cyan-300 transition" : "relative inline-flex h-7 w-12 cursor-pointer items-center rounded-full bg-white/10 transition"}
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
                <Input id="settings-scrollback" type="number" min={100} max={50000} bind:value={$formData.scrollback} class={$errors.scrollback ? "border-destructive bg-black/20 text-white" : "border-white/10 bg-black/20 text-white focus-visible:border-cyan-300/40"} />
                {#if $errors.scrollback}
                  <p class="text-xs text-destructive" role="alert">{$errors.scrollback}</p>
                {:else}
                  <p class="text-xs leading-5 text-slate-500">Higher values preserve more command history but increase memory use.</p>
                {/if}
              </div>
            </div>
          </section>
        </div>

        <div class="mt-5 rounded-[1.35rem] border border-white/8 bg-black/20 p-4 text-sm leading-6 text-slate-400">
          Reset returns you to JetBrains Mono, 14px text, block cursor, blinking caret, and a 5000-line scrollback buffer.
        </div>
      </div>

      <div class="flex shrink-0 gap-3 border-t border-white/10 px-6 py-4">
        <Button type="button" variant="outline" class="flex-1 rounded-2xl border-white/10 bg-white/[0.035] text-white hover:bg-white/8" onclick={handleReset}>
          <ScrollText class="mr-2 size-4" />
          Reset defaults
        </Button>
        <Button type="button" class="flex-1 rounded-2xl bg-cyan-300 text-slate-950 hover:bg-cyan-200" disabled={!!Object.keys($errors).length} onclick={handleSave}>
          Save settings
        </Button>
      </div>
    </div>
  </div>
{/if}
