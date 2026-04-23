import { HttpError, requestWithAuth, withAuthorizedRetry } from "./api-client.js";

import type { Setting } from "./types.js";

export async function upsertBackendSetting(setting: Setting): Promise<Setting> {
  return withAuthorizedRetry(async (accessToken) => {
    try {
      return await requestWithAuth<Setting>(
        `/bootstrap/settings/${encodeURIComponent(setting.key)}`,
        accessToken,
        { method: "PUT", body: JSON.stringify(setting) },
      );
    } catch (error) {
      if (!(error instanceof HttpError) || error.status !== 404) {
        throw error;
      }

      return requestWithAuth<Setting>("/bootstrap/settings", accessToken, {
        method: "POST",
        body: JSON.stringify(setting),
      });
    }
  });
}
