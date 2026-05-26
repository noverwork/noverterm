import { getContext, setContext } from "svelte";
import {
  createMutation,
  createQuery,
  type QueryClient,
} from "@tanstack/svelte-query";

import { loadAppSettings } from "$lib/api/api-client.js";
import type { AuthSessionStatus } from "$lib/api/auth-api.js";
import type { HostGroupRecord, SshKeyRecord } from "$lib/api/types.js";
import {
  buildNovertermConfig,
  createPortForwardId,
  parseNovertermConfig,
  parseRecentConnectionIds,
  selectConnections,
  selectSavedPortForwards,
  selectTerminalConfig,
  uniqueRecentConnectionIds,
} from "$lib/app-data-mappers.js";
import type {
  AppDataPhase,
  ConnectionConfig,
  SaveConnectionInput,
  SavePortForwardInput,
  SavedPortForwardConfig,
} from "$lib/app-data-types.js";
import {
  appDataMetadataQueryOptions,
  createHostGroupMutationOptions,
  createKeyMutationOptions,
  deleteConnectionMutationOptions,
  deleteHostGroupMutationOptions,
  deleteKeyMutationOptions,
  forgotPasswordMutationOptions,
  loginMutationOptions,
  logoutMutationOptions,
  queryKeys,
  registerMutationOptions,
  resetPasswordMutationOptions,
  restoreSessionMutationOptions,
  revealKeySecretMutationOptions,
  saveConnectionMutationOptions,
  updateKeyMutationOptions,
  upsertSettingMutationOptions,
} from "$lib/queries/index.js";
import { createPortForwardStore } from "$lib/stores/port-forward.svelte.js";
import {
  createSessionStore,
  type Session,
} from "$lib/stores/session.svelte.js";

