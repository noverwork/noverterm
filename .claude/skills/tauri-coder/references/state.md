# Tauri State Management Reference

## Registering State

State is registered in the `setup()` closure via `app.manage()`:

```rust
.setup(|app| -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;

    // Create and manage state
    let settings = SettingsManager::new(app_data_dir.join("settings.json"));
    app.manage(settings);

    let auth = AuthManager::from_backend_url(url, token_path);
    app.manage(auth);

    Ok(())
})
```

- **[Project]** All managers are created in `setup()` in `bootstrap/mod.rs`
- **[Project]** State paths use `app.path().app_data_dir()` for cross-platform data storage
- **[Official]** Each type can only have ONE instance managed — use wrapper types if you need multiple

## Injecting State into Commands

```rust
#[tauri::command]
#[specta::specta]
pub async fn get_settings(
    settings: tauri::State<'_, SettingsManager>,
) -> Result<Setting, String> {
    settings.load().await
}
```

- **[Project]** Use `tauri::State<'_, ManagerType>` — the lifetime MUST be explicit
- **[Official]** State is accessed by type, not by name — only one instance per type
- **[Official]** State is `&T` (immutable borrow) — use `RwLock` or `Mutex` for interior mutability

## Manager Pattern

```rust
pub struct SettingsManager {
    path: PathBuf,
    cache: RwLock<Option<Setting>>,
}

impl SettingsManager {
    pub fn new(path: PathBuf) -> Self {
        Self { path, cache: RwLock::new(None) }
    }

    pub async fn load(&self) -> Result<Setting, String> {
        // ...
    }
}
```

- **[Project]** Managers encapsulate domain logic — commands delegate to managers
- **[Project]** Manager names end with `Manager` suffix: `SettingsManager`, `SshSessionManager`
- **[Common]** Use `RwLock` for read-heavy state, `Mutex` for write-heavy state

## Multiple State Dependencies

```rust
#[tauri::command]
#[specta::specta]
pub async fn complex_operation(
    settings: tauri::State<'_, SettingsManager>,
    auth: tauri::State<'_, DesktopAuthManager>,
    ssh: tauri::State<'_, SshSessionManager>,
) -> Result<Output, String> {
    // Access multiple managers in one command
}
```

- **[Official]** You can inject multiple state types into a single command
- **[Common]** If a command needs 4+ state dependencies, consider whether the command does too much
