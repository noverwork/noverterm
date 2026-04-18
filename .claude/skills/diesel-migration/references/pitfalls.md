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
