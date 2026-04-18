import { cleanup } from "@testing-library/svelte";
import { afterEach } from "vitest";

declare global {
  var __novertermTestSetupLoaded: boolean | undefined;
}

globalThis.__novertermTestSetupLoaded = true;

afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
});

export {};
