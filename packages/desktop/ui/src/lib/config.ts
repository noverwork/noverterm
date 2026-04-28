import { invoke } from "@tauri-apps/api/core";

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
  password?: string;
  keyPath?: string;
}

export interface AppConfig {
  terminal: TerminalConfig;
  connections: ConnectionConfig[];
  lastActiveSessionId?: string;
}

const DEFAULT_CONFIG: AppConfig = {
  terminal: {
    fontSize: 14,
    fontFamily: "Sarasa Term TC SemiBold, Menlo, Monaco, 'Courier New', monospace",
    cursorStyle: "block",
    cursorBlink: true,
    scrollback: 5000,
  },
  connections: [],
};

const STORAGE_KEY = "noverterm-config";

export async function loadConfig(): Promise<AppConfig> {
  try {
    const settings = await invoke("get_all_settings");
    const configStr = (settings as Array<{ key: string; value: string }>).find(
      (s) => s.key === STORAGE_KEY
    )?.value;

    if (configStr) {
      return { ...DEFAULT_CONFIG, ...JSON.parse(configStr) };
    }
  } catch {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        return { ...DEFAULT_CONFIG, ...JSON.parse(stored) };
      }
    } catch {
      void 0;
    }
  }

  return DEFAULT_CONFIG;
}

export async function saveConfig(config: AppConfig): Promise<void> {
  try {
    await invoke("set_setting", {
      setting: {
        key: STORAGE_KEY,
        value: JSON.stringify(config),
      },
    });
  } catch {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
    } catch {
      void 0;
    }
  }
}

export async function saveConnections(connections: ConnectionConfig[]): Promise<void> {
  const config = await loadConfig();
  config.connections = connections;
  await saveConfig(config);
}

export async function addConnection(conn: Omit<ConnectionConfig, "id">): Promise<ConnectionConfig> {
  const config = await loadConfig();
  const newConn: ConnectionConfig = {
    ...conn,
    id: crypto.randomUUID(),
  };
  config.connections.push(newConn);
  await saveConfig(config);
  return newConn;
}

export async function updateConnection(conn: ConnectionConfig): Promise<void> {
  const config = await loadConfig();
  const idx = config.connections.findIndex((c) => c.id === conn.id);
  if (idx !== -1) {
    config.connections[idx] = conn;
    await saveConfig(config);
  }
}

export async function deleteConnection(id: string): Promise<void> {
  const config = await loadConfig();
  config.connections = config.connections.filter((c) => c.id !== id);
  await saveConfig(config);
}
