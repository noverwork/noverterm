import type { HostGroupRecord } from "../../bindings/host-group-record.js";
import type { KeyWriteRequest } from "../../bindings/key-write-request.js";
import type { Setting } from "../../bindings/setting.js";
import type { SshHostRecord } from "../../bindings/ssh-host-record.js";
import type { SshKeyRecord } from "../../bindings/ssh-key-record.js";

export type { AuthResponse } from "../../bindings/auth-response.js";
export type { ForgotPasswordRequest } from "../../bindings/forgot-password-request.js";
export type { HostGroupRecord } from "../../bindings/host-group-record.js";
export type { HostGroupWriteRequest } from "../../bindings/host-group-write-request.js";
export type { HostWriteRequest } from "../../bindings/host-write-request.js";
export type { KeyUpdateRequest } from "../../bindings/key-update-request.js";
export type { KeyWriteRequest } from "../../bindings/key-write-request.js";
export type { LoginRequest } from "../../bindings/login-request.js";
export type { LogoutRequest } from "../../bindings/logout-request.js";
export type { RefreshRequest } from "../../bindings/refresh-request.js";
export type { RegisterRequest } from "../../bindings/register-request.js";
export type { ResetPasswordRequest } from "../../bindings/reset-password-request.js";
export type { Setting } from "../../bindings/setting.js";
export type { SshHostAuthMaterial } from "../../bindings/ssh-host-auth-material.js";
export type { SshHostRecord } from "../../bindings/ssh-host-record.js";
export type { SshKeyRecord } from "../../bindings/ssh-key-record.js";
export type { SshKeySecret } from "../../bindings/ssh-key-secret.js";

export interface AppDataMetadata {
  settings: Setting[];
  host_groups: HostGroupRecord[];
  hosts: SshHostRecord[];
  keys: SshKeyRecord[];
}

export type KeyCreateRequest = KeyWriteRequest;
