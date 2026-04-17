# Migrator

This package uses Diesel CLI for all migration operations.

## Setup

1. Copy `.env.example` to `.env`.
2. Install Diesel CLI with PostgreSQL support:

```bash
cargo install diesel_cli --no-default-features --features postgres
```

## Usage

Run commands in `packages/migrator`:

```bash
diesel migration list
diesel migration run
diesel migration revert
diesel migration generate my_change --diff-schema
```

Or use workspace tasks from the repo root:

```bash
cargo make db:status
cargo make db:up
cargo make db:down
cargo make db:generate MIGRATION_NAME=my_change
```
