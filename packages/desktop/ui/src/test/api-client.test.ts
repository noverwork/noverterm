import { beforeEach, describe, expect, it, vi } from "vitest";

const mockFetch = vi.fn();
const mockInvoke = vi.fn();

vi.mock("@tauri-apps/plugin-http", () => ({
  fetch: (...args: unknown[]) => mockFetch(...args),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { loadAppSettings, requestJson } from "$lib/api/api-client.js";

describe("api client URL composition", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ ok: true }),
    });
  });

  it("adds the API prefix when API_URL is an origin", async () => {
    mockInvoke.mockResolvedValue({ api_url: "https://noverterm.noverwork.com" });

    await loadAppSettings();
    await requestJson("/keys");

    expect(mockFetch).toHaveBeenCalledWith(
      "https://noverterm.noverwork.com/api/keys",
      expect.any(Object),
    );
  });

  it("does not duplicate the API prefix when API_URL already includes it", async () => {
    mockInvoke.mockResolvedValue({ api_url: "https://noverterm.noverwork.com/api" });

    await loadAppSettings();
    await requestJson("/api/keys");

    expect(mockFetch).toHaveBeenCalledWith(
      "https://noverterm.noverwork.com/api/keys",
      expect.any(Object),
    );
  });
});
