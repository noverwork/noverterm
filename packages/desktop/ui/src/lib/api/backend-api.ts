import { invoke } from "@tauri-apps/api/core";
import { fetch } from "@tauri-apps/plugin-http";

import type { ConnectionConfig, SaveConnectionInput } from "$lib/stores/bootstrap.svelte.js";
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

export interface AuthBootstrapStatus {
  username: string;
  bootstrap_message: string;
}

export interface Setting {
  key: string;
  value: string;
}

export interface SshHostRecord {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_mode: string;
  ssh_key_id: string | null;
}

export interface SshKeyRecord {
  id: string;
  name: string;
  kind: string;
  fingerprint: string | null;
}

export interface BootstrapMetadata {
  settings: Setting[];
  hosts: SshHostRecord[];
  keys: SshKeyRecord[];
}

interface BackendAuthResponse {
  access_token: string;
  refresh_token: string;
  access_token_expires_at: string;
  username: string;
}

interface HostWriteRequest {
  name: string;
  host: string;
  port: number;
  username: string;
  auth_mode: string;
  ssh_key_id: string | null;
  encrypted_password: string | null;
}

interface KeyWriteRequest {
  name: string;
  kind: string;
  fingerprint: string | null;
  encrypted_private_key: string;
  encrypted_passphrase: string | null;
}

interface JsonRequestInit extends Omit<RequestInit, "headers"> {
  headers?: Record<string, string>;
}

interface ConnectMaterial {
  issuance_id: string;
  host_id: string;
  host: string;
  port: number;
  username: string;
  issued_for_username: string;
  issued_for_session_id: string;
  expires_at: string;
  auth:
    | { kind: "password"; password: string }
    | { kind: "public_key"; private_key: string; passphrase: string | null }
    | {
        kind: "public_key_and_password";
        private_key: string;
        passphrase: string | null;
        password: string;
      };
}

export interface IssuedConnectionMaterial {
  host: string;
  port: number;
  username: string;
  password?: string;
  privateKey?: string;
  passphrase?: string;
}

class HttpError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

class AuthExpiredError extends Error {}

function buildUrl(path: string): string {
  if (!backendBaseUrl) {
    throw new Error("backend URL not initialized. Call loadAppSettings() first.");
  }
  return `${backendBaseUrl.replace(/\/$/, "")}${path}`;
}

function toSessionTokens(response: BackendAuthResponse): AuthSessionTokens {
  return {
    access_token: response.access_token,
    refresh_token: response.refresh_token,
    username: response.username,
  };
}

