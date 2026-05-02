# Capabilities & Plugins

## Tauri 2 Security Model

Tauri 2 uses a default-deny capability and permission model. Frontend access to core/plugin APIs must be granted through capability files.

Project capability file:

```text
packages/desktop/capabilities/default.json
```

Current default capability grants:

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default",
    "http:default"
  ]
}
```

## Plugin Checklist

When adding a plugin:

1. Add Rust crate in `packages/desktop/Cargo.toml`.
2. Add frontend package in `packages/desktop/ui/package.json` if used from Svelte.
3. Register `.plugin(plugin::init())` in `packages/desktop/src/lib.rs`.
4. Grant required permissions in `capabilities/default.json`.
5. Verify by running the app, not only compiling.

## Builder Plugin Pattern

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_http::init())
    .invoke_handler(command_builder().invoke_handler())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

## Permission Entries

Permission strings for plugins usually include the plugin prefix.

```json
"dialog:default"
```

Scoped permissions use objects.

```json
{
  "identifier": "fs:allow-read-file",
  "allow": [{ "path": "$DOCUMENT/**" }]
}
```

## Custom Commands

Custom app commands are also subject to Tauri's IPC rules. If command permissions are introduced later, keep command identifiers and capability names stable.

## Avoid Overbroad Permissions

Prefer narrow path or URL scopes over broad default permissions when adding filesystem, shell, clipboard, or external network features.
