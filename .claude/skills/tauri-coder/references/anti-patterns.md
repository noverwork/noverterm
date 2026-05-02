# Anti-Patterns

Each item is labeled as **Official**, **Project**, or **Common**.

## 1. Command Missing From Builder (Project)

```rust
// ❌ ANTI-PATTERN
#[tauri::command]
#[specta::specta]
fn new_command() -> String { "ok".to_string() }

// command_builder() does not include new_command
```

```rust
// ✅ CORRECT
fn command_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        new_command,
    ])
}
```

## 2. Missing Specta Annotation (Project)

```rust
// ❌ ANTI-PATTERN for frontend-callable command
#[tauri::command]
fn get_value() -> String { "value".to_string() }

// ✅ CORRECT
#[tauri::command]
#[specta::specta]
fn get_value() -> String { "value".to_string() }
```

## 3. Raw Invoke Drift (Project)

```ts
// ❌ ANTI-PATTERN in app code
await invoke("scan_companies", { basePath, periodFolder });

// ✅ CORRECT
await commands.scanCompanies(basePath, periodFolder);
```

## 4. Missing Capability Permission (Official + Project)

```rust
// ❌ ANTI-PATTERN
tauri::Builder::default().plugin(tauri_plugin_fs::init());
```

If the frontend uses the plugin, add the needed permission to `capabilities/default.json`.

## 5. Trusting Frontend Paths (Project)

```rust
// ❌ ANTI-PATTERN
tokio::fs::read(pdf_path).await.map_err(|e| e.to_string())
```

Validate path intent, extension, and domain constraints before reading.

## 6. Blocking Work in Commands (Common + Project)

```rust
// ❌ ANTI-PATTERN for heavy work
#[tauri::command]
fn scan_large_tree(path: String) -> Result<Vec<String>, String> {
    std::fs::read_dir(path).map_err(|e| e.to_string())?;
    Ok(Vec::new())
}
```

Prefer async I/O or explicit blocking-task handling for expensive filesystem/network work.

## 7. Forgetting to Regenerate Bindings (Project)

After changing command signatures or Specta types, always run `cargo run --bin export-types`.
