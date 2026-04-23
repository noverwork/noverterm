export interface Setting {
  key: string;
  value: string;
}

export interface SshHostRecord {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  auth_mode: string;
  ssh_key_id: string | null;
}

export interface SshKeyRecord {
  id: string;
  name: string;
  kind: string;
  fingerprint: string | null;
}

export interface BootstrapMetadata {
  settings: Setting[];
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
  encrypted_private_key: string;
  encrypted_passphrase: string | null;
}
