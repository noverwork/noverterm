# Noverterm Database API Specification

## Overview

Local SQLite database for storing SSH sessions, keys, groups, and port forwards.

**Database Location**: `~/Library/Application Support/noverterm/noverterm.db` (macOS)

---

## Data Models

### Session
```typescript
interface DbSession {
  id: string;                // UUID
  name: string;
  group_id?: string;
  host: string;
  port: number;
  username: string;
  auth_method: 'password' | 'key' | 'agent';
  key_id?: string;           // Foreign key to ssh_keys
  created_at: number;        // Unix timestamp
  updated_at: number;        // Unix timestamp
}
```

### Group
```typescript
interface DbGroup {
  id: string;                // UUID
  name: string;              // Unique
  color?: string;            // Hex color
}
```

### SSH Key
```typescript
interface DbSshKey {
  id: string;                // UUID
  name: string;
  type: 'rsa' | 'ed25519' | 'ecdsa';
  public_key: string;
  private_key_path?: string;
  fingerprint: string;
  has_passphrase: boolean;
  created_at: number;        // Unix timestamp
}
```

### Port Forward
```typescript
interface DbPortForward {
  id: string;                // UUID
  session_id: string;        // Foreign key to sessions
  name: string;
  type: 'local' | 'remote' | 'dynamic';
  local_host: string;
  local_port: number;
  remote_host?: string;
  remote_port?: number;
}
```

---

## Tauri Commands

### Groups

| Command | Parameters | Returns |
|---------|------------|---------|
| `db_get_groups` | - | `DbGroup[]` |
| `db_create_group` | `{ name, color? }` | `DbGroup` |
| `db_update_group` | `id, { name?, color? }` | `DbGroup` |
| `db_delete_group` | `id` | `void` |

### SSH Keys

| Command | Parameters | Returns |
|---------|------------|---------|
| `db_get_ssh_keys` | - | `DbSshKey[]` |
| `db_get_ssh_key` | `id` | `DbSshKey` |
| `db_create_ssh_key` | `{ name, type, public_key, private_key_path?, fingerprint, has_passphrase }` | `DbSshKey` |
| `db_update_ssh_key` | `id, { name? }` | `DbSshKey` |
| `db_delete_ssh_key` | `id` | `void` |

### Sessions

| Command | Parameters | Returns |
|---------|------------|---------|
| `db_get_all_sessions` | - | `DbSession[]` |
| `db_get_session` | `id` | `DbSession` |
| `db_create_session` | `{ name, group_id?, host, port, username, auth_method, key_id? }` | `DbSession` |
| `db_update_session` | `id, { name?, group_id?, host?, port?, username?, auth_method?, key_id? }` | `DbSession` |
| `db_delete_session` | `id` | `void` |

### Port Forwards

| Command | Parameters | Returns |
|---------|------------|---------|
| `db_get_port_forwards` | `session_id?` | `DbPortForward[]` |
| `db_get_port_forward` | `id` | `DbPortForward` |
| `db_create_port_forward` | `{ session_id, name, type, local_host, local_port, remote_host?, remote_port? }` | `DbPortForward` |
| `db_update_port_forward` | `id, { name?, type?, local_host?, local_port?, remote_host?, remote_port? }` | `DbPortForward` |
| `db_delete_port_forward` | `id` | `void` |

---

## Frontend Integration Plan

### 1. Create Database Service Layer

