ALTER TABLE ssh_hosts DROP CONSTRAINT IF EXISTS fk_ssh_hosts_owner;
ALTER TABLE ssh_keys DROP CONSTRAINT IF EXISTS fk_ssh_keys_owner;
ALTER TABLE user_settings DROP CONSTRAINT IF EXISTS fk_user_settings_owner;
