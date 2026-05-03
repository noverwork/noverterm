export interface Setting {
  key: string;
  value: string;
}

export interface HostGroupRecord {
  id: string;
  name: string;
}

export interface SshHostRecord {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  ssh_key_id: string | null;
  group_id: string | null;
  auth: SshHostAuthMaterial | null;
}

export type SshHostAuthMaterial =
  | { kind: "password"; password: string }
  | { kind: "public_key"; private_key: string; passphrase: string | null }
  | {
      kind: "public_key_and_password";
      private_key: string;
      passphrase: string | null;
      password: string;
    };

export interface SshKeyRecord {
  id: string;
  name: string;
  kind: string;
  fingerprint: string | null;
}

export interface SshKeySecret {
  private_key: string;
  passphrase: string | null;
}

export interface BootstrapMetadata {
  settings: Setting[];
  host_groups: HostGroupRecord[];
  hosts: SshHostRecord[];
  keys: SshKeyRecord[];
}

export interface KeyCreateRequest {
  name: string;
  kind: string;
  encrypted_private_key: string;
  encrypted_passphrase: string | null;
}

export interface KeyUpdateRequest {
  name: string;
  kind: string;
  encrypted_private_key?: string;
  encrypted_passphrase?: string | null;
}
