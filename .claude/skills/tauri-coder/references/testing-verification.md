# Testing & Verification

## Rust Checks

Run checks in the desktop package after Rust command or Tauri setup changes.

```bash
cargo check
```

## Binding Export

After command signatures or Specta types change, regenerate TypeScript bindings.

```bash
cargo run --bin export-types
```

Inspect `packages/desktop/ui/src/bindings.ts` for expected camelCase names and result types.

## Frontend Checks

Run Svelte diagnostics after frontend IPC usage changes.

```bash
npx svelte-check
```

The package script is also available.

```bash
npm run check
```

## Tauri Runtime Smoke Test

`cargo check` does not verify capability permissions, plugin availability, webview IPC, or command invocation. Run the desktop app after changes that affect Tauri setup.

```bash
cargo run
```

or the repository's Tauri dev workflow if available.

## Search Checklist

Before finishing:

- Search for raw `invoke(` in app routes/components.
- Confirm new commands appear in `command_builder()`.
- Confirm new command types derive `specta::Type` when exported.
- Confirm required plugin permissions exist in `capabilities/default.json`.
- Confirm no generated binding contains `as any`; this repo sanitizes known generator output.

## Common Commands

```bash
cargo check
cargo run --bin export-types
npx svelte-check
npm run build
```
