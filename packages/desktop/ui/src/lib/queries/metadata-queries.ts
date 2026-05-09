import { queryOptions } from "@tanstack/svelte-query";

import { loadAppDataMetadataFromBackend } from "$lib/api/app-data-api.js";
import type {
  AppDataMetadata,
  HostGroupRecord,
  Setting,
  SshHostRecord,
  SshKeyRecord,
} from "$lib/api/types.js";
import { queryKeys } from "./query-keys.js";

export function appDataMetadataQueryOptions() {
  return queryOptions({
    queryKey: queryKeys.metadata(),
    queryFn: loadAppDataMetadataFromBackend,
  });
}

export function appDataSettingsQueryOptions() {
  return queryOptions({
    queryKey: queryKeys.metadata(),
    queryFn: loadAppDataMetadataFromBackend,
    select: (metadata: AppDataMetadata): Setting[] => metadata.settings,
  });
}

export function appDataHostsQueryOptions() {
  return queryOptions({
    queryKey: queryKeys.metadata(),
    queryFn: loadAppDataMetadataFromBackend,
    select: (metadata: AppDataMetadata): SshHostRecord[] => metadata.hosts,
  });
}

export function appDataHostGroupsQueryOptions() {
  return queryOptions({
    queryKey: queryKeys.metadata(),
    queryFn: loadAppDataMetadataFromBackend,
    select: (metadata: AppDataMetadata): HostGroupRecord[] => metadata.host_groups,
  });
}

export function appDataKeysQueryOptions() {
  return queryOptions({
    queryKey: queryKeys.metadata(),
    queryFn: loadAppDataMetadataFromBackend,
    select: (metadata: AppDataMetadata): SshKeyRecord[] => metadata.keys,
  });
}
