import { mutationOptions } from "@tanstack/svelte-query";

import {
  createSshKey,
  deleteSshKey,
  revealSshKeySecret,
  updateSshKey,
} from "$lib/api/keys-api.js";
import type {
  KeyCreateRequest,
  KeyUpdateRequest,
  SshKeyRecord,
} from "$lib/api/types.js";
import { mutationKeys } from "./query-keys.js";

export interface UpdateSshKeyInput {
  keyId: string;
  key: KeyUpdateRequest;
}

export function createKeyMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.createKey,
    mutationFn: (key: KeyCreateRequest): Promise<SshKeyRecord> => createSshKey(key),
  });
}

export function updateKeyMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.updateKey,
    mutationFn: ({ keyId, key }: UpdateSshKeyInput): Promise<SshKeyRecord> =>
      updateSshKey(keyId, key),
  });
}

export function deleteKeyMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.deleteKey,
    mutationFn: deleteSshKey,
  });
}

export function revealKeySecretMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.revealKeySecret,
    mutationFn: revealSshKeySecret,
  });
}
