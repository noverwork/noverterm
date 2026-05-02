# Commands & Bindings

## Command Definition

Commands exposed to Svelte use both Tauri and Specta attributes.

```rust
#[tauri::command]
#[specta::specta]
async fn read_pdf_file(pdf_path: String) -> Result<Vec<u8>, String> {
    let path = std::path::PathBuf::from(&pdf_path);
    if !path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
    {
        return Err("only PDF files can be read".to_string());
    }

    tokio::fs::read(path).await.map_err(|e| e.to_string())
}
```

## Return Types

Use serializable return values. For app-facing errors, prefer `Result<T, String>` because the generated TypeScript result is easy to handle.

```rust
#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScannedCompany {
    pub tax_id: String,
    pub company_name: String,
    pub group_name: String,
    pub pdf_paths: Vec<String>,
}
```

## Registration

Every frontend-callable command must be registered exactly once in `command_builder()`.

```rust
fn command_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        scan_companies,
        read_pdf_file,
        get_setting,
        set_setting,
    ])
}
```

Tauri uses one invoke handler. With `tauri-specta`, pass `command_builder().invoke_handler()` to `.invoke_handler(...)`.

## Exporting Bindings

After command signatures or Specta types change, run the desktop export binary.

```bash
cargo run --bin export-types
```

The generated file is `packages/desktop/ui/src/bindings.ts`.

## Frontend Usage

Use generated `commands.*` wrappers instead of raw `invoke(...)`.

```ts
import { commands } from "../../bindings";

const result = await commands.readPdfFile(pdfPath);
if (result.status === "error") {
  throw new Error(result.error);
}

const bytes = new Uint8Array(result.data);
```

## Raw Invoke

Raw `invoke` is acceptable only for experiments, newly introduced APIs before bindings are generated, or code outside the typed app boundary. Replace it with generated commands before finishing.
