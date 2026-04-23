import {
  HttpError,
  clearFrontendTokens,
  persistFrontendTokens,
  requestNoContent,
  requestJson,
  requestWithAuth,
  toSessionTokens,
  withAuthorizedRetry,
  isAuthExpiredError,
  type BackendAuthResponse,
} from "./api-client.js";

export interface AuthBootstrapStatus {
  email: string;
  bootstrap_message: string;
}

async function bootstrapSmoke(accessToken: string): Promise<AuthBootstrapStatus> {
  return requestWithAuth<AuthBootstrapStatus>("/bootstrap/smoke", accessToken);
}

export async function registerToBackend(
  email: string,
  password: string,
): Promise<AuthBootstrapStatus> {
  const authResponse = await requestJson<BackendAuthResponse>("/auth/register", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
  const tokens = toSessionTokens(authResponse);
  await persistFrontendTokens(tokens);

  try {
    return await withAuthorizedRetry(async (accessToken) => bootstrapSmoke(accessToken));
  } catch (error) {
    await clearFrontendTokens();
    throw error;
  }
}

export async function loginToBackend(
  email: string,
  password: string,
): Promise<AuthBootstrapStatus> {
  const authResponse = await requestJson<BackendAuthResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
  const tokens = toSessionTokens(authResponse);
  await persistFrontendTokens(tokens);

  try {
    return await withAuthorizedRetry(async (accessToken) => bootstrapSmoke(accessToken));
  } catch (error) {
    await clearFrontendTokens();
    throw error;
  }
}

export async function restoreBackendSession(): Promise<AuthBootstrapStatus | null> {
  const { loadStoredAuthTokens } = await import("$lib/stores/auth-token-store.js");
  const storedTokens = await loadStoredAuthTokens();
  if (!storedTokens) {
    return null;
  }

  try {
    return await withAuthorizedRetry(async (accessToken) => bootstrapSmoke(accessToken));
  } catch (error) {
    if (isAuthExpiredError(error)) {
      return null;
    }
    throw error;
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
