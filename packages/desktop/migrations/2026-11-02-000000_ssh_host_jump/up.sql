-- Saved host to tunnel through (ProxyJump); NULL connects directly.
ALTER TABLE ssh_hosts ADD COLUMN jump_host_id TEXT REFERENCES ssh_hosts (id) ON DELETE SET NULL;
