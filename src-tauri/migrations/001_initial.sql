-- Initial schema for Noverterm

-- Groups for organizing sessions
CREATE TABLE groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT
);

-- SSH keys
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

-- SSH sessions
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

-- Port forwards
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

-- Indexes
CREATE INDEX idx_sessions_group_id ON sessions(group_id);
CREATE INDEX idx_sessions_key_id ON sessions(key_id);
CREATE INDEX idx_port_forwards_session_id ON port_forwards(session_id);
