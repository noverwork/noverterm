import { getContext, setContext } from "svelte";

import { loadAppSettings } from "$lib/api/api-client.js";
import type { SshKeyRecord } from "$lib/api/types.js";
import type {
  ConnectionConfig,
  SaveConnectionInput,
  SavePortForwardInput,
  SavedPortForwardConfig,
  TerminalConfig,
} from "$lib/stores/bootstrap.svelte.js";
import { createBootstrapStore } from "$lib/stores/bootstrap.svelte.js";
import { createPortForwardStore } from "$lib/stores/port-forward.svelte.js";
import { createSessionStore, type Session } from "$lib/stores/session.svelte.js";

const APP_SHELL_CONTEXT = Symbol("app-shell");

export function createAppShellStore() {
  const bootstrapStore = createBootstrapStore();
  const sessionStore = createSessionStore();
  const portForwardStore = createPortForwardStore();

  let sidebarCollapsed = $state(false);
  let showSettings = $state(false);
  let connectionFormError = $state<string | null>(null);
  let connectionSaving = $state(false);
  let trustConfirming = $state(false);
  let trustError = $state<string | null>(null);
  let visibleTerminalSessionId = $state<string | null>(null);
  let startupLoading = $state(true);
  let startupError = $state<string | null>(null);

  const activeSession = $derived(
    sessionStore.activeSessionId ? sessionStore.sessions.get(sessionStore.activeSessionId) : undefined,
  );

  const activeSessions = $derived(
    Array.from(sessionStore.sessions.values()).filter(
      (session) => session.status !== "disconnected",
    ) as Session[],
  );

  const mountedTerminalSessions = $derived(
    activeSessions.filter((session) => session.status === "connected") as Session[],
  );

  const terminalConfig = $derived(bootstrapStore.getTerminalConfig());
  const connections = $derived(bootstrapStore.getConnections());
  const keys = $derived(bootstrapStore.getKeys());

  $effect(() => {
    if (activeSession?.status === "connected") {
      if (visibleTerminalSessionId !== activeSession.id) {
        visibleTerminalSessionId = activeSession.id;
      }
      return;
    }

    if (visibleTerminalSessionId && mountedTerminalSessions.some((session) => session.id === visibleTerminalSessionId)) {
      return;
    }

    const fallbackSessionId = mountedTerminalSessions[0]?.id ?? null;
    if (visibleTerminalSessionId !== fallbackSessionId) {
      visibleTerminalSessionId = fallbackSessionId;
    }
  });

  async function init() {
    startupLoading = true;
    startupError = null;

    try {
      await loadAppSettings();
      await sessionStore.init();
      await bootstrapStore.init();
    } catch (error) {
      startupError = error instanceof Error ? error.message : String(error);
    } finally {
      startupLoading = false;
    }
  }

  function cleanup() {
    sessionStore.cleanup();
    portForwardStore.cleanup();
  }

  async function login(email: string, password: string) {
    await bootstrapStore.login(email, password);
  }

  async function signup(email: string, password: string) {
    await bootstrapStore.register(email, password);
  }

  async function logout() {
    await bootstrapStore.logout();
  }

  async function connectSavedConnection(connection: ConnectionConfig): Promise<boolean> {
    try {
      await sessionStore.connectSavedConnection(connection, 80, 24);
      void bootstrapStore.recordRecentConnection(connection.id).catch(() => undefined);
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

  function activateSession(id: string) {
    sessionStore.setActiveSession(id);
  }

  function resetConnectionFormError() {
    connectionFormError = null;
    connectionSaving = false;
  }

  async function saveConnection(connection: SaveConnectionInput): Promise<boolean> {
    connectionSaving = true;
    connectionFormError = null;

    try {
      await bootstrapStore.saveConnection(connection);
      return true;
    } catch (error) {
      connectionFormError = error instanceof Error ? error.message : String(error);
      return false;
    } finally {
      connectionSaving = false;
    }
  }

  async function deleteConnection(connection: ConnectionConfig) {
    await bootstrapStore.deleteConnection(connection);
    sessionStore.disconnectConnectionSessions(connection.id);
  }

  async function saveKey(name: string, privateKey: string, passphrase: string) {
    await bootstrapStore.saveKey({
      name,
      kind: "inline",
      encrypted_private_key: privateKey,
      encrypted_passphrase: passphrase || null,
    });
  }

  async function updateKey(keyId: string, name: string, privateKey: string, passphrase: string) {
    await bootstrapStore.updateKey(keyId, {
      name,
      kind: "inline",
      encrypted_private_key: privateKey,
      encrypted_passphrase: passphrase || null,
    });
  }

  async function deleteKey(key: SshKeyRecord) {
    await bootstrapStore.deleteKey(key);
  }

  async function savePortForward(input: SavePortForwardInput) {
    return await bootstrapStore.savePortForward(input);
  }

  async function startSavedPortForward(forward: SavedPortForwardConfig) {
    const connection = connections.find((candidate) => candidate.id === forward.connectionId);
    if (!connection) {
      throw new Error("Saved connection not found. Open Connections and verify this host still exists.");
    }

    return await portForwardStore.startSavedForward({ preset: forward, connection });
  }

  async function stopPortForward(forwardId: string) {
    return await portForwardStore.stop(forwardId);
  }

  async function deleteRuntimePortForward(forwardId: string) {
    portForwardStore.remove(forwardId);
  }

  async function deleteSavedPortForward(forwardId: string) {
    await bootstrapStore.deletePortForward(forwardId);
  }

  function closeSession(id: string) {
    void sessionStore.disconnectSession(id);
  }

  async function saveSettings(config: TerminalConfig) {
    await bootstrapStore.saveTerminalConfig(config);
    showSettings = false;
  }

  async function retryActiveConnection(): Promise<boolean> {
    if (!activeSession) {
      return false;
    }

    if (activeSession.connectionId) {
      const connection = connections.find((candidate) => candidate.id === activeSession.connectionId);
      if (connection) {
        return await connectSavedConnection(connection);
      }
    }

    return false;
  }

  async function trustActiveHost(): Promise<boolean> {
    if (!activeSession?.trustPrompt || !activeSession.connectionId) {
      return false;
    }

    const connection = connections.find((candidate) => candidate.id === activeSession.connectionId);
    if (!connection) {
      trustError = "Saved connection not found. Open Connections and try again.";
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
      sessionStore.removeSession(failedSessionId);
      return await connectSavedConnection(connection);
    } catch (error) {
      trustError = error instanceof Error ? error.message : String(error);
      return false;
    } finally {
      trustConfirming = false;
    }
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
  }

  function openSettings() {
    showSettings = true;
  }

  function closeSettings() {
    showSettings = false;
  }

  return {
    bootstrapStore,
    sessionStore,
    portForwardStore,
    get sidebarCollapsed() {
      return sidebarCollapsed;
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
      return startupLoading || (!startupError && bootstrapStore.isLoading);
    },
    get isError() {
      return startupError !== null || bootstrapStore.isError;
    },
    get error() {
      return startupError ?? bootstrapStore.error;
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
    get keys() {
      return keys;
    },
    init,
    cleanup,
    login,
    signup,
    logout,
    connectSavedConnection,
    connectLocalTerminal,
    activateSession,
    resetConnectionFormError,
    saveConnection,
    deleteConnection,
    saveKey,
    updateKey,
    deleteKey,
    savePortForward,
    startSavedPortForward,
    stopPortForward,
    deleteRuntimePortForward,
    deleteSavedPortForward,
    closeSession,
    saveSettings,
    retryActiveConnection,
    trustActiveHost,
    toggleSidebar,
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
