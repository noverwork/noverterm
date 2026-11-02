import { commands, type AiApiFormat, type AiProvider } from "../../bindings.js";

export interface AiConfig {
  provider: AiProvider;
  model: string;
  /** Empty means the provider's default endpoint. */
  baseUrl: string;
  /** OpenCode Go only; `null` picks the format from the model name. */
  apiFormat: AiApiFormat | null;
}

const PROVIDERS: AiProvider[] = ["anthropic", "openai_compatible", "opencode_go"];
const API_FORMATS: AiApiFormat[] = ["anthropic", "openai", "responses", "gemini"];

const SETTING_KEY = "noverterm-ai";

export const DEFAULT_MODELS: Record<AiProvider, string> = {
  anthropic: "claude-haiku-5-5",
  openai_compatible: "",
  opencode_go: "minimax-m3",
};

/** Suggestions for the model field; any model the provider serves works. */
export const MODEL_SUGGESTIONS: Record<AiProvider, string[]> = {
  anthropic: ["claude-haiku-5-5", "claude-sonnet-5-5", "claude-opus-5-5"],
  openai_compatible: [],
  opencode_go: [
    "minimax-m3",
    "qwen3.8-max",
    "qwen3.8-flash",
    "glm-5.3",
    "glm-5.3-flash",
    "kimi-k3",
    "deepseek-v4-pro",
    "deepseek-v4.1-flash",
    "claude-haiku-5-5",
    "gpt-5.6-luna",
    "grok-4.7",
  ],
};

const config = $state<AiConfig>({
  provider: "anthropic",
  model: DEFAULT_MODELS.anthropic,
  baseUrl: "",
  apiFormat: null,
});
let loaded: Promise<void> | null = null;

/** Reads the saved config once; later calls reuse it. */
export function loadAiConfig(): Promise<void> {
  loaded ??= commands.getSetting(SETTING_KEY).then((result) => {
    if (result.status !== "ok" || !result.data) return;
    try {
      const saved = JSON.parse(result.data.value) as Partial<AiConfig>;
      if (saved.provider && PROVIDERS.includes(saved.provider)) config.provider = saved.provider;
      if (typeof saved.model === "string") config.model = saved.model;
      if (typeof saved.baseUrl === "string") config.baseUrl = saved.baseUrl;
      config.apiFormat =
        saved.apiFormat && API_FORMATS.includes(saved.apiFormat) ? saved.apiFormat : null;
    } catch {
      // A corrupt value falls back to the defaults.
    }
  });
  return loaded;
}

export function aiConfig(): AiConfig {
  return config;
}

export async function saveAiConfig(next: AiConfig): Promise<void> {
  Object.assign(config, next);
  const result = await commands.setSetting({ key: SETTING_KEY, value: JSON.stringify(next) });
  if (result.status === "error") throw new Error(result.error);
}
