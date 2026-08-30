import { invoke } from "@tauri-apps/api/core";

import type { HostGroupRecord, SshHostRecord } from "./types.js";
import type {
  SaveConnectionInput,
  ConnectionConfig,
} from "$lib/app-data-types.js";

function trimOptional(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

export async function saveConnection(
  connection: SaveConnectionInput,
): Promise<SshHostRecord> {
  return invoke<SshHostRecord>("host_save", {
    connection: {
      id: connection.id ?? null,
      name: connection.name,
      host: connection.host,
      port: connection.port,
      username: connection.username,
      group_id: trimOptional(connection.groupId),
      password:
        trimOptional(connection.password) ??
        trimOptional(connection.preservedPassword),
      private_key: trimOptional(connection.privateKey),
      passphrase: trimOptional(connection.passphrase),
      key_name: trimOptional(connection.keyName),
      existing_key_id: trimOptional(connection.existingKeyId),
    },
  });
}

export async function createHostGroup(name: string): Promise<HostGroupRecord> {
  return invoke<HostGroupRecord>("host_group_create", { name });
}

export async function deleteHostGroup(group: HostGroupRecord): Promise<void> {
  await invoke("host_group_delete", { id: group.id });
}

export async function deleteConnection(
  connection: ConnectionConfig,
): Promise<void> {
  await invoke("host_delete", {
    id: connection.id,
    sshKeyId: connection.sshKeyId,
  });
}
