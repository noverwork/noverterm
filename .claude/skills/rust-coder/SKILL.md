---
name: rust-coder
description: Rust coding guide for this monorepo. Use when writing Rust code in any package (desktop, orm, relay, shared, update-signer). Combines Rust API Guidelines with clearly marked project rules for errors, Tokio, tracing, and Clippy. For GPUI code use gpui-coder; for schema changes use diesel-migration.
---

# Rust Coder - Convention Guide

Rust conventions for this monorepo.

- **Official** = Rust API Guidelines / rustfmt / standard Rust conventions
- **Project** = local rules for this repo
- **Common** = widespread Rust practice, not a strict official requirement

## Forbidden Patterns

```rust
// ❌ NEVER (Project)
unsafe {
    do_unsafe_work();
}

value.unwrap();
todo!();
dbg!(&value);

// ❌ AVOID IN LIBRARY CODE (Project)
value.expect("must exist");
```

## Quick Reference

### Naming

| Type              | Convention             | Example                            | Source   |
| ----------------- | ---------------------- | ---------------------------------- | -------- |
| Files/dirs        | `snake_case`           | `message_handler.rs`               | Official |
| Types/traits      | `UpperCamelCase`       | `MessageHandler`, `DeviceId`       | Official |
| Functions/vars    | `snake_case`           | `handle_message()`, `device_count` | Official |
| Constants/statics | `SCREAMING_SNAKE_CASE` | `MAX_RETRY_COUNT`                  | Official |
| Cargo crate name  | `kebab-case`           | `update-signer`                    | Official |
| Rust module path  | `snake_case`           | `update_signer`                    | Official |

### Key Rules

1. **[Project]** No `unwrap()` outside tests — propagate with `?`. The one exception is `mutex.lock().unwrap()`: a poisoned lock is already a crash
2. **[Project]** `expect("why this cannot fail")` is allowed in tests and for startup invariants (the Tokio runtime, a backend task that panicked); not for input or I/O
3. **[Project]** No `unsafe` in this repo
4. **[Official]** Use `as_`, `to_`, `into_` consistently for conversions
5. **[Official]** Use `iter`, `iter_mut`, `into_iter` for iterator-producing methods
6. **[Official]** All public types should implement `Debug`
7. **[Official]** Prefer private fields and controlled construction
8. **[Official]** Avoid boolean flag parameters; prefer domain types
9. **[Official]** Destructors must not fail
10. **[Project]** Async work runs on Tokio; log with `tracing` macros, never `println!`
11. **[Official]** Public APIs should be documented and include examples where useful
12. **[Official]** Use `cargo fmt` defaults; do not customize rustfmt casually

### Error Handling

Two styles, by package (**Project**):

```rust
// packages/desktop: Result<T, String>. The string reaches the user, so say what failed.
pub fn init_pool(database_path: &Path) -> Result<DbPool, String> {
    r2d2::Pool::builder()
        .build(manager)
        .map_err(|error| format!("failed to open database: {error}"))
}

// packages/relay: anyhow at the binary's edge.
use anyhow::{Context, Result};

async fn connect(addr: &str) -> Result<TcpStream> {
    TcpStream::connect(addr)
        .await
        .with_context(|| format!("failed to connect to {addr}"))
}
```

Match the package you are in. Do not add `anyhow` or `thiserror` to `desktop`.

### Async Pattern

```rust
// Blocking work (SQLite, file I/O) leaves the async thread.
tokio::task::spawn_blocking(move || operation(&mut connection)).await

// Log with structured fields, message last.
tracing::warn!(error, "Failed to save setting");
```

The desktop app owns one Tokio runtime (`crate::app::runtime()`); GPUI's executor is separate. UI
code reaches backend futures through `backend(...)` in `ui/bridge.rs` (see gpui-coder).

## Detailed References

- **Naming conventions** (types, functions, traits, modules) → `references/naming.md`
- **Error handling** (String errors, anyhow, domain errors, validation) → `references/error-handling.md`
- **Async & Tokio** (spawning, cancellation, streams, tracing) → `references/async-tokio.md`
- **API design** (constructors, builders, conversions, newtypes, traits) → `references/api-design.md`
- **Clippy & correctness** (lint policy, commands, common fixes) → `references/clippy.md`
- **Anti-patterns** (official + project anti-patterns, clearly labeled) → `references/anti-patterns.md`
