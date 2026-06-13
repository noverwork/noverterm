import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  FileEntry,
  TransferComplete,
  TransferError,
  TransferProgress,
} from "$lib/types/sftp.js";

const TRANSFER_PROGRESS_LOG_BYTES = 1024 * 1024;
const TRANSFER_PROGRESS_LOG_MS = 2000;

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function joinPath(basePath: string, name: string): string {
  if (basePath === "" || basePath === "/") {
    return `${basePath}${name}`;
  }

  return `${basePath.replace(/\/+$/, "")}/${name}`;
}

function transferPercentage(progress: TransferProgress): number {
  if (progress.total_bytes <= 0) return 0;

  return (progress.bytes_transferred / progress.total_bytes) * 100;
}

interface TransferProgressLogState {
  bytesTransferred: number;
  loggedAt: number;
}

export type TransferConflictChoice = "overwrite" | "rename";

export interface TransferConflict {
  fileName: string;
  existingName: string;
  suggestedName: string;
  direction: "Upload" | "Download";
}

interface PendingTransferConflict extends TransferConflict {
  sourcePath: string;
  targetPath: string;
  renamedTargetPath: string;
}

function splitNameAndExtension(name: string): { baseName: string; extension: string } {
  const dotIndex = name.lastIndexOf(".");
  if (dotIndex <= 0) {
    return { baseName: name, extension: "" };
  }

  return {
    baseName: name.slice(0, dotIndex),
    extension: name.slice(dotIndex),
  };
}

function stripNumericCopySuffix(baseName: string): { rootName: string; nextIndex: number } {
  const match = /^(.*) \((\d+)\)$/.exec(baseName);
  if (!match) {
    return { rootName: baseName, nextIndex: 1 };
  }

  const parsed = Number.parseInt(match[2] ?? "", 10);
  return {
    rootName: match[1] ?? baseName,
    nextIndex: Number.isFinite(parsed) ? parsed + 1 : 1,
  };
}

