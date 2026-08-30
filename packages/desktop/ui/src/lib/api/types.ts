import type { HostGroupRecord } from "../../bindings/host-group-record.js";
import type { Setting } from "../../bindings/setting.js";
import type { SshHostRecord } from "../../bindings/ssh-host-record.js";
import type { SshKeyRecord } from "../../bindings/ssh-key-record.js";

export type { HostGroupRecord } from "../../bindings/host-group-record.js";
export type { Setting } from "../../bindings/setting.js";
export type { SnippetRecord } from "../../bindings/snippet-record.js";
export type { SnippetWriteRequest } from "../../bindings/snippet-write-request.js";
export type { SshHostAuthMaterial } from "../../bindings/ssh-host-auth-material.js";
export type { SshHostRecord } from "../../bindings/ssh-host-record.js";
export type { SshKeyRecord } from "../../bindings/ssh-key-record.js";
export type { SshKeySecret } from "../../bindings/ssh-key-secret.js";
export type { KeyInput, KeyUpdateInput } from "../../bindings.js";

export interface AppDataMetadata {
  settings: Setting[];
  host_groups: HostGroupRecord[];
  hosts: SshHostRecord[];
  keys: SshKeyRecord[];
}
