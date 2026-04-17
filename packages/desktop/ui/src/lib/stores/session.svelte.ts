import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type SessionStatus = "connecting" | "connected" | "disconnected" | "error";

export interface Session {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  status: SessionStatus;
  createdAt: Date;
  error?: string;
}

type SshConnectAuthPayload =
  | { type: "password"; password: string }
  | { type: "key"; key_path: string };

interface SessionState {
  sessions: Map<string, Session>;
  activeSessionId: string | null;
}

let state: SessionState = $state({
  sessions: new Map(),
  activeSessionId: null,
});

let eventUnlisten: UnlistenFn | null = null;

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
        }
      }
    );
  }

  function addSession(session: Session) {
    state.sessions.set(session.id, session);
  }

  function updateSession(id: string, updates: Partial<Session>) {
    const session = state.sessions.get(id);
    if (session) {
      Object.assign(session, updates);
    }
  }

  function removeSession(id: string) {
    state.sessions.delete(id);
    if (state.activeSessionId === id) {
      state.activeSessionId = state.sessions.size > 0
        ? Array.from(state.sessions.keys())[0]
        : null;
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

  async function connectToHost(
    host: string,
    port: number,
    username: string,
    authType: "password" | "key",
    credential: string,
    name: string,
    cols: number = 80,
    rows: number = 24
  ): Promise<string> {
    const tempId = crypto.randomUUID();
    const auth: SshConnectAuthPayload = authType === "password"
      ? { type: "password", password: credential }
      : { type: "key", key_path: credential };

    const session: Session = {
      id: tempId,
      name,
      host,
      port,
      username,
      status: "connecting",
      createdAt: new Date(),
    };
    addSession(session);
    state.activeSessionId = tempId;

    try {
      const sessionId: string = await invoke("ssh_connect", {
        host,
        port,
        user: username,
        auth,
        cols,
        rows,
      });

      const sess = state.sessions.get(tempId);
      if (sess) {
        state.sessions.delete(tempId);
        state.sessions.set(sessionId, {
          ...sess,
          id: sessionId,
          status: "connected",
          error: undefined,
        });
      }

      if (state.activeSessionId === tempId) {
        state.activeSessionId = sessionId;
      }

      return sessionId;
    } catch (e) {
      const sess = state.sessions.get(tempId);
      if (sess) {
        sess.status = "error";
        sess.error = e instanceof Error ? e.message : String(e);
      }
      throw e;
    }
  }

  async function disconnectSession(sessionId: string) {
    try {
      await invoke("ssh_disconnect", { sessionId });
    } catch {
      void 0;
    }
    removeSession(sessionId);
  }

  async function writeSession(sessionId: string, data: string) {
    await invoke("ssh_write", { sessionId, data });
  }

  async function resizeSession(sessionId: string, cols: number, rows: number) {
    await invoke("ssh_resize", { sessionId, cols, rows });
  }

  function cleanup() {
    if (eventUnlisten) {
      eventUnlisten();
      eventUnlisten = null;
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
    connectToHost,
    disconnectSession,
    writeSession,
    resizeSession,
    cleanup,
  };
}