export function nextAvailableTransferName(name: string, entries: FileEntry[]): string {
  const existingNames = new Set(entries.map((entry) => entry.name));
  if (!existingNames.has(name)) {
    return name;
  }

  const { baseName, extension } = splitNameAndExtension(name);
  const { rootName, nextIndex } = stripNumericCopySuffix(baseName);
  let index = nextIndex;
  let candidate = `${rootName} (${index})${extension}`;

  while (existingNames.has(candidate)) {
    index += 1;
    candidate = `${rootName} (${index})${extension}`;
  }

  return candidate;
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
  transferConflict = $state<TransferConflict | null>(null);
  selectedLocal = $state<FileEntry | null>(null);
  selectedRemote = $state<FileEntry | null>(null);
  sftpSessionId = $state<string | null>(null);
  sshSessionId = $state<string | null>(null);
  isDirectConnection = $state(false);

  isConnected = $derived(this.sftpSessionId != null && this.sftpSessionId.length > 0);

  private unlistenProgress: UnlistenFn | null = null;
  private unlistenComplete: UnlistenFn | null = null;
  private unlistenError: UnlistenFn | null = null;
  private nextErrorId = 0;
  private progressLogState = new Map<string, TransferProgressLogState>();
  private pendingTransferConflict: PendingTransferConflict | null = null;

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
      this.cancelTransferConflict();
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
      this.cancelTransferConflict();
      if (this.sftpSessionId) {
        await invoke("sftp_close", { sessionId: this.sftpSessionId });
        this.sftpSessionId = null;
      }

      this.sshSessionId = null;
      this.isDirectConnection = false;
      await this.teardownEventListeners();
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.remoteError = message;
      this.showError(message);
    }
  }

  async connectDirect(options: {
    host: string;
    port: number;
    username: string;
    password?: string;
    privateKey?: string;
    passphrase?: string;
  }): Promise<void> {
    try {
      this.cancelTransferConflict();
      this.sshSessionId = null;
      this.sftpSessionId = await invoke<string>("sftp_connect_direct", {
        host: options.host,
        port: options.port,
        username: options.username,
        password: options.password,
        privateKey: options.privateKey,
        passphrase: options.passphrase,
      });
      this.isDirectConnection = true;
      await this.setupEventListeners();
      try {
        const homeDir = await invoke<string>("sftp_home_dir", {
          sessionId: this.sftpSessionId,
        });
        await this.navigateRemote(homeDir);
      } catch {
        await this.navigateRemote(".");
      }
    } catch (error: unknown) {
      const message = errorMessage(error);
      this.sftpSessionId = null;
      this.isDirectConnection = false;
      this.remoteError = message;
      this.showError(message);
      throw error;
    }
  }

  async disconnect(): Promise<void> {
    await this.closeSftp();
    this.remotePath = "";
    this.remoteFiles = [];
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

    const localPath = joinPath(this.localPath, localEntry.name);
    const remotePath = joinPath(this.remotePath, localEntry.name);
    const targetName = nextAvailableTransferName(localEntry.name, this.remoteFiles);

    if (targetName !== localEntry.name) {
      this.setTransferConflict({
        fileName: localEntry.name,
        existingName: localEntry.name,
        suggestedName: targetName,
        direction: "Upload",
        sourcePath: localPath,
        targetPath: remotePath,
        renamedTargetPath: joinPath(this.remotePath, targetName),
      });
      return undefined;
    }

    try {
      return await this.invokePendingTransfer({
        direction: "Upload",
        sourcePath: localPath,
        targetPath: remotePath,
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

    const remotePath = joinPath(this.remotePath, remoteEntry.name);
    const localPath = joinPath(this.localPath, remoteEntry.name);
    const targetName = nextAvailableTransferName(remoteEntry.name, this.localFiles);

    if (targetName !== remoteEntry.name) {
      this.setTransferConflict({
        fileName: remoteEntry.name,
        existingName: remoteEntry.name,
        suggestedName: targetName,
        direction: "Download",
        sourcePath: remotePath,
        targetPath: localPath,
        renamedTargetPath: joinPath(this.localPath, targetName),
      });
      return undefined;
    }

    try {
      return await this.invokePendingTransfer({
        direction: "Download",
        sourcePath: remotePath,
        targetPath: localPath,
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

  cancelTransferConflict(): void {
    this.transferConflict = null;
    this.pendingTransferConflict = null;
  }

  async resolveTransferConflict(choice: TransferConflictChoice): Promise<string | undefined> {
    const pending = this.pendingTransferConflict;
    if (!pending) {
      this.transferConflict = null;
      return undefined;
    }

    this.transferConflict = null;
    this.pendingTransferConflict = null;
    const targetPath = choice === "rename" ? pending.renamedTargetPath : pending.targetPath;

    try {
      const transferId = await this.invokePendingTransfer({
        direction: pending.direction,
        sourcePath: pending.sourcePath,
        targetPath,
      });
      const action = pending.direction === "Upload" ? "Uploading" : "Downloading";
      const fileName = choice === "rename" ? pending.suggestedName : pending.fileName;
      this.showError(`${action} ${fileName}...`, "info");
      return transferId;
    } catch (error: unknown) {
      this.showError(errorMessage(error));
      return undefined;
    }
  }

  async dropTransfer(
    source: "local" | "remote",
    target: "local" | "remote",
    entry: FileEntry,
  ): Promise<void> {
    if (entry.file_type !== "File") {
      this.showError("Only files can be transferred via drag-and-drop", "warning");
      return;
    }
    if (source === target) return;
    if (target === "remote") {
      if (!this.isConnected) {
        this.showError("Connect to a server before dragging files to Remote", "warning");
        return;
      }
      const localPath = joinPath(this.localPath, entry.name);
      const remotePath = joinPath(this.remotePath, entry.name);
      const targetName = nextAvailableTransferName(entry.name, this.remoteFiles);
      if (targetName !== entry.name) {
        this.setTransferConflict({
          fileName: entry.name,
          existingName: entry.name,
          suggestedName: targetName,
          direction: "Upload",
          sourcePath: localPath,
          targetPath: remotePath,
          renamedTargetPath: joinPath(this.remotePath, targetName),
        });
        return;
      }

      try {
        await this.invokePendingTransfer({
          direction: "Upload",
          sourcePath: localPath,
          targetPath: remotePath,
        });
        this.showError(`Uploading ${entry.name}...`, "info");
      } catch (error: unknown) {
        this.showError(errorMessage(error));
      }
    } else {
      if (!this.isConnected) {
        this.showError("Connect to a server before dragging files from Remote", "warning");
        return;
      }
      const remotePath = joinPath(this.remotePath, entry.name);
      const localPath = joinPath(this.localPath, entry.name);
      const targetName = nextAvailableTransferName(entry.name, this.localFiles);
      if (targetName !== entry.name) {
        this.setTransferConflict({
          fileName: entry.name,
          existingName: entry.name,
          suggestedName: targetName,
          direction: "Download",
          sourcePath: remotePath,
          targetPath: localPath,
          renamedTargetPath: joinPath(this.localPath, targetName),
        });
        return;
      }

      try {
        await this.invokePendingTransfer({
          direction: "Download",
          sourcePath: remotePath,
          targetPath: localPath,
        });
        this.showError(`Downloading ${entry.name}...`, "info");
      } catch (error: unknown) {
        this.showError(errorMessage(error));
      }
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
    this.transferConflict = null;
    this.pendingTransferConflict = null;
    this.progressLogState.clear();
    this.selectedLocal = null;
    this.selectedRemote = null;
    this.sftpSessionId = null;
    this.sshSessionId = null;
    this.isDirectConnection = false;
  }

  private async setupEventListeners(): Promise<void> {
    if (!this.unlistenProgress) {
      console.info("[SFTP][Store] registering progress listener");
      this.unlistenProgress = await listen<TransferProgress>(
        "sftp://progress",
        (event) => {
          const progress = new Map(this.activeTransfers);
          progress.set(event.payload.transfer_id, event.payload);
          this.activeTransfers = progress;
          this.logTransferProgress(event.payload);
        },
      );
    }

    if (!this.unlistenComplete) {
      console.info("[SFTP][Store] registering complete listener");
      this.unlistenComplete = await listen<TransferComplete>(
        "sftp://complete",
        (event) => {
          console.info("[SFTP][Store] complete event", event.payload);
          this.progressLogState.delete(event.payload.transfer_id);
          const progress = new Map(this.activeTransfers);
          progress.delete(event.payload.transfer_id);
          this.activeTransfers = progress;
          if (event.payload.direction === "Download") {
            void this.refreshLocal();
          } else {
            void this.refreshRemote();
          }
        },
      );
    }

    if (!this.unlistenError) {
      console.info("[SFTP][Store] registering error listener");
      this.unlistenError = await listen<TransferError>(
        "sftp://error",
        (event) => {
          console.error("[SFTP][Store] error event", event.payload);
          this.progressLogState.delete(event.payload.transfer_id);
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
    console.info("[SFTP][Store] tearing down transfer listeners");
    this.unlistenProgress?.();
    this.unlistenComplete?.();
    this.unlistenError?.();
    this.unlistenProgress = null;
    this.unlistenComplete = null;
    this.unlistenError = null;
    this.progressLogState.clear();
  }

  private logTransferProgress(progress: TransferProgress): void {
    const now = Date.now();
    const previous = this.progressLogState.get(progress.transfer_id);
    const bytesDelta = previous
      ? progress.bytes_transferred - previous.bytesTransferred
      : progress.bytes_transferred;
    const elapsedMs = previous ? now - previous.loggedAt : TRANSFER_PROGRESS_LOG_MS;
    const complete = progress.total_bytes > 0
      && progress.bytes_transferred >= progress.total_bytes;

    if (previous && !complete && bytesDelta < TRANSFER_PROGRESS_LOG_BYTES && elapsedMs < TRANSFER_PROGRESS_LOG_MS) {
      return;
    }

    console.info("[SFTP][Store] progress event", {
      transferId: progress.transfer_id,
      direction: progress.direction,
      bytesTransferred: progress.bytes_transferred,
      totalBytes: progress.total_bytes,
      percentage: transferPercentage(progress),
      speedBps: progress.speed_bps,
    });

    this.progressLogState.set(progress.transfer_id, {
      bytesTransferred: progress.bytes_transferred,
      loggedAt: now,
    });
  }

  private setTransferConflict(conflict: PendingTransferConflict): void {
    this.pendingTransferConflict = conflict;
    this.transferConflict = {
      fileName: conflict.fileName,
      existingName: conflict.existingName,
      suggestedName: conflict.suggestedName,
      direction: conflict.direction,
    };
  }

  private async invokePendingTransfer(options: {
    direction: "Upload" | "Download";
    sourcePath: string;
    targetPath: string;
  }): Promise<string> {
    if (options.direction === "Upload") {
      return await invoke<string>("sftp_upload", {
        sessionId: this.sftpSessionId,
        localPath: options.sourcePath,
        remotePath: options.targetPath,
      });
    }

    return await invoke<string>("sftp_download", {
      sessionId: this.sftpSessionId,
      remotePath: options.sourcePath,
      localPath: options.targetPath,
    });
  }
}

export function createSftpStore(): SftpStore {
  return new SftpStore();
}

export const sftpStore = createSftpStore();
