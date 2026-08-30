import { mutationOptions } from "@tanstack/svelte-query";

import { upsertSetting } from "$lib/api/settings-api.js";
import type { Setting } from "$lib/api/types.js";
import { mutationKeys } from "./query-keys.js";

export function upsertSettingMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.upsertSetting,
    mutationFn: (setting: Setting): Promise<Setting> => upsertSetting(setting),
  });
}
