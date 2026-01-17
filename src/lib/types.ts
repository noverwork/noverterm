// ============================================================================
// Database Types
// ============================================================================

export interface DbSession {
  id: string;
  name: string;
  group_id?: string;
  host: string;
  port: number;
  username: string;
  auth_method: 'password' | 'key' | 'agent';
  key_id?: string;
  created_at: number;
  updated_at: number;
}

export interface DbGroup {
  id: string;
  name: string;
  color?: string;
}

export interface DbSshKey {
  id: string;
  name: string;
  type: 'rsa' | 'ed25519' | 'ecdsa';
  public_key: string;
  private_key_path?: string;
  fingerprint: string;
  has_passphrase: boolean;
  created_at: number;
}

export interface DbPortForward {
  id: string;
  session_id: string;
  name: string;
  type: 'local' | 'remote' | 'dynamic';
  local_host: string;
  local_port: number;
  remote_host?: string;
  remote_port?: number;
}

// Database Input Types
export interface CreateGroupInput {
  name: string;
  color?: string;
}

export interface UpdateGroupInput {
  name?: string;
  color?: string;
}

export interface CreateSessionInputDb {
  name: string;
  group_id?: string;
  host: string;
  port: number;
  username: string;
  auth_method: 'password' | 'key' | 'agent';
  key_id?: string;
}

export interface UpdateSessionInputDb {
  name?: string;
  group_id?: string;
  host?: string;
  port?: number;
  username?: string;
  auth_method?: 'password' | 'key' | 'agent';
  key_id?: string;
}

export interface CreateKeyInputDb {
  name: string;
  type: 'rsa' | 'ed25519' | 'ecdsa';
  public_key: string;
  private_key_path?: string;
  fingerprint: string;
  has_passphrase: boolean;
}

export interface UpdateKeyInputDb {
  name?: string;
}

export interface CreatePortForwardInputDb {
  session_id: string;
  name: string;
  type: 'local' | 'remote' | 'dynamic';
  local_host: string;
  local_port: number;
  remote_host?: string;
  remote_port?: number;
}

export interface UpdatePortForwardInputDb {
  name?: string;
  type?: 'local' | 'remote' | 'dynamic';
  local_host?: string;
  local_port?: number;
  remote_host?: string;
  remote_port?: number;
}

// ============================================================================
// SSH Session Types (UI)
// ============================================================================

export type SessionStatus = 'Disconnected' | 'Connecting' | 'Connected' | 'Error';

export type AuthMethod = 'password' | 'key' | 'agent';

export interface SSHSession {
  id: string;
  name: string;
  group?: string;
  host: string;
  port: number;
  username: string;
  authMethod: AuthMethod;
  keyId?: string;
  status: SessionStatus;
  error?: string;
  // Terminal state
  rows: number;
  cols: number;
  // Port forwards
  portForwards: PortForward[];
}

export interface CreateSessionInput {
  name: string;
  group?: string;
  host: string;
  port: number;
  username: string;
  authMethod: AuthMethod;
  keyId?: string;
}

// ============================================================================
// SSH Key Types
// ============================================================================

export type KeyType = 'rsa' | 'ed25519' | 'ecdsa';

export interface SSHKey {
  id: string;
  name: string;
  type: KeyType;
  publicKey: string;
  privateKeyPath?: string;
  fingerprint: string;
  createdAt: string;
  hasPassphrase: boolean;
}

export interface CreateKeyInput {
  name: string;
  type: KeyType;
  passphrase?: string;
}

export interface ImportKeyInput {
  name: string;
  publicKey: string;
  privateKeyPath: string;
}

// ============================================================================
// Port Forward Types
// ============================================================================

export type ForwardType = 'local' | 'remote' | 'dynamic';

export interface PortForward {
  id: string;
  sessionId: string;
  name: string;
  type: ForwardType;
  localHost: string;
  localPort: number;
  remoteHost?: string;
  remotePort?: number;
  active: boolean;
}

export interface CreatePortForwardInput {
  sessionId: string;
  name: string;
  type: ForwardType;
  localHost: string;
  localPort: number;
  remoteHost?: string;
  remotePort?: number;
}

// ============================================================================
// View Types
// ============================================================================

export type ViewType = 'dashboard' | 'terminal' | 'keys' | 'portForwards';

// ============================================================================
// Group Types
// ============================================================================

export interface SessionGroup {
  id: string;
  name: string;
  color?: string;
}

// ============================================================================
// Terminal Types
// ============================================================================

export interface TerminalCell {
  ch: string;
  fg: number;
  bg: number;
  bold: boolean;
  italic: boolean;
  underline: boolean;
}

export interface TerminalGrid {
  rows: number;
  cols: number;
  cells: TerminalCell[][];
  cursorRow: number;
  cursorCol: number;
}
