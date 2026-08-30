import { mutationOptions } from "@tanstack/svelte-query";

import {
  createHostGroup,
  deleteConnection,
  deleteHostGroup,
  saveConnection,
} from "$lib/api/connections-api.js";
import type { HostGroupRecord, SshHostRecord } from "$lib/api/types.js";
import type {
  ConnectionConfig,
  SaveConnectionInput,
} from "$lib/app-data-types.js";
import { mutationKeys } from "./query-keys.js";

export function saveConnectionMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.saveConnection,
    mutationFn: (connection: SaveConnectionInput): Promise<SshHostRecord> =>
      saveConnection(connection),
  });
}

export function deleteConnectionMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.deleteConnection,
    mutationFn: (connection: ConnectionConfig): Promise<void> =>
      deleteConnection(connection),
  });
}

export function createHostGroupMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.createHostGroup,
    mutationFn: createHostGroup,
  });
}

export function deleteHostGroupMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.deleteHostGroup,
    mutationFn: (group: HostGroupRecord): Promise<void> => deleteHostGroup(group),
  });
}
