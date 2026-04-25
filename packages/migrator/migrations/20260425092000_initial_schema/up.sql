CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('users');

CREATE TABLE ssh_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    fingerprint TEXT,
    encrypted_private_key TEXT NOT NULL,
    encrypted_passphrase TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT ssh_keys_owner_id_id_unique UNIQUE (owner_id, id)
);

CREATE INDEX ssh_keys_owner_id_idx ON ssh_keys (owner_id);

SELECT diesel_manage_updated_at('ssh_keys');

CREATE TABLE ssh_hosts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    username TEXT NOT NULL,
    ssh_key_id TEXT,
    encrypted_password TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT ssh_hosts_owner_id_id_unique UNIQUE (owner_id, id),
    CONSTRAINT ssh_hosts_owner_scoped_key_fkey
        FOREIGN KEY (ssh_key_id, owner_id)
        REFERENCES ssh_keys (id, owner_id)
        ON DELETE SET NULL (ssh_key_id)
);

CREATE INDEX ssh_hosts_owner_id_idx ON ssh_hosts (owner_id);

SELECT diesel_manage_updated_at('ssh_hosts');

CREATE TABLE user_settings (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT user_settings_owner_id_key_unique UNIQUE (owner_id, key)
);

CREATE INDEX user_settings_owner_id_idx ON user_settings (owner_id);

SELECT diesel_manage_updated_at('user_settings');
