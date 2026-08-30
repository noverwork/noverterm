import { beforeEach, describe, expect, it, vi } from "vitest";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

import { deleteConnection, saveConnection } from "$lib/api/connections-api.js";

describe("connections API", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(null);
  });

  it("maps the form input onto the snake_case command payload", async () => {
    await saveConnection({
      id: "h1",
      name: "prod",
      groupId: "g1",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "  hunter2  ",
      privateKey: "PRIVATE KEY",
      passphrase: " secret ",
      keyName: "prod key",
      existingKeyId: "k1",
    });

    expect(mockInvoke).toHaveBeenCalledWith("host_save", {
      connection: {
        id: "h1",
        name: "prod",
        host: "prod.example.com",
        port: 22,
        username: "deploy",
        group_id: "g1",
        password: "hunter2",
        private_key: "PRIVATE KEY",
        passphrase: "secret",
        key_name: "prod key",
        existing_key_id: "k1",
      },
    });
  });

  it("keeps the stored password when the form leaves the field untouched", async () => {
    await saveConnection({
      id: "h1",
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "",
      preservedPassword: "stored-password",
    });

    expect(mockInvoke.mock.calls[0]?.[1]).toMatchObject({
      connection: { password: "stored-password" },
    });
  });

  it("deletes the connection's inline key along with the host", async () => {
    await deleteConnection({
      id: "h1",
      name: "prod",
      groupId: null,
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      sshKeyId: "k1",
      hasPassword: false,
      auth: null,
    });

    expect(mockInvoke).toHaveBeenCalledWith("host_delete", {
      id: "h1",
      sshKeyId: "k1",
    });
  });
});
