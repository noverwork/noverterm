# Error Handling

## Boundary Errors vs Domain Errors

- **Project:** use `anyhow::Result<T>` + `.context()` at application boundaries
- **Common/Official-friendly:** use typed errors (`thiserror`) for reusable domain/library logic

```rust
use anyhow::{Context, Result};
use tokio::net::TcpStream;

async fn connect(addr: &str) -> Result<TcpStream> {
    TcpStream::connect(addr)
        .await
        .with_context(|| format!("failed to connect to {addr}"))
}
```

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("device {0} is not registered")]
    DeviceNotRegistered(String),

    #[error("invalid message: {0}")]
    InvalidMessage(String),
}
```

## `unwrap`, `expect`, `todo`, `dbg` (Project)

```rust
// ❌ BAD in production/library code
let value = map.get("key").unwrap();
dbg!(&value);

// ✅ GOOD
let value = map
    .get("key")
    .context("missing required key: key")?;

tracing::debug!(?value, "loaded config value");
```

`expect()` is acceptable in tests, binaries, and startup assertions where a crash indicates a programming/configuration error.

## Validation (Official)

Prefer static enforcement when practical.

```rust
#[derive(Debug, Clone)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn new(id: &str) -> Result<Self> {
        if id.is_empty() {
            anyhow::bail!("device id cannot be empty");
        }

        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            anyhow::bail!("device id contains invalid characters: {id}");
        }

        Ok(Self(id.to_owned()))
    }
}
```

## Destructors Never Fail (Official)

```rust
impl Connection {
    pub fn close(&mut self) -> std::io::Result<()> {
        self.stream.shutdown(std::net::Shutdown::Both)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}
```

## No Out-Parameters (Official)

```rust
// ❌ BAD
fn parse(input: &str, output: &mut Vec<Message>) {
    output.extend(parse_impl(input));
}

// ✅ GOOD
fn parse(input: &str) -> anyhow::Result<Vec<Message>> {
    Ok(parse_impl(input))
}
```
