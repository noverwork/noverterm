# Middleware & State

## State

Application state is created in `packages/backend/src/state.rs` and attached in `main.rs` with `.with_state(state)`.

```rust
let state = AppState::new(config).await;

let app = axum::Router::new()
    .nest("/api", customer::router())
    .with_state(state);
```

Handlers access it with `State<AppState>`.

```rust
pub async fn handler(State(state): State<AppState>) -> Result<Json<ResponseBody>> {
    let pool = state.pool.clone();
    Ok(Json(response))
}
```

## Layers

This project uses Tower/Tower HTTP layers for cross-cutting behavior.

```rust
let app = axum::Router::new()
    .nest("/api", customer::router())
    .layer(tower_http::cors::CorsLayer::permissive())
    .layer(tower_http::trace::TraceLayer::new_for_http())
    .with_state(state);
```

## Layer Ordering

Tower layers wrap services. Keep project-wide layers in `main.rs` unless the middleware is feature-specific.

Use `route_layer` for route-specific middleware when a layer should only run after a route has matched.

```rust
axum::Router::new()
    .route("/protected", get(protected))
    .route_layer(axum::middleware::from_fn(auth));
```

## Tracing

Use `tracing` macros in handlers and background tasks. Prefer structured fields over formatted strings when data should be searchable.

```rust
tracing::info!(customer_id = id, "loaded customer");
tracing::warn!(line_group_id = %group_id, "line group not found");
```

## CORS

Current desktop/backend development uses permissive CORS. If the backend becomes public-facing, replace permissive CORS with explicit origins, methods, and headers.
