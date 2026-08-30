export const queryKeys = {
  appData: ["app-data"] as const,
  metadata: () => [...queryKeys.appData, "metadata"] as const,
  settings: () => [...queryKeys.metadata(), "settings"] as const,
  connections: () => [...queryKeys.metadata(), "connections"] as const,
  hostGroups: () => [...queryKeys.metadata(), "host-groups"] as const,
  keys: () => [...queryKeys.metadata(), "keys"] as const,
  keySecret: (keyId: string) => [...queryKeys.keys(), keyId, "secret"] as const,
  snippets: () => ["snippets"] as const,
};

export const mutationKeys = {
  saveConnection: ["app-data", "connections", "save"] as const,
  deleteConnection: ["app-data", "connections", "delete"] as const,
  createHostGroup: ["app-data", "host-groups", "create"] as const,
  deleteHostGroup: ["app-data", "host-groups", "delete"] as const,
  createKey: ["app-data", "keys", "create"] as const,
  updateKey: ["app-data", "keys", "update"] as const,
  deleteKey: ["app-data", "keys", "delete"] as const,
  revealKeySecret: ["app-data", "keys", "reveal-secret"] as const,
  upsertSetting: ["app-data", "settings", "upsert"] as const,
  createSnippet: ["snippets", "create"] as const,
  updateSnippet: ["snippets", "update"] as const,
  deleteSnippet: ["snippets", "delete"] as const,
};
