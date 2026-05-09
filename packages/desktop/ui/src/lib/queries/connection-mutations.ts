import { mutationOptions } from "@tanstack/svelte-query";

import {
  createBackendHostGroup,
  deleteBackendConnection,
  deleteBackendHostGroup,
  saveBackendConnection,
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
      saveBackendConnection(connection),
  });
}

export function deleteConnectionMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.deleteConnection,
    mutationFn: (connection: ConnectionConfig): Promise<void> =>
      deleteBackendConnection(connection),
  });
}

export function createHostGroupMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.createHostGroup,
    mutationFn: createBackendHostGroup,
  });
}

export function deleteHostGroupMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.deleteHostGroup,
    mutationFn: (group: HostGroupRecord): Promise<void> => deleteBackendHostGroup(group),
  });
}