function trimOptional(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

async function readErrorMessage(response: Response): Promise<string> {
  const message = (await response.text()).trim();
  return message || `request failed with ${response.status}`;
}

async function requestJson<T>(path: string, init: JsonRequestInit = {}): Promise<T> {
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

async function requestNoContent(path: string, init: JsonRequestInit = {}): Promise<void> {
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

async function requestWithAccessToken<T>(
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

async function requestNoContentWithAccessToken(
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

async function withAuthorizedRetry<T>(
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

async function bootstrapSmoke(accessToken: string): Promise<AuthBootstrapStatus> {
  return requestWithAccessToken<AuthBootstrapStatus>("/bootstrap/smoke", accessToken);
}

export async function registerToBackend(
  username: string,
  password: string,
): Promise<AuthBootstrapStatus> {
  const authResponse = await requestJson<BackendAuthResponse>("/auth/register", {
    method: "POST",
    body: JSON.stringify({ username, password }),
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
  username: string,
  password: string,
): Promise<AuthBootstrapStatus> {
  const authResponse = await requestJson<BackendAuthResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify({ username, password }),
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
  const storedTokens = await loadStoredAuthTokens();
  if (!storedTokens) {
    return null;
  }

  try {
    return await withAuthorizedRetry(async (accessToken) => bootstrapSmoke(accessToken));
  } catch (error) {
    if (error instanceof AuthExpiredError) {
      return null;
    }

    throw error;
  }
}

export async function logoutFromBackend(): Promise<void> {
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

export async function loadBootstrapMetadataFromBackend(): Promise<BootstrapMetadata> {
  try {
    return await withAuthorizedRetry(async (accessToken) => {
      const [settings, hosts, keys] = await Promise.all([
        requestWithAccessToken<Setting[]>("/bootstrap/settings", accessToken),
        requestWithAccessToken<SshHostRecord[]>("/bootstrap/hosts", accessToken),
        requestWithAccessToken<SshKeyRecord[]>("/bootstrap/keys", accessToken),
      ]);

      return { settings, hosts, keys };
    });
  } catch (error) {
    if (error instanceof AuthExpiredError) {
      throw new Error("session expired");
    }

    throw error;
  }
}

export async function upsertBackendSetting(setting: Setting): Promise<Setting> {
  return withAuthorizedRetry(async (accessToken) => {
    try {
      return await requestWithAccessToken<Setting>(
        `/bootstrap/settings/${encodeURIComponent(setting.key)}`,
        accessToken,
        {
          method: "PUT",
          body: JSON.stringify(setting),
        },
      );
    } catch (error) {
      if (!(error instanceof HttpError) || error.status !== 404) {
        throw error;
      }

      return requestWithAccessToken<Setting>("/bootstrap/settings", accessToken, {
        method: "POST",
        body: JSON.stringify(setting),
      });
    }
  });
}

export async function saveBackendConnection(
  connection: SaveConnectionInput,
): Promise<SshHostRecord> {
  const trimmedPassword = trimOptional(connection.password);
  const trimmedPrivateKey = trimOptional(connection.privateKey);
  const trimmedPassphrase = trimOptional(connection.passphrase);

  return withAuthorizedRetry(async (accessToken) => {
    let sshKeyId = connection.existingKeyId ?? null;

    if (trimmedPrivateKey) {
      const keyInput: KeyWriteRequest = {
        name: `${connection.name} key`,
        kind: "inline",
        fingerprint: null,
        encrypted_private_key: trimmedPrivateKey,
        encrypted_passphrase: trimmedPassphrase,
      };

      const key = connection.existingKeyId
        ? await requestWithAccessToken<SshKeyRecord>(
            `/bootstrap/keys/${connection.existingKeyId}`,
            accessToken,
            {
              method: "PUT",
              body: JSON.stringify(keyInput),
            },
          )
        : await requestWithAccessToken<SshKeyRecord>("/bootstrap/keys", accessToken, {
            method: "POST",
            body: JSON.stringify(keyInput),
          });

      sshKeyId = key.id;
    }

    if (!trimmedPassword && !sshKeyId) {
      throw new Error("password or private key is required");
    }

    const authMode = sshKeyId && trimmedPassword
      ? "publickey_password"
      : sshKeyId
        ? "publickey"
        : "password";

    const hostInput: HostWriteRequest = {
      name: connection.name,
      host: connection.host,
      port: connection.port,
      username: connection.username,
      auth_mode: authMode,
      ssh_key_id: sshKeyId,
      encrypted_password: trimmedPassword,
    };

    return connection.id
      ? requestWithAccessToken<SshHostRecord>(
          `/bootstrap/hosts/${connection.id}`,
          accessToken,
          {
            method: "PUT",
            body: JSON.stringify(hostInput),
          },
        )
      : requestWithAccessToken<SshHostRecord>("/bootstrap/hosts", accessToken, {
          method: "POST",
          body: JSON.stringify(hostInput),
        });
  });
}

export async function deleteBackendConnection(connection: ConnectionConfig): Promise<void> {
  await withAuthorizedRetry(async (accessToken) => {
    await requestNoContentWithAccessToken(
      `/bootstrap/hosts/${encodeURIComponent(connection.id)}`,
      accessToken,
      { method: "DELETE" },
    );

    if (connection.sshKeyId) {
      try {
        await requestNoContentWithAccessToken(
          `/bootstrap/keys/${encodeURIComponent(connection.sshKeyId)}`,
          accessToken,
          { method: "DELETE" },
        );
      } catch (error) {
        if (!(error instanceof HttpError) || error.status !== 404) {
          throw error;
        }
      }
    }
  });
}

export async function issueBackendConnectionMaterial(
  connectionId: string,
): Promise<IssuedConnectionMaterial> {
  const material = await withAuthorizedRetry((accessToken) =>
    requestWithAccessToken<ConnectMaterial>(
      `/bootstrap/connect/${encodeURIComponent(connectionId)}/issue`,
      accessToken,
      { method: "POST" },
    ),
  );

  switch (material.auth.kind) {
    case "password":
      return {
        host: material.host,
        port: material.port,
        username: material.username,
        password: material.auth.password,
      };
    case "public_key":
      return {
        host: material.host,
        port: material.port,
        username: material.username,
        privateKey: material.auth.private_key,
        passphrase: material.auth.passphrase ?? undefined,
      };
    case "public_key_and_password":
      return {
        host: material.host,
        port: material.port,
        username: material.username,
        password: material.auth.password,
        privateKey: material.auth.private_key,
        passphrase: material.auth.passphrase ?? undefined,
      };
  }
}

export interface BootstrapApi {
  restore(): Promise<AuthBootstrapStatus | null>;
  register(username: string, password: string): Promise<AuthBootstrapStatus>;
  login(username: string, password: string): Promise<AuthBootstrapStatus>;
  logout(): Promise<void>;
  loadBootstrapMetadata(): Promise<BootstrapMetadata>;
  saveConnection(connection: SaveConnectionInput): Promise<SshHostRecord>;
  deleteConnection(connection: ConnectionConfig): Promise<void>;
  saveSetting(setting: Setting): Promise<Setting>;
  issueConnectionMaterial(connectionId: string): Promise<IssuedConnectionMaterial>;
}

export const backendApi: BootstrapApi = {
  restore: restoreBackendSession,
  register: registerToBackend,
  login: loginToBackend,
  logout: logoutFromBackend,
  loadBootstrapMetadata: loadBootstrapMetadataFromBackend,
  saveConnection: saveBackendConnection,
  deleteConnection: deleteBackendConnection,
  saveSetting: upsertBackendSetting,
  issueConnectionMaterial: issueBackendConnectionMaterial,
};
