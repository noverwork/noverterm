# Anti-Patterns

Each item is labeled as **Official**, **Project**, or **Common**.

## 1. Old Axum Route Capture Syntax (Official + Project)

```rust
// ❌ ANTI-PATTERN in axum 0.8
Router::new().route("/customers/:id", get(get_customer));

// ✅ CORRECT
Router::new().route("/customers/{id}", get(get_customer));
```

## 2. Old Wildcard Syntax (Official + Project)

```rust
// ❌ ANTI-PATTERN in axum 0.8
Router::new().route("/assets/*path", get(get_asset));

// ✅ CORRECT
Router::new().route("/assets/{*path}", get(get_asset));
```

## 3. `without_v07_checks()` (Project)

```rust
// ❌ ANTI-PATTERN
Router::new().without_v07_checks();
```

Migrate route strings instead. Keeping old syntax hides a version mismatch and can surprise future route readers.

## 4. Blocking Diesel Directly in Handlers (Project)

```rust
// ❌ ANTI-PATTERN
async fn handler(State(state): State<AppState>) -> Result<Json<Vec<Customer>>> {
    let mut conn = state.pool.get()?;
    let rows = customers::table.load::<Customer>(&mut conn)?;
    Ok(Json(rows))
}

// ✅ CORRECT
async fn handler(State(state): State<AppState>) -> Result<Json<Vec<Customer>>> {
    let pool = state.pool.clone();
    let rows = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| AppError::Internal(format!("db connection: {e}")))?;
        customers::table.load::<Customer>(&mut conn).map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError::Internal(format!("task failed: {e}")))??;

    Ok(Json(rows))
}
```

## 5. Body Extractor Before Parts Extractors (Official)

```rust
// ❌ ANTI-PATTERN
async fn handler(Json(req): Json<RequestBody>, State(state): State<AppState>) {}

// ✅ CORRECT
async fn handler(State(state): State<AppState>, Json(req): Json<RequestBody>) {}
```

Body-consuming extractors must be last.

## 6. Returning Raw Infrastructure Errors (Project)

```rust
// ❌ ANTI-PATTERN
async fn handler() -> std::result::Result<Json<T>, diesel::result::Error> { ... }

// ✅ CORRECT
async fn handler() -> crate::error::Result<Json<T>> { ... }
```

Map infrastructure errors into `AppError` so response formatting stays consistent.

## 7. Assuming `cargo check` Catches Route Panics (Project)

Axum route path syntax is validated at runtime when the router is constructed. Always run a startup smoke test after route changes.
