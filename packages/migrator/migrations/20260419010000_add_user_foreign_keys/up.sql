ALTER TABLE ssh_hosts
    ADD CONSTRAINT fk_ssh_hosts_owner
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE ssh_keys
    ADD CONSTRAINT fk_ssh_keys_owner
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE user_settings
    ADD CONSTRAINT fk_user_settings_owner
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;
