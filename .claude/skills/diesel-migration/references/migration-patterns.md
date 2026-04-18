# Common Migration Patterns

## Create new table

```sql
-- up.sql
CREATE TABLE user_profiles (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('user_profiles');

-- down.sql
DROP TABLE user_profiles;
```

## Add column with default

```sql
-- up.sql
ALTER TABLE memory_facts ADD COLUMN is_verified BOOLEAN NOT NULL DEFAULT FALSE;

-- down.sql
ALTER TABLE memory_facts DROP COLUMN is_verified;
```

## Add foreign key

```sql
-- up.sql
ALTER TABLE ssh_hosts
    ADD COLUMN ssh_key_id TEXT REFERENCES ssh_keys(id);

-- down.sql
ALTER TABLE ssh_hosts DROP COLUMN ssh_key_id;
```

## Add index

```sql
-- up.sql
CREATE INDEX idx_chats_principal_id ON chats (principal_id);

-- down.sql
DROP INDEX idx_chats_principal_id;
```

## Alter column type

```sql
-- up.sql
ALTER TABLE chats ALTER COLUMN metadata TYPE JSONB USING metadata::JSONB;

-- down.sql
ALTER TABLE chats ALTER COLUMN metadata TYPE TEXT USING metadata::TEXT;
```

## `diesel_manage_updated_at` Trigger

The initial Diesel migration (`00000000000000_diesel_initial_setup`) provides a helper:

```sql
SELECT diesel_manage_updated_at('table_name');
```

This creates a trigger that automatically sets `updated_at = NOW()` on every `UPDATE`. Use it for any table that has an `updated_at` column.

## Diff-Schema Mode

`cargo make db:generate --diff-schema <name>` compares the live DB against `schema.rs` and auto-generates the SQL diff.

**Use with caution**: always review the generated `up.sql` / `down.sql` before applying. Auto-generated SQL can be incomplete or incorrect for complex changes.
