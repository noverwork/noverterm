import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";

import SessionViewSwitcher from "$lib/components/session-view-switcher.svelte";
import type { Session } from "$lib/stores/session.svelte.js";
import type { FileEntry, TransferProgress } from "$lib/types/sftp.js";

type SftpStoreMock = {
  localPath: string;
  remotePath: string;
  localFiles: FileEntry[];
  remoteFiles: FileEntry[];
  localLoading: boolean;
  remoteLoading: boolean;
  localError: string | null;
  remoteError: string | null;
  errorQueue: Array<{
    id: string;
    message: string;
    type: "error" | "warning" | "info";
  }>;
  activeTransfers: Map<string, TransferProgress>;
  selectedLocal: FileEntry | null;
  selectedRemote: FileEntry | null;
  sftpSessionId: string | null;
  sshSessionId: string | null;
  navigateLocal: ReturnType<typeof vi.fn>;
  navigateRemote: ReturnType<typeof vi.fn>;
  refreshLocal: ReturnType<typeof vi.fn>;
  refreshRemote: ReturnType<typeof vi.fn>;
  localMkdir: ReturnType<typeof vi.fn>;
  remoteMkdir: ReturnType<typeof vi.fn>;
  localRemove: ReturnType<typeof vi.fn>;
  remoteRemove: ReturnType<typeof vi.fn>;
  localRename: ReturnType<typeof vi.fn>;
  remoteRename: ReturnType<typeof vi.fn>;
  startUpload: ReturnType<typeof vi.fn>;
  startDownload: ReturnType<typeof vi.fn>;
  cancelTransfer: ReturnType<typeof vi.fn>;
  dismissError: ReturnType<typeof vi.fn>;
  openSftp: ReturnType<typeof vi.fn>;
  closeSftp: ReturnType<typeof vi.fn>;
};

const { mockStore } = vi.hoisted(() => {
  const store: SftpStoreMock = {
    localPath: "/home/user",
    remotePath: "/var/log",
    localFiles: [],
    remoteFiles: [],
    localLoading: false,
    remoteLoading: false,
    localError: null,
    remoteError: null,
    errorQueue: [],
    activeTransfers: new Map<string, TransferProgress>(),
    selectedLocal: null,
    selectedRemote: null,
    sftpSessionId: null,
    sshSessionId: null,
    navigateLocal: vi.fn(),
    navigateRemote: vi.fn(),
    refreshLocal: vi.fn(),
    refreshRemote: vi.fn(),
    localMkdir: vi.fn(),
    remoteMkdir: vi.fn(),
    localRemove: vi.fn(),
    remoteRemove: vi.fn(),
    localRename: vi.fn(),
    remoteRename: vi.fn(),
    startUpload: vi.fn(),
    startDownload: vi.fn(),
    cancelTransfer: vi.fn(),
    dismissError: vi.fn(),
    openSftp: vi.fn(),
    closeSftp: vi.fn(),
  };
  return { mockStore: store };
});

vi.mock("$lib/stores/sftp.svelte.js", () => ({
  sftpStore: mockStore,
}));

function createSshSession(overrides: Partial<Session> = {}): Session {
  return {
    id: "ssh-1",
    name: "prod.example.com",
    host: "prod.example.com",
    port: 22,
    username: "deploy",
    status: "connected",
    type: "ssh",
    createdAt: new Date("2026-06-11T00:00:00.000Z"),
    connectionId: "conn-1",
    ...overrides,
  };
}

function resetMockStore(): void {
  mockStore.localPath = "/home/user";
  mockStore.remotePath = "/var/log";
  mockStore.localFiles = [];
  mockStore.remoteFiles = [];
  mockStore.localLoading = false;
  mockStore.remoteLoading = false;
  mockStore.localError = null;
  mockStore.remoteError = null;
  mockStore.errorQueue = [];
  mockStore.activeTransfers = new Map<string, TransferProgress>();
  mockStore.selectedLocal = null;
  mockStore.selectedRemote = null;
  mockStore.sftpSessionId = null;
  mockStore.sshSessionId = null;
  mockStore.navigateLocal.mockReset();
  mockStore.navigateRemote.mockReset();
  mockStore.refreshLocal.mockReset();
  mockStore.refreshRemote.mockReset();
  mockStore.localMkdir.mockReset();
  mockStore.remoteMkdir.mockReset();
  mockStore.localRemove.mockReset();
  mockStore.remoteRemove.mockReset();
  mockStore.localRename.mockReset();
  mockStore.remoteRename.mockReset();
  mockStore.startUpload.mockReset();
  mockStore.startDownload.mockReset();
  mockStore.cancelTransfer.mockReset();
  mockStore.dismissError.mockReset();
  mockStore.openSftp.mockReset();
  mockStore.openSftp.mockResolvedValue(undefined);
  mockStore.closeSftp.mockReset();
  mockStore.closeSftp.mockResolvedValue(undefined);
}

describe("SFTP session view integration", () => {
  beforeEach(() => {
    resetMockStore();
  });

  it("renders the terminal view before the Files tab is active", () => {
    render(SessionViewSwitcher, {
      props: { activeSession: createSshSession() },
    });

    const terminalPanel = screen.getByTestId("terminal-view-panel");
    expect(terminalPanel.getAttribute("aria-hidden")).toBe("false");
    expect(screen.getByTestId("terminal-view-placeholder")).toBeTruthy();
    expect(screen.queryByTestId("file-browser")).toBeNull();
  });

  it("renders FileBrowser and opens SFTP when the Files tab is active", async () => {
    render(SessionViewSwitcher, {
      props: { activeSession: createSshSession() },
    });

    await fireEvent.click(screen.getByTestId("files-view-tab"));

    expect(screen.getByTestId("file-browser")).toBeTruthy();
    expect(screen.getByTestId("terminal-view-panel").getAttribute("aria-hidden")).toBe("true");
    await waitFor(() => {
      expect(mockStore.openSftp).toHaveBeenCalledWith("ssh-1");
    });
  });

  it("toggles back to the terminal view", async () => {
    render(SessionViewSwitcher, {
      props: { activeSession: createSshSession() },
    });

    await fireEvent.click(screen.getByTestId("files-view-tab"));
    expect(screen.getByTestId("file-browser")).toBeTruthy();

    await fireEvent.click(screen.getByTestId("terminal-view-tab"));

    expect(screen.queryByTestId("file-browser")).toBeNull();
    expect(screen.getByTestId("terminal-view-panel").getAttribute("aria-hidden")).toBe("false");
  });

  it("shows a connection prompt instead of opening SFTP without an active SSH session", async () => {
    render(SessionViewSwitcher, {
      props: { activeSession: createSshSession({ type: "local", id: "local-1" }) },
    });

    await fireEvent.click(screen.getByTestId("files-view-tab"));

    expect(screen.getByText("Connect to a server first")).toBeTruthy();
    expect(screen.queryByTestId("file-browser")).toBeNull();
    expect(mockStore.openSftp).not.toHaveBeenCalled();
  });
});
