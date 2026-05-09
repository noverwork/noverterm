import type { SshHostAuthMaterial } from "$lib/api/types.js";

export type AppDataPhase =
  | "loading"
  | "authenticated"
  | "unauthenticated"
  | "error";

export interface TerminalConfig {
  fontSize: number;
  fontFamily: string;
  cursorStyle: "block" | "underline" | "bar";
  cursorBlink: boolean;
  scrollback: number;
}

export interface ConnectionConfig {
  id: string;
  name: string;
  groupId: string | null;
  host: string;
  port: number;
  username: string;
  sshKeyId: string | null;
  hasPassword: boolean;
  auth: SshHostAuthMaterial | null;
}

export interface SavedPortForwardConfig {
  id: string;
  name: string;
  connectionId: string;
  bind_host: string;
  bind_port: number;
  target_host: string;
  target_port: number;
}

export interface SavePortForwardInput {
  id?: string;
  name: string;
  connectionId: string;
  bind_host: string;
  bind_port: number;
  target_host: string;
  target_port: number;
}

export interface SaveConnectionInput {
  id?: string;
  name: string;
  groupId?: string | null;
  host: string;
  port: number;
  username: string;
  password?: string;
  preservedEncryptedPassword?: string;
  privateKey?: string;
  passphrase?: string;
  keyName?: string;
  existingKeyId?: string | null;
}
