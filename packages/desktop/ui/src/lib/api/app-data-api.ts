import {
  isAuthExpiredError,
  requestWithAuth,
  withAuthorizedRetry,
} from "./api-client.js";

import type {
  AppDataMetadata,
  HostGroupRecord,
  Setting,
  SshHostRecord,
  SshKeyRecord,
} from "./types.js";

export async function loadAppDataMetadataFromBackend(): Promise<AppDataMetadata> {
  try {
    return await withAuthorizedRetry(async (accessToken) => {
      const [settings, hostGroups, hosts, keys] = await Promise.all([
        requestWithAuth<Setting[]>("/settings", accessToken),
        requestWithAuth<HostGroupRecord[]>("/host-groups", accessToken),
        requestWithAuth<SshHostRecord[]>("/hosts", accessToken),
        requestWithAuth<SshKeyRecord[]>("/keys", accessToken),
      ]);

      return { settings, host_groups: hostGroups, hosts, keys };
    });
  } catch (error) {
    if (isAuthExpiredError(error)) {
      throw new Error("session expired", { cause: error });
    }
    throw error;
  }
}
