import {
  requestWithAuth,
  requestNoContentWithAuth,
  withAuthorizedRetry,
} from "./api-client.js";

import type { SshKeyRecord, SshKeySecret, KeyCreateRequest, KeyUpdateRequest } from "./types.js";
import { decryptSecret, encryptSecret } from "$lib/crypto/vault.js";

async function encryptCreateRequest(key: KeyCreateRequest): Promise<KeyCreateRequest> {
  const encryptedPrivateKey = await encryptSecret(key.encrypted_private_key.trim());
  return {
    ...key,
    encrypted_private_key: encryptedPrivateKey ?? "",
    encrypted_passphrase: await encryptSecret(key.encrypted_passphrase?.trim() || null),
  };
}

async function encryptUpdateRequest(key: KeyUpdateRequest): Promise<KeyUpdateRequest> {
  const trimmedPrivateKey = key.encrypted_private_key?.trim();
  if (!trimmedPrivateKey) {
    return key;
  }

  const encryptedPrivateKey = await encryptSecret(trimmedPrivateKey);
  return {
    ...key,
    encrypted_private_key: encryptedPrivateKey ?? "",
    encrypted_passphrase: await encryptSecret(key.encrypted_passphrase?.trim() || null),
  };
}

async function decryptRevealedSecret(secret: SshKeySecret): Promise<SshKeySecret> {
  return {
    private_key: (await decryptSecret(secret.private_key)) ?? "",
    passphrase: await decryptSecret(secret.passphrase),
  };
}

export async function createSshKey(key: KeyCreateRequest): Promise<SshKeyRecord> {
  const encryptedKey = await encryptCreateRequest(key);
  return withAuthorizedRetry(async (accessToken) =>
    requestWithAuth<SshKeyRecord>("/bootstrap/keys", accessToken, {
      method: "POST",
      body: JSON.stringify(encryptedKey),
    }),
  );
}

export async function updateSshKey(keyId: string, key: KeyUpdateRequest): Promise<SshKeyRecord> {
  const encryptedKey = await encryptUpdateRequest(key);
  return withAuthorizedRetry(async (accessToken) =>
    requestWithAuth<SshKeyRecord>(`/bootstrap/keys/${encodeURIComponent(keyId)}`, accessToken, {
      method: "PUT",
      body: JSON.stringify(encryptedKey),
    }),
  );
}

export async function deleteSshKey(keyId: string): Promise<void> {
  await withAuthorizedRetry(async (accessToken) =>
    requestNoContentWithAuth(`/bootstrap/keys/${encodeURIComponent(keyId)}`, accessToken, {
      method: "DELETE",
    }),
  );
}

export async function revealSshKeySecret(keyId: string): Promise<SshKeySecret> {
  const secret = await withAuthorizedRetry(async (accessToken) =>
    requestWithAuth<SshKeySecret>(`/bootstrap/keys/${encodeURIComponent(keyId)}/secret`, accessToken),
  );
  return await decryptRevealedSecret(secret);
}
