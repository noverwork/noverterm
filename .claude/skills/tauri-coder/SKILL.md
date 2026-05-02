---
name: tauri-coder
description: Tauri 2 desktop coding guide for this monorepo. Use when adding or changing Tauri commands, invoke handlers, tauri-specta bindings, capabilities/permissions, plugins, app state, events, tauri.conf.json, desktop filesystem/dialog/http access, or Svelte-to-Rust IPC.
---

# Tauri Coder - Convention Guide

Tauri conventions for this monorepo.

- **Official** = Tauri 2 docs / tauri.app examples / docs.rs API
- **Project** = local rules for `packages/desktop`
- **Common** = widespread Tauri practice, not a strict official requirement

## Forbidden Patterns

```rust
// ❌ NEVER expose a command without registering it in command_builder() (Project)
#[tauri::command]
async fn new_command() -> Result<String, String> { Ok("ok".to_string()) }

// ❌ NEVER skip specta when command is used from Svelte (Project)
#[tauri::command]
fn get_value() -> String { "value".to_string() }

// ❌ NEVER read arbitrary file types when command scope should be narrow (Project)
tokio::fs::read(user_path).await?;

// ❌ NEVER use frontend invoke strings directly when generated bindings exist (Project)
invoke("scan_companies", { basePath, periodFolder });

// ❌ NEVER add plugin use without checking capabilities/permissions (Official/Project)
tauri::Builder::default().plugin(tauri_plugin_fs::init());
```

## Quick Reference

### Project Stack

| Layer | Technology | Location |
| --- | --- | --- |
| Desktop shell | Tauri 2 | `packages/desktop` |
| Rust commands | `#[tauri::command]` + `#[specta::specta]` | `packages/desktop/src/lib.rs` |
| Type bindings | `tauri-specta` + `specta-typescript` | `packages/desktop/ui/src/bindings.ts` |
| Frontend | SvelteKit/Svelte 5 | `packages/desktop/ui` |
| Permissions | Tauri 2 capabilities | `packages/desktop/capabilities/default.json` |

### Key Rules

1. **[Official]** Tauri 2 commands use `#[tauri::command]` and are registered with an invoke handler.
2. **[Project]** Svelte-callable commands also need `#[specta::specta]` and registration in `command_builder()`.
3. **[Project]** Regenerate `ui/src/bindings.ts` after changing command signatures or Specta types.
4. **[Project]** Use generated `commands.*` from `ui/src/bindings.ts`; avoid raw `invoke(...)` in app code.
5. **[Official]** Register shared state with `.manage(...)`; access it via `tauri::State<'_, T>`.
6. **[Official]** Tauri 2 plugins and IPC features are gated by capabilities/permissions.
7. **[Project]** Update `capabilities/default.json` when adding frontend plugin APIs or custom permissions.
8. **[Project]** Keep command errors serializable and user-readable, usually `Result<T, String>`.
9. **[Project]** Use Tokio async I/O inside async commands; avoid blocking the UI thread.
10. **[Project]** Validate filesystem paths and file extensions before reading user-selected files.

### Command Pattern

```rust
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub file_paths: Vec<String>,
}

#[tauri::command]
#[specta::specta]
async fn scan_files(base_path: String) -> Result<ScanResult, String> {
    if base_path.trim().is_empty() {
        return Err("base_path is required".to_string());
    }

    Ok(ScanResult { file_paths: Vec::new() })
}
```

### Registration Pattern

```rust
fn command_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        scan_files,
        get_setting,
        set_setting,
    ])
}
```

### App Setup Pattern

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| -> Result<(), Box<dyn std::error::Error>> {
        let app_data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;
        app.manage(SettingsManager::new(app_data_dir.join("settings.json")));
        Ok(())
    })
    .invoke_handler(command_builder().invoke_handler())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

### Frontend Pattern

```ts
import { commands } from "../../bindings";

const result = await commands.scanCompanies(basePath, periodFolder);
if (result.status === "error") {
  throw new Error(result.error);
}
```

## Detailed References

- **Commands & bindings** (Tauri commands, tauri-specta, generated TS usage) → `references/commands-bindings.md`
- **Capabilities & plugins** (Tauri 2 permissions, default capability, plugin APIs) → `references/capabilities-plugins.md`
- **State & setup** (`manage`, `State`, setup hook, app data paths) → `references/state-setup.md`
- **Filesystem & security** (path validation, dialog-selected files, CSP, scoped access) → `references/filesystem-security.md`
- **Events & IPC** (frontend/Rust events, when to use commands vs events) → `references/events-ipc.md`
- **Testing & verification** (bindings export, cargo check, svelte-check, dev/build) → `references/testing-verification.md`
- **Anti-patterns** (unregistered commands, raw invoke, missing permissions, unsafe file reads) → `references/anti-patterns.md`

## Official Sources

- Tauri 2 docs: https://v2.tauri.app/
- Calling Rust from frontend: https://v2.tauri.app/develop/calling-rust/
- Security permissions: https://v2.tauri.app/learn/security/
- Capabilities config reference: https://v2.tauri.app/reference/config/#capability
- Plugin permissions: https://v2.tauri.app/learn/security/using-plugin-permissions/
- Tauri Rust docs: https://docs.rs/tauri/2/
