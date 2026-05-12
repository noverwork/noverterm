import { invoke } from "@tauri-apps/api/core";
import { fetch } from "@tauri-apps/plugin-http";

import type { AuthResponse, RefreshRequest } from "./types.js";

import {
  clearStoredAuthTokens,
  loadStoredAuthTokens,
  saveStoredAuthTokens,
  type AuthSessionTokens,
} from "$lib/stores/auth-token-store.js";

let backendBaseUrl: string | null = null;
let refreshTimer: ReturnType<typeof setTimeout> | null = null;
let authSessionVersion = 0;
let activeRefresh: ActiveRefresh | null = null;

const ACCESS_TOKEN_REFRESH_MARGIN_MS = 60_000;
const MIN_AUTO_REFRESH_DELAY_MS = 1_000;

export async function loadAppSettings(): Promise<void> {
  const settings = await invoke<{ api_url: string }>("get_app_settings");
  backendBaseUrl = settings.api_url;
}

interface JsonRequestInit extends Omit<RequestInit, "headers"> {
  headers?: Record<string, string>;
}

interface ActiveRefresh {
  refreshToken: string;
  startedAtVersion: number;
  promise: Promise<AuthSessionTokens>;
}

export class HttpError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

class AuthExpiredError extends Error {}

export function isAuthExpiredError(error: unknown): boolean {
  return error instanceof AuthExpiredError;
}

function buildUrl(path: string): string {
  if (!backendBaseUrl) {
    throw new Error("backend URL not initialized. Call loadAppSettings() first.");
  }

  const baseUrl = backendBaseUrl.replace(/\/$/, "");
  const apiBaseUrl = baseUrl.endsWith("/api") ? baseUrl : `${baseUrl}/api`;
  const apiPath = path.startsWith("/api/") || path === "/api" ? path.slice(4) || "/" : path;
  return `${apiBaseUrl}${apiPath}`;
}

async function readErrorMessage(response: Response): Promise<string> {
  const message = (await response.text()).trim();
  return message || `request failed with ${response.status}`;
}

export async function requestJson<T>(path: string, init: JsonRequestInit = {}): Promise<T> {
  const response = await fetch(buildUrl(path), {
    ...init,
    headers: {
      Accept: "application/json",
      ...(init.body ? { "Content-Type": "application/json" } : {}),
      ...init.headers,
    },
  });

  if (!response.ok) {
    throw new HttpError(response.status, await readErrorMessage(response));
  }

  return (await response.json()) as T;
}

export async function requestNoContent(path: string, init: JsonRequestInit = {}): Promise<void> {
  const response = await fetch(buildUrl(path), {
    ...init,
    headers: {
      ...(init.body ? { "Content-Type": "application/json" } : {}),
      ...init.headers,
    },
  });

  if (!response.ok) {
    throw new HttpError(response.status, await readErrorMessage(response));
  }
}

async function persistFrontendTokens(tokens: AuthSessionTokens): Promise<void> {
  await saveStoredAuthTokens(tokens);
  authSessionVersion += 1;
  scheduleTokenRefresh(tokens, authSessionVersion);
}

async function persistRefreshedTokens(
  tokens: AuthSessionTokens,
  startedAtVersion: number,
): Promise<void> {
  if (authSessionVersion !== startedAtVersion) {
    throw new AuthExpiredError("stale token refresh ignored");
  }

  await saveStoredAuthTokens(tokens);
  if (authSessionVersion !== startedAtVersion) {
    await clearStoredAuthTokens();
    throw new AuthExpiredError("stale token refresh ignored");
  }

  authSessionVersion += 1;
  scheduleTokenRefresh(tokens, authSessionVersion);
}

async function clearFrontendTokens(): Promise<void> {
  clearScheduledRefresh();
  activeRefresh = null;
  authSessionVersion += 1;
  await clearStoredAuthTokens();
}

async function refreshTokens(
  refreshToken: string,
  startedAtVersion: number = authSessionVersion,
): Promise<AuthSessionTokens> {
  if (authSessionVersion !== startedAtVersion) {
    throw new AuthExpiredError("stale token refresh ignored");
  }

  if (
    activeRefresh?.refreshToken === refreshToken &&
    activeRefresh.startedAtVersion === startedAtVersion
  ) {
    return activeRefresh.promise;
  }

  const promise = requestTokenRefresh(refreshToken, startedAtVersion);
  activeRefresh = {
    refreshToken,
    startedAtVersion,
    promise,
  };

  try {
    return await promise;
  } finally {
    if (activeRefresh?.promise === promise) {
      activeRefresh = null;
    }
  }
}

