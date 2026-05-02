# Extractors

## Common Extractors

| Extractor | Purpose |
| --- | --- |
| `Path<T>` | Route captures from `{name}` segments |
| `Query<T>` | Query string deserialization |
| `Json<T>` | JSON request body deserialization |
| `Multipart` | `multipart/form-data`, enabled by the `multipart` feature |
| `State<T>` | Application state from `.with_state(...)` |
| `HeaderMap` | Request headers |
| `Request` | Full request when lower-level access is needed |

## Path

Single parameter:

```rust
async fn get_customer(Path(id): Path<i32>) -> Result<Json<CustomerResponse>> {
    // handler body
}
```

Multiple parameters:

```rust
async fn remove_member(
    Path((customer_id, user_id)): Path<(i32, String)>,
) -> Result<Json<serde_json::Value>> {
    // handler body
}
```

Struct parameters are clearer when there are several captures.

```rust
#[derive(serde::Deserialize)]
struct MemberPath {
    id: i32,
    user_id: String,
}

async fn remove_member(Path(path): Path<MemberPath>) -> Result<Json<serde_json::Value>> {
    // handler body
}
```

Axum 0.8 requires tuple `Path` extractors to match the route capture count exactly.

## State

Use `State<AppState>` for shared application state.

```rust
async fn handler(State(state): State<AppState>) -> Result<Json<ResponseBody>> {
    // handler body
}
```

The state type must be cloneable when used with `Router::with_state`.

## JSON

`Json<T>` consumes the request body, so it must be the last extractor argument.

```rust
async fn create_customer(
    State(state): State<AppState>,
    Json(req): Json<CreateCustomerRequest>,
) -> Result<Json<CustomerResponse>> {
    // handler body
}
```

## Multipart

Use `Multipart` for file uploads. It also consumes the body and must be last.

```rust
async fn upload(State(state): State<AppState>, mut multipart: Multipart) -> Result<Json<Response>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart: {e}")))?
    {
        // process field
    }

    Ok(Json(response))
}
```

## Ordering Rules

- Extractors run left to right.
- Only one body-consuming extractor is allowed.
- Body-consuming extractors include `Json`, `Multipart`, `String`, `Bytes`, and `Form`.
- Put body-consuming extractors last.
