CREATE TABLE host_groups (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT host_groups_id_owner_id_unique UNIQUE (id, owner_id),
    CONSTRAINT host_groups_owner_id_name_unique UNIQUE (owner_id, name)
);

CREATE INDEX host_groups_owner_id_idx ON host_groups (owner_id);

SELECT diesel_manage_updated_at('host_groups');

ALTER TABLE ssh_hosts ADD COLUMN group_id TEXT;

ALTER TABLE ssh_hosts
    ADD CONSTRAINT ssh_hosts_owner_scoped_group_fkey
    FOREIGN KEY (group_id, owner_id)
    REFERENCES host_groups (id, owner_id)
    ON DELETE SET NULL (group_id);

CREATE INDEX ssh_hosts_owner_id_group_id_idx ON ssh_hosts (owner_id, group_id);
