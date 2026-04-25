import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SvelteDate } from "svelte/reactivity";

import { commands as tauriCommands } from "../../bindings.js";
import { decryptSecret } from "$lib/crypto/vault.js";
import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";

export type SessionType = "ssh" | "local";
export type SessionStatus = "connecting" | "connected" | "disconnected" | "error";

export interface DirectConnectionInput {
  host: string;
  port: number;
  username: string;
  password?: string;
  privateKey?: string;
  passphrase?: string;
}

export interface Session {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  status: SessionStatus;
  type: SessionType;
  createdAt: Date;
  connectionId?: string | null;
  error?: string;
}

interface SessionState {
  sessions: Map<string, Session>;
  activeSessionId: string | null;
}

const state: SessionState = $state({
  sessions: new Map(),
  activeSessionId: null,
});

let eventUnlisten: UnlistenFn | null = null;
let localEventUnlisten: UnlistenFn | null = null;

function commitSessions() {
  state.sessions = new Map(state.sessions);
}

  function sessionName(host: string, port: number, username: string) {
    return `${username}@${host}:${port}`;
  }

  function savedConnectionSessionName(connection: Pick<ConnectionConfig, "id" | "name">) {
    const matchingSessions = Array.from(state.sessions.values()).filter(
      (session) => session.connectionId === connection.id && session.status !== "disconnected",
    );

    if (matchingSessions.length === 0) {
      return connection.name;
    }

    return `${connection.name} #${matchingSessions.length + 1}`;
  }

  async function connectionAuthInput(connection: ConnectionConfig): Promise<{
    password: string | null;
    privateKey: string | null;
    passphrase: string | null;
  }> {
    switch (connection.auth?.kind) {
      case "password":
        return {
          password: await decryptSecret(connection.auth.password),
          privateKey: null,
          passphrase: null,
        };
      case "public_key":
        return {
          password: null,
          privateKey: await decryptSecret(connection.auth.private_key),
          passphrase: await decryptSecret(connection.auth.passphrase),
        };
      case "public_key_and_password":
        return {
          password: await decryptSecret(connection.auth.password),
          privateKey: await decryptSecret(connection.auth.private_key),
          passphrase: await decryptSecret(connection.auth.passphrase),
        };
      default:
        throw new Error("host has no connectable authentication material");
    }
  }

function connectResponseError(response: { status: string; prompt?: { host: string; fingerprint: string }; mismatch?: { host: string; expected_fingerprint: string; presented_fingerprint: string } }) {
  if (response.status === "trust_required" && response.prompt) {
    return `Host trust confirmation required for ${response.prompt.host} (${response.prompt.fingerprint})`;
  }

  if (response.status === "trust_mismatch" && response.mismatch) {
    return `Host trust mismatch for ${response.mismatch.host}: expected ${response.mismatch.expected_fingerprint}, got ${response.mismatch.presented_fingerprint}`;
  }

  return "SSH connection failed";
}

