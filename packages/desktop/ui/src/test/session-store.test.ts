import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

type EventCallback = (event: { payload: unknown }) => void;

const eventListeners = new Map<string, EventCallback>();
const mockListen = vi.fn(
  (eventName: string, callback: EventCallback): Promise<() => void> => {
    eventListeners.set(eventName, callback);
    return Promise.resolve(() => {
      eventListeners.delete(eventName);
    });
  },
);

vi.mock("@tauri-apps/api/event", () => ({
  listen: (eventName: string, callback: EventCallback) =>
    mockListen(eventName, callback),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import {
  createSessionStore,
  type Session,
  type TerminalOutputPayload,
} from "$lib/stores/session.svelte.js";

function emitOutput(eventName: string, payload: TerminalOutputPayload): void {
  const listener = eventListeners.get(eventName);
  if (!listener) {
    throw new Error(`No listener registered for ${eventName}`);
  }

  listener({ payload });
}

function createLocalSession(id: string): Session {
  return {
    id,
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

describe("session output transcript", () => {
  const store = createSessionStore();

  beforeEach(async () => {
    vi.clearAllMocks();
    eventListeners.clear();
    await store.init();
  });

  afterEach(() => {
    store.removeSession("session-1");
    store.cleanup();
  });

  it("replays prior terminal output when a subscriber is recreated", () => {
    const payload: TerminalOutputPayload = {
      session_id: "session-1",
      output: [65, 32, 108, 105, 110, 101, 10],
      closed: false,
    };
    const firstSubscriberPayloads: TerminalOutputPayload[] = [];
    const recreatedSubscriberPayloads: TerminalOutputPayload[] = [];

    store.addSession(createLocalSession("session-1"));
    const unsubscribe = store.subscribeSessionOutput("session-1", (event) => {
      firstSubscriberPayloads.push(event);
    });

    emitOutput("local_output", payload);
    unsubscribe();

    store.subscribeSessionOutput("session-1", (event) => {
      recreatedSubscriberPayloads.push(event);
    });

    expect(firstSubscriberPayloads).toEqual([payload]);
    expect(recreatedSubscriberPayloads).toEqual([payload]);
  });
});
