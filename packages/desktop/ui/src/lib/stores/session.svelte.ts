import { Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SvelteDate, SvelteMap, SvelteSet } from "svelte/reactivity";

import { commands as tauriCommands } from "../../bindings.js";
import type {
  HostTrustConfirmation,
  HostTrustMismatch,
  HostTrustPrompt,
  SshConnectResponse,
  SshPortForwardStatus,
} from "../../bindings.js";
import { createDirectSshConnectInput } from "$lib/services/ssh-connection-input.js";
import type { ConnectionConfig } from "$lib/app-data-types.js";
import { writeTerminalInput } from "$lib/terminal/input.js";

export type SessionType = "ssh" | "local";
export type SessionStatus =
  | "connecting"
  | "connected"
  | "trust_required"
  | "disconnected"
  | "error";

export interface TerminalOutputPayload {
  session_id: string;
  output: Uint8Array;
  closed: boolean;
  error?: string;
}

export type TerminalOutputCallback = (
  payload: TerminalOutputPayload,
  consumed?: () => void,
) => void;

interface TerminalTranscript {
  chunks: Uint8Array[];
  tailLength: number;
}

interface OutputSubscriber {
  callback: TerminalOutputCallback;
  pending: Set<() => void>;
}

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

let outputChannel: Channel<unknown> | null = null;
let portForwardEventUnlisten: UnlistenFn | null = null;
let initPromise: Promise<void> | null = null;
let outputGeneration = 0;

const closedOutput = new SvelteMap<string, TerminalOutputPayload>();
const removedSessions = new SvelteSet<string>();
const terminalTranscripts = new SvelteMap<string, TerminalTranscript>();
const outputSubscribers = new SvelteMap<string, Set<OutputSubscriber>>();
const outputDecoder = new TextDecoder();
const TRANSCRIPT_CHUNK_BYTES = 64 * 1024;
const MAX_TRANSCRIPT_CHUNKS = 160;

function appendTerminalTranscript(payload: TerminalOutputPayload) {
  if (payload.output.length === 0) return;
  const transcript = terminalTranscripts.get(payload.session_id) ?? {
    chunks: [],
    tailLength: TRANSCRIPT_CHUNK_BYTES,
  };
  let offset = 0;
  while (offset < payload.output.length) {
    if (transcript.tailLength === TRANSCRIPT_CHUNK_BYTES) {
      transcript.chunks.push(new Uint8Array(TRANSCRIPT_CHUNK_BYTES));
      transcript.tailLength = 0;
      if (transcript.chunks.length > MAX_TRANSCRIPT_CHUNKS) {
        transcript.chunks.shift();
      }
    }
    const length = Math.min(
      TRANSCRIPT_CHUNK_BYTES - transcript.tailLength,
      payload.output.length - offset,
    );
    transcript.chunks[transcript.chunks.length - 1].set(
      payload.output.subarray(offset, offset + length),
      transcript.tailLength,
    );
    transcript.tailLength += length;
    offset += length;
  }
  terminalTranscripts.set(payload.session_id, transcript);
}

function reportSessionError(sessionId: string, error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  const session = state.sessions.get(sessionId);
  if (session) {
    state.sessions.set(sessionId, {
      ...session,
      status: "error",
      error: message,
    });
  }
  console.error("[terminal]", sessionId, message);
}

function handleOutputFrame(frame: unknown) {
  if (!(frame instanceof ArrayBuffer)) {
    throw new Error("Invalid terminal output frame");
  }
  const bytes = new Uint8Array(frame);
  if (bytes.length < 37 || bytes[0] > 2) {
    throw new Error("Invalid terminal output frame");
  }
  const sessionId = outputDecoder.decode(bytes.subarray(1, 37));
  handleTerminalOutput({
    session_id: sessionId,
    output: bytes[0] === 0 ? bytes.subarray(37) : new Uint8Array(),
    closed: bytes[0] !== 0,
    error:
      bytes[0] === 2 ? outputDecoder.decode(bytes.subarray(37)) : undefined,
  });
}

function updatePortForward(status: LocalPortForward) {
  state.portForwards.set(status.forward_id, status);
}

