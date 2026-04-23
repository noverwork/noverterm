import {
  HttpError,
  requestWithAuth,
  requestNoContentWithAuth,
  withAuthorizedRetry,
} from "./api-client.js";

import type { SshHostRecord, SshKeyRecord } from "./types.js";
import type { SaveConnectionInput, ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";

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

function trimOptional(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
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
        name: connection.keyName || `${connection.name} key`,
        kind: "inline",
        fingerprint: null,
        encrypted_private_key: trimmedPrivateKey,
        encrypted_passphrase: trimmedPassphrase,
      };

      const key = connection.existingKeyId
        ? await requestWithAuth<SshKeyRecord>(
            `/bootstrap/keys/${connection.existingKeyId}`,
            accessToken,
            { method: "PUT", body: JSON.stringify(keyInput) },
          )
        : await requestWithAuth<SshKeyRecord>("/bootstrap/keys", accessToken, {
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
      ? requestWithAuth<SshHostRecord>(`/bootstrap/hosts/${connection.id}`, accessToken, {
          method: "PUT",
          body: JSON.stringify(hostInput),
        })
      : requestWithAuth<SshHostRecord>("/bootstrap/hosts", accessToken, {
          method: "POST",
          body: JSON.stringify(hostInput),
        });
  });
}

export async function deleteBackendConnection(connection: ConnectionConfig): Promise<void> {
  await withAuthorizedRetry(async (accessToken) => {
    await requestNoContentWithAuth(
      `/bootstrap/hosts/${encodeURIComponent(connection.id)}`,
      accessToken,
      { method: "DELETE" },
    );

    if (connection.sshKeyId) {
      try {
        await requestNoContentWithAuth(
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
