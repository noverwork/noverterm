import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => void 0),
}));

describe("createSessionStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("keeps the session map key aligned with the real session id after connect", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const { createSessionStore } = await import("./session.svelte.js");

    vi.mocked(invoke).mockResolvedValue("real-session-id");

    const store = createSessionStore();
    const connectedSessionId = await store.connectToHost(
      "example.com",
      22,
      "tester",
      "password",
      "secret",
      "tester@example.com:22",
    );

    expect(connectedSessionId).toBe("real-session-id");
    expect(store.activeSessionId).toBe("real-session-id");
    expect(store.sessions.size).toBe(1);

    const [[mapKey, session]] = Array.from(store.sessions.entries());

    expect(mapKey).toBe("real-session-id");
    expect(session.id).toBe("real-session-id");
    expect(store.sessions.get(store.activeSessionId ?? "")?.id).toBe("real-session-id");
  });
});
