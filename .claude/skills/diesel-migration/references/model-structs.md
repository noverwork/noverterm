# Model Struct Conventions

Each table follows a **three-struct pattern**:

| Struct | Derive Macros | Purpose |
|--------|---------------|---------|
| `SshKey` | `Queryable`, `Selectable` | Read/query results from DB |
| `NewSshKey` | `Insertable` | Insert new rows |
| `UpdateSshKey` | `AsChangeset` | Update existing rows |

## Read struct (`Queryable` + `Selectable`)

```rust
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::ssh_keys;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ssh_keys)]
pub struct SshKey {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
```

**Key rules:**
- Field order **must match** the column order in `schema.rs`'s `table!` macro (for `Queryable`)
- Use `Option<T>` for nullable columns
- `#[diesel(table_name = ssh_keys)]` links the struct to the table
- `Selectable` enables `.select(SshKey::as_select())` for type-safe queries

## Insert struct (`Insertable`)

```rust
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = ssh_keys)]
pub struct NewSshKey {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
```

**Key rules:**
- Must include **all non-nullable columns** (nullable ones can be `Option<T>`)
- Auto-generated columns (e.g., `SERIAL` / auto-increment) can be omitted
- Used with `diesel::insert_into(ssh_keys::table).values(&new_key)`

## Update struct (`AsChangeset`)

```rust
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = ssh_keys)]
pub struct UpdateSshKey {
    pub name: String,
    pub kind: String,
    pub fingerprint: Option<String>,
    pub encrypted_private_key: String,
    pub encrypted_passphrase: Option<String>,
    pub updated_at: NaiveDateTime,
}
```

**Key rules:**
- **Exclude** primary key columns (they're used in the `WHERE` clause, not `SET`)
- **Exclude** `created_at` (never changes)
- Include `updated_at` to track modification time
- Used with `diesel::update(ssh_keys::table.find(id)).set(&changes)`

## Relationships (`Associations` + `belongs_to`)

```rust
#[derive(Debug, Clone, Queryable, Selectable, Associations, Serialize, Deserialize)]
#[diesel(table_name = ssh_hosts)]
#[diesel(belongs_to(SshKey, foreign_key = ssh_key_id))]
pub struct SshHost {
    // ...
    pub ssh_key_id: Option<String>,
    // ...
}
```

- `Associations` enables `SshHost::belonging_to(&key)` queries
- `#[diesel(belongs_to(ParentStruct, foreign_key = column_name))]` defines the relationship
- The foreign key column must exist in the `table!` macro

## Export from `mod.rs`

```rust
pub mod ssh_key;
pub use ssh_key::{NewSshKey, SshKey, UpdateSshKey};
```
