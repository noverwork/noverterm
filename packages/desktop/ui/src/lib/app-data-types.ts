import type { SshHostAuthMaterial } from "$lib/api/types.js";
import type { AiAccess, SshHopInput } from "../bindings.js";

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
  jumpHostId: string | null;
  /** Resolved jump chain, outermost bastion first. */
  jumps: SshHopInput[];
  /** What MCP clients may do on this host without asking. */
  aiAccess: AiAccess;
}

export interface SaveConnectionInput {
  id?: string;
  name: string;
  groupId?: string | null;
  jumpHostId?: string | null;
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
