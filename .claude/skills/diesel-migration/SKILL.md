---
name: diesel-migration
description: Diesel + SQLite schema workflow for this monorepo. Use when adding or changing a table or column - writing a migration in packages/desktop/migrations, updating packages/orm/src/schema.rs, or writing the model structs in packages/orm/src/models.
---

# Diesel Migration & Model Workflow

The desktop app keeps its data in one SQLite file. Migrations are raw SQL, compiled into the binary
and applied at startup. There is no migrator package, no `diesel.toml`, no `DATABASE_URL`, and no
`cargo make db:*` task.

## Where things live

| Path | Role |
|---|---|
| `packages/desktop/migrations/<date>_<name>/{up,down}.sql` | SQL migrations, hand-written |
| `packages/desktop/src/store/mod.rs` | `embed_migrations!("migrations")`, `init_pool`, `run_db`, the per-connection PRAGMAs |
| `packages/orm/src/schema.rs` | Diesel `table!` macros, kept by hand (see step 3) |
| `packages/orm/src/models/<table>.rs` | Model structs, exported from `models/mod.rs` |
| `packages/desktop/src/store/<area>.rs` | Queries; map rows to the `shared` record types |
| `packages/shared/src/lib.rs` | Record and request types the UI sees |

`init_pool` runs every pending migration when the app opens the database, so a migration ships by
being in the directory. Never edit a migration that has been released: users already ran it. Add a
new one.

## Workflow

1. **Create the directory** `packages/desktop/migrations/YYYY-MM-DD-HHMMSS_<name>/` with `up.sql`
   and `down.sql`. Names sort by date; use `000000`, `010000`, ... for several on one day.
2. **Write the SQL** (patterns below). Start `up.sql` with a one-line comment saying what the
   column or table means when that is not obvious from its name.
3. **Update `schema.rs` by hand** to mirror the migration. A column added with `ALTER TABLE ... ADD
   COLUMN` goes at the **end** of its `table!` block, because that is where SQLite puts it. A new
   table also goes in `allow_tables_to_appear_in_same_query!`, and each foreign key gets a
   `joinable!` line.
4. **Update the models** in `packages/orm/src/models/` and the exports in `models/mod.rs`.
5. **Update the store** function and the `shared` record if the UI needs the new data.
6. **Verify**: `cargo test -p desktop store`. The store tests build a database from the embedded
   migrations and insert through the models, so a schema/model mismatch fails there.

### Checking `schema.rs` against the SQL

`diesel print-schema` can show what the migrations really produce:

```sh
diesel migration run --database-url /tmp/schema.db --migration-dir packages/desktop/migrations
diesel print-schema --database-url /tmp/schema.db
```

Use it as a reference, do not redirect it over `schema.rs`. Two differences are expected: it prints
`TEXT PRIMARY KEY` columns as `Nullable<Text>` (SQLite allows NULL there; the checked-in file says
`Text`), and it orders tables alphabetically.

## SQL conventions

```sql
-- A Link publishes target_host:target_port at https://<subdomain>.<relay domain>.
CREATE TABLE links (
    id TEXT PRIMARY KEY,
    subdomain TEXT NOT NULL UNIQUE,
    target_host TEXT NOT NULL,
    target_port INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

- `id TEXT PRIMARY KEY`, filled by the store with `Uuid::new_v4().to_string()`.
- `created_at` and `updated_at` on every table. There is no trigger: the store sets `updated_at`
  to `Utc::now().naive_utc()` on every write.
- Foreign keys say what happens on delete: `REFERENCES ssh_hosts (id) ON DELETE CASCADE` or
  `ON DELETE SET NULL`. They are enforced because `store/mod.rs` sets `PRAGMA foreign_keys = ON`
  on each connection.
- Index a foreign key column: `CREATE INDEX <table>_<column>_idx ON <table> (<column>);`
- Settings, tokens, and secrets are rows in the `settings` table (`store/settings.rs`), not new
  tables or side files.

### Patterns

```sql
-- add a column                              -- down.sql
ALTER TABLE host_snippets ADD COLUMN pin_color TEXT;
                                             ALTER TABLE host_snippets DROP COLUMN pin_color;

-- add a foreign key column
ALTER TABLE ssh_hosts ADD COLUMN jump_host_id TEXT REFERENCES ssh_hosts (id) ON DELETE SET NULL;
```

SQLite cannot change a column's type, nullability, or constraints in place. Rebuild the table, as
`2026-09-28-000000_local_snippets` does:

```sql
CREATE TABLE host_snippets_new ( ... );
INSERT INTO host_snippets_new SELECT id, host_id, title, body, created_at, updated_at FROM host_snippets;
DROP TABLE host_snippets;
ALTER TABLE host_snippets_new RENAME TO host_snippets;
CREATE INDEX host_snippets_host_id_idx ON host_snippets (host_id);   -- indexes go with the old table
```

A `NOT NULL` column added to an existing table needs a `DEFAULT`.

## Type mapping (SQLite)

| SQL | `schema.rs` | Rust |
|---|---|---|
| `TEXT` | `Text` | `String` |
| `INTEGER` | `Integer` | `i32` |
| `TIMESTAMP` | `Timestamp` | `chrono::NaiveDateTime` |
| nullable column | `Nullable<T>` | `Option<T>` |

Enums are stored as `TEXT` and converted in the store (`ai_access` on `ssh_hosts`). The `orm` crate
enables only diesel's `sqlite` and `chrono` features: no JSON, UUID, or array column types.

## Model structs

Three structs per table, in `packages/orm/src/models/<table>.rs`:

```rust
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = links)]
pub struct Link {            // read: every column, in schema.rs order
    pub id: String,
    pub subdomain: String,
    pub target_host: String,
    pub target_port: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = links)]
pub struct NewLink { /* every column, id and both timestamps included */ }

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = links)]
pub struct UpdateLink { /* no id, no created_at; updated_at included */ }
```

- `Queryable` reads by position: field order must match the `table!` column order, so a column
  appended to the table is appended to the struct.
- `AsChangeset` skips a `None` field instead of writing NULL. A nullable column that must be
  clearable is `Option<Option<T>>` in the update struct.
- Export from `models/mod.rs`: `pub use link::{Link, NewLink, UpdateLink};`

## Using them from the store

```rust
pub async fn link_create(link: LinkWriteRequest, pool: State<'_, DbPool>) -> Result<LinkRecord, String> {
    let pool = pool.inner().clone();
    run_db(pool, move |connection| {
        let now = Utc::now().naive_utc();
        diesel::insert_into(links::table)
            .values(&NewLink { id: Uuid::new_v4().to_string(), /* ... */ created_at: now, updated_at: now })
            .returning(Link::as_returning())
            .get_result(connection)
            .map(to_record)
            .map_err(internal_error)
    })
    .await
}
```

`run_db` moves the query onto a blocking thread; never touch a connection on the UI thread or
inside an async task directly. Errors leave the store as `String` (`internal_error`, or a specific
message for a constraint the user can fix, as `write_error` in `store/links.rs` does).
