---
name: diesel-migration
description: Diesel SQL migration workflow for this monorepo. Use when adding/changing database schema, creating migrations, or syncing Rust schema definitions.
---

# Diesel Migration & Model Workflow

Schema changes go through raw SQL migrations first, then sync to `schema.rs`, then hand-write model structs.

**Project source of truth:** SQL migrations are the source of truth. `schema.rs` is generated from the database state. ORM models are hand-written mirrors of `schema.rs`.

> Diesel 2.x does NOT auto-generate model structs. Third-party tools like `dsync` are early-stage (v0.1.0) with known bugs — not recommended.

## Project Policy: Migration-First Only

This repo intentionally uses **migration-first** database development.

Do **not** use an entity-first workflow. Do **not** treat `packages/orm/src/models/` as the source of truth for DB structure. Rust model structs cannot safely express every production schema concern, including constraints, indexes, foreign keys, triggers, generated columns, data backfills, and rollback behavior.

Required order for every DB schema change:

```text
1. Create migration scaffold
2. Write up.sql and down.sql
3. Apply migration to the dev database
4. Regenerate packages/orm/src/schema.rs from the migrated database
5. Update packages/orm/src/models/* by hand to mirror schema.rs
6. Update repositories, API types, seed data, and tests
7. Run verification commands
```

Never do this:

```text
1. Edit ORM model first
2. Guess schema.rs manually
3. Backfill migration afterward
```

Exception: `diesel migration generate --diff-schema <name>` may be used only as a **draft generator** after intentionally editing `schema.rs`. The generated SQL must be reviewed and corrected manually before use.

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

1. **Generate scaffold**: `cd packages/migrator && diesel migration generate <name>` for an empty migration, or `cargo make db:generate <name>` for a schema-diff draft
2. **Write SQL**: `up.sql` (forward) + `down.sql` (rollback)
3. **Apply**: `cargo make db:up`
4. **Sync schema**: `cargo make db:print-schema`
5. **Write models**: Create `packages/orm/src/models/<name>.rs` (see [Model Structs](references/model-structs.md))
6. **Export**: Add to `packages/orm/src/models/mod.rs`
7. **Update usage**: repositories, routes, seed data, shared API types, frontend types, and related tests
8. **Verify**: run the verification block below

## Verification Block

Run these after every migration-backed schema change:

```bash
cargo fmt --package orm --package backend --package desktop --package migrator
cargo check -p orm
cargo check -p backend
cargo check -p desktop
cargo test -p orm
cargo test -p backend
cargo test -p desktop
```

If frontend/shared API types changed, also run:

```bash
cd packages/desktop/ui
npm run check
npm run test
npm run build
```

Run drift check when available:

```bash
cargo make db:drift-check
```

## Key Rules

- **Never edit `schema.rs` manually** — always regenerate via `diesel print-schema`
- **Always write migration first**, then model — never the reverse
- **Every table should use `id` as the primary key** unless the user explicitly approves an exception
- **Use unique indexes/constraints for business uniqueness** (for example `(owner_id, key)`), not composite primary keys
- **`TIMESTAMP` → `NaiveDateTime`**, `TIMESTAMPTZ` → `DateTime<Utc>` — mismatching causes compile errors
- **`Queryable` field order must match `table!` column order** exactly
- **Main model structs should derive `AsChangeset`** when practical
- **Dedicated `Update*` changeset structs should exclude primary key, `created_at`, and ownership columns**
- **Use `diesel_manage_updated_at('table_name')`** after `CREATE TABLE` with `updated_at` column

## Model Checklist After `print-schema`

For every changed table, verify:

- [ ] `schema.rs` table columns match the migrated database
- [ ] `Queryable` model fields are in the exact same order as `schema.rs`
- [ ] Main model derives include `Debug`, `Clone`, `Queryable`, `Selectable`, and `AsChangeset` where practical
- [ ] `Insertable` model includes required columns, including `id`
- [ ] `Update*` changeset excludes `id`, `created_at`, and ownership columns like `owner_id`
- [ ] Associations and `joinable!` entries match foreign keys
- [ ] Unique constraints that replace composite keys are represented in tests
- [ ] Seed data includes all required fields

## Command Reference

| Command | Purpose |
|---------|---------|
| `cargo make db:status` | Migration status (applied vs pending) |
| `cd packages/migrator && diesel migration generate <name>` | Create an empty migration scaffold (`up.sql` + `down.sql`) |
| `cargo make db:generate <name>` | Repo wrapper around `diesel migration generate --diff-schema <name>`; creates a schema-diff draft, not an empty scaffold |
| `cargo make db:print-schema` | Run `diesel print-schema > packages/orm/src/schema.rs` |
| `cargo make db:up` | Apply all pending migrations |
| `cargo make db:down` | Rollback last migration |
| `cargo make db:reset` | Rollback all migrations |
| `cargo make db:fresh` | Drop DB, recreate, apply all |
| `cargo make db:drift-check` | CI gate: DB vs `schema.rs` |

## Checklist: Adding a New Table

- [ ] Generate migration with `cd packages/migrator && diesel migration generate <name>` for empty scaffold, or `cargo make db:generate <name>` only when a reviewed schema-diff draft is desired
- [ ] Write/review `up.sql` + `down.sql`
- [ ] `cargo make db:up`
- [ ] `cargo make db:print-schema`
- [ ] Use `id` as primary key unless explicitly approved otherwise
- [ ] Write model structs (Queryable, Selectable, Insertable, AsChangeset)
- [ ] Export from `mod.rs`
- [ ] Update repositories/routes/API types/tests
- [ ] Full verification block passes
- [ ] `cargo make db:drift-check` passes

## Detailed References

- **Model struct conventions** (Queryable, Insertable, AsChangeset, Associations) → `references/model-structs.md`
- **SQL → Rust type mapping** (core types, chrono, feature flags) → `references/type-mapping.md`
- **Common migration patterns** (create table, add column, FK, index, alter) → `references/migration-patterns.md`
- **Common pitfalls** (TIMESTAMP mismatch, missing features, field order) → `references/pitfalls.md`
