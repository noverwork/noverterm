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
#[tracing::instrument(skip_all, fields(device_id = %device.id()))]
async fn handle_message(device: &Device, msg: &Message) -> anyhow::Result<()> {
    tracing::debug!(kind = %msg.kind(), "received message");
    Ok(())
}
```

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

## Streams (Common)

Use combinators when they genuinely improve clarity. A `while let` loop is also perfectly idiomatic.

```rust
use tokio_stream::StreamExt;

let names: Vec<_> = stream
    .filter_map(|msg| async move {
        if msg.is_valid() {
            Some(msg.name().to_owned())
        } else {
            None
        }
    })
    .collect()
    .await;
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
