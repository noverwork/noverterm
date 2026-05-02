# Responses & Errors

## IntoResponse

Axum handlers can return any type implementing `IntoResponse`, including `Json<T>`, tuples like `(StatusCode, Json<T>)`, and `Result<T, E>` where both variants can become responses.

```rust
use axum::http::StatusCode;
use axum::Json;

async fn created(Json(body): Json<CreateCustomerRequest>) -> (StatusCode, Json<CustomerResponse>) {
    (StatusCode::CREATED, Json(response))
}
```

## Project Error Type

This repo uses `crate::error::{AppError, Result}`. Handlers should return project `Result<T>` and convert lower-level failures into `AppError`.

```rust
use crate::error::{AppError, Result};

pub async fn get_customer(Path(id): Path<i32>) -> Result<Json<CustomerResponse>> {
    customer
        .map(CustomerResponse::from)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("customer not found: {id}")))
}
```

## Diesel Error Mapping

Diesel errors are mapped through `AppError::from` in this project.

```rust
customers::table
    .filter(customers::id.eq(id))
    .select(Customer::as_select())
    .first::<Customer>(&mut conn)
    .optional()
    .map_err(AppError::from)
```

## Spawn Blocking Error Pattern

Synchronous database work runs inside `spawn_blocking`. There are two error layers: the join error and the closure result.

```rust
let value = tokio::task::spawn_blocking(move || -> Result<T> {
    // sync work
    Ok(value)
})
.await
.map_err(|e| AppError::Internal(format!("task failed: {e}")))??;
```

## Extractor Rejections

Axum normally turns extractor failures into responses automatically. If a handler needs custom rejection handling, wrap the extractor in `Result`.

```rust
use axum::extract::rejection::JsonRejection;

async fn handler(payload: std::result::Result<Json<RequestBody>, JsonRejection>) -> Result<Json<ResponseBody>> {
    let Json(payload) = payload.map_err(|e| AppError::BadRequest(e.to_string()))?;
    Ok(Json(handle(payload)))
}
```

## Middleware Errors

Tower middleware that can fail must be wrapped with `HandleErrorLayer` so errors become responses. Axum services should not expose arbitrary fallible service errors to Hyper.
