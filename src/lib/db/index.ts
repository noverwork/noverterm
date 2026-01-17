import { invoke } from '@tauri-apps/api/core';
import type {
  DbGroup,
  DbSshKey,
  DbSession,
  DbPortForward,
  CreateGroupInput,
  CreateKeyInputDb,
  CreateSessionInputDb,
  CreatePortForwardInputDb,
  UpdateGroupInput,
  UpdateKeyInputDb,
  UpdateSessionInputDb,
  UpdatePortForwardInputDb,
} from '../types';

// ============================================================================
// Database Service Layer
// ============================================================================

export const db = {
  // ========================================================================
  // Groups
  // ========================================================================

  getGroups: (): Promise<DbGroup[]> =>
    invoke<DbGroup[]>('db_get_groups'),

  createGroup: (input: CreateGroupInput): Promise<DbGroup> =>
    invoke<DbGroup>('db_create_group', { input }),

  updateGroup: (id: string, input: UpdateGroupInput): Promise<DbGroup> =>
    invoke<DbGroup>('db_update_group', { id, input }),

  deleteGroup: (id: string): Promise<void> =>
    invoke('db_delete_group', { id }),

  // ========================================================================
  // Sessions
  // ========================================================================

  getSessions: (): Promise<DbSession[]> =>
    invoke<DbSession[]>('db_get_all_sessions'),

  getSession: (id: string): Promise<DbSession> =>
    invoke<DbSession>('db_get_session', { id }),

  createSession: (input: CreateSessionInputDb): Promise<DbSession> =>
    invoke<DbSession>('db_create_session', { input }),

  updateSession: (id: string, input: UpdateSessionInputDb): Promise<DbSession> =>
    invoke<DbSession>('db_update_session', { id, input }),

  deleteSession: (id: string): Promise<void> =>
    invoke('db_delete_session', { id }),

  // ========================================================================
  // SSH Keys
  // ========================================================================

  getKeys: (): Promise<DbSshKey[]> =>
    invoke<DbSshKey[]>('db_get_ssh_keys'),

  getKey: (id: string): Promise<DbSshKey> =>
    invoke<DbSshKey>('db_get_ssh_key', { id }),

  createKey: (input: CreateKeyInputDb): Promise<DbSshKey> =>
    invoke<DbSshKey>('db_create_ssh_key', { input }),

  updateKey: (id: string, input: UpdateKeyInputDb): Promise<DbSshKey> =>
    invoke<DbSshKey>('db_update_ssh_key', { id, input }),

  deleteKey: (id: string): Promise<void> =>
    invoke('db_delete_ssh_key', { id }),

  // ========================================================================
  // Port Forwards
  // ========================================================================

  getPortForwards: (sessionId?: string): Promise<DbPortForward[]> =>
    invoke<DbPortForward[]>('db_get_port_forwards', { sessionId }),

  getPortForward: (id: string): Promise<DbPortForward> =>
    invoke<DbPortForward>('db_get_port_forward', { id }),

  createPortForward: (input: CreatePortForwardInputDb): Promise<DbPortForward> =>
    invoke<DbPortForward>('db_create_port_forward', { input }),

  updatePortForward: (id: string, input: UpdatePortForwardInputDb): Promise<DbPortForward> =>
    invoke<DbPortForward>('db_update_port_forward', { id, input }),

  deletePortForward: (id: string): Promise<void> =>
    invoke('db_delete_port_forward', { id }),
};
