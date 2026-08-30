import { invoke } from "@tauri-apps/api/core";

import type {
  AppDataMetadata,
  HostGroupRecord,
  Setting,
  SshHostRecord,
  SshKeyRecord,
} from "./types.js";

export async function loadAppDataMetadata(): Promise<AppDataMetadata> {
  const [settings, hostGroups, hosts, keys] = await Promise.all([
    invoke<Setting[]>("get_all_settings"),
    invoke<HostGroupRecord[]>("host_group_list"),
    invoke<SshHostRecord[]>("host_list"),
    invoke<SshKeyRecord[]>("key_list"),
  ]);

  return { settings, host_groups: hostGroups, hosts, keys };
}
