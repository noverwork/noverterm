import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  FileEntry,
  TransferComplete,
  TransferError,
  TransferProgress,
} from "$lib/types/sftp.js";

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function joinPath(basePath: string, name: string): string {
  if (basePath === "" || basePath === "/") {
    return `${basePath}${name}`;
  }

  return `${basePath.replace(/\/+$/, "")}/${name}`;
}

export interface ErrorToast {
  id: string;
  message: string;
  type: "error" | "warning" | "info";
}

export class SftpStore {
  localPath = $state<string>("~");
  remotePath = $state<string>("");
  localFiles = $state<FileEntry[]>([]);
  remoteFiles = $state<FileEntry[]>([]);
  localLoading = $state(false);
  remoteLoading = $state(false);
  localError = $state<string | null>(null);
  remoteError = $state<string | null>(null);
  lastError = $state<string | null>(null);
  errorQueue = $state<ErrorToast[]>([]);
  activeTransfers = $state<Map<string, TransferProgress>>(new Map());
  selectedLocal = $state<FileEntry | null>(null);
  selectedRemote = $state<FileEntry | null>(null);
  sftpSessionId = $state<string | null>(null);
  sshSessionId = $state<string | null>(null);

  private unlistenProgress: UnlistenFn | null = null;
  private unlistenComplete: UnlistenFn | null = null;
  private unlistenError: UnlistenFn | null = null;
  private nextErrorId = 0;

  showError(message: string, type: ErrorToast["type"] = "error"): void {
    this.lastError = message;
    this.errorQueue = [
      ...this.errorQueue,
      { id: `sftp-error-${++this.nextErrorId}`, message, type },
    ];
  }

  dismissError(id: string): void {
    this.errorQueue = this.errorQueue.filter((error) => error.id !== id);
  }

  async navigateLocal(path: string): Promise<void> {
    this.localLoading = true;
    this.localError = null;

    try {
      this.localPath = path;
      this.localFiles = await invoke<FileEntry[]>("local_list_dir", { path });
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.localError = message;
      this.showError(message);
    } finally {
      this.localLoading = false;
    }
  }

  async refreshLocal(): Promise<void> {
    await this.navigateLocal(this.localPath);
  }

  async localMkdir(name: string): Promise<void> {
    try {
      await invoke("local_mkdir", { path: joinPath(this.localPath, name) });
      await this.refreshLocal();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.localError = message;
      this.showError(message);
    }
  }

  async localRemove(entry: FileEntry): Promise<void> {
    try {
      await invoke("local_remove", { path: joinPath(this.localPath, entry.name) });
      await this.refreshLocal();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.localError = message;
      this.showError(message);
    }
  }

  async localRename(entry: FileEntry, newName: string): Promise<void> {
    try {
      await invoke("local_rename", {
        oldPath: joinPath(this.localPath, entry.name),
        newPath: joinPath(this.localPath, newName),
      });
      await this.refreshLocal();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.localError = message;
      this.showError(message);
    }
  }

  async openSftp(sshSessionId: string): Promise<void> {
    try {
      this.sshSessionId = sshSessionId;
      this.sftpSessionId = await invoke<string>("sftp_open", {
        sessionId: sshSessionId,
      });
      await this.setupEventListeners();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.sshSessionId = null;
      this.sftpSessionId = null;
      this.remoteError = message;
      this.showError(message);
    }
  }

  async closeSftp(): Promise<void> {
    try {
      if (this.sftpSessionId) {
        await invoke("sftp_close", { sessionId: this.sftpSessionId });
        this.sftpSessionId = null;
      }

      this.sshSessionId = null;
      await this.teardownEventListeners();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.remoteError = message;
      this.showError(message);
    }
  }

  async navigateRemote(path: string): Promise<void> {
    if (!this.sftpSessionId) {
      this.remoteError = "SFTP session is not connected.";
      this.showError(this.remoteError, "warning");
      return;
    }

    this.remoteLoading = true;
    this.remoteError = null;

    try {
      this.remotePath = path;
      this.remoteFiles = await invoke<FileEntry[]>("sftp_list_dir", {
        sessionId: this.sftpSessionId,
        path,
      });
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.remoteError = message;
      this.showError(message);
    } finally {
      this.remoteLoading = false;
    }
  }

  async refreshRemote(): Promise<void> {
    await this.navigateRemote(this.remotePath);
  }