function handleTerminalOutput(payload: TerminalOutputPayload) {
  const removed = removedSessions.has(payload.session_id);
  if (!removed) {
    appendTerminalTranscript(payload);
    if (payload.closed) closedOutput.set(payload.session_id, payload);
  } else if (payload.closed) {
    removedSessions.delete(payload.session_id);
  }

  const subscribers = outputSubscribers.get(payload.session_id);
  let remaining = subscribers?.size ?? 0;
  const acknowledge = () => {
    if (payload.output.length === 0) return;
    void tauriCommands
      .terminalOutputAck(payload.session_id, payload.output.length)
      .then((result) => {
        if (result.status === "error") {
          reportSessionError(payload.session_id, result.error);
        }
      })
      .catch((error: unknown) => reportSessionError(payload.session_id, error));
  };
  if (removed) {
    acknowledge();
    return;
  }
  if (remaining === 0) {
    acknowledge();
  } else {
    // Every view must finish parsing before the backend can reuse its credit.
    for (const subscriber of Array.from(subscribers!)) {
      let consumed = false;
      const done = () => {
        if (consumed) return;
        consumed = true;
        subscriber.pending.delete(done);
        if (--remaining === 0) acknowledge();
      };
      subscriber.pending.add(done);
      if (!outputSubscribers.get(payload.session_id)?.has(subscriber)) {
        done();
        continue;
      }
      try {
        subscriber.callback(payload, done);
      } catch (error) {
        done();
        reportSessionError(payload.session_id, error);
      }
    }
  }

  const session = state.sessions.get(payload.session_id);
  if (payload.error) {
    reportSessionError(payload.session_id, payload.error);
  } else if (session && session.status !== "error") {
    const status = payload.closed
      ? "disconnected"
      : session.status === "connecting"
        ? "connected"
        : session.status;
    if (status !== session.status) {
      state.sessions.set(payload.session_id, { ...session, status });
    }
  }
}

function sessionName(host: string, port: number, username: string) {
  return `${username}@${host}:${port}`;
}

