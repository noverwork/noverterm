# Naming Conventions

Follow Rust API Guidelines (RFC 430 / C-CASE).

## Naming Table

| Type                   | Convention                                  | Example                                         |
| ---------------------- | ------------------------------------------- | ----------------------------------------------- |
| Files/dirs             | `snake_case`                                | `message_handler.rs`, `connection_pool/`        |
| Cargo crates           | `kebab-case`                                | `relay-protocol`                                |
| Rust module paths      | `snake_case`                                | `use relay_protocol::message::Ping;`            |
| Types/traits/enums     | `UpperCamelCase`                            | `MessageHandler`, `DeviceId`, `ConnectionState` |
| Functions/methods/vars | `snake_case`                                | `handle_message()`, `device_count`              |
| Constants/statics      | `SCREAMING_SNAKE_CASE`                      | `MAX_RETRY_COUNT`, `DEFAULT_PORT`               |
| Lifetimes              | short lowercase                             | `'a`, `'de`, `'static`                          |
| Type params            | `T`, `K`, `V` or descriptive UpperCamelCase | `T`, `Handler`                                  |

## Conversion Method Names (Official)

| Prefix  | Meaning                   | Example                          |
| ------- | ------------------------- | -------------------------------- |
| `as_`   | cheap borrowed conversion | `fn as_str(&self) -> &str`       |
| `to_`   | allocates or clones       | `fn to_string(&self) -> String`  |
| `into_` | consumes `self`           | `fn into_bytes(self) -> Vec<u8>` |

## Iterator Method Names (Official)

```rust
pub struct Collection<T> {
    items: Vec<T>,
}

impl<T> Collection<T> {
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.items.iter_mut()
    }

    pub fn into_iter(self) -> std::vec::IntoIter<T> {
        self.items.into_iter()
    }
}
```

## Getter Names (Official)

```rust
impl User {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Cache {
    pub fn get_entry(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key)
    }
}
```

## Boolean Names (Official)

```rust
pub fn is_empty(&self) -> bool {
    self.items.is_empty()
}

pub fn can_connect(&self) -> bool {
    self.state == State::Ready
}
```
