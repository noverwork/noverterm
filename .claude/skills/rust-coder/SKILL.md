---
name: rust-coder
description: Rust coding guide for this monorepo. Use when writing Rust code for relay, relay-agent, or relay-protocol packages. Combines Rust API Guidelines with clearly marked project rules for anyhow, Tokio, tracing, and Clippy.
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
| Cargo crate name  | `kebab-case`           | `relay-protocol`                   | Official |
| Rust module path  | `snake_case`           | `relay_protocol`                   | Official |

### Key Rules

1. **[Project]** No `unwrap()` in production code — use `?` and `.context()`
2. **[Project]** `expect()` is allowed in tests, binaries, and startup assertions; avoid it in library code
3. **[Project]** No `unsafe` in this repo
4. **[Official]** Use `as_`, `to_`, `into_` consistently for conversions
5. **[Official]** Use `iter`, `iter_mut`, `into_iter` for iterator-producing methods
6. **[Official]** All public types should implement `Debug`
7. **[Official]** Prefer private fields and controlled construction
8. **[Official]** Avoid boolean flag parameters; prefer domain types
9. **[Official]** Destructors must not fail
10. **[Project]** Use Tokio + tracing for async work
11. **[Official]** Public APIs should be documented and include examples where useful
12. **[Official]** Use `cargo fmt` defaults; do not customize rustfmt casually

### Error Handling

```rust
use anyhow::{Context, Result};
use tokio::net::TcpStream;

async fn connect(addr: &str) -> Result<TcpStream> {
    TcpStream::connect(addr)
        .await
        .with_context(|| format!("failed to connect to {addr}"))
}
```

### Async Pattern

```rust
#[tracing::instrument(skip_all, fields(device_id = %device.id()))]
async fn handle_device(device: &Device) -> anyhow::Result<()> {
    tracing::debug!("processing device");
    Ok(())
}
```

## Detailed References

- **Naming conventions** (types, functions, traits, modules) → `references/naming.md`
- **Error handling** (anyhow, context, domain errors, validation) → `references/error-handling.md`
- **Async & Tokio** (spawning, cancellation, streams, tracing) → `references/async-tokio.md`
- **API design** (constructors, builders, conversions, newtypes, traits) → `references/api-design.md`
- **Clippy & correctness** (pedantic rules, common lints, deny list) → `references/clippy.md`
- **Anti-patterns** (official + project anti-patterns, clearly labeled) → `references/anti-patterns.md`
