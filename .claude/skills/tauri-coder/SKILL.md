---
name: tauri-coder
description: Tauri v2 development guide for this monorepo. Use when writing Tauri commands, events, capabilities, plugin integration, or frontend-backend communication. Combines Tauri official docs with clearly marked project-specific patterns for the desktop package.
---

# Tauri Coder - Convention Guide

Tauri v2 patterns for this monorepo.

- **Official** = Tauri v2 documentation / official API
- **Project** = local rules for this repo's desktop package
- **Common** = widespread Tauri practice, not a strict official requirement

## Forbidden Patterns

```rust
// ❌ NEVER (Project)
fn my_command() -> String { ... }
// Commands MUST be async if they do any I/O or state access

// ❌ NEVER (Project)
app.manage(MyState::new()).invoke_handler(generate_handler![cmd]);
// MUST use tauri-specta Builder for command registration + type export

// ❌ NEVER (Project)
#[tauri::command]
fn cmd(state: State<MyState>) { ... }
// MUST use tauri::State<'_, MyState> with explicit lifetime
```

## Quick Reference

### Naming

| Type | Convention | Example | Source |
|---|---|---|---|
| Command functions | `snake_case` with module prefix | `bootstrap_restore`, `ssh_connect_direct` | Project |
| Event names | `kebab-case` | `download-started`, `session-closed` | Official |
| Capability files | `kebab-case.json` | `default.json`, `ssh-access.json` | Official |
| State structs | `PascalCase` + `Manager` suffix | `SettingsManager`, `SshSessionManager` | Project |

### Key Rules

1. **[Project]** All commands MUST use `#[tauri::command]` + `#[specta::specta]` dual decorators
2. **[Project]** Commands MUST be registered via `tauri-specta` `Builder` with `collect_commands![]` macro
3. **[Project]** Command return type MUST be `Result<T, String>` — errors as `String`, never custom error types that don't impl `Serialize`
4. **[Project]** State injection uses `tauri::State<'_, ManagerType>` with explicit lifetime
5. **[Project]** State is registered in `Builder::setup()` via `app.manage(instance)`
6. **[Official]** Plugin initialization uses `tauri::Builder::plugin(tauri_plugin_xxx::init())`
7. **[Official]** Capabilities defined in `capabilities/*.json` control permission scope per window
8. **[Official]** Events use `app.emit()` (backend) and `listen()`/`emit()` (frontend) from `@tauri-apps/api/event`
9. **[Official]** Commands invoked from frontend via `invoke('command_name', { args })` from `@tauri-apps/api/core`
10. **[Common]** Use `AppHandle` parameter in commands when you need to emit events or access app-level APIs
11. **[Project]** Frontend uses generated TypeScript bindings from `tauri-specta` export — never manually type `invoke` calls
12. **[Official]** `tauri.conf.json` `"build"` section must have `frontendDist` pointing to the built output directory

### Command Pattern

```rust
#[tauri::command]
#[specta::specta]
pub async fn bootstrap_restore(
    auth_manager: tauri::State<'_, DesktopAuthManager>,
) -> Result<Option<crate::auth::AuthBootstrapStatus>, String> {
    auth_manager.restore().await
}
```

### Command Registration (tauri-specta)

```rust
fn command_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        greet,
        crate::connect::ssh_connect_direct,
        crate::connect::ssh_write,
    ])
}

// In run():
let specta_builder = command_builder();
tauri::Builder::default()
    .invoke_handler(specta_builder.invoke_handler())
    .run(tauri::generate_context!())
```

### State Management

```rust
// In setup():
.setup(|app| {
    let manager = SettingsManager::new(settings_path);
    app.manage(manager);
    Ok(())
})

// In command:
#[tauri::command]
async fn get_settings(
    settings: tauri::State<'_, SettingsManager>,
) -> Result<Setting, String> {
    settings.load().await
}
```

### Event Emission (Backend)

```rust
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn notify_progress(app: AppHandle, session_id: String, percent: u8) {
    app.emit("session-progress", (session_id, percent)).unwrap();
}
```

### Event Listening (Frontend - TypeScript)

```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen('session-progress', (event) => {
  const [sessionId, percent] = event.payload as [string, number];
  console.log(`Session ${sessionId}: ${percent}%`);
});

// Cleanup:
unlisten();
```

### Plugin Usage

```rust
// In run() - register plugins:
tauri::Builder::default()
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_store::Builder::default().build())
```

### Capability Configuration

```json
{
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "store:default",
    {
      "identifier": "http:default",
      "allow": [{ "url": "http://*" }, { "url": "https://*" }]
    }
  ]
}
```

## Project Structure

```
packages/desktop/
├── src/
│   ├── lib.rs              # Module declarations + feature boundaries
│   ├── main.rs             # Binary entrypoint → desktop_lib::run()
│   ├── bootstrap/mod.rs    # Commands + run() + specta setup
│   ├── auth/               # Auth module (commands + managers)
│   ├── connect/            # SSH/Local connection commands
│   ├── runtime/            # Session runtime (ssh/, local/)
│   ├── settings/           # Settings manager
│   └── trust/              # SSH host trust store
├── capabilities/
│   └── default.json        # Window permission capabilities
├── tauri.conf.json         # Tauri app config (v2 schema)
├── Cargo.toml              # Rust dependencies
└── ui/                     # SvelteKit frontend
```

## Detailed References

- **Commands** (definition, registration, specta integration, error handling) → `references/commands.md`
- **Events** (emit, listen, payload types, cleanup) → `references/events.md`
- **State Management** (app.manage, State injection, singleton patterns) → `references/state.md`
- **Plugins** (http, opener, store, dialog, fs) → `references/plugins.md`
- **Capabilities** (permission model, window scoping, custom permissions) → `references/capabilities.md`
- **Frontend Integration** (invoke, bindings, SvelteKit adapter, vite config) → `references/frontend.md`
- **Anti-patterns** (official + project anti-patterns, clearly labeled) → `references/anti-patterns.md`
