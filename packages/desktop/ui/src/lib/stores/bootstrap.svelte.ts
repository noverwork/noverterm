import type {
  AuthBootstrapStatus,
  BootstrapMetadata,
  Setting,
  SshHostRecord,
  SshKeyRecord,
} from "$lib/api/backend-api.js";
import { backendApi, type BootstrapApi } from "$lib/api/backend-api.js";

export type BootstrapPhase = "loading" | "authenticated" | "unauthenticated" | "error";

export interface TerminalConfig {
  theme: "dark" | "light";
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
  authMode: string;
  sshKeyId: string | null;
  hasPassword: boolean;
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
  existingKeyId?: string | null;
}

const DEFAULT_TERMINAL_CONFIG: TerminalConfig = {
  theme: "dark",
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

function parseTerminalConfig(settings: Setting[]): TerminalConfig {
  const configStr = settings.find((setting) => setting.key === "noverterm-config")?.value;
  if (configStr) {
    try {
      const parsed = JSON.parse(configStr) as { terminal?: Partial<TerminalConfig> };
      return { ...DEFAULT_TERMINAL_CONFIG, ...parsed.terminal };
    } catch {
      return DEFAULT_TERMINAL_CONFIG;
    }
  }
  return DEFAULT_TERMINAL_CONFIG;
}

function mapHostsToConnections(hosts: SshHostRecord[]): ConnectionConfig[] {
  return hosts.map((host) => ({
    id: host.id,
    name: host.name,
    host: host.host,
    port: host.port,
    username: host.username,
    authMode: host.auth_mode,
    sshKeyId: host.ssh_key_id,
    hasPassword: host.auth_mode === "password" || host.auth_mode === "publickey_password",
  }));
}

function applyTheme(theme: "dark" | "light") {
  if (theme === "dark") {
    document.documentElement.classList.add("dark");
  } else {
    document.documentElement.classList.remove("dark");
  }
}

export function createBootstrapStore(api: BootstrapApi = backendApi) {
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

  async function login(username: string, password: string) {
    state.phase = "loading";
    state.error = null;
    commit();

    try {
      state.authStatus = await api.login(username, password);
      await refreshMetadata();
    } catch (error) {
      state.phase = "unauthenticated";
      state.authStatus = null;
      state.error = error instanceof Error ? error.message : String(error);
      commit();
    }
  }

  async function register(username: string, password: string) {
    state.phase = "loading";
    state.error = null;
    commit();

    try {
      state.authStatus = await api.register(username, password);
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
    await refreshMetadata();
  }

  async function saveTerminalConfig(config: TerminalConfig) {
    await api.saveSetting({
      key: "noverterm-config",
      value: JSON.stringify({ terminal: config }),
    });

    applyTheme(config.theme);
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
    saveTerminalConfig,
    getTerminalConfig,
    getConnections,
    getKeys,
    getSettings,
    applyTheme,
  };
}
