import { beforeEach, describe, expect, it, vi } from "vitest";

const mockedState = vi.hoisted(() => ({
  api: {
    restore: vi.fn(),
    register: vi.fn(),
    login: vi.fn(),
    logout: vi.fn(),
    loadBootstrapMetadata: vi.fn(),
    saveConnection: vi.fn(),
    deleteConnection: vi.fn(),
    saveSetting: vi.fn(),
    issueConnectionMaterial: vi.fn(),
  },
}));

const createMockApi = () => ({
  restore: vi.fn(),
  register: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
  loadBootstrapMetadata: vi.fn(),
  saveConnection: vi.fn(),
  deleteConnection: vi.fn(),
  saveSetting: vi.fn(),
  issueConnectionMaterial: vi.fn(),
});

let mockApi = mockedState.api;

vi.mock("$lib/api/backend-api.js", () => ({
  backendApi: mockedState.api,
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
    Object.assign(mockedState.api, createMockApi());
    mockApi = mockedState.api;
    document.documentElement.classList.remove("dark");
  });

  it("starts in loading phase", () => {
    const store = createBootstrapStore(mockApi);
    expect(store.phase).toBe("loading");
  });

  it("transitions to unauthenticated when restore returns null", async () => {
    mockApi.restore.mockResolvedValue(null);
    const store = createBootstrapStore(mockApi);
    await store.init();
    expect(store.phase).toBe("unauthenticated");
  });

  it("loads metadata after a successful restore", async () => {
    mockApi.restore.mockResolvedValue(sampleAuthStatus);
    mockApi.loadBootstrapMetadata.mockResolvedValue(sampleMetadata);
    const store = createBootstrapStore(mockApi);
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
    mockApi.login.mockRejectedValue(new Error("invalid credentials"));
    const store = createBootstrapStore(mockApi);
    await store.login("alice", "wrong");
    expect(store.isUnauthenticated).toBe(true);
    expect(store.error).toBe("invalid credentials");
  });

  it("refreshes metadata after saving a connection", async () => {
    mockApi.restore.mockResolvedValue(sampleAuthStatus);
    mockApi.loadBootstrapMetadata
      .mockResolvedValueOnce({ settings: [], hosts: [], keys: [] })
      .mockResolvedValueOnce(sampleMetadata);
    mockApi.saveConnection.mockResolvedValue(sampleMetadata.hosts[0]);

    const store = createBootstrapStore(mockApi);
    await store.init();
    await store.saveConnection({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "secret",
      existingKeyId: null,
    });

    expect(mockApi.saveConnection).toHaveBeenCalledWith({
      name: "prod",
      host: "prod.example.com",
      port: 22,
      username: "deploy",
      password: "secret",
      existingKeyId: null,
    });
    expect(store.getConnections()).toHaveLength(1);
  });

  it("passes connection deletion through and refreshes metadata", async () => {
    mockApi.restore.mockResolvedValue(sampleAuthStatus);
    mockApi.loadBootstrapMetadata
      .mockResolvedValueOnce(sampleMetadata)
      .mockResolvedValueOnce({ settings: [], hosts: [], keys: [] });
    mockApi.deleteConnection.mockResolvedValue(undefined);

    const store = createBootstrapStore(mockApi);
    await store.init();
    await store.deleteConnection(store.getConnections()[0]);

    expect(mockApi.deleteConnection).toHaveBeenCalledWith(
      expect.objectContaining({ id: "h1", sshKeyId: "k1" }),
    );
    expect(store.getConnections()).toHaveLength(0);
  });

  it("saves terminal config via backend settings and reapplies theme", async () => {
    mockApi.restore.mockResolvedValue(sampleAuthStatus);
    mockApi.loadBootstrapMetadata
      .mockResolvedValueOnce(sampleMetadata)
      .mockResolvedValueOnce({
        ...sampleMetadata,
        settings: [{ key: "noverterm-config", value: '{"terminal":{"theme":"light","fontSize":18}}' }],
      });
    mockApi.saveSetting.mockResolvedValue({
      key: "noverterm-config",
      value: '{"terminal":{"theme":"light","fontSize":18}}',
    });

    const store = createBootstrapStore(mockApi);
    await store.init();
    await store.saveTerminalConfig({
      theme: "light",
      fontSize: 18,
      fontFamily: "JetBrains Mono, Fira Code, monospace",
      cursorStyle: "block",
      cursorBlink: true,
      scrollback: 5000,
    });

    expect(mockApi.saveSetting).toHaveBeenCalledWith({
      key: "noverterm-config",
      value: '{"terminal":{"theme":"light","fontSize":18,"fontFamily":"JetBrains Mono, Fira Code, monospace","cursorStyle":"block","cursorBlink":true,"scrollback":5000}}',
    });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(store.getTerminalConfig().theme).toBe("light");
  });
});
