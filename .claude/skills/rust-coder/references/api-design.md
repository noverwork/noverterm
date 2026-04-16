# API Design

## Constructors (Official)

```rust
impl DeviceId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
}

impl Message {
    pub fn ping() -> Self {
        Self { kind: MessageKind::Ping }
    }
}
```

## Builders (Official)

Prefer a builder when construction has many optional fields or validation steps.

```rust
#[derive(Default)]
pub struct ClientBuilder {
    timeout_ms: Option<u64>,
    device_id: Option<String>,
}

impl ClientBuilder {
    pub fn timeout_ms(mut self, value: u64) -> Self {
        self.timeout_ms = Some(value);
        self
    }

    pub fn device_id(mut self, value: impl Into<String>) -> Self {
        self.device_id = Some(value.into());
        self
    }

    pub fn build(self) -> anyhow::Result<Client> {
        let device_id = self.device_id.context("device_id is required")?;
        Ok(Client::new(device_id, self.timeout_ms.unwrap_or(5_000)))
    }
}
```

## Newtypes (Official)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);
```

## Custom Types Over Bool (Official)

```rust
pub enum RetryPolicy {
    None,
    Automatic { max: u32 },
}

pub enum Security {
    Plain,
    Tls(TlsConfig),
}

fn connect(retry: RetryPolicy, security: Security) {
    let _ = (retry, security);
}
```

## Common Traits (Official/Common)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(String);
```

## Conversion Traits (Official)

Prefer `From`, `TryFrom`, `AsRef`, and `Borrow` when they fit naturally.

```rust
impl From<DeviceId> for String {
    fn from(value: DeviceId) -> Self {
        value.0
    }
}

impl AsRef<str> for DeviceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
```

## Private Fields (Official)

```rust
pub struct Connection {
    stream: tokio::net::TcpStream,
    device_id: DeviceId,
}

impl Connection {
    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }
}
```

## Functions as Methods (Official)

```rust
impl Device {
    pub fn is_connected(&self) -> bool {
        self.state == DeviceState::Connected
    }
}
```

## Public API Documentation (Official)

Public APIs should explain what they do and include examples where useful.

```rust
/// Sends a ping message to the connected device.
///
/// # Errors
///
/// Returns an error if the connection is closed.
pub async fn send_ping(&mut self) -> anyhow::Result<()> {
    Ok(())
}
```
