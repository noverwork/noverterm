DROP TABLE user_settings;

DROP INDEX ssh_hosts_owner_id_idx;

ALTER TABLE ssh_hosts
    DROP CONSTRAINT ssh_hosts_owner_scoped_key_fkey;

ALTER TABLE ssh_hosts
    DROP CONSTRAINT ssh_hosts_owner_id_id_unique;

ALTER TABLE ssh_hosts
    ADD CONSTRAINT ssh_hosts_ssh_key_id_fkey FOREIGN KEY (ssh_key_id) REFERENCES ssh_keys (id);

ALTER TABLE ssh_hosts
    DROP COLUMN owner_id;

DROP INDEX ssh_keys_owner_id_idx;

ALTER TABLE ssh_keys
    DROP CONSTRAINT ssh_keys_owner_id_id_unique;

ALTER TABLE ssh_keys
    DROP COLUMN owner_id;
