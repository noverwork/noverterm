# Async & Tokio

## Async/Await (Official + Common)

Prefer `async`/`await` to manual future chaining unless chaining is clearer.

```rust
async fn fetch_data(url: &str) -> anyhow::Result<String> {
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

// ❌ BAD — unnecessary async
async fn get_value(value: i32) -> i32 {
    value
}

// ✅ GOOD
fn current_value(value: i32) -> i32 {
    value
}
```

## Tracing Instrumentation (Project)

```rust
tracing::warn!(error, "SFTP folder conflict scan failed");
tracing::debug!(kind = %message.kind(), "received message");
```

Structured fields first, a fixed message last. `#[tracing::instrument]` is not used in this repo.

## Spawning Tasks (Project/Common)

```rust
let handle = tokio::spawn(async move {
    process_device(device).await
});

match handle.await {
    Ok(Ok(())) => tracing::info!("task completed"),
    Ok(Err(error)) => tracing::error!(%error, "task failed"),
    Err(join_error) => tracing::error!(%join_error, "task panicked or was cancelled"),
}
```

## Cancellation (Project/Common)

```rust
async fn run_with_shutdown(
    mut shutdown: tokio::sync::oneshot::Receiver<()>,
    worker: impl std::future::Future<Output = anyhow::Result<()>>,
) -> anyhow::Result<()> {
    tokio::select! {
        result = worker => result,
        _ = &mut shutdown => {
            tracing::info!("shutdown requested");
            Ok(())
        }
    }
}

let task = tokio::spawn(async move {
    do_work().await
});

task.abort();
```

## Manual Channel Wrapper for Single Async Work (Project guidance)

Avoid wrapping a single async operation in a channel unless there is a real ownership or cancellation need.

```rust
// ❌ Usually unnecessary
let (tx, rx) = tokio::sync::oneshot::channel();
tokio::spawn(async move {
    let result = do_work().await;
    let _ = tx.send(result);
});

let output = rx.await??;

// ✅ Usually simpler
let output = do_work().await?;
```
