# Tauri Events Reference

## Backend: Emit Events

```rust
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn start_download(app: AppHandle, url: String) {
    app.emit("download-started", &url).unwrap();

    for progress in [10, 50, 100] {
        app.emit("download-progress", progress).unwrap();
    }

    app.emit("download-complete", &url).unwrap();
}
```

- **[Official]** `emit()` is fire-and-forget — no return value, no delivery confirmation
- **[Official]** Payloads must be JSON-serializable (impl `serde::Serialize`)
- **[Official]** `emit()` delivers to ALL listeners across ALL windows
- **[Common]** Use `AppHandle` as a command parameter to access `emit()`

### Targeted Events

```rust
// Emit to a specific window
use tauri::Manager;

#[tauri::command]
fn notify_specific(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        window.emit("private-event", "data").unwrap();
    }
}
```

## Frontend: Listen to Events

```typescript
import { listen } from '@tauri-apps/api/event';

// Typed event listener
type ProgressPayload = {
  downloadId: number;
  percent: number;
};

const unlisten = await listen<ProgressPayload>('download-progress', (event) => {
  console.log(`Download ${event.payload.downloadId}: ${event.payload.percent}%`);
});

// Cleanup when component unmounts (Svelte)
onDestroy(() => {
  unlisten();
});
```

- **[Official]** `listen()` returns an unlisten function — ALWAYS call it to prevent memory leaks
- **[Official]** Event listeners are asynchronous
- **[Common]** In Svelte components, use `onDestroy` lifecycle hook for cleanup

## Frontend: Emit Events to Backend

```typescript
import { emit } from '@tauri-apps/api/event';

await emit('user-action', { action: 'start', id: 123 });
```

## Backend: Listen to Frontend Events

```rust
use tauri::Listener;

tauri::Builder::default()
    .setup(|app| {
        app.listen("user-action", |event| {
            if let Ok(payload) = serde_json::from_str::<UserAction>(&event.payload()) {
                println!("Action: {:?}", payload.action);
            }
        });
        Ok(())
    })
```

- **[Official]** Backend listeners are registered in `setup()` or on `AppHandle`
- **[Official]** Event payload arrives as a JSON string — deserialize with `serde_json::from_str`

## Event Naming

- **[Official]** Use `kebab-case` for event names: `download-started`, `session-closed`
- **[Common]** Prefix events with domain: `ssh-connected`, `ssh-disconnected`, `settings-changed`
