ALTER TABLE ssh_keys
    ADD COLUMN owner_id TEXT NOT NULL DEFAULT 'admin';

ALTER TABLE ssh_keys
    ADD CONSTRAINT ssh_keys_owner_id_id_unique UNIQUE (owner_id, id);

CREATE INDEX ssh_keys_owner_id_idx ON ssh_keys (owner_id);

ALTER TABLE ssh_hosts
    ADD COLUMN owner_id TEXT NOT NULL DEFAULT 'admin';

ALTER TABLE ssh_hosts
    DROP CONSTRAINT ssh_hosts_ssh_key_id_fkey;

ALTER TABLE ssh_hosts
    ADD CONSTRAINT ssh_hosts_owner_id_id_unique UNIQUE (owner_id, id);

ALTER TABLE ssh_hosts
    ADD CONSTRAINT ssh_hosts_owner_scoped_key_fkey
        FOREIGN KEY (ssh_key_id, owner_id)
        REFERENCES ssh_keys (id, owner_id)
        ON DELETE SET NULL;

CREATE INDEX ssh_hosts_owner_id_idx ON ssh_hosts (owner_id);

CREATE TABLE user_settings (
    owner_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (owner_id, key)
);

SELECT diesel_manage_updated_at('user_settings');
