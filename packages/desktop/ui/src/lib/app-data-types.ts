import type { SshHostAuthMaterial } from "$lib/api/types.js";

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

export interface SaveConnectionInput {
  id?: string;
  name: string;
  groupId?: string | null;
  host: string;
  port: number;
  username: string;
  password?: string;
  preservedPassword?: string;
  privateKey?: string;
  passphrase?: string;
  keyName?: string;
  existingKeyId?: string | null;
}
