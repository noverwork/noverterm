CREATE TABLE port_forwards (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    host_id TEXT NOT NULL REFERENCES ssh_hosts (id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX port_forwards_host_id_idx ON port_forwards (host_id);

CREATE TABLE port_forward_mappings (
    id TEXT PRIMARY KEY,
    forward_id TEXT NOT NULL REFERENCES port_forwards (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    bind_host TEXT NOT NULL,
    bind_port INTEGER NOT NULL,
    target_host TEXT NOT NULL,
    target_port INTEGER NOT NULL
);

CREATE INDEX port_forward_mappings_forward_id_idx ON port_forward_mappings (forward_id);
