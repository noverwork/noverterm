import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { FileEntry, TransferProgress } from "$lib/types/sftp.js";

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
    activeTransfers: new Map<string, TransferProgress>(),
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
    cancelTransfer: vi.fn(),
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
  mockStore.activeTransfers = new Map<string, TransferProgress>();
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
  mockStore.cancelTransfer.mockReset();
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

  it("enables rename/delete buttons when a local entry is selected", () => {
    mockStore.selectedLocal = buildEntry({ name: "notes.md" });
    renderFileBrowser();

    const localRename = screen.getByTestId("local-rename") as HTMLButtonElement;
    const localDelete = screen.getByTestId("local-delete") as HTMLButtonElement;
    expect(localRename.disabled).toBe(false);
    expect(localDelete.disabled).toBe(false);
  });

  it("disables upload/download when no sftp session", () => {
    mockStore.sftpSessionId = null;
    mockStore.selectedLocal = buildEntry({ name: "local.txt" });
    mockStore.selectedRemote = buildEntry({ name: "remote.txt" });
    renderFileBrowser();

    const upload = screen.getByTestId("remote-upload") as HTMLButtonElement;
    const download = screen.getByTestId("remote-download") as HTMLButtonElement;
    expect(upload.disabled).toBe(true);
    expect(download.disabled).toBe(true);
  });

  it("upload invokes store.startUpload with the local selection", async () => {
    const selected = buildEntry({ name: "report.pdf" });
    mockStore.selectedLocal = selected;
    mockStore.startUpload.mockResolvedValue("upload-1");
    renderFileBrowser();

    const upload = screen.getByTestId("remote-upload") as HTMLButtonElement;
    expect(upload.disabled).toBe(false);

    await fireEvent.click(upload);

    expect(mockStore.startUpload).toHaveBeenCalledTimes(1);
    expect(mockStore.startUpload).toHaveBeenCalledWith(selected);
  });

  it("download invokes store.startDownload with the remote selection", async () => {
    const selected = buildEntry({ name: "data.json" });
    mockStore.selectedRemote = selected;
    mockStore.startDownload.mockResolvedValue("download-1");
    renderFileBrowser();

    const download = screen.getByTestId("remote-download") as HTMLButtonElement;
    expect(download.disabled).toBe(false);

    await fireEvent.click(download);

    expect(mockStore.startDownload).toHaveBeenCalledTimes(1);
    expect(mockStore.startDownload).toHaveBeenCalledWith(selected);
  });
});
