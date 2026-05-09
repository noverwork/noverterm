import type { AppDataMetadata, Setting, SshHostRecord } from "$lib/api/types.js";
import type {
  ConnectionConfig,
  SavedPortForwardConfig,
  TerminalConfig,
} from "$lib/app-data-types.js";

interface NovertermConfig {
  terminal?: Partial<TerminalConfig>;
  recentConnectionIds?: string[];
  savedPortForwards?: SavedPortForwardConfig[];
}

const DEFAULT_TERMINAL_CONFIG: TerminalConfig = {
  fontSize: 14,
  fontFamily:
    "Sarasa Term TC SemiBold, Menlo, Monaco, 'Courier New', monospace",
  cursorStyle: "block",
  cursorBlink: true,
  scrollback: 5000,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isCursorStyle(value: unknown): value is TerminalConfig["cursorStyle"] {
  return value === "block" || value === "underline" || value === "bar";
}

function parseTerminalPartial(value: unknown): Partial<TerminalConfig> | undefined {
  if (!isRecord(value)) {
    return undefined;
  }

  const terminal: Partial<TerminalConfig> = {};
  if (typeof value.fontSize === "number") {
    terminal.fontSize = value.fontSize;
  }
  if (typeof value.cursorBlink === "boolean") {
    terminal.cursorBlink = value.cursorBlink;
  }
  if (typeof value.scrollback === "number") {
    terminal.scrollback = value.scrollback;
  }
  if (isCursorStyle(value.cursorStyle)) {
    terminal.cursorStyle = value.cursorStyle;
  }

  return terminal;
}

function isSavedPortForward(value: unknown): value is SavedPortForwardConfig {
  return (
    isRecord(value) &&
    typeof value.id === "string" &&
    typeof value.name === "string" &&
    typeof value.connectionId === "string" &&
    typeof value.bind_host === "string" &&
    typeof value.bind_port === "number" &&
    typeof value.target_host === "string" &&
    typeof value.target_port === "number"
  );
}

export function normalizeTerminalConfig(
  config?: Partial<TerminalConfig>,
): TerminalConfig {
  return {
    ...DEFAULT_TERMINAL_CONFIG,
    ...config,
    fontFamily: DEFAULT_TERMINAL_CONFIG.fontFamily,
  };
}

export function parseNovertermConfig(settings: Setting[]): NovertermConfig {
  const configStr = settings.find(
    (setting) => setting.key === "noverterm-config",
  )?.value;
  if (!configStr) {
    return {};
  }

  try {
    const parsed: unknown = JSON.parse(configStr);
    if (!isRecord(parsed)) {
      return {};
    }

    const recentConnectionIds = Array.isArray(parsed.recentConnectionIds)
      ? parsed.recentConnectionIds.filter((id): id is string => typeof id === "string")
      : [];
    const savedPortForwards = Array.isArray(parsed.savedPortForwards)
      ? parsed.savedPortForwards.filter(isSavedPortForward)
      : undefined;

    return {
      terminal: parseTerminalPartial(parsed.terminal),
      recentConnectionIds,
      ...(savedPortForwards ? { savedPortForwards } : {}),
    };
  } catch {
    return {};
  }
}

export function buildNovertermConfig(
  settings: Setting[],
  updates: Partial<NovertermConfig>,
): string {
  return JSON.stringify({ ...parseNovertermConfig(settings), ...updates });
}

export function createPortForwardId(): string {
  return `pf-${globalThis.crypto.randomUUID()}`;
}

export function uniqueRecentConnectionIds(
  connectionId: string,
  currentIds: string[],
): string[] {
  return [
    connectionId,
    ...currentIds.filter((id) => id !== connectionId),
  ].slice(0, 12);
}

export function parseTerminalConfig(settings: Setting[]): TerminalConfig {
  return normalizeTerminalConfig(parseNovertermConfig(settings).terminal);
}

export function parseRecentConnectionIds(settings: Setting[]): string[] {
  return parseNovertermConfig(settings).recentConnectionIds ?? [];
}

export function mapHostsToConnections(
  hosts: SshHostRecord[],
): ConnectionConfig[] {
  return hosts.map((host) => ({
    id: host.id,
    name: host.name,
    groupId: host.group_id,
    host: host.host,
    port: host.port,
    username: host.username,
    sshKeyId: host.ssh_key_id,
    hasPassword:
      host.auth?.kind === "password" ||
      host.auth?.kind === "public_key_and_password",
    auth: host.auth,
  }));
}

export function filterExistingConnectionIds(
  connectionIds: string[],
  connections: ConnectionConfig[],
): string[] {
  const existingConnectionIds = new Set(
    connections.map((connection) => connection.id),
  );
  return connectionIds.filter((id) => existingConnectionIds.has(id));
}

export function filterExistingPortForwards(
  forwards: SavedPortForwardConfig[],
  connections: ConnectionConfig[],
): SavedPortForwardConfig[] {
  const existingConnectionIds = new Set(
    connections.map((connection) => connection.id),
  );
  return forwards.filter((forward) =>
    existingConnectionIds.has(forward.connectionId),
  );
}

export function selectConnections(
  metadata: AppDataMetadata | null,
): ConnectionConfig[] {
  return metadata ? mapHostsToConnections(metadata.hosts) : [];
}

export function selectTerminalConfig(
  metadata: AppDataMetadata | null,
): TerminalConfig {
  return metadata
    ? parseTerminalConfig(metadata.settings)
    : DEFAULT_TERMINAL_CONFIG;
}

export function selectRecentConnectionIds(
  metadata: AppDataMetadata | null,
): string[] {
  if (!metadata) {
    return [];
  }

  return filterExistingConnectionIds(
    parseRecentConnectionIds(metadata.settings),
    selectConnections(metadata),
  );
}

export function selectSavedPortForwards(
  metadata: AppDataMetadata | null,
): SavedPortForwardConfig[] {
  if (!metadata) {
    return [];
  }

  return filterExistingPortForwards(
    parseNovertermConfig(metadata.settings).savedPortForwards ?? [],
    selectConnections(metadata),
  );
}
