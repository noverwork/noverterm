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
  /** Label of the pane the file would land in, e.g. "Local" or a host name. */
  destination: string;
  isDirectory: boolean;
  /** Paths inside the folder that already exist; null while still scanning. */
  conflictingFiles: string[] | null;
}

interface PendingTransferConflict extends Omit<TransferConflict, "conflictingFiles"> {
  source: SftpPane;
  target: SftpPane;
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


export type PaneSide = "left" | "right";

/** Identifies a machine so both panes never show the same one. */
export function machineKey(target: { host: string; port: number }): string {
  return `${target.host}:${target.port}`;
}

export const LOCAL_MACHINE = "local";

/** One side of the file browser: this machine, or an SFTP session to any host. */
export class SftpPane {
  mode = $state<"local" | "remote">("remote");
  path = $state<string>("");
  files = $state<FileEntry[]>([]);
  loading = $state(false);
  error = $state<string | null>(null);
  selected = $state<FileEntry | null>(null);
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
  trustConfirming = $state(false);
  trustError = $state<string | null>(null);
  /** Bumped by the route to drop replies from connect attempts the user abandoned. */
  attemptGeneration = 0;

  isLocal = $derived(this.mode === "local");
  isConnected = $derived(
    this.mode === "remote" && this.sftpSessionId != null && this.sftpSessionId.length > 0,
  );
  /** Has a listing that files can be transferred to or from. */
  isReady = $derived(this.isLocal || this.isConnected);
  label = $derived(this.isLocal ? "Local" : this.connection?.name ?? "Remote");
  /** Machine this pane shows or is connecting to; null while picking. */
  machine = $derived(
    this.isLocal ? LOCAL_MACHINE : this.connection ? machineKey(this.connection) : null,
  );

  private connectionGeneration = 0;
  private requestId = 0;

  constructor(
    readonly side: PaneSide,
    private readonly store: SftpStore,
    mode: "local" | "remote",
  ) {
    this.mode = mode;
    this.path = mode === "local" ? "~" : "";
  }

  async useLocal(): Promise<void> {
    if (this.mode === "remote") {
      await this.closeSftp();
    }
    this.mode = "local";
    await this.navigate("~");
  }

  /** Leave local mode and show the machine picker. */
  chooseMachine(): void {
    this.connectionGeneration += 1;
    this.mode = "remote";
    this.path = "";
    this.files = [];
    this.selected = null;
    this.loading = false;
    this.error = null;
  }

  async navigate(path: string): Promise<void> {
    const sessionId = this.sftpSessionId;
    if (!this.isLocal && !sessionId) {
      this.error = "SFTP session is not connected.";
      this.store.showError(this.error, "warning");
      return;
    }

    const generation = this.connectionGeneration;
    const requestId = ++this.requestId;
    this.loading = true;
    this.error = null;
    this.path = path;
    this.selected = null;
    const isCurrent = () =>
      generation === this.connectionGeneration && requestId === this.requestId;

    try {
      const files = sessionId && !this.isLocal
        ? unwrapCommandResult(await commands.sftpListDir(sessionId, path))
        : await invoke<FileEntry[]>("local_list_dir", { path });
      if (isCurrent()) {
        this.files = files;
      }
    } catch (error: unknown) {
      if (isCurrent()) {
        this.error = errorMessage(error);
        this.store.showError(this.error);
      }
    } finally {
      if (isCurrent()) {
        this.loading = false;
      }
    }
  }

  async refresh(): Promise<void> {
    await this.navigate(this.path);
  }

  async mkdir(name: string): Promise<void> {
    await this.runFileOperation(async () => {
      const path = joinPath(this.path, name);
      unwrapCommandResult(await (this.isLocal
        ? commands.localMkdir(path)
        : commands.sftpMkdir(this.requireSession(), path)));
    });
  }

