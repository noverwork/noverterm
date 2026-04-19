# Tauri Commands Reference

## Defining Commands

Every command in this project uses dual decorators:

```rust
#[tauri::command]
#[specta::specta]
pub async fn command_name(
    state: tauri::State<'_, SomeManager>,
    param: String,
) -> Result<ReturnType, String> {
    // ...
}
```

### Rules

- **[Project]** `#[specta::specta]` MUST accompany `#[tauri::command]` — this enables TypeScript type generation
- **[Project]** Commands return `Result<T, String>` — errors are serialized as strings
- **[Project]** Use `tauri::State<'_, T>` with explicit lifetime for dependency injection
- **[Official]** Command names in `invoke()` match the Rust function name exactly (snake_case)
- **[Official]** Commands can accept `AppHandle`, `Window`, `Webview` as parameters for context access

### Async vs Sync

```rust
// Sync command (no I/O, no state access that's async)
#[tauri::command]
#[specta::specta]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// Async command (I/O, database, network)
#[tauri::command]
#[specta::specta]
pub async fn load_data(
    manager: tauri::State<'_, DataManager>,
) -> Result<Vec<Item>, String> {
    manager.fetch_all().await
}
```

### Registration via tauri-specta

```rust
use tauri_specta::{collect_commands, Builder};

fn command_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        greet,
        crate::module::some_command,
        crate::module::another_command,
    ])
}
```

- **[Project]** All commands go into `collect_commands![]` — never use `generate_handler![]` directly
- **[Project]** Cross-module commands use full path: `crate::module::command_name`
- **[Project]** The `command_builder()` function is reused for both `invoke_handler` and `export_types`

### Error Handling

```rust
// Simple string error
#[tauri::command]
async fn do_thing() -> Result<(), String> {
    something().await.map_err(|e| e.to_string())
}

// With context
#[tauri::command]
async fn do_thing() -> Result<Data, String> {
    fetch_data().await.map_err(|e| format!("failed to fetch: {e}"))
}
```

- **[Project]** Do NOT use `anyhow::Error` or `thiserror` in command signatures — they don't serialize cleanly to the frontend
- **[Common]** Use `format!()` for descriptive error messages
