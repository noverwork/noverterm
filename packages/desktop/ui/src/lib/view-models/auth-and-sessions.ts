import type { ConnectionConfig } from "$lib/app-data-types.js";
import type { SessionStatus } from "$lib/stores/session.svelte.js";

export interface SessionLookup {
  id: string;
  name: string;
  status: SessionStatus;
  connectionId?: string | null;
}

export function findConnectionSession(
  sessions: Iterable<SessionLookup>,
  connection: Pick<ConnectionConfig, "id" | "username" | "host" | "port">,
): SessionLookup | undefined {
  const fallbackName = `${connection.username}@${connection.host}:${connection.port}`;
  return Array.from(sessions).find(
    (session) => session.connectionId === connection.id || session.name === fallbackName,
  );
}
