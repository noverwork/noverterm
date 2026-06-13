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
import {
  createSftpStore,
  nextAvailableTransferName,
  type SftpStore,
} from "$lib/stores/sftp.svelte.js";

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
    expect(store.lastError).toBeNull();
    expect(store.errorQueue).toEqual([]);
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
    expect(store.lastError).toBe("permission denied");
    expect(store.errorQueue).toMatchObject([
      { message: "permission denied", type: "error" },
    ]);
    expect(store.localLoading).toBe(false);
  });

  it("creates a local folder and refreshes the current local directory", async () => {
    store.localPath = "/tmp";
    vi.mocked(invoke).mockResolvedValueOnce(null).mockResolvedValueOnce([testFile]);

    await store.localMkdir("new-folder");

    expect(invoke).toHaveBeenCalledWith("local_mkdir", { path: "/tmp/new-folder" });
    expect(invoke).toHaveBeenCalledWith("local_list_dir", { path: "/tmp" });
    expect(store.localFiles).toEqual([testFile]);
    expect(store.localError).toBeNull();
  });

  it("shows command errors when local folder creation fails", async () => {
    store.localPath = "/tmp";
    vi.mocked(invoke).mockRejectedValueOnce("already exists");

    await store.localMkdir("existing");

    expect(store.localError).toBe("already exists");
    expect(store.errorQueue).toMatchObject([
      { message: "already exists", type: "error" },
    ]);
    expect(invoke).not.toHaveBeenCalledWith("local_list_dir", expect.anything());
  });

  it("queues multiple errors and dismisses by id", () => {
    store.showError("first failure");
    store.showError("watch out", "warning");

    expect(store.lastError).toBe("watch out");
    expect(store.errorQueue).toMatchObject([
      { message: "first failure", type: "error" },
      { message: "watch out", type: "warning" },
    ]);

    const firstId = store.errorQueue[0]?.id;
    expect(firstId).toBeTruthy();
    store.dismissError(firstId ?? "");

    expect(store.errorQueue).toHaveLength(1);
    expect(store.errorQueue[0]?.message).toBe("watch out");
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

  it("clears SSH session state when opening a direct SFTP connection", async () => {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "sftp_connect_direct") return "direct-sftp-1";
      if (cmd === "sftp_home_dir") return "/home/direct";
      if (cmd === "sftp_list_dir") return [];
      return undefined;
    });
    store.sshSessionId = "ssh-1";

    await store.connectDirect({
      host: "example.com",
      port: 22,
      username: "user",
      password: "secret",
    });

    expect(store.sshSessionId).toBeNull();
    expect(store.sftpSessionId).toBe("direct-sftp-1");
    expect(store.isDirectConnection).toBe(true);
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

  it("creates a remote folder and refreshes the current remote directory", async () => {
    store.sftpSessionId = "sftp-1";
    store.remotePath = "/home/user";
    vi.mocked(invoke).mockResolvedValueOnce(null).mockResolvedValueOnce([testFile]);

    await store.remoteMkdir("new-folder");

    expect(invoke).toHaveBeenCalledWith("sftp_mkdir", {
      sessionId: "sftp-1",
      path: "/home/user/new-folder",
    });
    expect(invoke).toHaveBeenCalledWith("sftp_list_dir", {
      sessionId: "sftp-1",
      path: "/home/user",
    });
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
    const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => undefined);
    vi.mocked(invoke).mockResolvedValue("sftp-1");
    await store.openSftp("ssh-1");
    vi.mocked(invoke).mockClear();

    const progress: TransferProgress = {
      transfer_id: "transfer-1",
      bytes_transferred: 50,
      total_bytes: 100,
      speed_bps: 10,
      direction: "Upload",
    };

    emitEvent("sftp://progress", progress);
    expect(store.activeTransfers.get("transfer-1")).toEqual(progress);
    expect(debugSpy).not.toHaveBeenCalled();

    emitEvent("sftp://complete", {
      transfer_id: "transfer-1",
      total_bytes: 100,
      direction: "Upload",
    });

    expect(store.activeTransfers.has("transfer-1")).toBe(false);
    expect(invoke).toHaveBeenCalledWith("sftp_list_dir", {
      sessionId: "sftp-1",
      path: store.remotePath,
    });
    expect(invoke).not.toHaveBeenCalledWith("local_list_dir", expect.anything());
    debugSpy.mockRestore();
  });

  it("refreshes the local directory after a download completes", async () => {
    vi.mocked(invoke).mockResolvedValue("sftp-1");
    await store.openSftp("ssh-1");
    store.localPath = "/downloads";
    vi.mocked(invoke).mockClear();

    emitEvent("sftp://progress", {
      transfer_id: "transfer-2",
      bytes_transferred: 50,
      total_bytes: 100,
      speed_bps: 10,
      direction: "Download",
    });

    emitEvent("sftp://complete", {
      transfer_id: "transfer-2",
      total_bytes: 100,
      direction: "Download",
    });

    expect(store.activeTransfers.has("transfer-2")).toBe(false);
    expect(invoke).toHaveBeenCalledWith("local_list_dir", { path: "/downloads" });
    expect(invoke).not.toHaveBeenCalledWith("sftp_list_dir", expect.anything());
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
    expect(store.lastError).toBe("network reset");
    expect(store.errorQueue).toMatchObject([
      { message: "network reset", type: "error" },
    ]);
  });

  describe("dropTransfer", () => {
    const fileEntry: FileEntry = {
      name: "report.pdf",
      size: 1024,
      modified: 1700000000,
      file_type: "File",
    };

    it("uploads a local file when dropped to the remote panel", async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === "sftp_connect_direct") return "sftp-1";
        if (cmd === "sftp_home_dir") return "/home/user";
        if (cmd === "sftp_list_dir") return [];
        if (cmd === "sftp_upload") return "transfer-1";
        return undefined;
      });
      await store.connectDirect({
        host: "example.com",
        port: 22,
        username: "user",
        password: "secret",
      });

      vi.mocked(invoke).mockClear();
      await store.dropTransfer("local", "remote", fileEntry);

      expect(invoke).toHaveBeenCalledWith("sftp_upload", {
        sessionId: "sftp-1",
        localPath: "~/report.pdf",
        remotePath: "/home/user/report.pdf",
      });
    });

    it("prompts before uploading over an existing remote file", async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === "sftp_connect_direct") return "sftp-1";
        if (cmd === "sftp_home_dir") return "/home/user";
        if (cmd === "sftp_list_dir") return [];
        if (cmd === "sftp_upload") return "transfer-1";
        return undefined;
      });
      await store.connectDirect({
        host: "example.com",
        port: 22,
        username: "user",
        password: "secret",
      });
      store.remoteFiles = [fileEntry, { ...fileEntry, name: "report (1).pdf" }];

      vi.mocked(invoke).mockClear();
      await store.dropTransfer("local", "remote", fileEntry);

      expect(invoke).not.toHaveBeenCalledWith("sftp_upload", expect.anything());
      expect(store.transferConflict).toEqual({
        fileName: "report.pdf",
        existingName: "report.pdf",
        suggestedName: "report (2).pdf",
        direction: "Upload",
      });

      await store.resolveTransferConflict("rename");

      expect(invoke).toHaveBeenCalledWith("sftp_upload", {
        sessionId: "sftp-1",
        localPath: "~/report.pdf",
        remotePath: "/home/user/report (2).pdf",
      });
      expect(store.transferConflict).toBeNull();
    });

    it("overwrites the original target when confirmed", async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === "sftp_connect_direct") return "sftp-1";
        if (cmd === "sftp_home_dir") return "/home/user";
        if (cmd === "sftp_list_dir") return [];
        if (cmd === "sftp_upload") return "transfer-1";
        return undefined;
      });
      await store.connectDirect({
        host: "example.com",
        port: 22,
        username: "user",
        password: "secret",
      });
      store.remoteFiles = [fileEntry];

      vi.mocked(invoke).mockClear();
      await store.dropTransfer("local", "remote", fileEntry);
      await store.resolveTransferConflict("overwrite");

      expect(invoke).toHaveBeenCalledWith("sftp_upload", {
        sessionId: "sftp-1",
        localPath: "~/report.pdf",
        remotePath: "/home/user/report.pdf",
      });
    });

    it("downloads a remote file when dropped to the local panel", async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === "sftp_connect_direct") return "sftp-1";
        if (cmd === "sftp_home_dir") return "/home/user";
        if (cmd === "sftp_list_dir") return [];
        if (cmd === "sftp_download") return "transfer-2";
        return undefined;
      });
      await store.connectDirect({
        host: "example.com",
        port: 22,
        username: "user",
        password: "secret",
      });

      vi.mocked(invoke).mockClear();
      await store.dropTransfer("remote", "local", fileEntry);

      expect(invoke).toHaveBeenCalledWith("sftp_download", {
        sessionId: "sftp-1",
        remotePath: "/home/user/report.pdf",
        localPath: "~/report.pdf",
      });
    });

    it("prompts before downloading over an existing local file", async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === "sftp_connect_direct") return "sftp-1";
        if (cmd === "sftp_home_dir") return "/home/user";
        if (cmd === "sftp_list_dir") return [];
        if (cmd === "sftp_download") return "transfer-2";
        return undefined;
      });
      await store.connectDirect({
        host: "example.com",
        port: 22,
        username: "user",
        password: "secret",
      });
      store.localFiles = [fileEntry];

      vi.mocked(invoke).mockClear();
      await store.dropTransfer("remote", "local", fileEntry);

      expect(invoke).not.toHaveBeenCalledWith("sftp_download", expect.anything());
      expect(store.transferConflict).toEqual({
        fileName: "report.pdf",
        existingName: "report.pdf",
        suggestedName: "report (1).pdf",
        direction: "Download",
      });

      await store.resolveTransferConflict("rename");

      expect(invoke).toHaveBeenCalledWith("sftp_download", {
        sessionId: "sftp-1",
        remotePath: "/home/user/report.pdf",
        localPath: "~/report (1).pdf",
      });
    });

    it("ignores drops within the same panel", async () => {
      vi.mocked(invoke).mockResolvedValue("sftp-1");
      await store.openSftp("ssh-1");

      vi.mocked(invoke).mockClear();
      await store.dropTransfer("local", "local", fileEntry);
      await store.dropTransfer("remote", "remote", fileEntry);

      expect(invoke).not.toHaveBeenCalledWith("sftp_upload", expect.anything());
      expect(invoke).not.toHaveBeenCalledWith("sftp_download", expect.anything());
    });

    it("rejects non-file entries", async () => {
      vi.mocked(invoke).mockResolvedValue("sftp-1");
      await store.openSftp("ssh-1");

      vi.mocked(invoke).mockClear();
      const dirEntry: FileEntry = {
        name: "documents",
        size: 0,
        modified: null,
        file_type: "Dir",
      };
      await store.dropTransfer("local", "remote", dirEntry);

      expect(invoke).not.toHaveBeenCalledWith("sftp_upload", expect.anything());
      expect(store.errorQueue).toMatchObject([
        { type: "warning", message: expect.stringContaining("Only files") },
      ]);
    });

    it("warns when dropping to remote without an active connection", async () => {
      await store.dropTransfer("local", "remote", fileEntry);

      expect(store.errorQueue).toMatchObject([
        { type: "warning", message: expect.stringContaining("Connect to a server") },
      ]);
    });
  });
});

describe("nextAvailableTransferName", () => {
  function entry(name: string): FileEntry {
    return { ...testFile, name };
  }

  it("adds numeric suffix before the extension", () => {
    expect(nextAvailableTransferName("report.pdf", [entry("report.pdf")])).toBe("report (1).pdf");
  });

  it("increments an existing numeric suffix", () => {
    expect(nextAvailableTransferName("report (1).pdf", [
      entry("report (1).pdf"),
      entry("report (2).pdf"),
    ])).toBe("report (3).pdf");
  });

  it("handles names without extensions", () => {
    expect(nextAvailableTransferName("README", [entry("README")])).toBe("README (1)");
  });
});
