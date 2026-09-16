import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SvelteMap, SvelteSet } from "svelte/reactivity";

import { commands, type HostTrustMismatch, type HostTrustPrompt, type Result } from "../../bindings";
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

function unwrapCommandResult<T>(result: Result<T, string>): T {
  if (result.status === "error") {
    throw new Error(result.error);
  }

  return result.data;
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
  isDirectory: boolean;
  /** Paths inside the folder that already exist; null while still scanning. */
  conflictingFiles: string[] | null;
}

interface PendingTransferConflict extends Omit<TransferConflict, "conflictingFiles"> {
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
  const existingNames = new SvelteSet(entries.map((entry) => entry.name));
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

export interface SftpConnection {
  name: string;
  host: string;
  port: number;
  username: string;
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
  activeTransfers = $state<SvelteMap<string, TransferProgress>>(new SvelteMap());
  transferConflict = $state<TransferConflict | null>(null);
  selectedLocal = $state<FileEntry | null>(null);
  selectedRemote = $state<FileEntry | null>(null);
  sftpSessionId = $state<string | null>(null);
  sshSessionId = $state<string | null>(null);
  isDirectConnection = $state(false);
  connection = $state<SftpConnection | null>(null);
  connectionId = $state<string | null>(null);
  connectionError = $state<string | null>(null);
  trustPrompt = $state<HostTrustPrompt | null>(null);
  trustMismatch = $state<HostTrustMismatch | null>(null);
  isConnecting = $state(false);
  isClosing = $state(false);
  attemptedActiveSshSessionId = $state<string | null>(null);

  isConnected = $derived(this.sftpSessionId != null && this.sftpSessionId.length > 0);

  private unlistenProgress: UnlistenFn | null = null;
  private unlistenComplete: UnlistenFn | null = null;
  private unlistenError: UnlistenFn | null = null;
  private nextErrorId = 0;
  private progressLogState = new SvelteMap<string, TransferProgressLogState>();
  private pendingTransferConflict: PendingTransferConflict | null = null;
  private connectionGeneration = 0;
  private remoteRequestId = 0;

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
      unwrapCommandResult(await commands.localMkdir(joinPath(this.localPath, name)));
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

  async openSftp(sshSessionId: string, connection?: SftpConnection): Promise<void> {
    const generation = ++this.connectionGeneration;
    this.isConnecting = true;
    this.isDirectConnection = false;
    this.connectionId = null;
    this.connectionError = null;
    this.remoteError = null;
    this.trustPrompt = null;
    this.trustMismatch = null;
    this.connection = connection ? {
      name: connection.name,
      host: connection.host,
      port: connection.port,
      username: connection.username,
    } : null;
    try {
      this.cancelTransferConflict();
      this.sshSessionId = sshSessionId;
      const sessionId = unwrapCommandResult(await commands.sftpOpen(sshSessionId));
      if (generation !== this.connectionGeneration) {
        unwrapCommandResult(await commands.sftpClose(sessionId));
        return;
      }
      this.sftpSessionId = sessionId;
      await this.setupEventListeners(generation);
    } catch (error: unknown) {
      await this.failConnection(error, generation);
    } finally {
      if (generation === this.connectionGeneration) {
        this.isConnecting = false;
      }
    }
  }

  async closeSftp(): Promise<void> {
    if (this.isClosing) return;
    const generation = ++this.connectionGeneration;
    const sessionId = this.sftpSessionId;
    this.isClosing = sessionId !== null;
    this.isConnecting = false;
    this.cancelTransferConflict();
    this.sftpSessionId = null;
    this.sshSessionId = null;
    this.isDirectConnection = false;
    this.connection = null;
    this.connectionId = null;
    this.connectionError = null;
    this.trustPrompt = null;
    this.trustMismatch = null;
    this.remotePath = "";
    this.remoteFiles = [];
    this.selectedRemote = null;
    this.remoteLoading = false;
    this.remoteError = null;
    this.activeTransfers.clear();
    try {
      this.teardownEventListeners();
      if (sessionId) {
        unwrapCommandResult(await commands.sftpClose(sessionId));
      }
    } catch (error: unknown) {
      if (generation === this.connectionGeneration) {
        this.showError(`Disconnected. Remote cleanup failed: ${errorMessage(error)}`, "warning");
      }
    } finally {
      if (generation === this.connectionGeneration) {
        this.isClosing = false;
      }
    }
  }