async function requestTokenRefresh(
  refreshToken: string,
  startedAtVersion: number,
): Promise<AuthSessionTokens> {
  const request: RefreshRequest = { refresh_token: refreshToken };
  const response = await requestJson<BackendAuthResponse>("/auth/refresh", {
    method: "POST",
    body: JSON.stringify(request),
  });
  const refreshedTokens = toSessionTokens(response);
  if (authSessionVersion !== startedAtVersion) {
    throw new AuthExpiredError("stale token refresh ignored");
  }

  await persistRefreshedTokens(refreshedTokens, startedAtVersion);
  return refreshedTokens;
}

function clearScheduledRefresh(): void {
  if (refreshTimer !== null) {
    clearTimeout(refreshTimer);
    refreshTimer = null;
  }
}

function accessTokenRefreshDelayMs(tokens: AuthSessionTokens): number {
  const expiresAt = Date.parse(tokens.access_token_expires_at);
  if (Number.isNaN(expiresAt)) {
    return 0;
  }

  return Math.max(expiresAt - Date.now() - ACCESS_TOKEN_REFRESH_MARGIN_MS, 0);
}

function scheduleTokenRefresh(tokens: AuthSessionTokens, expectedVersion: number): void {
  if (authSessionVersion !== expectedVersion) {
    return;
  }

  clearScheduledRefresh();
  const delay = Math.max(accessTokenRefreshDelayMs(tokens), MIN_AUTO_REFRESH_DELAY_MS);
  refreshTimer = setTimeout(() => {
    refreshTimer = null;
    void refreshTokens(tokens.refresh_token, expectedVersion).catch(async (error: unknown) => {
      if (error instanceof HttpError && error.status === 401) {
        await clearFrontendTokens();
      }
    });
  }, delay);
}

async function freshStoredTokens(
  tokens: AuthSessionTokens,
  startedAtVersion: number,
): Promise<AuthSessionTokens> {
  if (authSessionVersion !== startedAtVersion) {
    throw new AuthExpiredError("stale stored tokens ignored");
  }

  if (accessTokenRefreshDelayMs(tokens) > 0) {
    scheduleTokenRefresh(tokens, startedAtVersion);
    return tokens;
  }

  try {
    return await refreshTokens(tokens.refresh_token, startedAtVersion);
  } catch (error) {
    if (error instanceof HttpError && error.status === 401) {
      await clearFrontendTokens();
      throw new AuthExpiredError("session expired");
    }

    throw error;
  }
}

type BackendAuthResponse = AuthResponse;

function toSessionTokens(response: BackendAuthResponse): AuthSessionTokens {
  return {
    access_token: response.access_token,
    refresh_token: response.refresh_token,
    access_token_expires_at: response.access_token_expires_at,
    email: response.email,
  };
}

export async function withAuthorizedRetry<T>(
  request: (accessToken: string) => Promise<T>,
): Promise<T> {
  const startedAtVersion = authSessionVersion;
  const storedTokens = await loadStoredAuthTokens();
  if (authSessionVersion !== startedAtVersion) {
    throw new AuthExpiredError("stale stored tokens ignored");
  }
  if (!storedTokens) {
    throw new Error("not authenticated");
  }
  const currentTokens = await freshStoredTokens(storedTokens, startedAtVersion);
  const currentTokensVersion = authSessionVersion;

  try {
    return await request(currentTokens.access_token);
  } catch (error) {
    if (!(error instanceof HttpError) || error.status !== 401) {
      throw error;
    }
  }

  try {
    const refreshedTokens = await refreshTokens(
      currentTokens.refresh_token,
      currentTokensVersion,
    );
    return await request(refreshedTokens.access_token);
  } catch (error) {
    if (error instanceof HttpError && error.status === 401) {
      await clearFrontendTokens();
      throw new AuthExpiredError("session expired");
    }

    throw error;
  }
}

export async function requestWithAuth<T>(
  path: string,
  accessToken: string,
  init: JsonRequestInit = {},
): Promise<T> {
  return requestJson<T>(path, {
    ...init,
    headers: {
      Authorization: `Bearer ${accessToken}`,
      ...init.headers,
    },
  });
}

export async function requestNoContentWithAuth(
  path: string,
  accessToken: string,
  init: JsonRequestInit = {},
): Promise<void> {
  return requestNoContent(path, {
    ...init,
    headers: {
      Authorization: `Bearer ${accessToken}`,
      ...init.headers,
    },
  });
}

export { clearFrontendTokens, persistFrontendTokens, toSessionTokens };
export type { AuthSessionTokens, BackendAuthResponse };
