import { mutationOptions } from "@tanstack/svelte-query";

import { upsertBackendSetting } from "$lib/api/settings-api.js";
import type { Setting } from "$lib/api/types.js";
import { mutationKeys } from "./query-keys.js";

export function upsertSettingMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.upsertSetting,
    mutationFn: (setting: Setting): Promise<Setting> => upsertBackendSetting(setting),
  });
}
