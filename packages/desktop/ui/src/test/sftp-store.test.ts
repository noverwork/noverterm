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

  function mockDirect(extra: Record<string, unknown> = {}) {
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd in extra) return extra[cmd];
      if (cmd === "sftp_connect_direct") return { status: "connected", session_id: "sftp-1" };
      if (cmd === "sftp_home_dir") return "/home/user";
      if (cmd === "sftp_list_dir" || cmd === "local_list_dir") return [];
      return undefined;
    });
  }

  async function connectRight() {
    await store.right.connectDirect({
      host: "example.com",
      port: 22,
      username: "user",
      password: "secret",
    });
  }

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockReset();
    eventListeners.clear();
    unlistenFns.clear();
    store = createSftpStore();
  });

  it("starts with this machine on the left and a picker on the right", () => {
    expect(store.left.isLocal).toBe(true);
    expect(store.left.path).toBe("~");
    expect(store.right.isLocal).toBe(false);
    expect(store.right.isReady).toBe(false);
    expect(store.right.path).toBe("");
    for (const pane of [store.left, store.right]) {
      expect(pane.files).toEqual([]);
      expect(pane.loading).toBe(false);
      expect(pane.error).toBeNull();
      expect(pane.selected).toBeNull();
      expect(pane.sftpSessionId).toBeNull();
      expect(pane.sshSessionId).toBeNull();
    }
    expect(store.lastError).toBeNull();
    expect(store.errorQueue).toEqual([]);
    expect(store.activeTransfers.size).toBe(0);
  });

  it("navigates a local pane", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([testFile]);

    await store.left.navigate("/tmp");

    expect(invoke).toHaveBeenCalledWith("local_list_dir", { path: "/tmp" });
    expect(store.left.path).toBe("/tmp");
    expect(store.left.files).toEqual([testFile]);
    expect(store.left.loading).toBe(false);
    expect(store.left.error).toBeNull();
  });

  it("records local navigation errors", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("permission denied"));

    await store.left.navigate("/root");

    expect(store.left.error).toBe("permission denied");
    expect(store.lastError).toBe("permission denied");
    expect(store.errorQueue).toMatchObject([
      { message: "permission denied", type: "error" },
    ]);
    expect(store.left.loading).toBe(false);
  });

  it("creates a local folder and refreshes the current local directory", async () => {
    store.left.path = "/tmp";
    vi.mocked(invoke).mockResolvedValueOnce(null).mockResolvedValueOnce([testFile]);

    await store.left.mkdir("new-folder");

    expect(invoke).toHaveBeenCalledWith("local_mkdir", { path: "/tmp/new-folder" });
    expect(invoke).toHaveBeenCalledWith("local_list_dir", { path: "/tmp" });
    expect(store.left.files).toEqual([testFile]);
    expect(store.left.error).toBeNull();
  });

  it("shows command errors when local folder creation fails", async () => {
    store.left.path = "/tmp";
    vi.mocked(invoke).mockRejectedValueOnce("already exists");

    await store.left.mkdir("existing");

    expect(store.left.error).toBe("already exists");
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

  it("opens and closes an SFTP session, keeping shared listeners until cleanup", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("sftp-1").mockResolvedValueOnce(undefined);

    await store.right.openSftp("ssh-1");

    expect(invoke).toHaveBeenCalledWith("sftp_open", { sessionId: "ssh-1" });
    expect(store.right.sshSessionId).toBe("ssh-1");
    expect(store.right.sftpSessionId).toBe("sftp-1");
    expect(mockListen).toHaveBeenCalledWith("sftp://progress", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("sftp://complete", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("sftp://error", expect.any(Function));

    await store.right.closeSftp();

    expect(invoke).toHaveBeenCalledWith("sftp_close", { sessionId: "sftp-1" });
    expect(store.right.sftpSessionId).toBeNull();
    expect(store.right.sshSessionId).toBeNull();
    expect(unlistenFns.get("sftp://progress")).not.toHaveBeenCalled();

    store.cleanup();

    expect(unlistenFns.get("sftp://progress")).toHaveBeenCalledOnce();
    expect(unlistenFns.get("sftp://complete")).toHaveBeenCalledOnce();
    expect(unlistenFns.get("sftp://error")).toHaveBeenCalledOnce();
  });

  it("registers transfer listeners once for both panes", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("sftp-1").mockResolvedValueOnce("sftp-2");

    await store.left.openSftp("ssh-1");
    await store.right.openSftp("ssh-2");

    expect(mockListen.mock.calls.filter(([name]) => name === "sftp://progress")).toHaveLength(1);
  });

  it("clears SSH session state when opening a direct SFTP connection", async () => {
    mockDirect({ sftp_connect_direct: { status: "connected", session_id: "direct-sftp-1" } });
    store.right.sshSessionId = "ssh-1";

    await connectRight();

    expect(store.right.sshSessionId).toBeNull();
    expect(store.right.sftpSessionId).toBe("direct-sftp-1");
    expect(store.right.isDirectConnection).toBe(true);
    expect(store.right.path).toBe("/home/user");
  });

  it("switches a remote pane back to this machine", async () => {
    mockDirect();
    await connectRight();

    await store.right.useLocal();

    expect(invoke).toHaveBeenCalledWith("sftp_close", { sessionId: "sftp-1" });
    expect(store.right.isLocal).toBe(true);
    expect(store.right.sftpSessionId).toBeNull();
    expect(invoke).toHaveBeenCalledWith("local_list_dir", { path: "~" });
  });

  it("navigates a remote pane when SFTP is open", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("sftp-1").mockResolvedValueOnce([testFile]);

    await store.right.openSftp("ssh-1");
    await store.right.navigate("/home/user");

    expect(invoke).toHaveBeenCalledWith("sftp_list_dir", {
      sessionId: "sftp-1",
      path: "/home/user",
    });
    expect(store.right.path).toBe("/home/user");
    expect(store.right.files).toEqual([testFile]);
    expect(store.right.error).toBeNull();
  });

  it("creates a remote folder and refreshes the current remote directory", async () => {
    store.right.sftpSessionId = "sftp-1";
    store.right.path = "/home/user";
    vi.mocked(invoke).mockResolvedValueOnce(null).mockResolvedValueOnce([testFile]);

    await store.right.mkdir("new-folder");

    expect(invoke).toHaveBeenCalledWith("sftp_mkdir", {
      sessionId: "sftp-1",
      path: "/home/user/new-folder",
    });
    expect(invoke).toHaveBeenCalledWith("sftp_list_dir", {
      sessionId: "sftp-1",
      path: "/home/user",
    });
    expect(store.right.files).toEqual([testFile]);
    expect(store.right.error).toBeNull();
  });

  it("tracks progress events and refreshes both panes when a transfer completes", async () => {
    const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => undefined);
    vi.mocked(invoke).mockResolvedValue("sftp-1");
    await store.right.openSftp("ssh-1");
    store.right.path = "/remote";
    store.left.path = "/downloads";
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
    expect(invoke).toHaveBeenCalledWith("sftp_list_dir", { sessionId: "sftp-1", path: "/remote" });
    expect(invoke).toHaveBeenCalledWith("local_list_dir", { path: "/downloads" });
    debugSpy.mockRestore();
  });

  it("removes failed transfers and shows the error", async () => {
    vi.mocked(invoke).mockResolvedValue("sftp-1");
    await store.right.openSftp("ssh-1");

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
    expect(store.lastError).toBe("network reset");
    expect(store.errorQueue).toMatchObject([
      { message: "network reset", type: "error" },
    ]);
  });

  describe("transfer", () => {
    const fileEntry: FileEntry = {
      name: "report.pdf",
      size: 1024,
      modified: 1700000000,
      file_type: "File",
    };

    it("uploads from a local pane to a remote pane", async () => {
      mockDirect({ sftp_upload: "transfer-1" });
      await connectRight();

      vi.mocked(invoke).mockClear();
      await expect(store.transfer(store.left, fileEntry)).resolves.toBe("transfer-1");

      expect(invoke).toHaveBeenCalledWith("sftp_upload", {
        sessionId: "sftp-1",
        localPath: "~/report.pdf",
        remotePath: "/home/user/report.pdf",
      });
    });

    it("prompts before uploading over an existing remote file", async () => {
      mockDirect({ sftp_upload: "transfer-1" });
      await connectRight();
      store.right.files = [fileEntry, { ...fileEntry, name: "report (1).pdf" }];

      vi.mocked(invoke).mockClear();
      await store.transfer(store.left, fileEntry);

      expect(invoke).not.toHaveBeenCalledWith("sftp_upload", expect.anything());
      expect(store.transferConflict).toEqual({
        fileName: "report.pdf",
        existingName: "report.pdf",
        suggestedName: "report (2).pdf",
        destination: "example.com",
        isDirectory: false,
        conflictingFiles: [],
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
      mockDirect({ sftp_upload: "transfer-1" });
      await connectRight();
      store.right.files = [fileEntry];

      vi.mocked(invoke).mockClear();
      await store.transfer(store.left, fileEntry);
      await store.resolveTransferConflict("overwrite");

      expect(invoke).toHaveBeenCalledWith("sftp_upload", {
        sessionId: "sftp-1",
        localPath: "~/report.pdf",
        remotePath: "/home/user/report.pdf",
      });
    });

    it("downloads from a remote pane to a local pane", async () => {
      mockDirect({ sftp_download: "transfer-2" });
      await connectRight();

      vi.mocked(invoke).mockClear();
      await store.transfer(store.right, fileEntry);

      expect(invoke).toHaveBeenCalledWith("sftp_download", {
        sessionId: "sftp-1",
        remotePath: "/home/user/report.pdf",
        localPath: "~/report.pdf",
      });
    });

    it("prompts before downloading over an existing local file", async () => {
      mockDirect({ sftp_download: "transfer-2" });
      await connectRight();
      store.left.files = [fileEntry];

      vi.mocked(invoke).mockClear();
      await store.transfer(store.right, fileEntry);

      expect(invoke).not.toHaveBeenCalledWith("sftp_download", expect.anything());
      expect(store.transferConflict).toMatchObject({ suggestedName: "report (1).pdf", destination: "Local" });

      await store.resolveTransferConflict("rename");

      expect(invoke).toHaveBeenCalledWith("sftp_download", {
        sessionId: "sftp-1",
        remotePath: "/home/user/report.pdf",
        localPath: "~/report (1).pdf",
      });
    });

    it("uploads directory entries", async () => {
      vi.mocked(invoke).mockResolvedValue("sftp-1");
      await store.right.openSftp("ssh-1");

      vi.mocked(invoke).mockClear();
      const dirEntry: FileEntry = {
        name: "documents",
        size: 0,
        modified: null,
        file_type: "Dir",
      };
      await store.transfer(store.left, dirEntry);

      expect(invoke).toHaveBeenCalledWith(
        "sftp_upload",
        expect.objectContaining({ localPath: "~/documents" }),
      );
    });

    it("copies between two remote panes", async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === "sftp_open") return "sftp-a";
        if (cmd === "sftp_connect_direct") return { status: "connected", session_id: "sftp-b" };
        if (cmd === "sftp_home_dir") return "/home/user";
        if (cmd === "sftp_copy") return "copy-1";
        return [];
      });
      await store.left.openSftp("ssh-a", { name: "a", host: "a.example.com", port: 22, username: "me" });
      store.left.path = "/srv";
      await connectRight();

      await expect(store.transfer(store.left, fileEntry)).resolves.toBe("copy-1");

      expect(invoke).toHaveBeenCalledWith("sftp_copy", {
        sourceSessionId: "sftp-a",
        sourcePath: "/srv/report.pdf",
        targetSessionId: "sftp-b",
        targetPath: "/home/user/report.pdf",
      });
    });

    it("scans folder conflicts across two remote panes", async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === "sftp_open") return "sftp-a";
        if (cmd === "sftp_connect_direct") return { status: "connected", session_id: "sftp-b" };
        if (cmd === "sftp_home_dir") return "/home/user";
        if (cmd === "sftp_copy_conflicts") return ["docs/a.txt"];
        return [];
      });
      const dirEntry: FileEntry = { name: "docs", size: 0, modified: null, file_type: "Dir" };
      await store.left.openSftp("ssh-a");
      store.left.path = "/srv";
      await connectRight();
      store.right.files = [dirEntry];

      await store.transfer(store.left, dirEntry);

      await vi.waitFor(() => expect(store.transferConflict?.conflictingFiles).toEqual(["docs/a.txt"]));
      expect(invoke).toHaveBeenCalledWith("sftp_copy_conflicts", {
        sourceSessionId: "sftp-a",
        sourcePath: "/srv/docs",
        targetSessionId: "sftp-b",
        targetPath: "/home/user/docs",
      });
    });

    it("refuses transfers when both panes show the same machine", async () => {
      vi.mocked(invoke).mockImplementation(async (cmd: string) => {
        if (cmd === "sftp_open") return "sftp-a";
        if (cmd === "sftp_connect_direct") return { status: "connected", session_id: "sftp-b" };
        if (cmd === "sftp_home_dir") return "/home/user";
        return [];
      });
      await store.left.openSftp("ssh-a", { name: "same", host: "example.com", port: 22, username: "other" });
      await connectRight();
      vi.mocked(invoke).mockClear();

      await store.transfer(store.left, fileEntry);

      expect(invoke).not.toHaveBeenCalled();
      expect(store.errorQueue).toMatchObject([
        { type: "warning", message: expect.stringContaining("same machine") },
      ]);
    });

    it("refuses local-to-local transfers", async () => {
      store.right.mode = "local";

      await store.transfer(store.left, fileEntry);

      expect(invoke).not.toHaveBeenCalled();
      expect(store.errorQueue).toMatchObject([
        { type: "warning", message: expect.stringContaining("same machine") },
      ]);
    });

    it("warns when the other pane has no connection", async () => {
      await store.transfer(store.left, fileEntry);

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
