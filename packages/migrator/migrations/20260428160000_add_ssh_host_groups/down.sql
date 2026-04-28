DROP INDEX IF EXISTS ssh_hosts_owner_id_group_id_idx;

ALTER TABLE ssh_hosts DROP CONSTRAINT IF EXISTS ssh_hosts_owner_scoped_group_fkey;

ALTER TABLE ssh_hosts DROP COLUMN IF EXISTS group_id;

DROP TABLE IF EXISTS host_groups;