  async remove(entry: FileEntry): Promise<void> {
    await this.runFileOperation(async () => {
      const path = joinPath(this.path, entry.name);
      if (this.isLocal) {
        await invoke("local_remove", { path });
      } else {
        await invoke("sftp_remove", { sessionId: this.requireSession(), path });
      }
    });
  }

  async rename(entry: FileEntry, newName: string): Promise<void> {
    await this.runFileOperation(async () => {
      const oldPath = joinPath(this.path, entry.name);
      const newPath = joinPath(this.path, newName);
      if (this.isLocal) {
        await invoke("local_rename", { oldPath, newPath });
      } else {
        await invoke("sftp_rename", { sessionId: this.requireSession(), oldPath, newPath });
      }
    });
  }

  async openSftp(sshSessionId: string, connection?: SftpConnection): Promise<void> {
    const generation = ++this.connectionGeneration;
    this.mode = "remote";
    this.isConnecting = true;
    this.isDirectConnection = false;
    this.connectionId = null;
    this.connectionError = null;
    this.error = null;
    this.trustPrompt = null;
    this.trustMismatch = null;
    this.connection = connection ? {
      name: connection.name,
      host: connection.host,
      port: connection.port,
      username: connection.username,
    } : null;
    try {
      this.store.cancelTransferConflict();
      this.sshSessionId = sshSessionId;
      const sessionId = unwrapCommandResult(await commands.sftpOpen(sshSessionId));
      if (generation !== this.connectionGeneration) {
        unwrapCommandResult(await commands.sftpClose(sessionId));
        return;
      }
      this.sftpSessionId = sessionId;
      await this.store.setupEventListeners();
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
    this.store.cancelTransferConflict();
    this.sftpSessionId = null;
    this.sshSessionId = null;
    this.isDirectConnection = false;
    this.connection = null;
    this.connectionId = null;
    this.connectionError = null;
    this.trustPrompt = null;
    this.trustMismatch = null;
    this.path = "";
    this.files = [];
    this.selected = null;
    this.loading = false;
    this.error = null;
    try {
      if (sessionId) {
        unwrapCommandResult(await commands.sftpClose(sessionId));
      }
    } catch (error: unknown) {
      if (generation === this.connectionGeneration) {
        this.store.showError(`Disconnected. Remote cleanup failed: ${errorMessage(error)}`, "warning");
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
    this.mode = "remote";
    this.isConnecting = true;
    this.isDirectConnection = true;
    this.connectionError = null;
    this.error = null;
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
      this.store.cancelTransferConflict();
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
      await this.store.setupEventListeners();
      if (generation !== this.connectionGeneration) return;
      let homeDir = ".";
      try {
        homeDir = unwrapCommandResult(await commands.sftpHomeDir(sessionId));
      } catch {
        // Some servers do not expose a home directory; use their working directory.
      }
      if (generation === this.connectionGeneration) {
        await this.navigate(homeDir);
      }
    } catch (error: unknown) {
      await this.failConnection(error, generation);
    } finally {
      if (generation === this.connectionGeneration) {
        this.isConnecting = false;
      }
    }
  }

  async disconnect(): Promise<void> {
    await this.closeSftp();
  }

  reset(): void {
    this.connectionGeneration += 1;
    const mode = this.side === "left" ? "local" : "remote";
    this.mode = mode;
    this.path = mode === "local" ? "~" : "";
    this.files = [];
    this.loading = false;
    this.error = null;
    this.selected = null;
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
    this.trustConfirming = false;
    this.trustError = null;
  }

  private requireSession(): string {
    if (!this.sftpSessionId) throw new Error("SFTP session is not connected.");
    return this.sftpSessionId;
  }

  private async runFileOperation(operation: () => Promise<void>): Promise<void> {
    try {
      await operation();
      await this.refresh();
    } catch (error: unknown) {
      this.error = errorMessage(error);
      this.store.showError(this.error);
    }
  }

  private async failConnection(error: unknown, generation: number): Promise<void> {
    if (generation !== this.connectionGeneration) return;
    const sessionId = this.sftpSessionId;
    this.sftpSessionId = null;
    this.path = "";
    this.files = [];
    this.selected = null;
    this.loading = false;
    this.connectionError = errorMessage(error);
    this.error = this.connectionError;
    if (sessionId) {
      try {
        unwrapCommandResult(await commands.sftpClose(sessionId));
      } catch (cleanupError: unknown) {
        if (generation === this.connectionGeneration) {
          this.store.showError(`Remote cleanup failed: ${errorMessage(cleanupError)}`, "warning");
        }
      }
    }
  }
}

export class SftpStore {
  readonly left: SftpPane = new SftpPane("left", this, "local");
  readonly right: SftpPane = new SftpPane("right", this, "remote");
  lastError = $state<string | null>(null);
  errorQueue = $state<ErrorToast[]>([]);
  activeTransfers = $state<SvelteMap<string, TransferProgress>>(new SvelteMap());
  transferConflict = $state<TransferConflict | null>(null);
  attemptedActiveSshSessionId = $state<string | null>(null);

  private listenersReady: Promise<void> | null = null;
  private unlisteners: UnlistenFn[] = [];
  private listenerGeneration = 0;
  private nextErrorId = 0;
  private progressLogState = new SvelteMap<string, TransferProgressLogState>();
  private pendingTransferConflict: PendingTransferConflict | null = null;

  pane(side: PaneSide): SftpPane {
    return side === "left" ? this.left : this.right;
  }

  otherPane(pane: SftpPane): SftpPane {
    return pane === this.left ? this.right : this.left;
  }

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

  /** Copy `entry` from `source` into the other pane's current folder. */
  async transfer(source: SftpPane, entry: FileEntry): Promise<string | undefined> {
    const target = this.otherPane(source);
    if (source.machine !== null && source.machine === target.machine) {
      this.showError("Both sides are the same machine. Pick a different one on one side.", "warning");
      return undefined;
    }
    if (!source.isReady || !target.isReady) {
      this.showError("Connect to a server on both sides before transferring files", "warning");
      return undefined;
    }

    const sourcePath = joinPath(source.path, entry.name);
    const targetPath = joinPath(target.path, entry.name);
    const targetName = nextAvailableTransferName(entry.name, target.files);

    if (targetName !== entry.name) {
      this.setTransferConflict({
        fileName: entry.name,
        existingName: entry.name,
        suggestedName: targetName,
        destination: target.label,
        isDirectory: entry.file_type === "Dir",
        source,
        target,
        sourcePath,
        targetPath,
        renamedTargetPath: joinPath(target.path, targetName),
      });
      return undefined;
    }

    return await this.startTransfer(source, target, sourcePath, targetPath, entry.name);
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
    const fileName = choice === "rename" ? pending.suggestedName : pending.fileName;
    return await this.startTransfer(
      pending.source,
      pending.target,
      pending.sourcePath,
      targetPath,
      fileName,
    );
  }

  cleanup(): void {
    this.listenerGeneration += 1;
    this.teardownEventListeners();
    this.left.reset();
    this.right.reset();
    this.lastError = null;
    this.errorQueue = [];
    this.activeTransfers = new SvelteMap();
    this.transferConflict = null;
    this.pendingTransferConflict = null;
    this.progressLogState.clear();
    this.attemptedActiveSshSessionId = null;
  }

  /** Transfer events are shared by both panes, so listen once until cleanup. */
  async setupEventListeners(): Promise<void> {
    this.listenersReady ??= this.registerEventListeners(this.listenerGeneration);
    await this.listenersReady;
  }

  private async registerEventListeners(generation: number): Promise<void> {
    const handlers: Array<[string, (payload: never) => void]> = [
      ["sftp://progress", (payload: TransferProgress) => {
        this.activeTransfers.set(payload.transfer_id, payload);
        this.logTransferProgress(payload);
      }],
      ["sftp://complete", (payload: TransferComplete) => {
        console.info("[SFTP][Store] complete event", payload);
        this.finishTransfer(payload.transfer_id);
        for (const pane of [this.left, this.right]) {
          if (pane.isReady) void pane.refresh();
        }
      }],
      ["sftp://error", (payload: TransferError) => {
        console.error("[SFTP][Store] error event", payload);
        this.finishTransfer(payload.transfer_id);
        this.showError(payload.error);
      }],
    ];

    for (const [name, handler] of handlers) {
      console.info("[SFTP][Store] registering listener", name);
      const unlisten = await listen(name, (event) => {
        if (generation === this.listenerGeneration) handler(event.payload as never);
      });
      if (generation !== this.listenerGeneration) {
        unlisten();
        return;
      }
      this.unlisteners.push(unlisten);
    }
  }

  private finishTransfer(transferId: string): void {
    this.progressLogState.delete(transferId);
    this.activeTransfers.delete(transferId);
  }

  private teardownEventListeners(): void {
    const unlisteners = this.unlisteners;
    this.unlisteners = [];
    this.listenersReady = null;
    this.progressLogState.clear();
    for (const unlisten of unlisteners) {
      try {
        unlisten();
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
      destination: conflict.destination,
      isDirectory: conflict.isDirectory,
      conflictingFiles: conflict.isDirectory ? null : [],
    };

    if (conflict.isDirectory) {
      void this.loadFolderConflicts(conflict);
    }
  }

  /** Fill in which files inside a folder an overwrite would clobber. */
  private async loadFolderConflicts(conflict: PendingTransferConflict): Promise<void> {
    const { source, target, sourcePath, targetPath } = conflict;
    let files: string[] = [];
    try {
      files = source.isLocal || target.isLocal
        ? await invoke<string[]>("sftp_transfer_conflicts", {
          sessionId: source.isLocal ? target.sftpSessionId : source.sftpSessionId,
          direction: source.isLocal ? "Upload" : "Download",
          sourcePath,
          targetPath,
        })
        : await invoke<string[]>("sftp_copy_conflicts", {
          sourceSessionId: source.sftpSessionId,
          sourcePath,
          targetSessionId: target.sftpSessionId,
          targetPath,
        });
    } catch (error: unknown) {
      console.warn("[SFTP][Store] folder conflict scan failed", errorMessage(error));
    }

    if (this.pendingTransferConflict !== conflict || !this.transferConflict) return;
    this.transferConflict = { ...this.transferConflict, conflictingFiles: files };
  }

  private async startTransfer(
    source: SftpPane,
    target: SftpPane,
    sourcePath: string,
    targetPath: string,
    fileName: string,
  ): Promise<string | undefined> {
    try {
      await this.setupEventListeners();
      let transferId: string;
      let action: string;
      if (source.isLocal) {
        action = "Uploading";
        transferId = await invoke<string>("sftp_upload", {
          sessionId: target.sftpSessionId,
          localPath: sourcePath,
          remotePath: targetPath,
        });
      } else if (target.isLocal) {
        action = "Downloading";
        transferId = await invoke<string>("sftp_download", {
          sessionId: source.sftpSessionId,
          remotePath: sourcePath,
          localPath: targetPath,
        });
      } else {
        action = "Copying";
        transferId = await invoke<string>("sftp_copy", {
          sourceSessionId: source.sftpSessionId,
          sourcePath,
          targetSessionId: target.sftpSessionId,
          targetPath,
        });
      }
      this.showError(`${action} ${fileName}...`, "info");
      return transferId;
    } catch (error: unknown) {
      this.showError(errorMessage(error));
      return undefined;
    }
  }
}

export function createSftpStore(): SftpStore {
  return new SftpStore();
}

export const sftpStore = createSftpStore();
