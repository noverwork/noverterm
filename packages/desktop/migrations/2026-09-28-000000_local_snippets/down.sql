DELETE FROM host_snippets WHERE host_id IS NULL;

CREATE TABLE host_snippets_old (
    id TEXT PRIMARY KEY,
    host_id TEXT NOT NULL REFERENCES ssh_hosts (id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO host_snippets_old SELECT id, host_id, title, body, created_at, updated_at FROM host_snippets;
DROP TABLE host_snippets;
ALTER TABLE host_snippets_old RENAME TO host_snippets;

CREATE INDEX host_snippets_host_id_idx ON host_snippets (host_id);
