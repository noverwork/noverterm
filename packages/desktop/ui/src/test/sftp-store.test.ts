import { beforeEach, describe, expect, it, vi } from "vitest";

import type {
  FileEntry,
  TransferError,
  TransferProgress,
} from "$lib/types/sftp.js";

type EventCallback<T = unknown> = (event: { payload: T }) => void;

const eventListeners = new Map<string, EventCallback>();
const unlistenFns = new Map<string, ReturnType<typeof vi.fn>>();
const mockListen = vi.fn(
  (eventName: string, callback: EventCallback): Promise<() => void> => {
    eventListeners.set(eventName, callback);
    const unlisten = vi.fn(() => {
      eventListeners.delete(eventName);
    });
    unlistenFns.set(eventName, unlisten);
    return Promise.resolve(unlisten);
  },
);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (eventName: string, callback: EventCallback) =>
    mockListen(eventName, callback),
}));

import { invoke } from "@tauri-apps/api/core";
import { createSftpStore, type SftpStore } from "$lib/stores/sftp.svelte.js";

const testFile: FileEntry = {
  name: "test.txt",
  size: 100,
  modified: 1234,
  file_type: "File",
};

function emitEvent<T>(eventName: string, payload: T): void {
  const listener = eventListeners.get(eventName);
  if (!listener) {
    throw new Error(`No listener registered for ${eventName}`);
  }

  listener({ payload });
}

describe("sftpStore", () => {
  let store: SftpStore;

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockReset();
    eventListeners.clear();
    unlistenFns.clear();
    store = createSftpStore();
  });

  it("initializes with empty state", () => {
    expect(store.localPath).toBe("~");
    expect(store.remotePath).toBe("");
    expect(store.localFiles).toEqual([]);
    expect(store.remoteFiles).toEqual([]);
    expect(store.localLoading).toBe(false);
    expect(store.remoteLoading).toBe(false);
    expect(store.localError).toBeNull();
    expect(store.remoteError).toBeNull();
    expect(store.activeTransfers.size).toBe(0);
    expect(store.selectedLocal).toBeNull();
    expect(store.selectedRemote).toBeNull();
    expect(store.sftpSessionId).toBeNull();
    expect(store.sshSessionId).toBeNull();
  });

  it("navigates local directory", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([testFile]);

    await store.navigateLocal("/tmp");

    expect(invoke).toHaveBeenCalledWith("local_list_dir", { path: "/tmp" });
    expect(store.localPath).toBe("/tmp");
    expect(store.localFiles).toEqual([testFile]);
    expect(store.localLoading).toBe(false);
    expect(store.localError).toBeNull();
  });

  it("records local navigation errors", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("permission denied"));

    await store.navigateLocal("/root");

    expect(store.localError).toBe("permission denied");
    expect(store.localLoading).toBe(false);
  });

  it("opens and closes an SFTP session", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("sftp-1").mockResolvedValueOnce(undefined);

    await store.openSftp("ssh-1");

    expect(invoke).toHaveBeenCalledWith("sftp_open", { sessionId: "ssh-1" });
    expect(store.sshSessionId).toBe("ssh-1");
    expect(store.sftpSessionId).toBe("sftp-1");
    expect(mockListen).toHaveBeenCalledWith("sftp://progress", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("sftp://complete", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("sftp://error", expect.any(Function));

    await store.closeSftp();

    expect(invoke).toHaveBeenCalledWith("sftp_close", { sessionId: "sftp-1" });
    expect(store.sftpSessionId).toBeNull();
    expect(store.sshSessionId).toBeNull();
    expect(unlistenFns.get("sftp://progress")).toHaveBeenCalledOnce();
    expect(unlistenFns.get("sftp://complete")).toHaveBeenCalledOnce();
    expect(unlistenFns.get("sftp://error")).toHaveBeenCalledOnce();
  });

  it("navigates remote directory when SFTP is open", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("sftp-1").mockResolvedValueOnce([testFile]);

    await store.openSftp("ssh-1");
    await store.navigateRemote("/home/user");

    expect(invoke).toHaveBeenCalledWith("sftp_list_dir", {
      sessionId: "sftp-1",
      path: "/home/user",
    });
    expect(store.remotePath).toBe("/home/user");
    expect(store.remoteFiles).toEqual([testFile]);
    expect(store.remoteError).toBeNull();
  });

  it("starts uploads and downloads with joined paths", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce("sftp-1")
      .mockResolvedValueOnce("upload-1")
      .mockResolvedValueOnce("download-1");

    await store.openSftp("ssh-1");
    store.localPath = "/local";
    store.remotePath = "/remote";

    await expect(store.startUpload(testFile)).resolves.toBe("upload-1");
    await expect(store.startDownload(testFile)).resolves.toBe("download-1");

    expect(invoke).toHaveBeenCalledWith("sftp_upload", {
      sessionId: "sftp-1",
      localPath: "/local/test.txt",
      remotePath: "/remote/test.txt",
    });
    expect(invoke).toHaveBeenCalledWith("sftp_download", {
      sessionId: "sftp-1",
      remotePath: "/remote/test.txt",
      localPath: "/local/test.txt",
    });
  });

  it("tracks progress events and removes completed transfers", async () => {
    vi.mocked(invoke).mockResolvedValue("sftp-1");
    await store.openSftp("ssh-1");

    const progress: TransferProgress = {
      transfer_id: "transfer-1",
      bytes_transferred: 50,
      total_bytes: 100,
      speed_bps: 10,
      direction: "Upload",
    };

    emitEvent("sftp://progress", progress);
    expect(store.activeTransfers.get("transfer-1")).toEqual(progress);

    emitEvent("sftp://complete", {
      transfer_id: "transfer-1",
      total_bytes: 100,
      direction: "Upload",
    });

    expect(store.activeTransfers.has("transfer-1")).toBe(false);
  });

  it("removes failed transfers and exposes remote error", async () => {
    vi.mocked(invoke).mockResolvedValue("sftp-1");
    await store.openSftp("ssh-1");

    const progress: TransferProgress = {
      transfer_id: "transfer-1",
      bytes_transferred: 50,
      total_bytes: 100,
      speed_bps: 10,
      direction: "Download",
    };
    const error: TransferError = {
      transfer_id: "transfer-1",
      error: "network reset",
      direction: "Download",
    };

    emitEvent("sftp://progress", progress);
    emitEvent("sftp://error", error);

    expect(store.activeTransfers.has("transfer-1")).toBe(false);
    expect(store.remoteError).toBe("network reset");
  });
});
