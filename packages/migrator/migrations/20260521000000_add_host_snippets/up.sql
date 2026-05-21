CREATE TABLE host_snippets (
    id TEXT PRIMARY KEY,
    host_id TEXT NOT NULL,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT host_snippets_owner_host_fkey
        FOREIGN KEY (host_id, owner_id)
        REFERENCES ssh_hosts (id, owner_id)
        ON DELETE CASCADE
);

CREATE INDEX host_snippets_owner_id_idx ON host_snippets (owner_id);
CREATE INDEX host_snippets_host_id_idx ON host_snippets (host_id);

SELECT diesel_manage_updated_at('host_snippets');
