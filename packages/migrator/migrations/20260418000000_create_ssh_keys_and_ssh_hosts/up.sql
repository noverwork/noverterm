CREATE TABLE ssh_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    fingerprint TEXT,
    encrypted_private_key TEXT NOT NULL,
    encrypted_passphrase TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('ssh_keys');

CREATE TABLE ssh_hosts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    username TEXT NOT NULL,
    auth_mode TEXT NOT NULL,
    ssh_key_id TEXT REFERENCES ssh_keys(id),
    encrypted_password TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_connected_at TIMESTAMP
);

SELECT diesel_manage_updated_at('ssh_hosts');
