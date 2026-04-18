import { describe, expect, it } from "vitest";

import { findConnectionSession } from "$lib/view-models/auth-and-sessions.js";

describe("desktop UI redesign view models", () => {
  it("matches saved connection sessions by connectionId before display name", () => {
    const session = findConnectionSession(
      [
        {
          id: "session-1",
          name: "Custom Runtime Label",
          status: "connected",
          connectionId: "conn-1",
        },
      ],
      {
        id: "conn-1",
        host: "prod.example.com",
        port: 22,
        username: "deploy",
      },
    );

    expect(session?.id).toBe("session-1");
    expect(session?.status).toBe("connected");
  });
});
