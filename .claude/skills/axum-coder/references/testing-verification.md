# Testing & Verification

## Static Checks

Run backend compilation after handler, router, or model changes.

```bash
cargo check
```

Use the backend package directory for focused checks.

```bash
cargo check
```

## Route Syntax Search

Before finishing Axum 0.8 route work, search Rust route strings for old syntax.

```bash
rg '"[^"]*/:\w+|"[^"]*/\*\w+|without_v07_checks' packages/backend/src
```

Expected result: no matches.

## Startup Smoke Test

`cargo check` does not validate route patterns. Axum validates routes while constructing the router at runtime. Run a startup smoke test after route changes.

```bash
cargo run
```

For automated smoke checks, start the backend briefly and stop it after it binds successfully.

## Router Tests

Axum routers can be tested without opening a TCP listener using `tower::ServiceExt`.

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn healthcheck_returns_ok() {
    let app = router();
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

## Response Body Tests

Use `axum::body::to_bytes` to read response bodies.

```rust
let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
    .await
    .unwrap();
let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
```

## Verification Checklist

- `cargo check` passes for `packages/backend`.
- Route syntax search finds no old `/:param` or `/*wildcard` strings.
- Startup smoke test reaches listener bind or otherwise proves router construction succeeds.
- If frontend routes call backend endpoints, run `npx svelte-check` in `packages/desktop/ui`.