function savedConnectionSessionName(
  connection: Pick<ConnectionConfig, "id" | "name">,
) {
  const matchingSessions = Array.from(state.sessions.values()).filter(
    (session) =>
      session.connectionId === connection.id &&
      session.status !== "disconnected",
  );

  if (matchingSessions.length === 0) {
    return connection.name;
  }

  return `${connection.name} #${matchingSessions.length + 1}`;
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
    if (outputChannel && portForwardEventUnlisten) return;
    if (initPromise) return initPromise;
    const generation = outputGeneration;

    initPromise = (async () => {
      if (!outputChannel) {
        const channel = new Channel<unknown>(handleOutputFrame);
        const result = await tauriCommands.terminalOutputSubscribe(channel);
        if (result.status === "error") throw new Error(result.error);
        if (generation !== outputGeneration) {
          await tauriCommands.terminalOutputUnsubscribe(channel.id);
          throw new Error("Terminal output subscription cancelled");
        }
        outputChannel = channel;
      }

      if (!portForwardEventUnlisten) {
        const unlisten = await listen(
          "ssh_port_forward",
          (event: { payload: LocalPortForward }) => {
            updatePortForward(event.payload);
          },
        );
        if (generation !== outputGeneration) {
          unlisten();
          throw new Error("Terminal output subscription cancelled");
        }
        portForwardEventUnlisten = unlisten;
      }
    })();

    try {
      await initPromise;
    } finally {
      if (generation === outputGeneration) initPromise = null;
    }
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

  function finishConnection(pendingSessionId: string, sessionId: string) {
    const session = state.sessions.get(pendingSessionId);
    if (!session) return sessionId;
    const closed = closedOutput.get(sessionId);
    state.sessions.delete(pendingSessionId);
    state.sessions.set(sessionId, {
      ...session,
      id: sessionId,
      status: closed?.error ? "error" : closed ? "disconnected" : "connected",
      error: closed?.error,
      trustPrompt: undefined,
      trustMismatch: undefined,
    });
    state.activeSessionId = sessionId;
    return sessionId;
  }

  function removeSession(id: string) {
    state.sessions.delete(id);
    if (!closedOutput.has(id)) removedSessions.add(id);
    closedOutput.delete(id);
    terminalTranscripts.delete(id);
    for (const subscriber of outputSubscribers.get(id) ?? []) {
      for (const done of subscriber.pending) done();
    }
    outputSubscribers.delete(id);
    for (const [forwardId, forward] of state.portForwards.entries()) {
      if (forward.session_id === id) {
        state.portForwards.delete(forwardId);
      }
    }

    if (state.activeSessionId === id) {
      state.activeSessionId =
        state.sessions.size > 0 ? Array.from(state.sessions.keys())[0] : null;
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

  function subscribeSessionOutput(
    sessionId: string,
    callback: TerminalOutputCallback,
  ) {
    const subscribers =
      outputSubscribers.get(sessionId) ?? new SvelteSet<OutputSubscriber>();
    const subscriber: OutputSubscriber = { callback, pending: new SvelteSet() };
    subscribers.add(subscriber);
    outputSubscribers.set(sessionId, subscribers);

    try {
      const transcript = terminalTranscripts.get(sessionId);
      if (transcript) {
        for (let index = 0; index < transcript.chunks.length; index++) {
          const chunk = transcript.chunks[index];
          const output =
            index === transcript.chunks.length - 1
              ? chunk.subarray(0, transcript.tailLength)
              : chunk;
          callback({ session_id: sessionId, output, closed: false });
        }
      }
      const closed = closedOutput.get(sessionId);
      if (closed) callback(closed);
    } catch (error) {
      subscribers.delete(subscriber);
      if (subscribers.size === 0) outputSubscribers.delete(sessionId);
      throw error;
    }

    return () => {
      subscribers.delete(subscriber);
      for (const done of subscriber.pending) done();
      if (subscribers.size === 0) {
        outputSubscribers.delete(sessionId);
      }
    };
  }

  async function connectSavedConnection(
    connection: ConnectionConfig,
    cols: number = 80,
    rows: number = 24,
  ): Promise<string> {
    await init();

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

    return await connectSavedConnectionInSession(
      tempId,
      connection,
      cols,
      rows,
    );
  }

  async function retrySavedConnection(
    sessionId: string,
    connection: ConnectionConfig,
    cols: number = 80,
    rows: number = 24,
  ): Promise<string> {
    await init();

    const session = state.sessions.get(sessionId);
    if (!session) {
      throw new Error("Session not found");
    }

    state.activeSessionId = sessionId;
    updateSession(sessionId, {
      name: session.name,
      host: connection.host,
      port: connection.port,
      username: connection.username,
      type: "ssh",
      status: "connecting",
      error: undefined,
      trustPrompt: undefined,
      trustMismatch: undefined,
      connectionId: connection.id,
    });

    return await connectSavedConnectionInSession(
      sessionId,
      connection,
      cols,
      rows,
    );
  }

  async function connectSavedConnectionInSession(
    pendingSessionId: string,
    connection: ConnectionConfig,
    cols: number,
    rows: number,
  ): Promise<string> {
    const result = await tauriCommands.sshConnectDirect(
      await createDirectSshConnectInput(connection),
      cols,
      rows,
    );
    if (result.status === "error") {
      updateSession(pendingSessionId, { status: "error", error: result.error });
      throw new Error(result.error);
    }

    if (result.data.status !== "connected") {
      const updates = connectResponseSessionUpdates(result.data);
      updateSession(pendingSessionId, updates);
      throw new Error(updates.error ?? "SSH connection failed");
    }

    return finishConnection(pendingSessionId, result.data.session_id);
  }

  async function connectDirect(
    input: DirectConnectionInput,
    cols: number = 80,
    rows: number = 24,
  ): Promise<string> {
    await init();

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

    return finishConnection(tempId, result.data.session_id);
  }

  async function connectLocal(
    name: string = "Local Terminal",
    cols: number = 80,
    rows: number = 24,
  ): Promise<string> {
    await init();

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

    return finishConnection(tempId, result.data);
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
    if (!session) throw new Error("Session not found");
    try {
      await writeTerminalInput(sessionId, session.type, data);
    } catch (error) {
      reportSessionError(sessionId, error);
      throw error;
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

  async function confirmHostTrust(
    confirmation: HostTrustConfirmation,
  ): Promise<void> {
    const result = await tauriCommands.sshConfirmHostTrust(confirmation);

    if (result.status === "error") {
      throw new Error(result.error);
    }
  }

  function disconnectConnectionSessions(connectionId: string) {
    const sessionIds = Array.from(state.sessions.values())
      .filter((session) => session.connectionId === connectionId)
      .map((session) => session.id);

    void Promise.all(
      sessionIds.map((sessionId) => disconnectSession(sessionId)),
    );
  }

  function cleanup() {
    outputGeneration += 1;
    if (outputChannel) {
      const channel = outputChannel;
      outputChannel = null;
      channel.onmessage = () => {};
      void tauriCommands
        .terminalOutputUnsubscribe(channel.id)
        .catch(console.error);
    }
    if (portForwardEventUnlisten) {
      portForwardEventUnlisten();
      portForwardEventUnlisten = null;
    }
    initPromise = null;
    closedOutput.clear();
    removedSessions.clear();
    terminalTranscripts.clear();
    for (const subscribers of outputSubscribers.values()) {
      for (const subscriber of subscribers) {
        for (const done of subscriber.pending) done();
      }
    }
    outputSubscribers.clear();
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
    subscribeSessionOutput,
    connectSavedConnection,
    retrySavedConnection,
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
