// ============================================================================
// SSH Session Types
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
