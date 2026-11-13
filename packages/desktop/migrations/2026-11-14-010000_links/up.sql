-- A Link publishes target_host:target_port at https://<subdomain>.<relay domain>.
CREATE TABLE links (
    id TEXT PRIMARY KEY,
    subdomain TEXT NOT NULL UNIQUE,
    target_host TEXT NOT NULL,
    target_port INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
