import { invoke } from "@tauri-apps/api/core";
import { fetch } from "@tauri-apps/plugin-http";

import {
  clearStoredAuthTokens,
  loadStoredAuthTokens,
  saveStoredAuthTokens,
  type AuthSessionTokens,
} from "$lib/stores/auth-token-store.js";

let backendBaseUrl: string | null = null;

export async function loadAppSettings(): Promise<void> {
  const settings = await invoke<{ api_url: string }>("get_app_settings");
  backendBaseUrl = settings.api_url;
}

interface JsonRequestInit extends Omit<RequestInit, "headers"> {
  headers?: Record<string, string>;
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
  return `${backendBaseUrl.replace(/\/$/, "")}${path}`;
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
}

async function clearFrontendTokens(): Promise<void> {
  await clearStoredAuthTokens();
}

async function refreshTokens(refreshToken: string): Promise<AuthSessionTokens> {
  const response = await requestJson<BackendAuthResponse>("/auth/refresh", {
    method: "POST",
    body: JSON.stringify({ refresh_token: refreshToken }),
  });
  const refreshedTokens = toSessionTokens(response);
  await persistFrontendTokens(refreshedTokens);
  return refreshedTokens;
}

interface BackendAuthResponse {
  access_token: string;
  refresh_token: string;
  access_token_expires_at: string;
  email: string;
}

function toSessionTokens(response: BackendAuthResponse): AuthSessionTokens {
  return {
    access_token: response.access_token,
    refresh_token: response.refresh_token,
    email: response.email,
  };
}

export async function withAuthorizedRetry<T>(
  request: (accessToken: string) => Promise<T>,
): Promise<T> {
  const storedTokens = await loadStoredAuthTokens();
  if (!storedTokens) {
    throw new Error("not authenticated");
  }

  try {
    return await request(storedTokens.access_token);
  } catch (error) {
    if (!(error instanceof HttpError) || error.status !== 401) {
      throw error;
    }
  }

  try {
    const refreshedTokens = await refreshTokens(storedTokens.refresh_token);
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
