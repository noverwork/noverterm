import type { AuthBootstrapStatus } from "$lib/api/auth-api.js";
import type { BootstrapMetadata, KeyCreateRequest, KeyUpdateRequest, Setting, SshHostAuthMaterial, SshHostRecord, SshKeyRecord } from "$lib/api/types.js";

import { restoreBackendSession, registerToBackend, loginToBackend, logoutFromBackend } from "$lib/api/auth-api.js";
import { loadBootstrapMetadataFromBackend } from "$lib/api/bootstrap-api.js";
import { saveBackendConnection, deleteBackendConnection } from "$lib/api/connections-api.js";
import { upsertBackendSetting } from "$lib/api/settings-api.js";
import { createSshKey, updateSshKey, deleteSshKey } from "$lib/api/keys-api.js";

export interface BootstrapApi {
  restore(): Promise<AuthBootstrapStatus | null>;
  register(email: string, password: string): Promise<AuthBootstrapStatus>;
  login(email: string, password: string): Promise<AuthBootstrapStatus>;
  logout(): Promise<void>;
  loadBootstrapMetadata(): Promise<BootstrapMetadata>;
  saveConnection(connection: SaveConnectionInput): Promise<SshHostRecord>;
  deleteConnection(connection: ConnectionConfig): Promise<void>;
  saveSetting(setting: Setting): Promise<Setting>;
  createKey(key: KeyCreateRequest): Promise<SshKeyRecord>;
  updateKey(keyId: string, key: KeyUpdateRequest): Promise<SshKeyRecord>;
  deleteKey(keyId: string): Promise<void>;
}

const defaultApi: BootstrapApi = {
  restore: restoreBackendSession,
  register: registerToBackend,
  login: loginToBackend,
  logout: logoutFromBackend,
  loadBootstrapMetadata: loadBootstrapMetadataFromBackend,
  saveConnection: saveBackendConnection,
  deleteConnection: deleteBackendConnection,
  saveSetting: upsertBackendSetting,
  createKey: createSshKey,
  updateKey: updateSshKey,
  deleteKey: deleteSshKey,
};

export type BootstrapPhase = "loading" | "authenticated" | "unauthenticated" | "error";

export interface TerminalConfig {
  fontSize: number;
  fontFamily: string;
  cursorStyle: "block" | "underline" | "bar";
  cursorBlink: boolean;
  scrollback: number;
}

export interface ConnectionConfig {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  sshKeyId: string | null;
  hasPassword: boolean;
  auth: SshHostAuthMaterial | null;
}

export interface SaveConnectionInput {
  id?: string;
  name: string;
  host: string;
  port: number;
  username: string;
  password?: string;
  privateKey?: string;
  passphrase?: string;
  keyName?: string;
  existingKeyId?: string | null;
}

interface NovertermConfig {
  terminal?: Partial<TerminalConfig>;
  recentConnectionIds?: string[];
}

const DEFAULT_TERMINAL_CONFIG: TerminalConfig = {
  fontSize: 14,
  fontFamily: "JetBrains Mono, Fira Code, monospace",
  cursorStyle: "block",
  cursorBlink: true,
  scrollback: 5000,
};

interface BootstrapState {
  phase: BootstrapPhase;
  authStatus: AuthBootstrapStatus | null;
  metadata: BootstrapMetadata | null;
  error: string | null;
}

let state: BootstrapState = $state({
  phase: "loading",
  authStatus: null,
  metadata: null,
  error: null,
});

function commit() {
  state = { ...state };
}

export function resetBootstrapState() {
  state = {
    phase: "loading",
    authStatus: null,
    metadata: null,
    error: null,
  };
  commit();
}

function parseNovertermConfig(settings: Setting[]): NovertermConfig {
  const configStr = settings.find((setting) => setting.key === "noverterm-config")?.value;
  if (configStr) {
    try {
      const parsed = JSON.parse(configStr) as NovertermConfig;
      return {
        terminal: parsed.terminal,
        recentConnectionIds: Array.isArray(parsed.recentConnectionIds)
          ? parsed.recentConnectionIds.filter((id) => typeof id === "string")
          : [],
      };
    } catch {
      return {};
    }
  }
  return {};
}

function parseTerminalConfig(settings: Setting[]): TerminalConfig {
  const config = parseNovertermConfig(settings);
  return { ...DEFAULT_TERMINAL_CONFIG, ...config.terminal };
}

function parseRecentConnectionIds(settings: Setting[]): string[] {
  const config = parseNovertermConfig(settings);
  return config.recentConnectionIds ?? [];
}

function buildNovertermConfig(settings: Setting[], updates: Partial<NovertermConfig>): string {
  return JSON.stringify({ ...parseNovertermConfig(settings), ...updates });
}

function uniqueRecentConnectionIds(connectionId: string, currentIds: string[]): string[] {
  return [connectionId, ...currentIds.filter((id) => id !== connectionId)].slice(0, 12);
}

function filterExistingConnectionIds(connectionIds: string[], connections: ConnectionConfig[]): string[] {
  const existingConnectionIds = new Set(connections.map((connection) => connection.id));
  return connectionIds.filter((id) => existingConnectionIds.has(id));
}

function mapRecentConnections(connectionIds: string[], connections: ConnectionConfig[]): ConnectionConfig[] {
  return connectionIds
    .map((id) => connections.find((connection) => connection.id === id))
    .filter((connection): connection is ConnectionConfig => connection !== undefined);
}

