import { mutationOptions } from "@tanstack/svelte-query";

import {
  createPortForward,
  deletePortForward,
  updatePortForward,
} from "$lib/api/port-forwards-api.js";
import type {
  PortForwardRecord,
  PortForwardWriteRequest,
} from "$lib/api/types.js";
import { mutationKeys } from "./query-keys.js";

export function savePortForwardMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.savePortForward,
    mutationFn: ({
      id,
      forward,
    }: {
      id?: string;
      forward: PortForwardWriteRequest;
    }): Promise<PortForwardRecord> =>
      id ? updatePortForward(id, forward) : createPortForward(forward),
  });
}

export function deletePortForwardMutationOptions() {
  return mutationOptions({
    mutationKey: mutationKeys.deletePortForward,
    mutationFn: (id: string): Promise<void> => deletePortForward(id),
  });
}
