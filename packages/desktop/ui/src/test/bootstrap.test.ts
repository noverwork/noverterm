import { beforeEach, describe, expect, it, vi } from "vitest";

const mockedState = vi.hoisted(() => ({
  commands: {
    bootstrapRestore: vi.fn(),
    bootstrapLoadMetadata: vi.fn(),
    bootstrapSaveConnection: vi.fn(),
    bootstrapDeleteConnection: vi.fn(),
    bootstrapSaveSetting: vi.fn(),
    authLogin: vi.fn(),
    authLogout: vi.fn(),
  },
}));

const createMockCommands = () => ({
  bootstrapRestore: vi.fn(),
  bootstrapLoadMetadata: vi.fn(),
  bootstrapSaveConnection: vi.fn(),
  bootstrapDeleteConnection: vi.fn(),
  bootstrapSaveSetting: vi.fn(),
  authLogin: vi.fn(),
  authLogout: vi.fn(),
});

let mockCommands = mockedState.commands;

vi.mock("$lib/bindings.js", () => ({
  commands: mockedState.commands,
}));

import { createBootstrapStore } from "$lib/stores/bootstrap.svelte.js";

const sampleMetadata = {
  settings: [{ key: "noverterm-config", value: '{"terminal":{"theme":"dark","fontSize":16}}' }],
  hosts: [{ id: "h1", name: "prod", host: "prod.example.com", port: 22, username: "deploy", auth_mode: "publickey_password", ssh_key_id: "k1" }],
  keys: [{ id: "k1", name: "deploy-key", kind: "ed25519", fingerprint: "SHA256:abc" }],
};

const sampleAuthStatus = { username: "alice", bootstrap_message: "bootstrap ready" };

describe("bootstrap store", () => {
  beforeEach(() => {
    Object.assign(mockedState.commands, createMockCommands());
    mockCommands = mockedState.commands;
    document.documentElement.classList.remove("dark");
  });

  it("starts in loading phase", () => {
    const store = createBootstrapStore(mockCommands);
    expect(store.phase).toBe("loading");
  });

  it("transitions to unauthenticated when bootstrap returns null", async () => {
    mockCommands.bootstrapRestore.mockResolvedValue({ status: "ok", data: null });
    const store = createBootstrapStore(mockCommands);
    await store.init();
    expect(store.phase).toBe("unauthenticated");
  });

  it("loads metadata after a successful restore", async () => {
    mockCommands.bootstrapRestore.mockResolvedValue({ status: "ok", data: sampleAuthStatus });
    mockCommands.bootstrapLoadMetadata.mockResolvedValue({ status: "ok", data: sampleMetadata });
    const store = createBootstrapStore(mockCommands);
    await store.init();
    expect(store.isAuthenticated).toBe(true);
    expect(store.getConnections()[0]).toMatchObject({
      id: "h1",
      authMode: "publickey_password",
      hasPassword: true,
      sshKeyId: "k1",
    });
  });

  it("keeps login failures recoverable", async () => {
    mockCommands.authLogin.mockResolvedValue({ status: "error", error: "invalid credentials" });
    const store = createBootstrapStore(mockCommands);
    await store.login("alice", "wrong");
    expect(store.isUnauthenticated).toBe(true);
    expect(store.error).toBe("invalid credentials");
  });

  it("refreshes metadata after saving a connection", async () => {
    mockCommands.bootstrapRestore.mockResolvedValue({ status: "ok", data: sampleAuthStatus });
    mockCommands.bootstrapLoadMetadata
      .mockResolvedValueOnce({ status: "ok", data: { settings: [], hosts: [], keys: [] } })
      .mockResolvedValueOnce({ status: "ok", data: sampleMetadata });
    mockCommands.bootstrapSaveConnection.mockResolvedValue({ status: "ok", data: sampleMetadata.hosts[0] });

    const store = createBootstrapStore(mockCommands);
    await store.init();
    await store.saveConnection({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "secret",
      existingKeyId: null,
    });

    expect(mockCommands.bootstrapSaveConnection).toHaveBeenCalledWith({
      id: null,
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "secret",
      private_key: null,
      passphrase: null,
      existing_key_id: null,
    });
    expect(store.getConnections()).toHaveLength(1);
  });

  it("passes connection deletion through and refreshes metadata", async () => {
    mockCommands.bootstrapRestore.mockResolvedValue({ status: "ok", data: sampleAuthStatus });
    mockCommands.bootstrapLoadMetadata
      .mockResolvedValueOnce({ status: "ok", data: sampleMetadata })
      .mockResolvedValueOnce({ status: "ok", data: { settings: [], hosts: [], keys: [] } });
    mockCommands.bootstrapDeleteConnection.mockResolvedValue({ status: "ok", data: null });

    const store = createBootstrapStore(mockCommands);
    await store.init();
    await store.deleteConnection(store.getConnections()[0]);

    expect(mockCommands.bootstrapDeleteConnection).toHaveBeenCalledWith("h1", "k1");
    expect(store.getConnections()).toHaveLength(0);
  });

  it("saves terminal config via backend settings and reapplies theme", async () => {
    mockCommands.bootstrapRestore.mockResolvedValue({ status: "ok", data: sampleAuthStatus });
    mockCommands.bootstrapLoadMetadata
      .mockResolvedValueOnce({ status: "ok", data: sampleMetadata })
      .mockResolvedValueOnce({
        status: "ok",
        data: {
          ...sampleMetadata,
          settings: [{ key: "noverterm-config", value: '{"terminal":{"theme":"light","fontSize":18}}' }],
        },
      });
    mockCommands.bootstrapSaveSetting.mockResolvedValue({
      status: "ok",
      data: { key: "noverterm-config", value: '{"terminal":{"theme":"light","fontSize":18}}' },
    });

    const store = createBootstrapStore(mockCommands);
    await store.init();
    await store.saveTerminalConfig({
      theme: "light",
      fontSize: 18,
      fontFamily: "JetBrains Mono, Fira Code, monospace",
      cursorStyle: "block",
      cursorBlink: true,
      scrollback: 5000,
    });

    expect(mockCommands.bootstrapSaveSetting).toHaveBeenCalledWith({
      key: "noverterm-config",
      value: '{"terminal":{"theme":"light","fontSize":18,"fontFamily":"JetBrains Mono, Fira Code, monospace","cursorStyle":"block","cursorBlink":true,"scrollback":5000}}',
    });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(store.getTerminalConfig().theme).toBe("light");
  });
});
