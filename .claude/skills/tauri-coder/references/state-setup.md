# State & Setup

## Setup Hook

Use `.setup(...)` for initialization that needs an `App` handle, app paths, managed state, or one-time startup work.

Project pattern:

```rust
tauri::Builder::default()
    .setup(|app| -> Result<(), Box<dyn std::error::Error>> {
        let app_data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;

        let settings_path = app_data_dir.join("settings.json");
        let settings = SettingsManager::new(settings_path);
        app.manage(settings);

        Ok(())
    })
```

## Managed State

Register shared state with `app.manage(value)` and access it in commands with `tauri::State<'_, T>`.

```rust
#[tauri::command]
#[specta::specta]
fn get_setting(key: String, settings: tauri::State<'_, SettingsManager>) -> Option<Setting> {
    settings.get(&key)
}
```

## Mutable State

Use synchronization primitives for mutable shared state.

```rust
struct AppCache {
    values: std::sync::Mutex<Vec<String>>,
}
```

Avoid holding locks across `.await`. Clone or extract needed values before awaiting.

## App Paths

Use Tauri's path resolver instead of hard-coded platform paths.

```rust
let app_data_dir = app.path().app_data_dir()?;
```

Create directories before reading/writing files in them.

## Startup Work

Keep setup fast. If heavy work is required, spawn it with Tauri/Tokio runtime and report progress to the UI via events or channels.
