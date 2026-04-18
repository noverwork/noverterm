import { describe, expect, it } from "vitest";

import {
  buildSignupSummary,
  findConnectionSession,
  validateSignupDraft,
} from "$lib/view-models/auth-and-sessions.js";

describe("desktop UI redesign view models", () => {
  it("validates signup as product account creation without password fields", () => {
    expect(
      validateSignupDraft({
        fullName: "",
        email: "",
        team: "",
        useCase: "",
      }),
    ).toEqual({
      fullName: "Full name is required",
      email: "Work email is required",
      team: "Workspace or team is required",
      useCase: "Tell us what infrastructure you need to access",
    });

    const summary = buildSignupSummary({
      fullName: "Alex Chen",
      email: "alex@company.com",
      team: "Platform",
      useCase: "Production SSH hosts",
    });

    expect(summary).toContain("Noverterm account setup");
    expect(summary).toContain("Alex Chen");
    expect(summary).not.toContain("Password provided");
    expect(summary).not.toContain("request access");
  });

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
