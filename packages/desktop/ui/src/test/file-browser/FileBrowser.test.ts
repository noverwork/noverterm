import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { FileEntry, TransferProgress } from "$lib/types/sftp.js";
import type { TransferConflict } from "$lib/stores/sftp.svelte.js";

import FileBrowser from "$lib/components/file-browser/FileBrowser.svelte";

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
  transferConflict: TransferConflict | null;
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
  dropTransfer: ReturnType<typeof vi.fn>;
  cancelTransfer: ReturnType<typeof vi.fn>;
  resolveTransferConflict: ReturnType<typeof vi.fn>;
  cancelTransferConflict: ReturnType<typeof vi.fn>;
  dismissError: ReturnType<typeof vi.fn>;
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
    transferConflict: null,
    selectedLocal: null,
    selectedRemote: null,
    sftpSessionId: "sftp-1",
    sshSessionId: "ssh-1",
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
    dropTransfer: vi.fn(),
    cancelTransfer: vi.fn(),
    resolveTransferConflict: vi.fn(),
    cancelTransferConflict: vi.fn(),
    dismissError: vi.fn(),
  };
  return { mockStore: store };
});

vi.mock("$lib/stores/sftp.svelte.js", () => ({
  sftpStore: mockStore,
}));

function buildEntry(overrides: Partial<FileEntry> = {}): FileEntry {
  return {
    name: "file.txt",
    size: 0,
    modified: null,
    file_type: "File",
    ...overrides,
  };
}

function renderFileBrowser() {
  return render(FileBrowser, { props: {} });
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
  mockStore.transferConflict = null;
  mockStore.selectedLocal = null;
  mockStore.selectedRemote = null;
  mockStore.sftpSessionId = "sftp-1";
  mockStore.sshSessionId = "ssh-1";
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
  mockStore.dropTransfer.mockReset();
  mockStore.cancelTransfer.mockReset();
  mockStore.resolveTransferConflict.mockReset();
  mockStore.cancelTransferConflict.mockReset();
  mockStore.dismissError.mockReset();
}