```typescript
// src/lib/db/index.ts

import { invoke } from '@tauri-apps/api/core';

// Groups
export const db = {
  // Groups
  getGroups: () => invoke<DbGroup[]>('db_get_groups'),
  createGroup: (input: CreateGroupInput) => invoke<DbGroup>('db_create_group', { input }),
  updateGroup: (id: string, input: UpdateGroupInput) => invoke<DbGroup>('db_update_group', { id, input }),
  deleteGroup: (id: string) => invoke('db_delete_group', { id }),

  // Sessions
  getSessions: () => invoke<DbSession[]>('db_get_all_sessions'),
  getSession: (id: string) => invoke<DbSession>('db_get_session', { id }),
  createSession: (input: CreateSessionInput) => invoke<DbSession>('db_create_session', { input }),
  updateSession: (id: string, input: UpdateSessionInput) => invoke<DbSession>('db_update_session', { id, input }),
  deleteSession: (id: string) => invoke('db_delete_session', { id }),

  // SSH Keys
  getKeys: () => invoke<DbSshKey[]>('db_get_ssh_keys'),
  getKey: (id: string) => invoke<DbSshKey>('db_get_ssh_key', { id }),
  createKey: (input: CreateKeyInput) => invoke<DbSshKey>('db_create_ssh_key', { input }),
  updateKey: (id: string, input: UpdateKeyInput) => invoke<DbSshKey>('db_update_ssh_key', { id, input }),
  deleteKey: (id: string) => invoke('db_delete_ssh_key', { id }),

  // Port Forwards
  getPortForwards: (sessionId?: string) => invoke<DbPortForward[]>('db_get_port_forwards', { sessionId }),
  getPortForward: (id: string) => invoke<DbPortForward>('db_get_port_forward', { id }),
  createPortForward: (input: CreatePortForwardInput) => invoke<DbPortForward>('db_create_port_forward', { input }),
  updatePortForward: (id: string, input: UpdatePortForwardInput) => invoke<DbPortForward>('db_update_port_forward', { id, input }),
  deletePortForward: (id: string) => invoke('db_delete_port_forward', { id }),
};
```

### 2. Update Zustand Store

Replace in-memory state with database persistence:

```typescript
// src/lib/stores/useSessionStore.ts

import { db } from '@/lib/db';

export const useSessionStore = create<SessionStore>((set, get) => ({
  sessions: [],
  groups: [],

  // Load from database on init
  init: async () => {
    const [sessions, groups] = await Promise.all([
      db.getSessions(),
      db.getGroups(),
    ]);
    set({ sessions, groups });
  },

  // CRUD operations sync with database
  createSession: async (input) => {
    const session = await db.createSession(input);
    set((state) => ({ sessions: [...state.sessions, mapToUiSession(session)] }));
    return session;
  },

  updateSession: async (id, input) => {
    const session = await db.updateSession(id, input);
    set((state) => ({
      sessions: state.sessions.map((s) => s.id === id ? mapToUiSession(session) : s),
    }));
  },

  deleteSession: async (id) => {
    await db.deleteSession(id);
    set((state) => ({ sessions: state.sessions.filter((s) => s.id !== id) }));
  },
}));
```

### 3. Data Type Mapping

Database models differ from current UI types. Need mapper:

```typescript
// DbSession -> SSHSession
function mapToUiSession(db: DbSession): SSHSession {
  return {
    id: db.id,
    name: db.name,
    group: db.group_id,
    host: db.host,
    port: db.port,
    username: db.username,
    authMethod: db.auth_method,
    keyId: db.key_id,
    status: 'Disconnected',
    rows: 24,
    cols: 80,
    portForwards: [], // Load separately
  };
}
```

---

## Migration Steps

1. **Add database service** - Create `src/lib/db/index.ts`
2. **Add type definitions** - Add Db* types to `src/lib/types.ts`
3. **Update stores** - Replace in-memory operations with db calls
4. **Initialize on app start** - Load data from DB in `App.tsx`
5. **Remove mock data** - Clean up hardcoded sessions/keys

---

## Open Questions

1. Should we load all data on startup, or lazy load?
2. Should portForwards be embedded in Session or loaded separately?
3. Error handling - how to display DB errors to users?
4. Offline mode - what happens if DB is corrupted?

---

## Database Schema

```sql
CREATE TABLE groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT
);

CREATE TABLE ssh_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL,
    public_key TEXT NOT NULL,
    private_key_path TEXT,
    fingerprint TEXT NOT NULL,
    has_passphrase INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    group_id TEXT,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL,
    auth_method TEXT NOT NULL,
    key_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE SET NULL,
    FOREIGN KEY (key_id) REFERENCES ssh_keys(id) ON DELETE SET NULL
);

CREATE TABLE port_forwards (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    name TEXT NOT NULL,
    type TEXT NOT NULL,
    local_host TEXT NOT NULL DEFAULT 'localhost',
    local_port INTEGER NOT NULL,
    remote_host TEXT,
    remote_port INTEGER,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);
```