const APP_SHELL_CONTEXT = Symbol("app-shell");

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function createAppShellStore(queryClient: QueryClient) {
  const sessionStore = createSessionStore();
  const portForwardStore = createPortForwardStore();

  let authPhase = $state<AppDataPhase>("loading");
  let authStatus = $state<AuthSessionStatus | null>(null);
  let authError = $state<string | null>(null);
  let showSettings = $state(false);
  let connectionFormError = $state<string | null>(null);
  let connectionSaving = $state(false);
  let trustConfirming = $state(false);
  let trustError = $state<string | null>(null);
  let visibleTerminalSessionId = $state<string | null>(null);
  let startupLoading = $state(true);
  let startupError = $state<string | null>(null);
  let sessionOrder = $state<string[]>([]);

  const metadataQuery = createQuery(
    () => ({
      ...appDataMetadataQueryOptions(),
      enabled: authPhase === "authenticated",
    }),
    () => queryClient,
  );

  const restoreSessionMutation = createMutation(
    () => restoreSessionMutationOptions(),
    () => queryClient,
  );
  const loginMutation = createMutation(
    () => loginMutationOptions(),
    () => queryClient,
  );
  const registerMutation = createMutation(
    () => registerMutationOptions(),
    () => queryClient,
  );
  const logoutMutation = createMutation(
    () => logoutMutationOptions(),
    () => queryClient,
  );
  const forgotPasswordMutation = createMutation(
    () => forgotPasswordMutationOptions(),
    () => queryClient,
  );
  const resetPasswordMutation = createMutation(
    () => resetPasswordMutationOptions(),
    () => queryClient,
  );
  const saveConnectionMutation = createMutation(
    () => saveConnectionMutationOptions(),
    () => queryClient,
  );
  const deleteConnectionMutation = createMutation(
    () => deleteConnectionMutationOptions(),
    () => queryClient,
  );
  const createHostGroupMutation = createMutation(
    () => createHostGroupMutationOptions(),
    () => queryClient,
  );
  const deleteHostGroupMutation = createMutation(
    () => deleteHostGroupMutationOptions(),
    () => queryClient,
  );
  const createKeyMutation = createMutation(
    () => createKeyMutationOptions(),
    () => queryClient,
  );
  const updateKeyMutation = createMutation(
    () => updateKeyMutationOptions(),
    () => queryClient,
  );
  const deleteKeyMutation = createMutation(
    () => deleteKeyMutationOptions(),
    () => queryClient,
  );
  const revealKeySecretMutation = createMutation(
    () => revealKeySecretMutationOptions(),
    () => queryClient,
  );
  const upsertSettingMutation = createMutation(
    () => upsertSettingMutationOptions(),
    () => queryClient,
  );

  const metadata = $derived(metadataQuery.data ?? null);
  const terminalConfig = $derived(selectTerminalConfig(metadata));
  const connections = $derived(selectConnections(metadata));
  const hostGroups = $derived(metadata?.host_groups ?? []);
  const keys = $derived(metadata?.keys ?? []);
  const savedPortForwards = $derived(selectSavedPortForwards(metadata));

  const activeSession = $derived(
    sessionStore.activeSessionId
      ? sessionStore.sessions.get(sessionStore.activeSessionId)
      : undefined,
  );

  const activeSessions = $derived.by(() => {
    const allSessions = Array.from(sessionStore.sessions.values()) as Session[];

    let effectiveOrder = sessionOrder.filter((id) => sessionStore.sessions.has(id));
    const allIds = allSessions.map((s) => s.id);
    const newIds = allIds.filter((id) => !effectiveOrder.includes(id));
    effectiveOrder = [...effectiveOrder, ...newIds];

    if (effectiveOrder.length === 0) {
      return allSessions;
    }

    const orderMap = new Map<string, number>();
    effectiveOrder.forEach((id, index) => orderMap.set(id, index));

    return [...allSessions].sort((a, b) => {
      const aIdx = orderMap.get(a.id) ?? Infinity;
      const bIdx = orderMap.get(b.id) ?? Infinity;
      return aIdx - bIdx;
    });
  });

  const mountedTerminalSessions = $derived(
    Array.from(sessionStore.sessions.values()).filter(
      (session) => session.status === "connected" || session.status === "connecting",
    ) as Session[],
  );

  let lastSyncedSessionIds = new Set<string>();

  $effect(() => {
    const currentIds = new Set(Array.from(sessionStore.sessions.keys()));

    const hasNew = [...currentIds].some((id) => !lastSyncedSessionIds.has(id));
    const hasRemoved = [...lastSyncedSessionIds].some((id) => !currentIds.has(id));

    if (!hasNew && !hasRemoved) return;

    const filtered = sessionOrder.filter((id) => currentIds.has(id));
    const newIds = [...currentIds].filter((id) => !filtered.includes(id));
    sessionOrder = [...filtered, ...newIds];
    lastSyncedSessionIds = currentIds;
  });

  $effect(() => {
    if (
      activeSession?.status === "connected" ||
      activeSession?.status === "connecting"
    ) {
      if (visibleTerminalSessionId !== activeSession.id) {
        visibleTerminalSessionId = activeSession.id;
      }
      return;
    }

    if (
      visibleTerminalSessionId &&
      mountedTerminalSessions.some(
        (session) => session.id === visibleTerminalSessionId,
      )
    ) {
      return;
    }

    const fallbackSessionId = mountedTerminalSessions[0]?.id ?? null;
    if (visibleTerminalSessionId !== fallbackSessionId) {
      visibleTerminalSessionId = fallbackSessionId;
    }
  });

  async function fetchAppDataMetadata() {
    return await queryClient.fetchQuery(appDataMetadataQueryOptions());
  }

  async function refreshMetadata() {
    await queryClient.invalidateQueries({ queryKey: queryKeys.metadata() });
    return await fetchAppDataMetadata();
  }

  async function clearAppDataCache() {
    await queryClient.cancelQueries({ queryKey: queryKeys.appData });
    queryClient.removeQueries({ queryKey: queryKeys.appData });
  }

  function currentSettings() {
    return metadata?.settings ?? [];
  }

  async function init() {
    startupLoading = true;
    startupError = null;
    authPhase = "loading";
    authError = null;

    try {
      await loadAppSettings();
      await sessionStore.init();
      const restoredAuthStatus = await restoreSessionMutation.mutateAsync();

      if (restoredAuthStatus === null) {
        authPhase = "unauthenticated";
        authStatus = null;
        await clearAppDataCache();
        return;
      }

      authStatus = restoredAuthStatus;
      authPhase = "authenticated";
      await fetchAppDataMetadata();
    } catch (error) {
      authPhase = "error";
      authError = errorMessage(error);
      startupError = authError;
    } finally {
      startupLoading = false;
    }
  }

  function cleanup() {
    sessionStore.cleanup();
    portForwardStore.cleanup();
  }

  async function login(email: string, password: string) {
    authPhase = "loading";
    authError = null;

    try {
      authStatus = await loginMutation.mutateAsync({ email, password });
      authPhase = "authenticated";
      await fetchAppDataMetadata();
    } catch (error) {
      authPhase = "unauthenticated";
      authStatus = null;
      authError = errorMessage(error);
      await clearAppDataCache();
    }
  }

  async function signup(email: string, password: string) {
    authPhase = "loading";
    authError = null;

    try {
      authStatus = await registerMutation.mutateAsync({ email, password });
      authPhase = "authenticated";
      await fetchAppDataMetadata();
    } catch (error) {
      authPhase = "unauthenticated";
      authStatus = null;
      authError = errorMessage(error);
      await clearAppDataCache();
    }
  }

  async function forgotPassword(email: string) {
    await forgotPasswordMutation.mutateAsync(email);
  }

  async function resetAccountPassword(token: string, password: string) {
    await resetPasswordMutation.mutateAsync({ token, password });
  }

  async function logout() {
    await logoutMutation.mutateAsync();
    authPhase = "unauthenticated";
    authStatus = null;
    authError = null;
    await clearAppDataCache();
  }

  async function connectSavedConnection(
    connection: ConnectionConfig,
  ): Promise<boolean> {
    try {
      await sessionStore.connectSavedConnection(connection, 80, 24);
      void recordRecentConnection(connection.id).catch(() => undefined);
      return true;
    } catch {
      return false;
    }
  }

  async function connectLocalTerminal(): Promise<boolean> {
    try {
      await sessionStore.connectLocal("Local Terminal");
      return true;
    } catch {
      return false;
    }
  }

  async function openK9sTerminal(): Promise<boolean> {
    try {
      const sessionId = await sessionStore.connectLocal("k9s");
      await sessionStore.writeSession(sessionId, "k9s\n");
      return true;
    } catch {
      return false;
    }
  }

  async function openClaudeCodeTerminal(): Promise<boolean> {
    try {
      const sessionId = await sessionStore.connectLocal("Claude Code");
      await sessionStore.writeSession(
        sessionId,
        "mkdir -p ~/noverterm/agent && cd ~/noverterm/agent && claude\n",
      );
      return true;
    } catch {
      return false;
    }
  }

  async function openOpencodeTerminal(): Promise<boolean> {
    try {
      const sessionId = await sessionStore.connectLocal("OpenCode");
      await sessionStore.writeSession(
        sessionId,
        "mkdir -p ~/noverterm/agent && cd ~/noverterm/agent && opencode\n",
      );
      return true;
    } catch {
      return false;
    }
  }

  function activateSession(id: string) {
    sessionStore.setActiveSession(id);
  }

  function resetConnectionFormError() {
    connectionFormError = null;
    connectionSaving = false;
  }

  async function saveConnection(
    connection: SaveConnectionInput,
  ): Promise<boolean> {
    connectionSaving = true;
    connectionFormError = null;

    try {
      await saveConnectionMutation.mutateAsync(connection);
      await refreshMetadata();
      return true;
    } catch (error) {
      connectionFormError = errorMessage(error);
      return false;
    } finally {
      connectionSaving = false;
    }
  }

  async function deleteConnection(connection: ConnectionConfig) {
    const settings = currentSettings();
    await deleteConnectionMutation.mutateAsync(connection);
    await upsertSettingMutation.mutateAsync({
      key: "noverterm-config",
      value: buildNovertermConfig(settings, {
        recentConnectionIds: parseRecentConnectionIds(settings).filter(
          (id) => id !== connection.id,
        ),
      }),
    });
    await refreshMetadata();
    sessionStore.disconnectConnectionSessions(connection.id);
  }

  async function createHostGroup(name: string) {
    const group = await createHostGroupMutation.mutateAsync(name);
    await refreshMetadata();
    return group;
  }

  async function deleteHostGroup(group: HostGroupRecord) {
    await deleteHostGroupMutation.mutateAsync(group);
    await refreshMetadata();
  }

  async function moveConnectionToGroup(
    connection: ConnectionConfig,
    groupId: string | null,
  ) {
    await saveConnection({
      id: connection.id,
      name: connection.name,
      groupId,
      host: connection.host,
      port: connection.port,
      username: connection.username,
      existingKeyId: connection.sshKeyId,
      ...(connection.auth?.kind === "password" ||
      connection.auth?.kind === "public_key_and_password"
        ? { preservedEncryptedPassword: connection.auth.password }
        : {}),
    });
  }

  async function saveKey(name: string, privateKey: string, passphrase: string) {
    await createKeyMutation.mutateAsync({
      name,
      kind: "inline",
      encrypted_private_key: privateKey,
      encrypted_passphrase: passphrase || null,
    });
    await refreshMetadata();
  }

  async function updateKey(
    keyId: string,
    name: string,
    privateKey?: string,
    passphrase?: string,
  ) {
    await updateKeyMutation.mutateAsync({
      keyId,
      key: {
        name,
        kind: "inline",
        ...(privateKey ? { encrypted_private_key: privateKey } : {}),
        ...(privateKey ? { encrypted_passphrase: passphrase || null } : {}),
      },
    });
    await refreshMetadata();
  }

  async function deleteKey(key: SshKeyRecord) {
    await deleteKeyMutation.mutateAsync(key.id);
    await refreshMetadata();
  }

  async function revealKeySecret(keyId: string) {
    return await revealKeySecretMutation.mutateAsync(keyId);
  }

  async function recordRecentConnection(connectionId: string) {
    const settings = currentSettings();
    await upsertSettingMutation.mutateAsync({
      key: "noverterm-config",
      value: buildNovertermConfig(settings, {
        recentConnectionIds: uniqueRecentConnectionIds(
          connectionId,
          parseRecentConnectionIds(settings),
        ),
      }),
    });
    await refreshMetadata();
  }

  async function savePortForward(input: SavePortForwardInput) {
    const settings = currentSettings();
    const existingForwards =
      parseNovertermConfig(settings).savedPortForwards ?? [];
    const savedForward: SavedPortForwardConfig = {
      id: input.id ?? createPortForwardId(),
      name: input.name,
      connectionId: input.connectionId,
      bind_host: input.bind_host,
      bind_port: input.bind_port,
      target_host: input.target_host,
      target_port: input.target_port,
    };
    const nextForwards = [
      savedForward,
      ...existingForwards.filter((forward) => forward.id !== savedForward.id),
    ];

    await upsertSettingMutation.mutateAsync({
      key: "noverterm-config",
      value: buildNovertermConfig(settings, { savedPortForwards: nextForwards }),
    });
    await refreshMetadata();
    return savedForward;
  }

  async function startSavedPortForward(forward: SavedPortForwardConfig) {
    const connection = connections.find(
      (candidate) => candidate.id === forward.connectionId,
    );
    if (!connection) {
      throw new Error(
        "Saved connection not found. Open Connections and verify this host still exists.",
      );
    }

    return await portForwardStore.startSavedForward({
      preset: forward,
      connection,
    });
  }

  async function stopPortForward(forwardId: string) {
    return await portForwardStore.stop(forwardId);
  }

  async function deleteRuntimePortForward(forwardId: string) {
    portForwardStore.remove(forwardId);
  }

  async function deleteSavedPortForward(forwardId: string) {
    const settings = currentSettings();
    const nextForwards = (
      parseNovertermConfig(settings).savedPortForwards ?? []
    ).filter((forward) => forward.id !== forwardId);

    await upsertSettingMutation.mutateAsync({
      key: "noverterm-config",
      value: buildNovertermConfig(settings, { savedPortForwards: nextForwards }),
    });
    await refreshMetadata();
  }

  function closeSession(id: string) {
    void sessionStore.disconnectSession(id);
  }

  function reorderSessions(fromSessionId: string, toSessionId: string) {
    if (fromSessionId === toSessionId) return;

    const visibleOrder = activeSessions.map((session) => session.id);
    const fromIndex = visibleOrder.indexOf(fromSessionId);
    const toIndex = visibleOrder.indexOf(toSessionId);
    if (fromIndex === -1 || toIndex === -1) return;

    const [movedId] = visibleOrder.splice(fromIndex, 1);
    if (!movedId) return;

    visibleOrder.splice(toIndex, 0, movedId);
    const visibleIds = new Set(visibleOrder);
    const hiddenIds = sessionOrder.filter((id) => !visibleIds.has(id));
    sessionOrder = [...visibleOrder, ...hiddenIds];
  }

  async function duplicateSession(id: string): Promise<boolean> {
    const session = sessionStore.sessions.get(id);
    if (!session) {
      return false;
    }

    if (session.type === "local") {
      try {
        await sessionStore.connectLocal(session.name);
        return true;
      } catch {
        return false;
      }
    }

    if (!session.connectionId) {
      return false;
    }

    const connection = connections.find(
      (candidate) => candidate.id === session.connectionId,
    );
    if (!connection) {
      return false;
    }

    return await connectSavedConnection(connection);
  }

  async function retryActiveConnection(): Promise<boolean> {
    if (!activeSession) {
      return false;
    }

    if (activeSession.connectionId) {
      const connection = connections.find(
        (candidate) => candidate.id === activeSession.connectionId,
      );
      if (connection) {
        try {
          await sessionStore.retrySavedConnection(
            activeSession.id,
            connection,
            80,
            24,
          );
          void recordRecentConnection(connection.id).catch(() => undefined);
          return true;
        } catch {
          return false;
        }
      }
    }

    return false;
  }

  async function trustActiveHost(): Promise<boolean> {
    if (!activeSession?.trustPrompt || !activeSession.connectionId) {
      return false;
    }

    const connection = connections.find(
      (candidate) => candidate.id === activeSession.connectionId,
    );
    if (!connection) {
      trustError =
        "Saved connection not found. Open Connections and try again.";
      return false;
    }

    const prompt = activeSession.trustPrompt;
    const failedSessionId = activeSession.id;
    trustConfirming = true;
    trustError = null;

    try {
      await sessionStore.confirmHostTrust({
        host: prompt.host,
        port: prompt.port,
        algorithm: prompt.algorithm,
        fingerprint: prompt.fingerprint,
      });
      await sessionStore.retrySavedConnection(
        failedSessionId,
        connection,
        80,
        24,
      );
      void recordRecentConnection(connection.id).catch(() => undefined);
      return true;
    } catch (error) {
      trustError = errorMessage(error);
      return false;
    } finally {
      trustConfirming = false;
    }
  }

  async function runSnippet(
    connection: ConnectionConfig,
    command: string,
  ): Promise<boolean> {
    const existingSession = Array.from(sessionStore.sessions.values()).find(
      (s) => s.connectionId === connection.id && s.status !== "disconnected",
    );

    if (existingSession) {
      sessionStore.setActiveSession(existingSession.id);
      await sessionStore.writeSession(existingSession.id, command + "\n");
      return true;
    }

    const connected = await connectSavedConnection(connection);
    if (!connected) {
      return false;
    }

    const newSession = Array.from(sessionStore.sessions.values()).find(
      (s) => s.connectionId === connection.id && s.status !== "disconnected",
    );
    if (newSession) {
      sessionStore.setActiveSession(newSession.id);
      await sessionStore.writeSession(newSession.id, command + "\n");
      return true;
    }

    return false;
  }

  function openSettings() {
    showSettings = true;
  }

  function closeSettings() {
    showSettings = false;
  }

  return {
    sessionStore,
    portForwardStore,
    get authStatus() {
      return authStatus;
    },
    get showSettings() {
      return showSettings;
    },
    get connectionFormError() {
      return connectionFormError;
    },
    get connectionSaving() {
      return connectionSaving;
    },
    get trustConfirming() {
      return trustConfirming;
    },
    get trustError() {
      return trustError;
    },
    get visibleTerminalSessionId() {
      return visibleTerminalSessionId;
    },
    get isLoading() {
      return (
        startupLoading ||
        authPhase === "loading" ||
        (authPhase === "authenticated" && metadataQuery.isPending)
      );
    },
    get isUnauthenticated() {
      return authPhase === "unauthenticated";
    },
    get isError() {
      return (
        startupError !== null ||
        authPhase === "error" ||
        metadataQuery.isError
      );
    },
    get error() {
      return startupError ?? authError ?? metadataQuery.error?.message ?? null;
    },
    get activeSession() {
      return activeSession;
    },
    get activeSessions() {
      return activeSessions;
    },
    get mountedTerminalSessions() {
      return mountedTerminalSessions;
    },
    get terminalConfig() {
      return terminalConfig;
    },
    get connections() {
      return connections;
    },
    get hostGroups() {
      return hostGroups;
    },
    get keys() {
      return keys;
    },
    get savedPortForwards() {
      return savedPortForwards;
    },
    init,
    cleanup,
    login,
    signup,
    forgotPassword,
    resetAccountPassword,
    logout,
    connectSavedConnection,
    connectLocalTerminal,
    openK9sTerminal,
    openClaudeCodeTerminal,
    openOpencodeTerminal,
    activateSession,
    resetConnectionFormError,
    saveConnection,
    deleteConnection,
    createHostGroup,
    deleteHostGroup,
    moveConnectionToGroup,
    saveKey,
    updateKey,
    deleteKey,
    revealKeySecret,
    savePortForward,
    startSavedPortForward,
    stopPortForward,
    deleteRuntimePortForward,
    deleteSavedPortForward,
    closeSession,
    reorderSessions,
    duplicateSession,
    retryActiveConnection,
    trustActiveHost,
    runSnippet,
    openSettings,
    closeSettings,
  };
}

export type AppShellStore = ReturnType<typeof createAppShellStore>;

export function setAppShellContext(appShell: AppShellStore) {
  setContext(APP_SHELL_CONTEXT, appShell);
}

export function getAppShellContext(): AppShellStore {
  return getContext<AppShellStore>(APP_SHELL_CONTEXT);
}
