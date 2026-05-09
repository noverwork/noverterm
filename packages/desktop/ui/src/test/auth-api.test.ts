import { beforeEach, describe, expect, it, vi } from "vitest";

const mockClearFrontendTokens = vi.fn();
const mockPersistFrontendTokens = vi.fn();
const mockRequestJson = vi.fn();
const mockRequestWithAuth = vi.fn();
const mockToSessionTokens = vi.fn();
const mockWithAuthorizedRetry = vi.fn();
const mockSetActiveVaultEmail = vi.fn();
const mockUnlockVaultWithPassword = vi.fn();

vi.mock("$lib/api/api-client.js", () => ({
  HttpError: class HttpError extends Error {
    readonly status: number;

    constructor(status: number, message: string) {
      super(message);
      this.status = status;
    }
  },
  clearFrontendTokens: () => mockClearFrontendTokens(),
  persistFrontendTokens: (tokens: unknown) => mockPersistFrontendTokens(tokens),
  requestNoContent: vi.fn(),
  requestJson: (...args: unknown[]) => mockRequestJson(...args),
  requestWithAuth: (...args: unknown[]) => mockRequestWithAuth(...args),
  toSessionTokens: (response: unknown) => mockToSessionTokens(response),
  withAuthorizedRetry: (request: (accessToken: string) => Promise<unknown>) =>
    mockWithAuthorizedRetry(request),
}));

vi.mock("$lib/crypto/vault.js", () => ({
  setActiveVaultEmail: (...args: unknown[]) => mockSetActiveVaultEmail(...args),
  unlockVaultWithPassword: (...args: unknown[]) => mockUnlockVaultWithPassword(...args),
}));

import { loginToBackend, registerToBackend } from "$lib/api/auth-api.js";

const backendAuthResponse = {
  access_token: "access.jwt",
  refresh_token: "refresh.jwt",
  access_token_expires_at: "2026-05-04T12:00:00Z",
  email: "alice@example.com",
};

const sessionTokens = {
  access_token: "access.jwt",
  refresh_token: "refresh.jwt",
  access_token_expires_at: "2026-05-04T12:00:00Z",
  email: "alice@example.com",
};

describe("auth API token cleanup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockRequestJson.mockResolvedValue(backendAuthResponse);
    mockToSessionTokens.mockReturnValue(sessionTokens);
    mockPersistFrontendTokens.mockResolvedValue(undefined);
    mockWithAuthorizedRetry.mockResolvedValue({
      email: "alice@example.com",
        status: "ok",
    });
  });

  it("clears persisted tokens when login fails after token persistence", async () => {
    mockUnlockVaultWithPassword.mockRejectedValue(new Error("vault failed"));

    await expect(loginToBackend("alice@example.com", "password")).rejects.toThrow(
      "vault failed",
    );

    expect(mockPersistFrontendTokens).toHaveBeenCalledWith(sessionTokens);
    expect(mockClearFrontendTokens).toHaveBeenCalledOnce();
  });

  it("clears persisted tokens when register fails after token persistence", async () => {
    mockUnlockVaultWithPassword.mockRejectedValue(new Error("vault failed"));

    await expect(registerToBackend("alice@example.com", "password")).rejects.toThrow(
      "vault failed",
    );

    expect(mockPersistFrontendTokens).toHaveBeenCalledWith(sessionTokens);
    expect(mockClearFrontendTokens).toHaveBeenCalledOnce();
  });
});
