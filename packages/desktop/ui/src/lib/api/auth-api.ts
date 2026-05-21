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
import type {
  ForgotPasswordRequest,
  LoginRequest,
  LogoutRequest,
  RegisterRequest,
  ResetPasswordRequest,
} from "./types.js";
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
  const request: RegisterRequest = { email, password };
  const authResponse = await requestJson<BackendAuthResponse>("/auth/register", {
    method: "POST",
    body: JSON.stringify(request),
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
  const request: LoginRequest = { email, password };
  const authResponse = await requestJson<BackendAuthResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify(request),
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
  const request: ForgotPasswordRequest = { email };
  await requestNoContent("/auth/forgot-password", {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export async function resetPassword(token: string, password: string): Promise<void> {
  const request: ResetPasswordRequest = { token, password };
  await requestNoContent("/auth/reset-password", {
    method: "POST",
    body: JSON.stringify(request),
  });
}

export async function restoreBackendSession(): Promise<AuthSessionStatus | null> {
  const { loadStoredAuthTokens } = await import("$lib/stores/auth-token-store.js");
  const storedTokens = await loadStoredAuthTokens();
  if (!storedTokens) {
    return null;
  }

  try {
    const status = await Promise.race([
      withAuthorizedRetry(async (accessToken) => checkSession(accessToken)),
      new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error("restore session timed out")), 8_000),
      ),
    ]);
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
      const request: LogoutRequest = { refresh_token: storedTokens.refresh_token };
      await requestNoContent("/auth/logout", {
        method: "POST",
        body: JSON.stringify(request),
      });
    }
  } catch (error) {
    if (!(error instanceof HttpError) || (error.status !== 400 && error.status !== 401)) {
      throw error;
    }
  }

  await clearFrontendTokens();
}
