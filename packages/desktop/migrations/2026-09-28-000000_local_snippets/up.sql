-- A NULL host_id targets the local terminal.
CREATE TABLE host_snippets_new (
    id TEXT PRIMARY KEY,
    host_id TEXT REFERENCES ssh_hosts (id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    body TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO host_snippets_new SELECT id, host_id, title, body, created_at, updated_at FROM host_snippets;
DROP TABLE host_snippets;
ALTER TABLE host_snippets_new RENAME TO host_snippets;

CREATE INDEX host_snippets_host_id_idx ON host_snippets (host_id);
