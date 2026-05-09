import { queryOptions } from "@tanstack/svelte-query";

import { revealSshKeySecret } from "$lib/api/keys-api.js";
import type { SshKeySecret } from "$lib/api/types.js";
import { queryKeys } from "./query-keys.js";

export function sshKeySecretQueryOptions(keyId: string) {
  return queryOptions({
    queryKey: queryKeys.keySecret(keyId),
    queryFn: (): Promise<SshKeySecret> => revealSshKeySecret(keyId),
    enabled: keyId.length > 0,
    staleTime: 0,
    gcTime: 0,
  });
}
