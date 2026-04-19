# Tauri Anti-Patterns Reference

## Forbidden Patterns

### ❌ Direct `invoke()` Without Type Safety

```typescript
// ❌ BAD — no type checking, error-prone
const result = await invoke('some_command', { data: input });
```

```typescript
// ✅ GOOD — use generated bindings
const result = await commands.some_command({ data: input });
```

**Why:** Manual `invoke` calls lose type safety. `tauri-specta` generates typed wrappers.

---

### ❌ Using `generate_handler![]` Instead of `tauri-specta`

```rust
// ❌ BAD — no type export, no frontend type safety
.invoke_handler(tauri::generate_handler![my_command])
```

```rust
// ✅ GOOD — specta builder handles both invoke + type export
let builder = command_builder();
.invoke_handler(builder.invoke_handler())
```

**Why:** This project requires TypeScript bindings. `tauri-specta` is the single source of truth.

---

### ❌ Custom Error Types Without `Serialize`

```rust
// ❌ BAD — won't serialize to frontend
#[tauri::command]
async fn do_thing() -> Result<Data, MyError> { ... }
```

```rust
// ✅ GOOD — String error serializes cleanly
#[tauri::command]
async fn do_thing() -> Result<Data, String> {
    inner().await.map_err(|e| format!("operation failed: {e}"))
}
```

**Why:** Tauri serializes command results to JSON. Custom errors must impl `Serialize` or use `String`.

---

### ❌ Missing `#[specta::specta]` Decorator

```rust
// ❌ BAD — command won't be included in type export
#[tauri::command]
pub async fn my_command() -> Result<(), String> { ... }
```

```rust
// ✅ GOOD — both decorators present
#[tauri::command]
#[specta::specta]
pub async fn my_command() -> Result<(), String> { ... }
```

**Why:** Without `#[specta::specta]`, the command won't appear in generated TypeScript bindings.

---

### ❌ Implicit State Lifetime

```rust
// ❌ BAD — missing explicit lifetime
#[tauri::command]
async fn cmd(state: tauri::State<MyManager>) -> Result<(), String> { ... }
```

```rust
// ✅ GOOD — explicit lifetime
#[tauri::command]
async fn cmd(state: tauri::State<'_, MyManager>) -> Result<(), String> { ... }
```

**Why:** Explicit lifetimes make borrow semantics clear and satisfy Clippy.

---

### ❌ Unbounded HTTP Permissions

```json
// ❌ BAD — allows any URL
{
  "identifier": "http:default",
  "allow": [{ "url": "*" }]
}
```

```json
// ✅ GOOD — scoped to specific domains
{
  "identifier": "http:default",
  "allow": [
    { "url": "https://api.example.com/*" },
    { "url": "http://localhost:3000/*" }
  ]
}
```

**Why:** Unrestricted HTTP access is a security risk. Scope to known endpoints.

---

### ❌ Not Cleaning Up Event Listeners

```typescript
// ❌ BAD — listener leaks on every mount
onMount(async () => {
  await listen('event', handler);
});
```

```typescript
// ✅ GOOD — cleanup on destroy
let unlisten: (() => void) | undefined;

onMount(async () => {
  unlisten = await listen('event', handler);
});

onDestroy(() => {
  unlisten?.();
});
```

**Why:** Uncleaned listeners accumulate and cause memory leaks + duplicate handlers.

---

### ❌ Using `unwrap()` in Commands

```rust
// ❌ BAD — panics crash the entire app
#[tauri::command]
async fn cmd() -> Result<(), String> {
    do_thing().unwrap();
    Ok(())
}
```

```rust
// ✅ GOOD — errors propagate to frontend
#[tauri::command]
async fn cmd() -> Result<(), String> {
    do_thing().await.map_err(|e| e.to_string())?;
    Ok(())
}
```

**Why:** `unwrap()` panics crash the Tauri app. Commands should return errors gracefully.

---

### ❌ Mixing SvelteKit Adapters

```javascript
// ❌ BAD — node adapter doesn't work with Tauri
import adapter from '@sveltejs/adapter-node';
```

```javascript
// ✅ GOOD — static adapter for Tauri
import adapter from '@sveltejs/adapter-static';
```

**Why:** Tauri serves static files. Node adapter produces a server, which Tauri can't use.
