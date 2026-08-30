import { invoke } from "@tauri-apps/api/core";

import type { KeyInput, KeyUpdateInput, SshKeyRecord, SshKeySecret } from "./types.js";

export async function createSshKey(key: KeyInput): Promise<SshKeyRecord> {
  return invoke<SshKeyRecord>("key_create", { key });
}

export async function updateSshKey(
  keyId: string,
  key: KeyUpdateInput,
): Promise<SshKeyRecord> {
  return invoke<SshKeyRecord>("key_update", { id: keyId, key });
}

export async function deleteSshKey(keyId: string): Promise<void> {
  await invoke("key_delete", { id: keyId });
}

export async function revealSshKeySecret(keyId: string): Promise<SshKeySecret> {
  return invoke<SshKeySecret>("key_secret", { id: keyId });
}
