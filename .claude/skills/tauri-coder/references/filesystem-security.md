# Filesystem & Security

## Path Validation

Commands that read files must validate the path and intended file type before reading.

```rust
let path = std::path::PathBuf::from(&pdf_path);
if !path
    .extension()
    .and_then(|ext| ext.to_str())
    .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
{
    return Err("only PDF files can be read".to_string());
}
```

## Dialog-Selected Files

Frontend dialog selection is not a substitute for backend validation. Treat all paths received by commands as untrusted user input.

```ts
import { open } from "@tauri-apps/plugin-dialog";

const path = await open({ directory: true, multiple: false });
```

## App Data

Prefer app data/config directories for app-owned files.

```rust
let app_data_dir = app.path().app_data_dir()?;
std::fs::create_dir_all(&app_data_dir)?;
```

## Capabilities

Tauri plugin APIs are controlled by `capabilities/default.json`. When adding filesystem APIs, prefer scoped permissions rather than broad defaults.

```json
{
  "identifier": "fs:allow-read-file",
  "allow": [{ "path": "$DOCUMENT/**" }]
}
```

## CSP

The project currently has `"csp": null` in `tauri.conf.json`. If remote content or broader network access is added, revisit CSP and avoid leaving it unrestricted without a specific reason.

## Backend Commands vs Frontend Plugin FS

Prefer backend commands for domain-specific file reads, especially when validation is required. Use frontend plugin FS only when direct UI-side file operations are genuinely simpler and properly scoped.
