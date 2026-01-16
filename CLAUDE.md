# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Noverterm is a Tauri desktop application combining a React TypeScript frontend with a Rust backend. This hybrid architecture enables native desktop functionality with web technologies.

## Development Commands

### Running the App
- `npm run tauri dev` - Start development mode (runs Vite dev server on port 1420 and launches Tauri app)
- `npm run dev` - Start Vite dev server only (frontend-only development)

### Building
- `npm run tauri build` - Build production binaries for the target platform
- `npm run build` - Build frontend only (outputs to `dist/`)
- `npm run preview` - Preview the production build locally

### IDE Setup
Recommended VS Code extensions:
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Architecture

### Frontend-Backend Communication
The app uses Tauri's invoke system for JavaScript ↔ Rust communication:

1. Frontend calls Rust commands via `invoke()` from `@tauri-apps/api/core`
2. Commands are registered in `src-tauri/src/lib.rs` using `#[tauri::command]` macro
3. The `invoke_handler` macro exposes commands to the frontend
4. Results return as Promises

**Example pattern from `src-tauri/src/lib.rs`:**
```rust
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// In run():
.invoke_handler(tauri::generate_handler![greet])
```

### Project Structure
- `src/` - React frontend source code
- `src-tauri/src/` - Rust backend code
- `src-tauri/tauri.conf.json` - Tauri configuration (window settings, build commands, app metadata)
- `public/` - Static assets served directly

### Key Configuration Files
- `tauri.conf.json` - App identifier (`com.noverwork.noverterm`), window size (800x600), build commands, dev URL
- `vite.config.ts` - Vite bundler configuration
- `tsconfig.json` - TypeScript compiler options
- `src-tauri/Cargo.toml` - Rust dependencies

## Adding New Tauri Commands

1. Define the function in `src-tauri/src/lib.rs` with `#[tauri::command]`
2. Add the command to `generate_handler![]` macro
3. Import and use `invoke()` in frontend components
4. Rebuild - `tauri dev` supports hot reload for Rust changes

## Notes

- CSP (Content Security Policy) is currently disabled in `tauri.conf.json` for development
- The app uses Tauri v2 with the `tauri-plugin-opener` plugin for URL/file operations
- No testing framework is currently configured
- No linting/formatting tools are set up
