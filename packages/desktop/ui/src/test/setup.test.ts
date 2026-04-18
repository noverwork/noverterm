import { describe, expect, it } from "vitest";

describe("vitest harness", () => {
  it("loads the shared setup file", () => {
    expect(globalThis.__novertermTestSetupLoaded).toBe(true);
  });
});
