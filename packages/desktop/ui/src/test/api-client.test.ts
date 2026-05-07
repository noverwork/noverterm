import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mockFetch = vi.fn();
const mockInvoke = vi.fn();
const mockLoadStoredAuthTokens = vi.fn();
const mockSaveStoredAuthTokens = vi.fn();
const mockClearStoredAuthTokens = vi.fn();

vi.mock("@tauri-apps/plugin-http", () => ({
  fetch: (...args: unknown[]) => mockFetch(...args),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("$lib/stores/auth-token-store.js", () => ({
  loadStoredAuthTokens: () => mockLoadStoredAuthTokens(),
  saveStoredAuthTokens: (tokens: unknown) => mockSaveStoredAuthTokens(tokens),
  clearStoredAuthTokens: () => mockClearStoredAuthTokens(),
}));

import {
  clearFrontendTokens,
  HttpError,
  loadAppSettings,
  persistFrontendTokens,
  requestJson,
  toSessionTokens,
  withAuthorizedRetry,
} from "$lib/api/api-client.js";

function authResponse(accessToken: string, refreshToken: string) {
  return {
    access_token: accessToken,
    refresh_token: refreshToken,
    access_token_expires_at: "2026-05-04T12:00:00Z",
    email: "alice@example.com",
  };
}

function jsonResponse(body: unknown): Response {
  return {
    ok: true,
    json: async () => body,
  } as Response;
}

async function flushPromises(times: number): Promise<void> {
  for (let index = 0; index < times; index += 1) {
    await Promise.resolve();
  }
}

describe("api client URL composition", () => {
  beforeEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({ ok: true }),
    });
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
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

  it("keeps access token expiration in persisted session tokens", () => {
    const tokens = toSessionTokens({
      access_token: "access.jwt",
      refresh_token: "refresh.jwt",
      access_token_expires_at: "2026-05-04T12:00:00Z",
      email: "alice@example.com",
    });

    expect(tokens).toEqual({
      access_token: "access.jwt",
      refresh_token: "refresh.jwt",
      access_token_expires_at: "2026-05-04T12:00:00Z",
      email: "alice@example.com",
    });
  });

  it("does not persist an in-flight refresh after tokens are cleared", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue({ api_url: "https://noverterm.noverwork.com" });
    mockLoadStoredAuthTokens.mockResolvedValue({
      access_token: "expired-access.jwt",
      refresh_token: "old-refresh.jwt",
      access_token_expires_at: "2020-01-01T00:00:00Z",
      email: "alice@example.com",
    });
    let resolveRefresh: (response: Response) => void = () => {};
    const refreshResponse = new Promise<Response>((resolve) => {
      resolveRefresh = resolve;
    });
    mockFetch.mockReturnValueOnce(refreshResponse);

    await loadAppSettings();
    const request = vi.fn().mockResolvedValue("ok");
    const pendingRequest = withAuthorizedRetry(request);
    await Promise.resolve();

    await clearFrontendTokens();
    resolveRefresh(jsonResponse(authResponse("new-access.jwt", "new-refresh.jwt")));

    await expect(pendingRequest).rejects.toThrow("stale token refresh ignored");
    expect(mockSaveStoredAuthTokens).not.toHaveBeenCalled();
    expect(request).not.toHaveBeenCalled();
  });

  it("clears stale tokens when logout happens during refresh persistence", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue({ api_url: "https://noverterm.noverwork.com" });
    mockLoadStoredAuthTokens.mockResolvedValue({
      access_token: "expired-access.jwt",
      refresh_token: "old-refresh.jwt",
      access_token_expires_at: "2020-01-01T00:00:00Z",
      email: "alice@example.com",
    });
    let resolveSave: () => void = () => {};
    mockSaveStoredAuthTokens.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        resolveSave = resolve;
      }),
    );
    mockFetch.mockResolvedValueOnce(
      jsonResponse(authResponse("new-access.jwt", "new-refresh.jwt")),
    );

    await loadAppSettings();
    const request = vi.fn().mockResolvedValue("ok");
    const pendingRequest = withAuthorizedRetry(request);
    await flushPromises(5);
    expect(mockSaveStoredAuthTokens).toHaveBeenCalledOnce();

    await clearFrontendTokens();
    resolveSave();

    await expect(pendingRequest).rejects.toThrow("stale token refresh ignored");
    expect(mockSaveStoredAuthTokens).toHaveBeenCalledWith({
      access_token: "new-access.jwt",
      refresh_token: "new-refresh.jwt",
      access_token_expires_at: "2026-05-04T12:00:00Z",
      email: "alice@example.com",
    });
    expect(mockClearStoredAuthTokens).toHaveBeenCalledTimes(2);
    expect(request).not.toHaveBeenCalled();
  });

  it("ignores stored tokens that resolve after tokens are cleared", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue({ api_url: "https://noverterm.noverwork.com" });
    let resolveLoad: (tokens: unknown) => void = () => {};
    mockLoadStoredAuthTokens.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveLoad = resolve;
      }),
    );

    await loadAppSettings();
    const request = vi.fn().mockResolvedValue("ok");
    const pendingRequest = withAuthorizedRetry(request);
    await Promise.resolve();

    await clearFrontendTokens();
    resolveLoad({
      access_token: "expired-access.jwt",
      refresh_token: "old-refresh.jwt",
      access_token_expires_at: "2020-01-01T00:00:00Z",
      email: "alice@example.com",
    });

    await expect(pendingRequest).rejects.toThrow("stale stored tokens ignored");
    expect(mockFetch).not.toHaveBeenCalled();
    expect(mockSaveStoredAuthTokens).not.toHaveBeenCalled();
    expect(request).not.toHaveBeenCalled();
  });

  it("does not refresh after tokens are cleared during an authorized request", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-04T11:00:00Z"));
    mockInvoke.mockResolvedValue({ api_url: "https://noverterm.noverwork.com" });
    mockLoadStoredAuthTokens.mockResolvedValue({
      access_token: "valid-access.jwt",
      refresh_token: "old-refresh.jwt",
      access_token_expires_at: "2026-05-04T12:00:00Z",
      email: "alice@example.com",
    });
    let rejectRequest: (error: Error) => void = () => {};
    const protectedRequest = vi.fn(
      () =>
        new Promise<string>((_resolve, reject) => {
          rejectRequest = reject;
        }),
    );

    await loadAppSettings();
    const pendingRequest = withAuthorizedRetry(protectedRequest);
    await flushPromises(3);
    expect(protectedRequest).toHaveBeenCalledWith("valid-access.jwt");

    await clearFrontendTokens();
    rejectRequest(new HttpError(401, "unauthorized"));

    await expect(pendingRequest).rejects.toThrow("stale token refresh ignored");
    expect(mockFetch).not.toHaveBeenCalled();
    expect(mockSaveStoredAuthTokens).not.toHaveBeenCalled();
    expect(protectedRequest).toHaveBeenCalledTimes(1);
  });

  it("does not refresh from a stale scheduled timer callback", async () => {
    const scheduledCallbacks: Array<() => void> = [];
    const setTimeoutSpy = vi
      .spyOn(globalThis, "setTimeout")
      .mockImplementation((callback: TimerHandler) => {
        if (typeof callback === "function") {
          scheduledCallbacks.push(() => {
            callback();
          });
        }
        return 1 as unknown as ReturnType<typeof setTimeout>;
      });
    const clearTimeoutSpy = vi
      .spyOn(globalThis, "clearTimeout")
      .mockImplementation(() => {});

    await persistFrontendTokens({
      access_token: "old-access.jwt",
      refresh_token: "old-refresh.jwt",
      access_token_expires_at: "2026-05-04T12:00:00Z",
      email: "alice@example.com",
    });
    await persistFrontendTokens({
      access_token: "new-access.jwt",
      refresh_token: "new-refresh.jwt",
      access_token_expires_at: "2026-05-04T12:00:00Z",
      email: "bob@example.com",
    });

    scheduledCallbacks[0]?.();
    await flushPromises(3);

    expect(mockFetch).not.toHaveBeenCalled();
    expect(mockSaveStoredAuthTokens).toHaveBeenCalledTimes(2);

    setTimeoutSpy.mockRestore();
    clearTimeoutSpy.mockRestore();
  });

  it("deduplicates concurrent refreshes for the same stored session", async () => {
    vi.useFakeTimers();
    mockInvoke.mockResolvedValue({ api_url: "https://noverterm.noverwork.com" });
    mockLoadStoredAuthTokens.mockResolvedValue({
      access_token: "expired-access.jwt",
      refresh_token: "same-refresh.jwt",
      access_token_expires_at: "2020-01-01T00:00:00Z",
      email: "alice@example.com",
    });
    let resolveRefresh: (response: Response) => void = () => {};
    const refreshResponse = new Promise<Response>((resolve) => {
      resolveRefresh = resolve;
    });
    mockFetch.mockReturnValueOnce(refreshResponse);

    await loadAppSettings();
    const firstRequest = vi.fn().mockResolvedValue("first");
    const secondRequest = vi.fn().mockResolvedValue("second");
    const first = withAuthorizedRetry(firstRequest);
    const second = withAuthorizedRetry(secondRequest);
    await Promise.resolve();

    expect(mockFetch).toHaveBeenCalledTimes(1);
    resolveRefresh(jsonResponse(authResponse("fresh-access.jwt", "fresh-refresh.jwt")));

    await expect(first).resolves.toBe("first");
    await expect(second).resolves.toBe("second");
    expect(firstRequest).toHaveBeenCalledWith("fresh-access.jwt");
    expect(secondRequest).toHaveBeenCalledWith("fresh-access.jwt");
    expect(mockSaveStoredAuthTokens).toHaveBeenCalledOnce();
  });
});
