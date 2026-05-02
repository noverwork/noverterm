---
name: axum-coder
description: Axum 0.8 backend coding guide for this monorepo. Use when adding or changing Axum routes, handlers, extractors, routers, middleware, HTTP APIs, backend request/response types, or route tests. Covers route syntax, Path/State/Json/Multipart extractors, IntoResponse errors, Tower middleware, startup validation, and project-specific Diesel/AppError patterns.
---

# Axum Coder - Convention Guide

Axum conventions for this monorepo.

- **Official** = Axum 0.8 docs / changelog / docs.rs examples
- **Project** = local rules for this repo's `packages/backend`
- **Common** = widespread Axum/Tower practice, not a strict official requirement

## Forbidden Patterns

```rust
// ❌ NEVER in axum 0.8 (Project/Official)
Router::new().route("/customers/:id", get(get_customer));
Router::new().route("/files/*path", get(get_file));

// ❌ NEVER hide invalid old route syntax (Project)
Router::new().without_v07_checks();

// ❌ NEVER block async runtime with sync Diesel work directly in handler (Project)
let mut conn = state.pool.get()?;
customers::table.load::<Customer>(&mut conn)?;

// ❌ NEVER leak raw database errors from handlers (Project)
async fn handler() -> Result<Json<T>, diesel::result::Error> { todo!() }
```

## Quick Reference

### Axum 0.8 Route Syntax

| Intent | Correct | Incorrect | Source |
| --- | --- | --- | --- |
| Single capture | `/customers/{id}` | `/customers/:id` | Official |
| Multiple captures | `/customers/{id}/members/{user_id}` | `/customers/:id/members/:user_id` | Official |
| Wildcard capture | `/assets/{*path}` | `/assets/*path` | Official |
| Literal braces | `/literal/{{name}}` | `/literal/{name}` | Official |

### Key Rules

1. **[Official]** Axum 0.8 captures use `{name}` and wildcards use `{*name}`.
2. **[Official]** Old `:name` / `*name` route syntax panics at router construction.
3. **[Project]** Do not use `without_v07_checks()`; migrate routes instead.
4. **[Official]** `Path<T>` can deserialize into a scalar, tuple, struct, or map.
5. **[Official]** Tuple `Path` extractor length must exactly match route captures in Axum 0.8.
6. **[Project]** Handlers return `crate::error::Result<T>` where possible.
7. **[Project]** Convert database/task failures into `AppError`.
8. **[Project]** Run Diesel queries in `tokio::task::spawn_blocking`.
9. **[Project]** Register feature routers in `main.rs` with `.nest("/api", module::router())`.
10. **[Project]** Verify route changes with backend startup smoke test, not only `cargo check`.

### Router Pattern

```rust
use axum::routing::{delete, get, post, put};

use crate::state::AppState;

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/customers", get(list_customers))
        .route("/customers", post(create_customer))
        .route("/customers/{id}", get(get_customer))
        .route("/customers/{id}", put(update_customer))
        .route("/customers/{id}", delete(delete_customer))
        .route("/customers/{id}/members/{user_id}", delete(remove_customer_member))
}
```

### Handler Pattern

```rust
use axum::extract::{Path, State};
use axum::Json;

use crate::error::{AppError, Result};
use crate::state::AppState;

pub async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<CustomerResponse>> {
    let pool = state.pool.clone();
    let customer = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| AppError::Internal(format!("db connection: {e}")))?;

        customers::table
            .filter(customers::id.eq(id))
            .select(Customer::as_select())
            .first::<Customer>(&mut conn)
            .optional()
            .map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::Internal(format!("task failed: {e}")))?;

    customer?
        .map(CustomerResponse::from)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("customer not found: {id}")))
}
```

## Detailed References

- **Routing & nesting** (Axum 0.8 route syntax, captures, wildcards, nesting) → `references/routing.md`
- **Extractors** (Path, State, Json, Query, Multipart, extractor ordering) → `references/extractors.md`
- **Responses & errors** (IntoResponse, Result, AppError pattern, rejection handling) → `references/responses-errors.md`
- **Middleware & state** (Tower layers, TraceLayer, CorsLayer, shared state) → `references/middleware-state.md`
- **Testing & verification** (router tests, startup smoke tests, search checklist) → `references/testing-verification.md`
- **Anti-patterns** (old syntax, blocking DB, bad extractor order, route panics) → `references/anti-patterns.md`

## Official Sources

- Axum 0.8.8 docs: https://docs.rs/axum/0.8.8/axum/
- Axum `Router` docs: https://docs.rs/axum/0.8.8/axum/routing/struct.Router.html
- Axum `Path` extractor docs: https://docs.rs/axum/0.8.8/axum/extract/path/struct.Path.html
- Axum `IntoResponse` docs: https://docs.rs/axum/0.8.8/axum/response/trait.IntoResponse.html
- Axum changelog: https://docs.rs/axum/0.8.8/source/CHANGELOG.md
- Axum v0.8.0 release notes: https://github.com/tokio-rs/axum/releases/tag/axum-v0.8.0
