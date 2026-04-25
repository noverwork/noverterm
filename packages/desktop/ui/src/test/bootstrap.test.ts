import { beforeEach, describe, expect, it, vi } from "vitest";

const createMockApi = () => ({
  restore: vi.fn(),
  register: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
  loadBootstrapMetadata: vi.fn(),
  saveConnection: vi.fn(),
  deleteConnection: vi.fn(),
  saveSetting: vi.fn(),
  createKey: vi.fn(),
  updateKey: vi.fn(),
  deleteKey: vi.fn(),
});

import { createBootstrapStore } from "$lib/stores/bootstrap.svelte.js";

const sampleMetadata = {
  settings: [{ key: "noverterm-config", value: '{"terminal":{"fontSize":16},"recentConnectionIds":["h1"]}' }],
  hosts: [{
    id: "h1",
    name: "prod",
    host: "prod.example.com",
    port: 22,
    username: "deploy",
    ssh_key_id: "k1",
    auth: {
      kind: "public_key_and_password",
      private_key: "private-key",
      passphrase: null,
      password: "secret",
    },
  }],
  keys: [{ id: "k1", name: "deploy-key", kind: "ed25519", fingerprint: "SHA256:abc" }],
};

const sampleAuthStatus = { email: "alice", bootstrap_message: "bootstrap ready" };

describe("bootstrap store", () => {
  let mockApi: ReturnType<typeof createMockApi>;

  beforeEach(() => {
    mockApi = createMockApi();
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
      hasPassword: true,
      sshKeyId: "k1",
      auth: expect.objectContaining({ kind: "public_key_and_password" }),
    });
    expect(store.getRecentConnectionIds()).toEqual(["h1"]);
    expect(store.getRecentConnections()[0]).toMatchObject({ id: "h1" });
  });

  it("records recent connections in backend settings", async () => {
    mockApi.restore.mockResolvedValue(sampleAuthStatus);
    mockApi.loadBootstrapMetadata
      .mockResolvedValueOnce({ settings: [{ key: "noverterm-config", value: '{"terminal":{"fontSize":16}}' }], hosts: sampleMetadata.hosts, keys: [] })
      .mockResolvedValueOnce(sampleMetadata);
    mockApi.saveSetting.mockResolvedValue({ key: "noverterm-config", value: "" });

    const store = createBootstrapStore(mockApi);
    await store.init();
    await store.recordRecentConnection("h1");

    expect(mockApi.saveSetting).toHaveBeenCalledWith({
      key: "noverterm-config",
      value: '{"terminal":{"fontSize":16},"recentConnectionIds":["h1"]}',
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
});