  async connectDirect(options: {
    connectionId?: string;
    name?: string;
    host: string;
    port: number;
    username: string;
    password?: string;
    privateKey?: string;
    passphrase?: string;
  }): Promise<void> {
    const generation = ++this.connectionGeneration;
    this.isConnecting = true;
    this.isDirectConnection = true;
    this.connectionError = null;
    this.remoteError = null;
    this.trustPrompt = null;
    this.trustMismatch = null;
    this.connectionId = options.connectionId ?? null;
    this.connection = {
      name: options.name ?? options.host,
      host: options.host,
      port: options.port,
      username: options.username,
    };
    try {
      this.cancelTransferConflict();
      this.sshSessionId = null;
      const response = unwrapCommandResult(await commands.sftpConnectDirect(
        options.host,
        options.port,
        options.username,
        options.password ?? null,
        options.privateKey ?? null,
        options.passphrase ?? null,
      ));
      if (generation !== this.connectionGeneration) {
        if (response.status === "connected") {
          unwrapCommandResult(await commands.sftpClose(response.session_id));
        }
        return;
      }
      if (response.status === "trust_required") {
        this.trustPrompt = response.prompt;
        return;
      }
      if (response.status === "trust_mismatch") {
        this.trustMismatch = response.mismatch;
        this.connectionError = `Host trust mismatch for ${response.mismatch.host}: expected ${response.mismatch.expected_fingerprint}, got ${response.mismatch.presented_fingerprint}`;
        return;
      }
      const sessionId = response.session_id;
      this.sftpSessionId = sessionId;
      await this.setupEventListeners(generation);
      if (generation !== this.connectionGeneration) return;
      let homeDir = ".";
      try {
        homeDir = unwrapCommandResult(await commands.sftpHomeDir(sessionId));
      } catch {
        // Some servers do not expose a home directory; use their working directory.
      }
      if (generation === this.connectionGeneration) {
        await this.navigateRemote(homeDir);
      }
    } catch (error: unknown) {
      await this.failConnection(error, generation);
    } finally {
      if (generation === this.connectionGeneration) {
        this.isConnecting = false;
      }
    }
  }

  private async failConnection(error: unknown, generation: number): Promise<void> {
    if (generation !== this.connectionGeneration) return;
    const sessionId = this.sftpSessionId;
    this.sftpSessionId = null;
    this.remotePath = "";
    this.remoteFiles = [];
    this.selectedRemote = null;
    this.remoteLoading = false;
    this.activeTransfers.clear();
    this.teardownEventListeners();
    this.connectionError = errorMessage(error);
    this.remoteError = this.connectionError;
    if (sessionId) {
      try {
        unwrapCommandResult(await commands.sftpClose(sessionId));
      } catch (cleanupError: unknown) {
        if (generation === this.connectionGeneration) {
          this.showError(`Remote cleanup failed: ${errorMessage(cleanupError)}`, "warning");
        }
      }
    }
  }

  async disconnect(): Promise<void> {
    await this.closeSftp();
  }

