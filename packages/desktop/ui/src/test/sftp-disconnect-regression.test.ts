import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => vi.fn()),
}));

import { invoke } from "@tauri-apps/api/core";
import { createSftpStore } from "$lib/stores/sftp.svelte.js";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

describe("SFTP disconnection", () => {
  beforeEach(() => { vi.mocked(invoke).mockReset(); });

  it("retires an expired remote session even when the server cannot close it", async () => {
    const store = createSftpStore();
    vi.mocked(invoke).mockResolvedValueOnce("expired-sftp");
    await store.openSftp("ssh-1");
    store.remotePath = "/home/user";
    store.remoteFiles = [{ name: "old.txt", size: 12, modified: null, file_type: "File" }];
    vi.mocked(invoke).mockRejectedValueOnce(new Error("SFTP session not found: expired-sftp"));

    await store.disconnect();

    expect(store.isConnected).toBe(false);
    expect(store.sftpSessionId).toBeNull();
    expect(store.sshSessionId).toBeNull();
    expect(store.remotePath).toBe("");
    expect(store.remoteFiles).toEqual([]);
  });

  it("does not restore a closed machine's listing after a delayed response", async () => {
    const store = createSftpStore();
    const listing = deferred<unknown>();
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "sftp_open") return "sftp-1";
      if (command === "sftp_list_dir") return listing.promise;
      return undefined;
    });
    await store.openSftp("ssh-1");
    const navigation = store.navigateRemote("/old-machine");

    await store.disconnect();
    listing.resolve([{ name: "stale.txt", size: 10, modified: null, file_type: "File" }]);
    await navigation;

    expect(store.isConnected).toBe(false);
    expect(store.remoteFiles).toEqual([]);
    expect(store.remotePath).toBe("");
  });
});