  async remoteMkdir(name: string): Promise<void> {
    if (!this.sftpSessionId) {
      this.remoteError = "SFTP session is not connected.";
      this.showError(this.remoteError, "warning");
      return;
    }

    try {
      await invoke("sftp_mkdir", {
        sessionId: this.sftpSessionId,
        path: joinPath(this.remotePath, name),
      });
      await this.refreshRemote();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.remoteError = message;
      this.showError(message);
    }
  }

  async remoteRemove(entry: FileEntry): Promise<void> {
    if (!this.sftpSessionId) {
      this.remoteError = "SFTP session is not connected.";
      this.showError(this.remoteError, "warning");
      return;
    }

    try {
      await invoke("sftp_remove", {
        sessionId: this.sftpSessionId,
        path: joinPath(this.remotePath, entry.name),
      });
      await this.refreshRemote();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.remoteError = message;
      this.showError(message);
    }
  }

  async remoteRename(entry: FileEntry, newName: string): Promise<void> {
    if (!this.sftpSessionId) {
      this.remoteError = "SFTP session is not connected.";
      this.showError(this.remoteError, "warning");
      return;
    }

    try {
      await invoke("sftp_rename", {
        sessionId: this.sftpSessionId,
        oldPath: joinPath(this.remotePath, entry.name),
        newPath: joinPath(this.remotePath, newName),
      });
      await this.refreshRemote();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.remoteError = message;
      this.showError(message);
    }
  }

  async startUpload(localEntry: FileEntry): Promise<string | undefined> {
    if (!this.sftpSessionId) {
      this.remoteError = "SFTP session is not connected.";
      this.showError(this.remoteError, "warning");
      return undefined;
    }

    try {
      return await invoke<string>("sftp_upload", {
        sessionId: this.sftpSessionId,
        localPath: joinPath(this.localPath, localEntry.name),
        remotePath: joinPath(this.remotePath, localEntry.name),
      });
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.remoteError = message;
      this.showError(message);
      return undefined;
    }
  }

  async startDownload(remoteEntry: FileEntry): Promise<string | undefined> {
    if (!this.sftpSessionId) {
      this.remoteError = "SFTP session is not connected.";
      this.showError(this.remoteError, "warning");
      return undefined;
    }

    try {
      return await invoke<string>("sftp_download", {
        sessionId: this.sftpSessionId,
        remotePath: joinPath(this.remotePath, remoteEntry.name),
        localPath: joinPath(this.localPath, remoteEntry.name),
      });
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.remoteError = message;
      this.showError(message);
      return undefined;
    }
  }

  async cancelTransfer(transferId: string): Promise<void> {
    try {
      await invoke("sftp_cancel_transfer", { transferId });
    } catch (error: unknown) {
      this.showError(errorMessage(error));
    }
  }

  cleanup(): void {
    void this.teardownEventListeners();
    this.localPath = "~";
    this.remotePath = "";
    this.localFiles = [];
    this.remoteFiles = [];
    this.localLoading = false;
    this.remoteLoading = false;
    this.localError = null;
    this.remoteError = null;
    this.lastError = null;
    this.errorQueue = [];
    this.activeTransfers = new Map();
    this.selectedLocal = null;
    this.selectedRemote = null;
    this.sftpSessionId = null;
    this.sshSessionId = null;
  }

  private async setupEventListeners(): Promise<void> {
    if (!this.unlistenProgress) {
      this.unlistenProgress = await listen<TransferProgress>(
        "sftp://progress",
        (event) => {
          const progress = new Map(this.activeTransfers);
          progress.set(event.payload.transfer_id, event.payload);
          this.activeTransfers = progress;
        },
      );
    }

    if (!this.unlistenComplete) {
      this.unlistenComplete = await listen<TransferComplete>(
        "sftp://complete",
        (event) => {
          const progress = new Map(this.activeTransfers);
          progress.delete(event.payload.transfer_id);
          this.activeTransfers = progress;
          void this.refreshRemote();
        },
      );
    }

    if (!this.unlistenError) {
      this.unlistenError = await listen<TransferError>(
        "sftp://error",
        (event) => {
          const progress = new Map(this.activeTransfers);
          progress.delete(event.payload.transfer_id);
          this.activeTransfers = progress;
          this.remoteError = event.payload.error;
          this.showError(event.payload.error);
        },
      );
    }
  }

  private async teardownEventListeners(): Promise<void> {
    this.unlistenProgress?.();
    this.unlistenComplete?.();
    this.unlistenError?.();
    this.unlistenProgress = null;
    this.unlistenComplete = null;
    this.unlistenError = null;
  }
}

export function createSftpStore(): SftpStore {
  return new SftpStore();
}

export const sftpStore = createSftpStore();
