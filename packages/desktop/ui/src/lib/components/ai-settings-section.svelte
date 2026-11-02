<script lang="ts">
  import { Sparkles } from "@lucide/svelte";

  import { commands, type AiApiFormat, type AiProvider } from "../../bindings.js";
  import {
    aiConfig,
    DEFAULT_MODELS,
    loadAiConfig,
    MODEL_SUGGESTIONS,
    saveAiConfig,
  } from "$lib/ai/ai-config.svelte.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { t } from "$lib/i18n/index.svelte.js";

  const fieldClass =
    "h-8 w-full rounded-[2px] border border-white/10 bg-rail px-2 text-sm text-stencil placeholder:text-stencil-faint focus:border-stencil focus:outline-none";

  let provider = $state<AiProvider>("anthropic");
  let model = $state("");
  let baseUrl = $state("");
  let apiFormat = $state<AiApiFormat | null>(null);
  let apiKey = $state("");
  let hasKey = $state(false);
  let saving = $state(false);
  let notice = $state<string | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void loadAiConfig().then(() => {
      const config = aiConfig();
      provider = config.provider;
      model = config.model;
      baseUrl = config.baseUrl;
      apiFormat = config.apiFormat;
      void refreshKeyState();
    });
  });

  async function refreshKeyState() {
    const result = await commands.aiHasApiKey(provider);
    hasKey = result.status === "ok" && result.data;
  }

  function handleProviderChange(next: AiProvider) {
    provider = next;
    model = DEFAULT_MODELS[next];
    baseUrl = "";
    apiFormat = null;
    apiKey = "";
    void refreshKeyState();
  }

  async function handleSave() {
    saving = true;
    notice = null;
    error = null;
    try {
      await saveAiConfig({ provider, model: model.trim(), baseUrl: baseUrl.trim(), apiFormat });
      if (apiKey.trim()) {
        const result = await commands.aiSetApiKey(provider, apiKey);
        if (result.status === "error") throw new Error(result.error);
        apiKey = "";
      }
      await refreshKeyState();
      notice = t("Saved");
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function handleRemoveKey() {
    const result = await commands.aiSetApiKey(provider, null);
    if (result.status === "error") {
      error = result.error;
      return;
    }
    await refreshKeyState();
  }
</script>

<div class="rounded-[2px] border border-white/10 bg-transparent px-5 py-5">
  <div class="flex items-center gap-3">
    <div
      class="grid size-10 place-items-center rounded-[2px] border border-white/10 bg-transparent text-stencil-dim"
    >
      <Sparkles class="size-4" />
    </div>
    <p class="flex-1 text-sm font-semibold text-stencil">{t("AI assistant")}</p>
  </div>

  <div class="mt-4 grid grid-cols-[6rem_1fr] items-center gap-x-3 gap-y-2 text-sm">
    <label for="ai-provider" class="text-stencil-mute">{t("Provider")}</label>
    <select
      id="ai-provider"
      class={fieldClass}
      value={provider}
      onchange={(event) => handleProviderChange(event.currentTarget.value as AiProvider)}
    >
      <option value="anthropic">Anthropic</option>
      <option value="opencode_go">OpenCode Go</option>
      <option value="openai_compatible">{t("OpenAI-compatible (OpenAI, Ollama…)")}</option>
    </select>

    <label for="ai-model" class="text-stencil-mute">{t("Model")}</label>
    <input
      id="ai-model"
      class={fieldClass}
      bind:value={model}
      list="ai-model-suggestions"
      placeholder={t("e.g. {example}", {
        example: provider === "openai_compatible" ? "gpt-5-mini, llama3.2" : DEFAULT_MODELS[provider],
      })}
    />
    <datalist id="ai-model-suggestions">
      {#each MODEL_SUGGESTIONS[provider] as suggestion (suggestion)}
        <option value={suggestion}></option>
      {/each}
    </datalist>

    {#if provider === "opencode_go"}
      <label for="ai-api-format" class="text-stencil-mute">{t("API format")}</label>
      <select
        id="ai-api-format"
        class={fieldClass}
        value={apiFormat ?? ""}
        onchange={(event) =>
          (apiFormat = (event.currentTarget.value || null) as AiApiFormat | null)}
      >
        <option value="">{t("Auto (by model)")}</option>
        <option value="anthropic">Anthropic · /messages</option>
        <option value="openai">OpenAI · /chat/completions</option>
        <option value="responses">Responses · /responses</option>
        <option value="gemini">Gemini · generateContent</option>
      </select>
    {/if}

    {#if provider === "openai_compatible"}
      <label for="ai-base-url" class="text-stencil-mute">{t("Base URL")}</label>
      <input
        id="ai-base-url"
        class={fieldClass}
        bind:value={baseUrl}
        placeholder={t("e.g. {example}", { example: "http://localhost:11434/v1" })}
      />
    {/if}

    <label for="ai-api-key" class="text-stencil-mute">{t("API key")}</label>
    <div class="flex items-center gap-2">
      <input
        id="ai-api-key"
        type="password"
        autocomplete="off"
        class={fieldClass}
        bind:value={apiKey}
        placeholder={hasKey ? t("Saved in keychain") : undefined}
      />
      {#if hasKey}
        <Button
          variant="ghost"
          size="sm"
          class="rounded-[2px] text-stencil-mute hover:bg-white/8 hover:text-stencil"
          onclick={handleRemoveKey}
        >
          {t("Remove")}
        </Button>
      {/if}
    </div>
  </div>

  <div class="mt-4 flex items-center gap-3">
    <Button
      variant="outline"
      size="sm"
      class="rounded-[2px] border-white/10 bg-transparent text-sm text-stencil hover:border-white/20 hover:bg-white/5"
      onclick={handleSave}
      disabled={saving}
    >
      {t("Save")}
    </Button>
    {#if notice}
      <p class="text-xs text-emerald-400">{notice}</p>
    {/if}
    {#if error}
      <p class="text-xs text-red-300">{error}</p>
    {/if}
  </div>
</div>
