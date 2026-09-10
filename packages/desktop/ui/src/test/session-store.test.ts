import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, channels } = vi.hoisted(() => ({
  invoke: vi.fn(
    async (
      _command: string,
      _args?: Record<string, unknown>,
    ): Promise<unknown> => undefined,
  ),
  channels: [] as { id: number; onmessage: (message: unknown) => void }[],
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  Channel: class {
    id = channels.length;
    constructor(public onmessage: (message: unknown) => void) {
      channels.push(this);
    }
  },
}));

import {
  createSessionStore,
  type Session,
  type TerminalOutputPayload,
} from "$lib/stores/session.svelte.js";

const SESSION_ID = "00000000-0000-4000-8000-000000000001";
const encoder = new TextEncoder();

function emitOutput(output: Uint8Array, kind = 0): void {
  const frame = new Uint8Array(37 + output.length);
  frame[0] = kind;
  frame.set(encoder.encode(SESSION_ID), 1);
  frame.set(output, 37);
  channels[channels.length - 1].onmessage(frame.buffer);
}

function createLocalSession(): Session {
  return {
    id: SESSION_ID,
    name: "Local Terminal",
    host: "localhost",
    port: 0,
    username: "",
    status: "connected",
    type: "local",
    createdAt: new Date("2026-05-07T00:00:00.000Z"),
    connectionId: null,
  };
}

function acknowledgedBytes() {
  return invoke.mock.calls
    .filter(([command]) => command === "terminal_output_ack")
    .reduce(
      (sum, [, args]) =>
        sum + (typeof args?.bytes === "number" ? args.bytes : 0),
      0,
    );
}

describe("session output transport", () => {
  const store = createSessionStore();
  beforeEach(async () => {
    vi.clearAllMocks();
    invoke.mockImplementation(async () => undefined);
    await store.init();
    store.addSession(createLocalSession());
  });

  afterEach(() => {
    for (const session of store.getSessions()) store.removeSession(session.id);
    store.cleanup();
  });

  it("retains exact bytes for recreated subscribers without acknowledging replay", () => {
    const bytes = encoder.encode("a\r\n中文\x1b[31m");
    const first: Uint8Array[] = [];
    const unsubscribe = store.subscribeSessionOutput(
      SESSION_ID,
      (event, consumed) => {
        first.push(event.output);
        consumed?.();
      },
    );
    emitOutput(bytes);
    unsubscribe();
    const replay: Uint8Array[] = [];
    store.subscribeSessionOutput(SESSION_ID, (event) => {
      replay.push(event.output);
    });
    expect(first.flatMap((chunk) => Array.from(chunk))).toEqual(
      Array.from(bytes),
    );
    expect(replay.flatMap((chunk) => Array.from(chunk))).toEqual(
      Array.from(bytes),
    );
    expect(acknowledgedBytes()).toBe(bytes.length);
  });

  it("releases credit only after every view parses or unsubscribes", () => {
    let parsed: (() => void) | undefined;
    store.subscribeSessionOutput(SESSION_ID, (_event, consumed) => {
      parsed = consumed;
    });
    const unsubscribe = store.subscribeSessionOutput(SESSION_ID, () => {});
    emitOutput(encoder.encode("abc"));
    expect(acknowledgedBytes()).toBe(0);
    parsed!();
    parsed!();
    expect(acknowledgedBytes()).toBe(0);
    unsubscribe();
    expect(acknowledgedBytes()).toBe(3);
  });

  it("acknowledges unmounted output and replays data before terminal failure", () => {
    emitOutput(encoder.encode("last bytes"));
    emitOutput(encoder.encode("PTY write failed"), 2);
    const replay: TerminalOutputPayload[] = [];
    store.subscribeSessionOutput(SESSION_ID, (event) => {
      replay.push(event);
    });
    expect(
      replay.map((event) => new TextDecoder().decode(event.output)),
    ).toEqual(["last bytes", ""]);
    expect(replay[1].error).toBe("PTY write failed");
    expect(store.sessions.get(SESSION_ID)?.status).toBe("error");
    expect(acknowledgedBytes()).toBe(10);
  });

  it("bounds retained transcript to 10 MiB while preserving its newest bytes", () => {
    for (let index = 0; index < 161; index++) {
      emitOutput(new Uint8Array(64 * 1024).fill(index));
    }
    const replay: Uint8Array[] = [];
    store.subscribeSessionOutput(SESSION_ID, (event) => {
      replay.push(event.output);
    });
    expect(replay.reduce((sum, chunk) => sum + chunk.length, 0)).toBe(
      10 * 1024 * 1024,
    );
    expect(replay[0]).toEqual(new Uint8Array(64 * 1024).fill(1));
    expect(replay[replay.length - 1]).toEqual(
      new Uint8Array(64 * 1024).fill(160),
    );
  });

  it("preserves a close that arrives before the connect command completes", async () => {
    store.removeSession(SESSION_ID);
    store.cleanup();
    await store.init();
    invoke.mockImplementation(async (command) => {
      if (command === "local_connect") {
        emitOutput(encoder.encode("bye"));
        emitOutput(new Uint8Array(), 1);
        return SESSION_ID;
      }
      return undefined;
    });
    await store.connectLocal();
    expect(store.sessions.get(SESSION_ID)?.status).toBe("disconnected");
  });

  it("reports actual input failure to session state and callers", async () => {
    invoke.mockImplementation(async (command) => {
      if (command === "local_write") throw "PTY closed";
      return undefined;
    });
    await expect(store.writeSession(SESSION_ID, "x")).rejects.toThrow(
      "PTY closed",
    );
    expect(store.sessions.get(SESSION_ID)).toMatchObject({
      status: "error",
      error: "PTY closed",
    });
  });
});
