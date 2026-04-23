import {
  requestWithAuth,
  requestNoContentWithAuth,
  withAuthorizedRetry,
} from "./api-client.js";

import type { SshKeyRecord, KeyCreateRequest, KeyUpdateRequest } from "./types.js";

export async function createSshKey(key: KeyCreateRequest): Promise<SshKeyRecord> {
  return withAuthorizedRetry(async (accessToken) =>
    requestWithAuth<SshKeyRecord>("/bootstrap/keys", accessToken, {
      method: "POST",
      body: JSON.stringify(key),
    }),
  );
}

export async function updateSshKey(keyId: string, key: KeyUpdateRequest): Promise<SshKeyRecord> {
  return withAuthorizedRetry(async (accessToken) =>
    requestWithAuth<SshKeyRecord>(`/bootstrap/keys/${encodeURIComponent(keyId)}`, accessToken, {
      method: "PUT",
      body: JSON.stringify(key),
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