export function createSessionStore() {
  async function init() {
    if (eventUnlisten) return;

    eventUnlisten = await listen(
      "ssh_output",
      (event: { payload: { session_id: string; output: string; closed: boolean } }) => {
        const { session_id, closed } = event.payload;
        const session = state.sessions.get(session_id);
        if (session) {
          if (closed) {
            session.status = "disconnected";
          } else if (session.status === "connecting") {
            session.status = "connected";
          }
          commitSessions();
        }
      },
    );

    localEventUnlisten = await listen(
      "local_output",
      (event: { payload: { session_id: string; output: string; closed: boolean } }) => {
        const { session_id, closed } = event.payload;
        const session = state.sessions.get(session_id);
        if (session) {
          if (closed) {
            session.status = "disconnected";
          } else if (session.status === "connecting") {
            session.status = "connected";
          }
          commitSessions();
        }
      },
    );
  }

  function addSession(session: Session) {
    state.sessions.set(session.id, session);
    state.activeSessionId = session.id;
    commitSessions();
  }

  function updateSession(id: string, updates: Partial<Session>) {
    const session = state.sessions.get(id);
    if (session) {
      Object.assign(session, updates);
      commitSessions();
    }
  }

  function removeSession(id: string) {
    state.sessions.delete(id);
    commitSessions();
    if (state.activeSessionId === id) {
      state.activeSessionId = state.sessions.size > 0 ? Array.from(state.sessions.keys())[0] : null;
    }
  }

  function setActiveSession(id: string | null) {
    state.activeSessionId = id;
  }

  function getActiveSession(): Session | undefined {
    if (!state.activeSessionId) return undefined;
    return state.sessions.get(state.activeSessionId);
  }

  function getSessions(): Session[] {
    return Array.from(state.sessions.values());
  }

  async function connectSavedConnection(
    connection: ConnectionConfig,
    cols: number = 80,
    rows: number = 24,
  ): Promise<string> {
    const tempId = crypto.randomUUID();
    addSession({
      id: tempId,
      name: savedConnectionSessionName(connection),
      host: connection.host,
      port: connection.port,
      username: connection.username,
      type: "ssh",
      status: "connecting",
      createdAt: new SvelteDate(),
      connectionId: connection.id,
    });

    const auth = await connectionAuthInput(connection);
    const result = await tauriCommands.sshConnectDirect(
      {
        host: connection.host,
        port: connection.port,
        username: connection.username,
        password: auth.password,
        private_key: auth.privateKey,
        passphrase: auth.passphrase,
      },
      cols,
      rows,
    );
    if (result.status === "error") {
      updateSession(tempId, { status: "error", error: result.error });
      throw new Error(result.error);
    }

    if (result.data.status !== "connected") {
      const error = connectResponseError(result.data);
      updateSession(tempId, { status: "error", error });
      throw new Error(error);
    }

    const sessionId = result.data.session_id;
    const session = state.sessions.get(tempId);
    if (!session) {
      return sessionId;
    }

    state.sessions.delete(tempId);
    state.sessions.set(sessionId, {
      ...session,
      id: sessionId,
      status: "connected",
      error: undefined,
    });
    state.activeSessionId = sessionId;
    commitSessions();
    return sessionId;
  }

  async function connectDirect(
    input: DirectConnectionInput,
    cols: number = 80,
    rows: number = 24,
  ): Promise<string> {
    const tempId = crypto.randomUUID();
    addSession({
      id: tempId,
      name: sessionName(input.host, input.port, input.username),
      host: input.host,
      port: input.port,
      username: input.username,
      type: "ssh",
      status: "connecting",
      createdAt: new SvelteDate(),
      connectionId: null,
    });

    const result = await tauriCommands.sshConnectDirect(
      {
        host: input.host,
        port: input.port,
        username: input.username,
        password: input.password?.trim() || null,
        private_key: input.privateKey?.trim() || null,
        passphrase: input.passphrase?.trim() || null,
      },
      cols,
      rows,
    );

    if (result.status === "error") {
      updateSession(tempId, { status: "error", error: result.error });
      throw new Error(result.error);
    }

    if (result.data.status !== "connected") {
      const error = connectResponseError(result.data);
      updateSession(tempId, { status: "error", error });
      throw new Error(error);
    }

    const sessionId = result.data.session_id;
    const session = state.sessions.get(tempId);
    if (!session) {
      return sessionId;
    }

    state.sessions.delete(tempId);
    state.sessions.set(sessionId, {
      ...session,
      id: sessionId,
      status: "connected",
      error: undefined,
    });
    state.activeSessionId = sessionId;
    commitSessions();
    return sessionId;
  }

  async function connectLocal(
    name: string = "Local Terminal",
    cols: number = 80,
    rows: number = 24,
  ): Promise<string> {
    const tempId = crypto.randomUUID();

    addSession({
      id: tempId,
      name,
      host: "localhost",
      port: 0,
      username: "",
      type: "local",
      status: "connecting",
      createdAt: new SvelteDate(),
      connectionId: null,
    });

    const result = await tauriCommands.localConnect(cols, rows);
    if (result.status === "error") {
      updateSession(tempId, { status: "error", error: result.error });
      throw new Error(result.error);
    }

    const sessionId = result.data;
    const session = state.sessions.get(tempId);
    if (!session) {
      return sessionId;
    }

    state.sessions.delete(tempId);
    state.sessions.set(sessionId, {
      ...session,
      id: sessionId,
      status: "connected",
      error: undefined,
    });
    state.activeSessionId = sessionId;
    commitSessions();
    return sessionId;
  }

  async function disconnectSession(sessionId: string) {
    const session = state.sessions.get(sessionId);
    try {
      if (session?.type === "local") {
        await tauriCommands.localDisconnect(sessionId);
      } else {
        await tauriCommands.sshDisconnect(sessionId);
      }
    } catch {
      // Ignore teardown failures and drop the local session state.
    }
    removeSession(sessionId);
  }

  async function writeSession(sessionId: string, data: string) {
    const session = state.sessions.get(sessionId);
    if (session?.type === "local") {
      await tauriCommands.localWrite(sessionId, data);
    } else {
      await tauriCommands.sshWrite(sessionId, data);
    }
  }

  async function resizeSession(sessionId: string, cols: number, rows: number) {
    const session = state.sessions.get(sessionId);
    if (session?.type === "local") {
      await tauriCommands.localResize(sessionId, cols, rows);
    } else {
      await tauriCommands.sshResize(sessionId, cols, rows);
    }
  }

  function disconnectConnectionSessions(connectionId: string) {
    const sessionIds = Array.from(state.sessions.values())
      .filter((session) => session.connectionId === connectionId)
      .map((session) => session.id);

    void Promise.all(sessionIds.map((sessionId) => disconnectSession(sessionId)));
  }

  function cleanup() {
    if (eventUnlisten) {
      eventUnlisten();
      eventUnlisten = null;
    }
    if (localEventUnlisten) {
      localEventUnlisten();
      localEventUnlisten = null;
    }
  }

  return {
    get sessions() {
      return state.sessions;
    },
    get activeSessionId() {
      return state.activeSessionId;
    },
    init,
    addSession,
    updateSession,
    removeSession,
    setActiveSession,
    getActiveSession,
    getSessions,
    connectSavedConnection,
    connectDirect,
    connectLocal,
    disconnectSession,
    disconnectConnectionSessions,
    writeSession,
    resizeSession,
    cleanup,
  };
}