function mapHostsToConnections(hosts: SshHostRecord[]): ConnectionConfig[] {
  return hosts.map((host) => ({
    id: host.id,
    name: host.name,
    host: host.host,
    port: host.port,
    username: host.username,
    sshKeyId: host.ssh_key_id,
    hasPassword: host.auth?.kind === "password" || host.auth?.kind === "public_key_and_password",
    auth: host.auth,
  }));
}

export function createBootstrapStore(api: BootstrapApi = defaultApi) {
  async function refreshMetadata() {
    const metadata = await api.loadBootstrapMetadata();
    state.metadata = metadata;
    state.phase = "authenticated";
    state.error = null;
    commit();
    return metadata;
  }

  async function init() {
    state.phase = "loading";
    state.error = null;
    commit();

    try {
      const authStatus = await api.restore();

      if (authStatus === null) {
        state.phase = "unauthenticated";
        state.authStatus = null;
        state.metadata = null;
        commit();
        return;
      }

      state.authStatus = authStatus;
      await refreshMetadata();
    } catch (error) {
      state.phase = "error";
      state.error = error instanceof Error ? error.message : String(error);
      commit();
    }
  }

  async function login(email: string, password: string) {
    state.phase = "loading";
    state.error = null;
    commit();

    try {
      state.authStatus = await api.login(email, password);
      await refreshMetadata();
    } catch (error) {
      state.phase = "unauthenticated";
      state.authStatus = null;
      state.error = error instanceof Error ? error.message : String(error);
      commit();
    }
  }

  async function register(email: string, password: string) {
    state.phase = "loading";
    state.error = null;
    commit();

    try {
      state.authStatus = await api.register(email, password);
      await refreshMetadata();
    } catch (error) {
      state.phase = "unauthenticated";
      state.authStatus = null;
      state.error = error instanceof Error ? error.message : String(error);
      commit();
    }
  }

  async function logout() {
    await api.logout();
    state.phase = "unauthenticated";
    state.authStatus = null;
    state.metadata = null;
    state.error = null;
    commit();
  }

  async function saveConnection(connection: SaveConnectionInput) {
    const savedConnection = await api.saveConnection(connection);
    await refreshMetadata();
    return savedConnection;
  }

  async function deleteConnection(connection: ConnectionConfig) {
    await api.deleteConnection(connection);
    const currentSettings = state.metadata?.settings ?? [];
    await api.saveSetting({
      key: "noverterm-config",
      value: buildNovertermConfig(currentSettings, {
        recentConnectionIds: parseRecentConnectionIds(currentSettings).filter((id) => id !== connection.id),
      }),
    });
    await refreshMetadata();
  }

  async function saveKey(key: KeyCreateRequest) {
    await api.createKey(key);
    await refreshMetadata();
  }

  async function updateKey(keyId: string, key: KeyUpdateRequest) {
    await api.updateKey(keyId, key);
    await refreshMetadata();
  }

  async function deleteKey(key: SshKeyRecord) {
    await api.deleteKey(key.id);
    await refreshMetadata();
  }

  async function saveTerminalConfig(config: TerminalConfig) {
    const currentSettings = state.metadata?.settings ?? [];
    await api.saveSetting({
      key: "noverterm-config",
      value: buildNovertermConfig(currentSettings, { terminal: config }),
    });

    await refreshMetadata();
  }

  async function recordRecentConnection(connectionId: string) {
    const currentSettings = state.metadata?.settings ?? [];
    await api.saveSetting({
      key: "noverterm-config",
      value: buildNovertermConfig(currentSettings, {
        recentConnectionIds: uniqueRecentConnectionIds(connectionId, parseRecentConnectionIds(currentSettings)),
      }),
    });

    await refreshMetadata();
  }

  function getTerminalConfig(): TerminalConfig {
    if (state.metadata) {
      return parseTerminalConfig(state.metadata.settings);
    }
    return DEFAULT_TERMINAL_CONFIG;
  }

  function getConnections(): ConnectionConfig[] {
    if (state.metadata) {
      return mapHostsToConnections(state.metadata.hosts);
    }
    return [];
  }

  function getRecentConnectionIds(): string[] {
    if (!state.metadata) {
      return [];
    }

    return filterExistingConnectionIds(
      parseRecentConnectionIds(state.metadata.settings),
      getConnections(),
    );
  }

  function getRecentConnections(): ConnectionConfig[] {
    return mapRecentConnections(getRecentConnectionIds(), getConnections());
  }

  function getKeys(): SshKeyRecord[] {
    return state.metadata?.keys ?? [];
  }

  function getSettings(): Setting[] {
    return state.metadata?.settings ?? [];
  }

  return {
    get phase() {
      return state.phase;
    },
    get authStatus() {
      return state.authStatus;
    },
    get metadata() {
      return state.metadata;
    },
    get error() {
      return state.error;
    },
    get isAuthenticated() {
      return state.phase === "authenticated";
    },
    get isLoading() {
      return state.phase === "loading";
    },
    get isUnauthenticated() {
      return state.phase === "unauthenticated";
    },
    get isError() {
      return state.phase === "error";
    },
    init,
    login,
    register,
    logout,
    refreshMetadata,
    saveConnection,
    deleteConnection,
    saveKey,
    updateKey,
    deleteKey,
    saveTerminalConfig,
    recordRecentConnection,
    getTerminalConfig,
    getConnections,
    getRecentConnectionIds,
    getRecentConnections,
    getKeys,
    getSettings,
  };
}
