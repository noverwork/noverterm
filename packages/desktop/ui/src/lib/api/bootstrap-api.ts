import { requestWithAuth, withAuthorizedRetry, isAuthExpiredError } from "./api-client.js";

import type { Setting, HostGroupRecord, SshHostRecord, SshKeyRecord, BootstrapMetadata } from "./types.js";

export async function loadBootstrapMetadataFromBackend(): Promise<BootstrapMetadata> {
  try {
    return await withAuthorizedRetry(async (accessToken) => {
      const [settings, hostGroups, hosts, keys] = await Promise.all([
        requestWithAuth<Setting[]>("/bootstrap/settings", accessToken),
        requestWithAuth<HostGroupRecord[]>("/bootstrap/host-groups", accessToken),
        requestWithAuth<SshHostRecord[]>("/bootstrap/hosts", accessToken),
        requestWithAuth<SshKeyRecord[]>("/bootstrap/keys", accessToken),
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
