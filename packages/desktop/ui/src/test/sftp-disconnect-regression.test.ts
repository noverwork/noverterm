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
    await store.right.openSftp("ssh-1");
    store.right.path = "/home/user";
    store.right.files = [{ name: "old.txt", size: 12, modified: null, file_type: "File" }];
    vi.mocked(invoke).mockRejectedValueOnce(new Error("SFTP session not found: expired-sftp"));

    await store.right.disconnect();

    expect(store.right.isConnected).toBe(false);
    expect(store.right.sftpSessionId).toBeNull();
    expect(store.right.sshSessionId).toBeNull();
    expect(store.right.path).toBe("");
    expect(store.right.files).toEqual([]);
  });

  it("does not restore a closed machine's listing after a delayed response", async () => {
    const store = createSftpStore();
    const listing = deferred<unknown>();
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "sftp_open") return "sftp-1";
      if (command === "sftp_list_dir") return listing.promise;
      return undefined;
    });
    await store.right.openSftp("ssh-1");
    const navigation = store.right.navigate("/old-machine");

    await store.right.disconnect();
    listing.resolve([{ name: "stale.txt", size: 10, modified: null, file_type: "File" }]);
    await navigation;

    expect(store.right.isConnected).toBe(false);
    expect(store.right.files).toEqual([]);
    expect(store.right.path).toBe("");
  });
});
