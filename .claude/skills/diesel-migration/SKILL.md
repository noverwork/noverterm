---
name: diesel-migration
description: Diesel SQL migration workflow for this monorepo. Use when adding/changing database schema, creating migrations, or syncing Rust schema definitions.
---

# Diesel Migration & Model Workflow

Schema changes go through raw SQL migrations first, then sync to `schema.rs`, then hand-write model structs.

> Diesel 2.x does NOT auto-generate model structs. Third-party tools like `dsync` are early-stage (v0.1.0) with known bugs — not recommended.

## Architecture

| Path | Role | Generated? |
|------|------|------------|
| `packages/migrator/migrations/` | SQL migration files | **Hand-written** |
| `packages/orm/src/schema.rs` | Diesel `table!` macro | **Auto-generated** |
| `packages/orm/src/models/` | Rust structs with derives | **Hand-written** |
| `packages/migrator/diesel.toml` | `print_schema` config | Hand-written (once) |
| `packages/migrator/.env` | `DATABASE_URL` | Hand-written (once) |

## Prerequisites

```bash
cargo install diesel_cli --no-default-features --features postgres
cargo install cargo-make
```

## Workflow: New Table or Column

1. **Generate scaffold**: `cargo make db:generate <name>`
2. **Write SQL**: `up.sql` (forward) + `down.sql` (rollback)
3. **Apply**: `cargo make db:up`
4. **Sync schema**: `cd packages/migrator && diesel print-schema > ../orm/src/schema.rs`
5. **Write models**: Create `packages/orm/src/models/<name>.rs` (see [Model Structs](references/model-structs.md))
6. **Export**: Add to `packages/orm/src/models/mod.rs`
7. **Verify**: `cargo check -p orm`

## Key Rules

- **Never edit `schema.rs` manually** — always regenerate via `diesel print-schema`
- **Always write migration first**, then model — never the reverse
- **`TIMESTAMP` → `NaiveDateTime`**, `TIMESTAMPTZ` → `DateTime<Utc>` — mismatching causes compile errors
- **`Queryable` field order must match `table!` column order** exactly
- **`AsChangeset` excludes primary key and `created_at`**
- **Use `diesel_manage_updated_at('table_name')`** after `CREATE TABLE` with `updated_at` column

## Command Reference

| Command | Purpose |
|---------|---------|
| `cargo make db:status` | Migration status (applied vs pending) |
| `cargo make db:generate <name>` | Create new migration scaffold |
| `cargo make db:generate --diff-schema <name>` | Auto-generate SQL from schema diff |
| `cargo make db:up` | Apply all pending migrations |
| `cargo make db:down` | Rollback last migration |
| `cargo make db:reset` | Rollback all migrations |
| `cargo make db:fresh` | Drop DB, recreate, apply all |
| `cargo make db:drift-check` | CI gate: DB vs `schema.rs` |

## Checklist: Adding a New Table

- [ ] Write `up.sql` + `down.sql`
- [ ] `cargo make db:up`
- [ ] If table uses custom PostgreSQL enum types: add `#[derive(SqlType)]` struct to `types.rs`
- [ ] **Add enum type markers to `diesel.toml` `import_types`** (see Pitfall #8)
- [ ] `diesel print-schema > ../orm/src/schema.rs`
- [ ] Write model structs (Queryable, Insertable, AsChangeset)
- [ ] Export from `mod.rs`
- [ ] `cargo check -p orm` passes
- [ ] `cargo make db:drift-check` passes

## Detailed References

- **Model struct conventions** (Queryable, Insertable, AsChangeset, Associations) → `references/model-structs.md`
- **SQL → Rust type mapping** (core types, chrono, feature flags) → `references/type-mapping.md`
- **Common migration patterns** (create table, add column, FK, index, alter) → `references/migration-patterns.md`
- **Common pitfalls** (TIMESTAMP mismatch, missing features, field order) → `references/pitfalls.md`
