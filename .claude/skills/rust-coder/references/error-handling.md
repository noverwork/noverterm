# Error Handling

## Boundary Errors vs Domain Errors

- **Project, `packages/desktop`:** functions return `Result<T, String>`. Convert at the call that
  failed with `.map_err(|error| format!("failed to open database: {error}"))`. The string is shown
  to the user, so name the operation. Shared converters exist per area (`store::internal_error`).
- **Project, `packages/relay`:** `anyhow::Result<T>` + `.context()`.
- **Project, domain errors:** when callers branch on the kind of failure, write a plain enum with
  hand-written `Display` and `impl std::error::Error` (`SftpError` in `runtime/sftp/types.rs`).
  `thiserror` is not a dependency; do not add it for one enum.

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SftpError {
    AlreadyExists,
    NotFound(String),
}

impl fmt::Display for SftpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => write!(formatter, "SFTP destination already exists"),
            Self::NotFound(path) => write!(formatter, "{path} not found"),
        }
    }
}

impl std::error::Error for SftpError {}
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

`expect()` is acceptable in tests and for startup invariants where a crash indicates a programming error. `mutex.lock().unwrap()` is the accepted way to take a `std::sync::Mutex`.

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
