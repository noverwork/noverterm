import {
  HttpError,
  clearFrontendTokens,
  persistFrontendTokens,
  requestNoContent,
  requestJson,
  requestWithAuth,
  toSessionTokens,
  withAuthorizedRetry,
  type BackendAuthResponse,
} from "./api-client.js";
import { setActiveVaultEmail, unlockVaultWithPassword } from "$lib/crypto/vault.js";

export interface AuthSessionStatus {
  email: string;
}

async function checkSession(accessToken: string): Promise<AuthSessionStatus> {
  return requestWithAuth<AuthSessionStatus>("/smoke", accessToken);
}

export async function registerToBackend(
  email: string,
  password: string,
): Promise<AuthSessionStatus> {
  const authResponse = await requestJson<BackendAuthResponse>("/auth/register", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
  const tokens = toSessionTokens(authResponse);

  try {
    await persistFrontendTokens(tokens);
    await unlockVaultWithPassword(email, password);
    return await withAuthorizedRetry(async (accessToken) => checkSession(accessToken));
  } catch (error) {
    await clearFrontendTokens();
    throw error;
  }
}

export async function loginToBackend(
  email: string,
  password: string,
): Promise<AuthSessionStatus> {
  const authResponse = await requestJson<BackendAuthResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
  const tokens = toSessionTokens(authResponse);

  try {
    await persistFrontendTokens(tokens);
    await unlockVaultWithPassword(email, password);
    return await withAuthorizedRetry(async (accessToken) => checkSession(accessToken));
  } catch (error) {
    await clearFrontendTokens();
    throw error;
  }
}

export async function requestPasswordReset(email: string): Promise<void> {
  await requestNoContent("/auth/forgot-password", {
    method: "POST",
    body: JSON.stringify({ email }),
  });
}

export async function resetPassword(token: string, password: string): Promise<void> {
  await requestNoContent("/auth/reset-password", {
    method: "POST",
    body: JSON.stringify({ token, password }),
  });
}

export async function restoreBackendSession(): Promise<AuthSessionStatus | null> {
  const { loadStoredAuthTokens } = await import("$lib/stores/auth-token-store.js");
  const storedTokens = await loadStoredAuthTokens();
  if (!storedTokens) {
    return null;
  }

  try {
    const status = await withAuthorizedRetry(async (accessToken) => checkSession(accessToken));
    setActiveVaultEmail(status.email);
    return status;
  } catch {
    return null;
  }
}

export async function logoutFromBackend(): Promise<void> {
  const { loadStoredAuthTokens } = await import("$lib/stores/auth-token-store.js");
  const storedTokens = await loadStoredAuthTokens();

  try {
    if (storedTokens) {
      await requestNoContent("/auth/logout", {
        method: "POST",
        body: JSON.stringify({ refresh_token: storedTokens.refresh_token }),
      });
    }
  } catch (error) {
    if (!(error instanceof HttpError) || (error.status !== 400 && error.status !== 401)) {
      throw error;
    }
  }

  await clearFrontendTokens();
}