describe("FileBrowser", () => {
  beforeEach(() => {
    resetMockStore();
  });

  it("renders dual-panel layout", () => {
    renderFileBrowser();

    const panels = screen.getByTestId("file-browser-panels");
    expect(panels).toBeTruthy();

    expect(screen.getByTestId("file-browser-local-panel")).toBeTruthy();
    expect(screen.getByTestId("file-browser-remote-panel")).toBeTruthy();

    expect(panels.querySelector('[data-testid="file-browser-local-panel"]')).toBeTruthy();
    expect(panels.querySelector('[data-testid="file-browser-remote-panel"]')).toBeTruthy();
  });

  it("renders Local panel header", () => {
    renderFileBrowser();

    const localPanel = screen.getByTestId("file-browser-local-panel");
    expect(localPanel.textContent).toContain("Local");
    expect(localPanel.querySelector('[data-testid="local-breadcrumb"]')).toBeTruthy();
    expect(localPanel.querySelector('[data-testid="local-toolbar"]')).toBeTruthy();
  });

  it("renders Remote panel header", () => {
    renderFileBrowser();

    const remotePanel = screen.getByTestId("file-browser-remote-panel");
    expect(remotePanel.textContent).toContain("Remote");
    expect(remotePanel.querySelector('[data-testid="remote-breadcrumb"]')).toBeTruthy();
    expect(remotePanel.querySelector('[data-testid="remote-toolbar"]')).toBeTruthy();
  });

  it("renders TransferProgress when transfers exist", () => {
    mockStore.activeTransfers = new Map<string, TransferProgress>([
      [
        "t-1",
        {
          transfer_id: "t-1",
          bytes_transferred: 500,
          total_bytes: 1000,
          speed_bps: 1024,
          direction: "Upload",
        },
      ],
    ]);

    renderFileBrowser();

    const rows = document.querySelectorAll('[data-testid="transfer-row"]');
    expect(rows).toHaveLength(1);
    expect(rows[0]?.getAttribute("data-transfer-id")).toBe("t-1");
  });

  it("hides TransferProgress when showTransferProgress is false", () => {
    mockStore.activeTransfers = new Map<string, TransferProgress>([
      [
        "t-1",
        {
          transfer_id: "t-1",
          bytes_transferred: 500,
          total_bytes: 1000,
          speed_bps: 1024,
          direction: "Upload",
        },
      ],
    ]);

    render(FileBrowser, { props: { showTransferProgress: false } });

    const rows = document.querySelectorAll('[data-testid="transfer-row"]');
    expect(rows).toHaveLength(0);
  });

  it("renders dismissable error toasts from the store queue", async () => {
    mockStore.errorQueue = [
      {
        id: "error-1",
        message: "Path not found: /missing",
        type: "error",
      },
    ];

    renderFileBrowser();

    expect(screen.getByTestId("error-toast-stack")).toBeTruthy();
    expect(screen.getByTestId("error-toast-message").textContent).toContain(
      "Path not found: /missing",
    );

    await fireEvent.click(screen.getByTestId("error-toast-dismiss"));

    expect(mockStore.dismissError).toHaveBeenCalledTimes(1);
    expect(mockStore.dismissError).toHaveBeenCalledWith("error-1");
  });

  it("clicking breadcrumb navigates", async () => {
    mockStore.localPath = "/home/user/docs";
    renderFileBrowser();

    const segments = screen.getAllByTestId("local-breadcrumb-segment");
    expect(segments).toHaveLength(3);

    const userSegment = segments[1] as HTMLElement;
    expect(userSegment.textContent?.trim()).toBe("user");
    expect(userSegment.getAttribute("data-segment-index")).toBe("1");

    await fireEvent.click(userSegment);

    expect(mockStore.navigateLocal).toHaveBeenCalledTimes(1);
    expect(mockStore.navigateLocal).toHaveBeenCalledWith("/home/user");
  });

  it("clicking remote breadcrumb navigates the remote path", async () => {
    mockStore.remotePath = "/var/log/app";
    renderFileBrowser();

    const segments = screen.getAllByTestId("remote-breadcrumb-segment");
    expect(segments.length).toBeGreaterThanOrEqual(2);

    const logSegment = segments[1] as HTMLElement;
    await fireEvent.click(logSegment);

    expect(mockStore.navigateRemote).toHaveBeenCalledTimes(1);
    expect(mockStore.navigateRemote).toHaveBeenCalledWith("/var/log");
  });

  it("shows remote empty-state when remotePath is empty", () => {
    mockStore.remotePath = "";
    renderFileBrowser();

    expect(screen.getByTestId("remote-breadcrumb-empty")).toBeTruthy();
    expect(screen.queryAllByTestId("remote-breadcrumb-segment")).toHaveLength(0);
  });

  it("responsive layout classes present on the panels grid", () => {
    renderFileBrowser();

    const panels = screen.getByTestId("file-browser-panels");
    expect(panels.className).toContain("grid");
    expect(panels.className).toContain("grid-cols-1");
    expect(panels.className).toContain("lg:grid-cols-2");
  });

  it("disables rename/delete buttons when no local selection", () => {
    mockStore.selectedLocal = null;
    renderFileBrowser();

    const localRename = screen.getByTestId("local-rename") as HTMLButtonElement;
    const localDelete = screen.getByTestId("local-delete") as HTMLButtonElement;
    expect(localRename.disabled).toBe(true);
    expect(localDelete.disabled).toBe(true);

    const newFolder = screen.getByTestId("local-new-folder") as HTMLButtonElement;
    expect(newFolder.disabled).toBe(false);
  });

  it("creates a local folder from the new-folder dialog", async () => {
    renderFileBrowser();

    await fireEvent.click(screen.getByTestId("local-new-folder"));
    await fireEvent.input(screen.getByLabelText(/folder name/i), {
      target: { value: "reports" },
    });
    await fireEvent.click(screen.getByRole("button", { name: /create/i }));

    expect(mockStore.localMkdir).toHaveBeenCalledWith("reports");
    expect(mockStore.remoteMkdir).not.toHaveBeenCalled();
  });

  it("creates a remote folder from the new-folder dialog", async () => {
    renderFileBrowser();

    await fireEvent.click(screen.getByTestId("remote-new-folder"));
    await fireEvent.input(screen.getByLabelText(/folder name/i), {
      target: { value: "logs" },
    });
    await fireEvent.click(screen.getByRole("button", { name: /create/i }));

    expect(mockStore.remoteMkdir).toHaveBeenCalledWith("logs");
    expect(mockStore.localMkdir).not.toHaveBeenCalled();
  });

  it("enables rename/delete buttons when a local entry is selected", () => {
    mockStore.selectedLocal = buildEntry({ name: "notes.md" });
    renderFileBrowser();

    const localRename = screen.getByTestId("local-rename") as HTMLButtonElement;
    const localDelete = screen.getByTestId("local-delete") as HTMLButtonElement;
    expect(localRename.disabled).toBe(false);
    expect(localDelete.disabled).toBe(false);
  });

  it("uploads internal local file drops on the remote panel", async () => {
    const entry = buildEntry({ name: "report.pdf" });
    mockStore.localFiles = [entry];
    renderFileBrowser();
    const remotePanel = screen.getByTestId("file-browser-remote-panel");

    await fireEvent.drop(remotePanel, {
      dataTransfer: {
        types: ["application/x-sftp-entry"],
        getData: (type: string) =>
          type === "application/x-sftp-entry"
            ? JSON.stringify({ panel: "local", entry })
            : "",
        dropEffect: "copy",
      },
    });

    expect(mockStore.dropTransfer).toHaveBeenCalledTimes(1);
    expect(mockStore.dropTransfer).toHaveBeenCalledWith("local", "remote", entry);
  });

  it("renders transfer conflict actions", async () => {
    mockStore.transferConflict = {
      fileName: "report.pdf",
      existingName: "report.pdf",
      suggestedName: "report (1).pdf",
      direction: "Upload",
    };
    renderFileBrowser();

    expect(screen.getByTestId("transfer-conflict-dialog")).toBeTruthy();
    expect(screen.getByTestId("transfer-conflict-original").textContent).toContain("report.pdf");
    expect(screen.getByTestId("transfer-conflict-suggested").textContent).toContain("report (1).pdf");

    await fireEvent.click(screen.getByTestId("transfer-conflict-rename"));
    expect(mockStore.resolveTransferConflict).toHaveBeenCalledWith("rename");

    await fireEvent.click(screen.getByTestId("transfer-conflict-overwrite"));
    expect(mockStore.resolveTransferConflict).toHaveBeenCalledWith("overwrite");
  });
});
