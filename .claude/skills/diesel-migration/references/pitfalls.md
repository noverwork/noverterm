# Common Pitfalls

## 1. `TIMESTAMP` vs `TIMESTAMPTZ` mismatch

```
error: the trait bound `NaiveDateTime: AsExpression<Timestamptz>` is not satisfied
```

**Fix**: Either change SQL to `TIMESTAMP` or change Rust type to `DateTime<Utc>`.

## 2. Missing `chrono` feature in `Cargo.toml`

```
error: the trait bound `NaiveDateTime: AsExpression<Timestamp>` is not satisfied
```

**Fix**: Add `"chrono"` to diesel features in `Cargo.toml`.

## 3. Field order mismatch in `Queryable` struct

`Queryable` reads columns in positional order. The struct field order **must match** the `table!` macro column order.

## 4. Model exists but migration doesn't

This happens when someone hand-writes model structs but forgets to write the migration SQL. Always write migration first, then model.

## 5. `schema.rs` edited manually

Never edit `schema.rs` by hand. Always regenerate via `diesel print-schema > ../orm/src/schema.rs`.

## 6. `AsChangeset` includes primary key

Primary key columns should be excluded from `AsChangeset` structs — they're used in the `WHERE` clause, not `SET`.

## 7. `AsChangeset` includes `created_at`

`created_at` should never be updated after insert. Exclude it from `AsChangeset`.

## 8. Custom PostgreSQL enum types missing from `diesel.toml` `import_types`

When a migration adds a PostgreSQL enum column (e.g. `ALTER TABLE ... ADD COLUMN ... my_enum_type`),
and you regenerate `schema.rs` via `diesel print-schema`, the generated `table!` macro will reference
the enum type name directly:

```rust
// auto-generated schema.rs
my_column -> MyEnumType,  // ← needs to be imported
```

If `diesel.toml` `import_types` does not include the corresponding `#[derive(SqlType)]` struct,
compilation fails with:

```
error[E0425]: cannot find type `MyEnumType` in this scope
```

**Fix workflow:**

1. In `packages/orm/src/types.rs`, add the SQL type marker:
   ```rust
   #[derive(Debug, Clone, Copy, Default, SqlType)]
   #[diesel(postgres_type(name = "my_enum_type"))]
   pub struct MyEnumTypeType;
   ```

2. **Update `packages/migrator/diesel.toml` `import_types`:**
   ```toml
   import_types = [
       "diesel::sql_types::*",
       "crate::types::Vector",
       "crate::types::MyEnumTypeType",  # ← ADD THIS
   ]
   ```

3. Regenerate schema: `diesel print-schema > ../orm/src/schema.rs`
4. Write the Rust enum with `FromSqlRow` + `AsExpression` + `ToSql`/`FromSql` impls in `models/`
5. `cargo check -p orm`

**Rule: Every new PostgreSQL enum column requires BOTH a `SqlType` struct in `types.rs` AND an entry in `diesel.toml` `import_types`.**

## 9. `Array<Nullable<Text>>` does not map to `Vec<String>`

PostgreSQL `TEXT[]` with nullable elements maps to `Vec<Option<String>>`, not `Vec<String>`.
Nullable arrays (`Nullable<Array<...>>`) map to `Option<Vec<...>>`.

| PostgreSQL type | Rust model type |
|-----------------|-----------------|
| `Array<Text>` | `Vec<String>` |
| `Array<Nullable<Text>>` | `Vec<Option<String>>` |
| `Nullable<Array<Nullable<Text>>>` | `Option<Vec<Option<String>>>` |

**Conversion at repository boundary:** Keep domain types as `Vec<String>` (clean API),
and convert to/from DB types in the repository layer:

```rust
fn to_db_strings(strings: Vec<String>) -> Vec<Option<String>> {
    strings.into_iter().map(Some).collect()
}

fn from_db_strings(strings: Vec<Option<String>>) -> Vec<String> {
    strings.into_iter().flatten().collect()
}
```
