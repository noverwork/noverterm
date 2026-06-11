export type FileType = "File" | "Dir" | "Symlink" | "Other";

export interface FileEntry {
  name: string;
  size: number;
  modified: number | null;
  file_type: FileType;
}

export type TransferDirection = "Upload" | "Download";

export interface TransferProgress {
  transfer_id: string;
  bytes_transferred: number;
  total_bytes: number;
  speed_bps: number;
  direction: TransferDirection;
}

export interface TransferComplete {
  transfer_id: string;
  total_bytes: number;
  direction: TransferDirection;
}

export interface TransferError {
  transfer_id: string;
  error: string;
  direction: TransferDirection;
}
