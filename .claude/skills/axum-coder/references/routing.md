# Routing & Nesting

Official sources: Axum `Router` docs and Axum 0.8 changelog.

## Axum 0.8 Capture Syntax

Axum 0.8 upgraded `matchit` and changed route captures from old colon/star syntax to brace syntax.

| Intent | Axum 0.8 | Old syntax |
| --- | --- | --- |
| Single capture | `/users/{id}` | `/users/:id` |
| Multiple captures | `/users/{user_id}/teams/{team_id}` | `/users/:user_id/teams/:team_id` |
| Wildcard | `/assets/{*path}` | `/assets/*path` |

Old syntax panics when constructing the router. Do not use `without_v07_checks()`.

## Project Router Pattern

Feature modules expose a `router()` function and are mounted in `packages/backend/src/main.rs`.

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
}
```

Mount feature routers under `/api` unless the module has a more specific prefix.

```rust
let app = axum::Router::new()
    .merge(health::router())
    .nest("/api", customer::router())
    .with_state(state);
```

## Nesting

Nested routers strip the matched prefix from the URI passed to inner routes.

```rust
let user_routes = axum::Router::new().route("/{id}", get(get_user));
let app = axum::Router::new().nest("/api/users", user_routes);
```

This accepts `GET /api/users/{id}`.

## Method Routing

Prefer chaining methods on one path when it improves readability.

```rust
axum::Router::new().route(
    "/customers/{id}",
    get(get_customer).put(update_customer).delete(delete_customer),
)
```

Separate `.route()` calls for the same path are also acceptable when they match existing file style.
