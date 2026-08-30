import { invoke } from "@tauri-apps/api/core";

import type { Setting } from "./types.js";

export async function upsertSetting(setting: Setting): Promise<Setting> {
  return invoke<Setting>("set_setting", { setting });
}
