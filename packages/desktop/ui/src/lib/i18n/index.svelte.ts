import connections from "./zh-tw/connections.js";
import shell from "./zh-tw/shell.js";
import transfer from "./zh-tw/transfer.js";

export type Locale = "en" | "zh-TW";

export const LOCALES: { value: Locale; label: string }[] = [
  { value: "en", label: "English" },
  { value: "zh-TW", label: "繁體中文" },
];

/** English source text → 繁體中文. English needs no table: the key is the text. */
const zhTW: Record<string, string> = { ...shell, ...connections, ...transfer };

const STORAGE_KEY = "noverterm.locale";

let locale = $state<Locale>(initialLocale());

function initialLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "en" || saved === "zh-TW") return saved;
  } catch {
    // Storage can be unavailable; fall back to the system language.
  }
  return typeof navigator !== "undefined" && navigator.language.toLowerCase().startsWith("zh")
    ? "zh-TW"
    : "en";
}

export function getLocale(): Locale {
  return locale;
}

export function setLocale(next: Locale) {
  locale = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // Not persisted; the choice still applies until restart.
  }
}

/**
 * Translates English UI text. `{name}` placeholders are filled from `params`.
 * Text missing from the table shows in English rather than breaking.
 */
export function t(text: string, params?: Record<string, string | number>): string {
  const template = locale === "zh-TW" ? (zhTW[text] ?? text) : text;
  return params
    ? template.replace(/\{(\w+)\}/g, (match, key: string) =>
        key in params ? String(params[key]) : match,
      )
    : template;
}

/**
 * Like `t`, but leaves the `{name}` slots listed in `slots` unfilled so markup
 * can style them. Returns text runs and slot names in translated order, e.g.
 * `["", "destination", " 中已有名為 ", "name", " 的檔案。"]`; odd indexes are slots.
 */
export function tParts(
  text: string,
  slots: string[],
  params: Record<string, string | number> = {},
): string[] {
  const kept = Object.fromEntries(slots.map((slot) => [slot, `{${slot}}`]));
  return t(text, { ...params, ...kept }).split(/\{(\w+)\}/);
}

/** Exposed for the coverage test. */
export const zhTWTable = zhTW;
