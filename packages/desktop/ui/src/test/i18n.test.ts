import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { setLocale, t, tParts, zhTWTable } from "$lib/i18n/index.svelte.js";

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.(svelte|ts)$/.test(name) && !name.endsWith(".test.ts") ? [path] : [];
  });
}

describe("i18n", () => {
  it("fills placeholders and falls back to English", () => {
    setLocale("en");
    expect(t("Import {count}", { count: 3 })).toBe("Import 3");
    setLocale("zh-TW");
    expect(t("not in the table")).toBe("not in the table");
    setLocale("en");
  });

  it("keeps styled slots in translated order", () => {
    setLocale("zh-TW");
    const parts = tParts("A {kind} named {name} already exists in {destination}.", ["name", "destination"], {
      kind: "file",
    });
    setLocale("en");
    expect(parts.filter((_, index) => index % 2 === 1)).toEqual(["destination", "name"]);
  });

  it("has a 繁體中文 entry for every t() literal", () => {
    const root = join(import.meta.dirname, "..");
    const missing = new Set<string>();
    for (const file of sourceFiles(root)) {
      for (const match of readFileSync(file, "utf8").matchAll(/\bt(?:Parts)?\(\s*(["'`])((?:\\.|(?!\1).)*)\1/g)) {
        const text = match[2].replace(/\\(["'`])/g, "$1");
        if (!(text in zhTWTable)) missing.add(`${file}: ${text}`);
      }
    }
    expect([...missing]).toEqual([]);
  });
});
