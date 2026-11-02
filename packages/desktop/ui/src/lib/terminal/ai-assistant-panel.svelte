<script lang="ts">
  import { Copy, CornerDownLeft, Loader2, Sparkles, X } from "@lucide/svelte";
  import { onMount, tick } from "svelte";

  import { commands } from "../../bindings.js";
  import { aiConfig, loadAiConfig } from "$lib/ai/ai-config.svelte.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { getLocale, t } from "$lib/i18n/index.svelte.js";

  interface Props {
    /** Selected terminal text; when present the panel explains it. */
    selection: string | null;
    getContext: () => string;
    onInsert: (command: string) => void;
    onClose: () => void;
  }

  let { selection, getContext, onInsert, onClose }: Props = $props();

  let input = $state("");
  let reply = $state<string | null>(null);
  let error = $state<string | null>(null);
  let asking = $state(false);
  let inputElement = $state<HTMLInputElement | null>(null);

  const explaining = $derived(selection !== null);
  // A newline would run the command, so only a single line goes straight into the prompt.
  const insertable = $derived(!explaining && reply !== null && !reply.includes("\n"));

  onMount(() => {
    void tick().then(() => inputElement?.focus());
    if (explaining) void ask();
  });

  async function ask() {
    if (!explaining && !input.trim()) return;
    asking = true;
    error = null;
    reply = null;
    try {
      await loadAiConfig();
      const config = aiConfig();
      const result = await commands.aiAsk({
        provider: config.provider,
        model: config.model,
        base_url: config.baseUrl || null,
        api_format: config.apiFormat,
        task: explaining ? "explain" : "command",
        input: explaining ? (selection ?? "") : input,
        context: getContext(),
        question: explaining ? input : null,
        language: getLocale() === "zh-TW" ? "Traditional Chinese (Taiwan)" : "English",
      });
      if (result.status === "error") throw new Error(result.error);
      reply = result.data;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      asking = false;
    }
  }

  function insert() {
    if (!insertable || reply === null) return;
    onInsert(reply);
  }

  function handleKeydown(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    } else if (event.key === "Enter" && !event.isComposing) {
      event.preventDefault();
      if (insertable && !input.trim()) insert();
      else void ask();
    }
  }
</script>

<div
  class="absolute inset-x-3 bottom-3 z-30 mx-auto max-w-2xl rounded-[2px] border border-white/15 bg-rail text-stencil shadow-lg"
  role="dialog"
  aria-label={explaining ? t("Explain selection") : t("Generate command")}
>
  <div class="flex items-center gap-2 px-3 py-2">
    <Sparkles class="size-4 shrink-0 text-stencil-dim" aria-hidden="true" />
    <input
      bind:this={inputElement}
      bind:value={input}
      class="h-7 min-w-0 flex-1 bg-transparent text-sm text-stencil placeholder:text-stencil-faint focus:outline-none"
      placeholder={explaining
        ? t("Ask a follow-up about the selection")
        : t("Describe what you want to do")}
      aria-label={explaining ? t("Question about the selection") : t("Command request")}
      onkeydown={handleKeydown}
      disabled={asking}
    />
    {#if asking}
      <Loader2 class="size-4 animate-spin text-stencil-mute" aria-label={t("Asking AI…")} />
    {/if}
    <Button
      variant="ghost"
      size="icon-xs"
      class="size-6 text-stencil-mute hover:bg-white/10 hover:text-stencil"
      aria-label={t("Close AI assistant")}
      onclick={onClose}
    >
      <X class="size-3" />
    </Button>
  </div>

  {#if error}
    <p class="border-t border-white/10 px-3 py-2 text-xs text-red-300" role="alert">{error}</p>
  {:else if reply !== null}
    <div class="border-t border-white/10 px-3 py-2">
      {#if explaining}
        <p class="max-h-64 overflow-y-auto whitespace-pre-wrap text-sm leading-6">{reply}</p>
      {:else}
        <div class="flex items-start gap-2">
          <pre class="max-h-40 min-w-0 flex-1 overflow-auto whitespace-pre-wrap font-mono text-sm text-stencil">{reply}</pre>
          {#if insertable}
            <Button
              size="sm"
              class="h-7 gap-1.5 rounded-[2px] bg-stencil px-2 text-xs text-port hover:bg-white"
              onclick={insert}
            >
              <CornerDownLeft class="size-3" />
              {t("Insert")}
            </Button>
          {/if}
          <Button
            variant="ghost"
            size="sm"
            class="h-7 gap-1.5 rounded-[2px] px-2 text-xs text-stencil-mute hover:bg-white/10 hover:text-stencil"
            onclick={() => void navigator.clipboard.writeText(reply ?? "")}
          >
            <Copy class="size-3" />
            {t("Copy")}
          </Button>
        </div>
      {/if}
    </div>
  {/if}
</div>