  async navigateRemote(path: string): Promise<void> {
    const sessionId = this.sftpSessionId;
    if (!sessionId) {
      this.remoteError = "SFTP session is not connected.";
      this.showError(this.remoteError, "warning");
      return;
    }

    const generation = this.connectionGeneration;
    const requestId = ++this.remoteRequestId;
    this.remoteLoading = true;
    this.remoteError = null;
    this.remotePath = path;
    this.selectedRemote = null;

    try {
      const files = unwrapCommandResult(await commands.sftpListDir(sessionId, path));
      if (generation === this.connectionGeneration && requestId === this.remoteRequestId) {
        this.remoteFiles = files;
      }
    } catch (error: unknown) {
      if (generation === this.connectionGeneration && requestId === this.remoteRequestId) {
        const message = errorMessage(error);
        this.remoteError = message;
        this.showError(message);
      }
    } finally {
      if (generation === this.connectionGeneration && requestId === this.remoteRequestId) {
        this.remoteLoading = false;
      }
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
      unwrapCommandResult(
        await commands.sftpMkdir(this.sftpSessionId, joinPath(this.remotePath, name)),
      );
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
        isDirectory: localEntry.file_type === "Dir",
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
        isDirectory: remoteEntry.file_type === "Dir",
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
          isDirectory: entry.file_type === "Dir",
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
          isDirectory: entry.file_type === "Dir",
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
    this.connectionGeneration += 1;
    this.teardownEventListeners();
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
    this.activeTransfers = new SvelteMap();
    this.transferConflict = null;
    this.pendingTransferConflict = null;
    this.progressLogState.clear();
    this.selectedLocal = null;
    this.selectedRemote = null;
    this.sftpSessionId = null;
    this.sshSessionId = null;
    this.isDirectConnection = false;
    this.connection = null;
    this.connectionId = null;
    this.connectionError = null;
    this.trustPrompt = null;
    this.trustMismatch = null;
    this.isConnecting = false;
    this.isClosing = false;
    this.attemptedActiveSshSessionId = null;
  }

  private async setupEventListeners(generation: number): Promise<void> {
    if (!this.unlistenProgress) {
      console.info("[SFTP][Store] registering progress listener");
      const unlisten = await listen<TransferProgress>(
        "sftp://progress",
        (event) => {
          if (generation !== this.connectionGeneration) return;
          this.activeTransfers.set(event.payload.transfer_id, event.payload);
          this.logTransferProgress(event.payload);
        },
      );
      if (generation !== this.connectionGeneration) {
        unlisten();
        return;
      }
      this.unlistenProgress = unlisten;
    }

    if (!this.unlistenComplete) {
      console.info("[SFTP][Store] registering complete listener");
      const unlisten = await listen<TransferComplete>(
        "sftp://complete",
        (event) => {
          if (generation !== this.connectionGeneration) return;
          console.info("[SFTP][Store] complete event", event.payload);
          this.progressLogState.delete(event.payload.transfer_id);
          this.activeTransfers.delete(event.payload.transfer_id);
          if (event.payload.direction === "Download") {
            void this.refreshLocal();
          } else {
            void this.refreshRemote();
          }
        },
      );
      if (generation !== this.connectionGeneration) {
        unlisten();
        return;
      }
      this.unlistenComplete = unlisten;
    }

    if (!this.unlistenError) {
      console.info("[SFTP][Store] registering error listener");
      const unlisten = await listen<TransferError>(
        "sftp://error",
        (event) => {
          if (generation !== this.connectionGeneration) return;
          console.error("[SFTP][Store] error event", event.payload);
          this.progressLogState.delete(event.payload.transfer_id);
          this.activeTransfers.delete(event.payload.transfer_id);
          this.remoteError = event.payload.error;
          this.showError(event.payload.error);
        },
      );
      if (generation !== this.connectionGeneration) {
        unlisten();
        return;
      }
      this.unlistenError = unlisten;
    }
  }

  private teardownEventListeners(): void {
    console.info("[SFTP][Store] tearing down transfer listeners");
    const listeners = [this.unlistenProgress, this.unlistenComplete, this.unlistenError];
    this.unlistenProgress = null;
    this.unlistenComplete = null;
    this.unlistenError = null;
    this.progressLogState.clear();
    for (const unlisten of listeners) {
      try {
        unlisten?.();
      } catch (error: unknown) {
        this.showError(`Could not remove SFTP listener: ${errorMessage(error)}`, "warning");
      }
    }
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
      isDirectory: conflict.isDirectory,
      conflictingFiles: conflict.isDirectory ? null : [],
    };

    if (conflict.isDirectory) {
      void this.loadFolderConflicts(conflict);
    }
  }

  /** Fill in which files inside a folder an overwrite would clobber. */
  private async loadFolderConflicts(conflict: PendingTransferConflict): Promise<void> {
    let files: string[] = [];
    try {
      files = await invoke<string[]>("sftp_transfer_conflicts", {
        sessionId: this.sftpSessionId,
        direction: conflict.direction,
        sourcePath: conflict.sourcePath,
        targetPath: conflict.targetPath,
      });
    } catch (error: unknown) {
      console.warn("[SFTP][Store] folder conflict scan failed", errorMessage(error));
    }

    if (this.pendingTransferConflict !== conflict || !this.transferConflict) return;
    this.transferConflict = { ...this.transferConflict, conflictingFiles: files };
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
