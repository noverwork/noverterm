import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SvelteDate, SvelteMap } from "svelte/reactivity";

import { commands as tauriCommands } from "../../bindings.js";
import type {
  HostTrustConfirmation,
  HostTrustMismatch,
  HostTrustPrompt,
  SshConnectResponse,
  SshPortForwardStatus,
} from "../../bindings.js";
import { decryptSecret } from "$lib/crypto/vault.js";
import type { ConnectionConfig } from "$lib/stores/bootstrap.svelte.js";

export type SessionType = "ssh" | "local";
export type SessionStatus = "connecting" | "connected" | "trust_required" | "disconnected" | "error";

export interface DirectConnectionInput {
  host: string;
  port: number;
  username: string;
  password?: string;
  privateKey?: string;
  passphrase?: string;
}

export interface StartLocalPortForwardInput {
  sessionId: string;
  bindHost: string;
  bindPort: number;
  targetHost: string;
  targetPort: number;
}

export type LocalPortForward = SshPortForwardStatus;

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
  trustPrompt?: HostTrustPrompt;
  trustMismatch?: HostTrustMismatch;
}

interface SessionState {
  sessions: SvelteMap<string, Session>;
  portForwards: SvelteMap<string, LocalPortForward>;
  activeSessionId: string | null;
}

const state: SessionState = $state({
  sessions: new SvelteMap(),
  portForwards: new SvelteMap(),
  activeSessionId: null,
});

let eventUnlisten: UnlistenFn | null = null;
let localEventUnlisten: UnlistenFn | null = null;
let portForwardEventUnlisten: UnlistenFn | null = null;

function updatePortForward(status: LocalPortForward) {
  state.portForwards.set(status.forward_id, status);
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

function connectResponseError(response: SshConnectResponse) {
  if (response.status === "trust_required" && response.prompt) {
    return `Host trust confirmation required for ${response.prompt.host} (${response.prompt.fingerprint})`;
  }

  if (response.status === "trust_mismatch" && response.mismatch) {
    return `Host trust mismatch for ${response.mismatch.host}: expected ${response.mismatch.expected_fingerprint}, got ${response.mismatch.presented_fingerprint}`;
  }

  return "SSH connection failed";
}

function connectResponseSessionUpdates(
  response: SshConnectResponse,
): Partial<Session> {
  const error = connectResponseError(response);

  if (response.status === "trust_required") {
    return {
      status: "trust_required",
      error: undefined,
      trustPrompt: response.prompt,
      trustMismatch: undefined,
    };
  }

  if (response.status === "trust_mismatch") {
    return {
      status: "error",
      error,
      trustPrompt: undefined,
      trustMismatch: response.mismatch,
    };
  }

  return { status: "error", error };
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
          updateSession(session_id, {
            status: closed
              ? "disconnected"
              : session.status === "connecting"
                ? "connected"
                : session.status,
          });
        }
      },
    );

    localEventUnlisten = await listen(
      "local_output",
      (event: { payload: { session_id: string; output: string; closed: boolean } }) => {
        const { session_id, closed } = event.payload;
        const session = state.sessions.get(session_id);
        if (session) {
          updateSession(session_id, {
            status: closed
              ? "disconnected"
              : session.status === "connecting"
                ? "connected"
                : session.status,
          });
        }
      },
    );

    portForwardEventUnlisten = await listen(
      "ssh_port_forward",
      (event: { payload: LocalPortForward }) => {
        updatePortForward(event.payload);
      },
    );
  }

  function addSession(session: Session) {
    state.sessions.set(session.id, session);
    state.activeSessionId = session.id;
  }

  function updateSession(id: string, updates: Partial<Session>) {
    const session = state.sessions.get(id);
    if (session) {
      state.sessions.set(id, { ...session, ...updates });
    }
  }

  function removeSession(id: string) {
    state.sessions.delete(id);
    for (const [forwardId, forward] of state.portForwards.entries()) {
      if (forward.session_id === id) {
        state.portForwards.delete(forwardId);
      }
    }

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

  function getPortForwardsForSession(sessionId: string): LocalPortForward[] {
    return Array.from(state.portForwards.values()).filter(
      (forward) => forward.session_id === sessionId,
    );
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
      const updates = connectResponseSessionUpdates(result.data);
      updateSession(tempId, updates);
      throw new Error(updates.error ?? "SSH connection failed");
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
      trustPrompt: undefined,
      trustMismatch: undefined,
    });
    state.activeSessionId = sessionId;
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
      const updates = connectResponseSessionUpdates(result.data);
      updateSession(tempId, updates);
      throw new Error(updates.error ?? "SSH connection failed");
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
      trustPrompt: undefined,
      trustMismatch: undefined,
    });
    state.activeSessionId = sessionId;
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

  async function startLocalPortForward(
    input: StartLocalPortForwardInput,
  ): Promise<LocalPortForward> {
    const result = await tauriCommands.sshStartLocalPortForward({
      session_id: input.sessionId,
      bind_host: input.bindHost,
      bind_port: input.bindPort,
      target_host: input.targetHost,
      target_port: input.targetPort,
    });

    if (result.status === "error") {
      throw new Error(result.error);
    }

    updatePortForward(result.data);
    return result.data;
  }

  async function stopLocalPortForward(
    sessionId: string,
    forwardId: string,
  ): Promise<LocalPortForward> {
    const result = await tauriCommands.sshStopPortForward(sessionId, forwardId);

    if (result.status === "error") {
      throw new Error(result.error);
    }

    updatePortForward(result.data);
    return result.data;
  }

  async function confirmHostTrust(confirmation: HostTrustConfirmation): Promise<void> {
    const result = await tauriCommands.sshConfirmHostTrust(confirmation);

    if (result.status === "error") {
      throw new Error(result.error);
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
    if (portForwardEventUnlisten) {
      portForwardEventUnlisten();
      portForwardEventUnlisten = null;
    }
  }

  return {
    get sessions() {
      return state.sessions;
    },
    get activeSessionId() {
      return state.activeSessionId;
    },
    get portForwards() {
      return state.portForwards;
    },
    init,
    addSession,
    updateSession,
    removeSession,
    setActiveSession,
    getActiveSession,
    getSessions,
    getPortForwardsForSession,
    connectSavedConnection,
    connectDirect,
    connectLocal,
    disconnectSession,
    disconnectConnectionSessions,
    writeSession,
    resizeSession,
    startLocalPortForward,
    stopLocalPortForward,
    confirmHostTrust,
    cleanup,
  };
}
