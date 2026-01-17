import type {
  DbSession,
  DbGroup,
  DbSshKey,
  DbPortForward,
  SSHSession,
  SessionGroup,
  SSHKey,
  PortForward,
  CreateSessionInput,
  CreateKeyInput,
  ImportKeyInput,
  CreatePortForwardInput,
  CreateSessionInputDb,
  CreateKeyInputDb,
  CreatePortForwardInputDb,
} from './types';

// ============================================================================
// Session Mappers
// ============================================================================

export function mapDbSessionToUi(db: DbSession, portForwards: PortForward[] = []): SSHSession {
  return {
    id: db.id,
    name: db.name,
    group: db.group_id, // Store group_id as group name (will be resolved separately)
    host: db.host,
    port: db.port,
    username: db.username,
    authMethod: db.auth_method,
    keyId: db.key_id,
    status: 'Disconnected',
    rows: 24,
    cols: 80,
    portForwards,
  };
}

export function mapUiSessionToDbInput(ui: CreateSessionInput, groupId?: string): CreateSessionInputDb {
  return {
    name: ui.name,
    group_id: groupId,
    host: ui.host,
    port: ui.port,
    username: ui.username,
    auth_method: ui.authMethod,
    key_id: ui.keyId,
  };
}

// ============================================================================
// Group Mappers
// ============================================================================

export function mapDbGroupToUi(db: DbGroup): SessionGroup {
  return {
    id: db.id,
    name: db.name,
    color: db.color,
  };
}

// ============================================================================
// SSH Key Mappers
// ============================================================================

export function mapDbKeyToUi(db: DbSshKey): SSHKey {
  return {
    id: db.id,
    name: db.name,
    type: db.type,
    publicKey: db.public_key,
    privateKeyPath: db.private_key_path,
    fingerprint: db.fingerprint,
    createdAt: new Date(db.created_at * 1000).toISOString(),
    hasPassphrase: db.has_passphrase,
  };
}

export function mapUiKeyToDbInput(ui: CreateKeyInput, publicKey: string, fingerprint: string): CreateKeyInputDb {
  return {
    name: ui.name,
    type: ui.type,
    public_key: publicKey,
    fingerprint,
    has_passphrase: !!ui.passphrase,
  };
}

export function mapUiImportKeyToDbInput(ui: ImportKeyInput, fingerprint: string, hasPassphrase: boolean): CreateKeyInputDb {
  return {
    name: ui.name,
    type: 'rsa', // Will be detected from actual key
    public_key: ui.publicKey,
    private_key_path: ui.privateKeyPath,
    fingerprint,
    has_passphrase: hasPassphrase,
  };
}

// ============================================================================
// Port Forward Mappers
// ============================================================================

export function mapDbPortForwardToUi(db: DbPortForward): PortForward {
  return {
    id: db.id,
    sessionId: db.session_id,
    name: db.name,
    type: db.type,
    localHost: db.local_host,
    localPort: db.local_port,
    remoteHost: db.remote_host,
    remotePort: db.remote_port,
    active: false,
  };
}

export function mapUiPortForwardToDbInput(ui: CreatePortForwardInput): CreatePortForwardInputDb {
  return {
    session_id: ui.sessionId,
    name: ui.name,
    type: ui.type,
    local_host: ui.localHost,
    local_port: ui.localPort,
    remote_host: ui.remoteHost,
    remote_port: ui.remotePort,
  };
}
