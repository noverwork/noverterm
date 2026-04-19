# Tauri Plugins Reference

## Currently Used Plugins

This project uses these plugins (from `Cargo.toml`):

```toml
[dependencies]
tauri-plugin-http = "2"
tauri-plugin-opener = "2"
tauri-plugin-store = "2"
```

### Registration

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_store::Builder::default().build())
```

- **[Official]** Plugins are registered BEFORE `invoke_handler()` and `run()`
- **[Official]** Some plugins use `::init()`, others use `Builder::default().build()` — check plugin docs

## Plugin: HTTP (`tauri-plugin-http`)

Enables HTTP requests from the frontend with scope-limited permissions.

**Capability:**
```json
{
  "identifier": "http:default",
  "allow": [{ "url": "http://*" }, { "url": "https://*" }]
}
```

**Frontend usage:**
```typescript
import { fetch } from '@tauri-apps/plugin-http';

const response = await fetch('https://api.example.com/data');
const data = await response.json();
```

## Plugin: Opener (`tauri-plugin-opener`)

Opens URLs in the default browser, files in their default app.

**Frontend usage:**
```typescript
import { open } from '@tauri-apps/plugin-opener';

await open('https://example.com');
```

## Plugin: Store (`tauri-plugin-store`)

Persistent key-value storage backed by a JSON file.

**Frontend usage:**
```typescript
import { Store } from '@tauri-apps/plugin-store';

const store = await Store.load('settings.dat');
await store.set('theme', 'dark');
const theme = await store.get('theme');
```

## Other Common Plugins

### Dialog (`tauri-plugin-dialog`)

```bash
# Install
cargo add tauri-plugin-dialog
```

```rust
// Register
.plugin(tauri_plugin_dialog::init())
```

```typescript
import { open, message, confirm } from '@tauri-apps/plugin-dialog';

const file = await open({ multiple: false });
await message('Operation complete!', { title: 'Success' });
```

### Filesystem (`tauri-plugin-fs`)

```bash
cargo add tauri-plugin-fs
```

```typescript
import { readTextFile, writeTextFile, BaseDirectory } from '@tauri-apps/plugin-fs';

await writeTextFile('config.json', '{"key":"value"}', {
  baseDir: BaseDirectory.AppConfig
});
```

- **[Official]** Filesystem permissions must be explicitly granted in capabilities
- **[Official]** Use `BaseDirectory` constants for cross-platform paths
