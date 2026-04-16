# Anti-Patterns

Each item is labeled as **Official** or **Project**.

## 1. `unsafe` Code (Project)

```rust
// ❌ ANTI-PATTERN
unsafe {
    do_unsafe_work();
}
```

## 2. `unwrap()` in Production Code (Project)

```rust
// ❌ ANTI-PATTERN
let value = config.get("port").unwrap();

// ✅ CORRECT
let value = config
    .get("port")
    .context("missing config: port")?;
```

## 3. `todo!()` / `unimplemented!()` (Project)

```rust
// ❌ ANTI-PATTERN
fn handle_message(msg: &Message) -> anyhow::Result<()> {
    let _ = msg;
    todo!()
}
```

## 4. `dbg!()` in Production Code (Project)

```rust
// ❌ ANTI-PATTERN
dbg!(&device);

// ✅ CORRECT
tracing::debug!(?device, "processing device");
```

## 5. `println!()` for Structured Logging (Project)

```rust
// ❌ ANTI-PATTERN
println!("server started on port 9080");

// ✅ CORRECT
tracing::info!(port = 9080, "server started");
```

## 6. Unnecessary Heap Allocation (Common)

```rust
// ❌ ANTI-PATTERN
let stream = Box::new(tokio::net::TcpStream::connect(addr).await?);

// ✅ CORRECT
let stream = tokio::net::TcpStream::connect(addr).await?;
```

## 7. Boolean Flag Parameters (Official)

```rust
// ❌ ANTI-PATTERN
fn connect(retry: bool, secure: bool) {
    let _ = (retry, secure);
}

// ✅ CORRECT
fn connect(retry: RetryPolicy, security: Security) {
    let _ = (retry, security);
}
```

## 8. Public Fields Without Encapsulation (Official)

```rust
// ❌ ANTI-PATTERN
pub struct Connection {
    pub is_connected: bool,
}

// ✅ CORRECT
pub struct Connection {
    is_connected: bool,
}

impl Connection {
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
}
```

## 9. Missing `Debug` on Public Types (Official)

```rust
// ❌ ANTI-PATTERN
pub struct DeviceId(String);

// ✅ CORRECT
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(String);
```

## 10. Silently Discarding Errors (Common)

```rust
// ❌ ANTI-PATTERN
let _ = do_work();

// ✅ CORRECT
do_work()?;
```

## 11. `len() == 0` for Emptiness (Common)

```rust
// ❌ ANTI-PATTERN
if items.len() == 0 {
    return;
}

// ✅ CORRECT
if items.is_empty() {
    return;
}
```

## 12. Incomplete Generic Types (Official/Common)

```rust
// ❌ ANTI-PATTERN
let map: std::collections::HashMap<_, _> = std::collections::HashMap::new();

// ✅ CORRECT
let map: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
```

## 13. Reimplementing `From` / `AsRef` as Ad-hoc Helpers (Official)

```rust
// ❌ ANTI-PATTERN
fn device_id_as_str(id: &DeviceId) -> &str {
    &id.0
}

// ✅ CORRECT
impl AsRef<str> for DeviceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
```

## 14. Failing Destructors (Official)

```rust
// ❌ ANTI-PATTERN
impl Drop for Connection {
    fn drop(&mut self) {
        panic!("failed to close connection");
    }
}
```
