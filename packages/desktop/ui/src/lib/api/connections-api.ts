import {
  HttpError,
  requestWithAuth,
  requestNoContentWithAuth,
  withAuthorizedRetry,
} from "./api-client.js";

import type { HostGroupRecord, SshHostRecord, SshKeyRecord } from "./types.js";
import { encryptSecret } from "$lib/crypto/vault.js";
import type {
  SaveConnectionInput,
  ConnectionConfig,
} from "$lib/app-data-types.js";

interface HostWriteRequest {
  name: string;
  host: string;
  port: number;
  username: string;
  ssh_key_id: string | null;
  encrypted_password: string | null;
  group_id: string | null;
}

interface HostGroupWriteRequest {
  name: string;
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
  const encryptedPassword = trimmedPassword
    ? await encryptSecret(trimmedPassword)
    : (connection.preservedEncryptedPassword ?? null);
  const encryptedPrivateKey = await encryptSecret(trimmedPrivateKey);
  const encryptedPassphrase = await encryptSecret(trimmedPassphrase);

  return withAuthorizedRetry(async (accessToken) => {
    let sshKeyId =
      connection.existingKeyId && connection.existingKeyId.trim()
        ? connection.existingKeyId
        : null;

    if (encryptedPrivateKey) {
      const keyInput: KeyWriteRequest = {
        name: connection.keyName || `${connection.name} key`,
        kind: "inline",
        fingerprint: null,
        encrypted_private_key: encryptedPrivateKey,
        encrypted_passphrase: encryptedPassphrase,
      };

      const key = connection.existingKeyId
        ? await requestWithAuth<SshKeyRecord>(
            `/keys/${connection.existingKeyId}`,
            accessToken,
            { method: "PUT", body: JSON.stringify(keyInput) },
          )
        : await requestWithAuth<SshKeyRecord>("/keys", accessToken, {
            method: "POST",
            body: JSON.stringify(keyInput),
          });

      sshKeyId = key.id;
    }

    const hostInput: HostWriteRequest = {
      name: connection.name,
      host: connection.host,
      port: connection.port,
      username: connection.username,
      ssh_key_id: sshKeyId,
      encrypted_password: encryptedPassword,
      group_id: trimOptional(connection.groupId),
    };

    return connection.id
      ? requestWithAuth<SshHostRecord>(
          `/hosts/${connection.id}`,
          accessToken,
          {
            method: "PUT",
            body: JSON.stringify(hostInput),
          },
        )
      : requestWithAuth<SshHostRecord>("/hosts", accessToken, {
          method: "POST",
          body: JSON.stringify(hostInput),
        });
  });
}

export async function createBackendHostGroup(name: string): Promise<HostGroupRecord> {
  return withAuthorizedRetry(async (accessToken) => {
    const input: HostGroupWriteRequest = { name };
    return await requestWithAuth<HostGroupRecord>("/host-groups", accessToken, {
      method: "POST",
      body: JSON.stringify(input),
    });
  });
}

export async function deleteBackendHostGroup(group: HostGroupRecord): Promise<void> {
  await withAuthorizedRetry(async (accessToken) => {
    await requestNoContentWithAuth(
      `/host-groups/${encodeURIComponent(group.id)}`,
      accessToken,
      { method: "DELETE" },
    );
  });
}

export async function deleteBackendConnection(
  connection: ConnectionConfig,
): Promise<void> {
  await withAuthorizedRetry(async (accessToken) => {
    await requestNoContentWithAuth(
      `/hosts/${encodeURIComponent(connection.id)}`,
      accessToken,
      { method: "DELETE" },
    );

    if (connection.sshKeyId) {
      try {
        await requestNoContentWithAuth(
          `/keys/${encodeURIComponent(connection.sshKeyId)}`,
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
