# Noverterm

**A native SSH terminal client built with Tauri 2 & Rust. No Electron, no bloat.**

[![License](https://img.shields.io/badge/License-BSL_1.1-5f3dc4)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-blue)]()
[![Release](https://img.shields.io/github/v/release/noverwork/noverterm)](https://github.com/noverwork/noverterm/releases)

---

## Why Noverterm?

Most SSH clients today are Electron wrappers — hundreds of MBs of RAM just to run a terminal.

Noverterm is different. Built on **Tauri 2** and **Rust**, it delivers:

- **~50MB memory footprint** (vs 500MB+ for Electron apps)
- **Native performance** — no JavaScript runtime overhead
- **Real security** — Rust's memory safety, no CVE-ridden dependencies

## Features

### ✅ What's Done

| Feature | Description |
|---------|-------------|
| **SSH Connections** | Password & SSH key auth, host fingerprint verification, known hosts manager |
| **Multi-Session Tabs** | Multiple SSH sessions in tabs, duplicate sessions, instant local terminal |
| **Port Forwarding** | Local & remote SSH tunneling with saved presets |
| **SFTP File Transfer** | Drag & drop file browser, transfer progress, conflict resolution, create/rename/delete |
| **Snippets** | Save and execute frequent commands across active sessions |
| **SSH Key Manager** | Import and manage SSH keys |
| **Connection Groups** | Organize connections into folders |
| **Cloud Sync** | Sync connections, keys, and settings across devices (email auth, password reset) |
| **Auto-Update** | Built-in updater — download and install updates from GitHub Releases |
| **Modern UI** | Dark theme, responsive layout, keyboard shortcuts, context menus |

### 🚧 In Progress

- Linux support

## Screenshots

> _Coming soon_

## Installation

### macOS (Apple Silicon)

Download the latest `.dmg` from [Releases](https://github.com/noverwork/noverterm/releases).

### Windows

Download the `.exe` installer from [Releases](https://github.com/noverwork/noverterm/releases).

### Build from Source

```bash
# Prerequisites: Node.js 22+, Rust stable, cargo-make
cargo make frontend:install
cargo make tauri:build
```

The bundled app will be in `target/release/bundle/`.

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Frontend** | Svelte 5, Vite, TailwindCSS |
| **Desktop** | Tauri 2 |
| **SSH** | Russh (pure Rust SSH library) |
| **Async Runtime** | Tokio |
| **API** | Axum (for cloud sync backend) |
| **Database** | PostgreSQL + Diesel ORM |

## Project Structure

```
noverterm/
├── packages/
│   ├── desktop/        # Tauri app (Rust + Svelte UI)
│   ├── shared/         # Shared types between desktop & backend
│   ├── backend/        # Cloud sync API server (Axum)
│   ├── orm/            # Diesel ORM models
│   └── migrator/       # Database migrations
├── Cargo.toml          # Rust workspace
└── Makefile.toml       # cargo-make tasks
```

## Development

```bash
# Install dependencies
cargo make frontend:install

# Run in dev mode
cargo make tauri:dev

# Run lint & typecheck
cargo make lint          # Rust clippy
cargo make fmt           # Rust format
npm run lint             # Frontend ESLint (in packages/desktop/ui)
npm run check            # Svelte typecheck (in packages/desktop/ui)
```

## License

Noverterm is **Source Available** under the **Business Source License 1.1 (BSL 1.1)**.

- ✅ **Free for personal & internal company use**
- ✅ **Fork and modify for your own use**
- 🚫 **No commercial resale or managed hosting**
- 📅 **Auto-converts to MIT License on 2029-05-25**

See [LICENSE](LICENSE) for details.

## Contributing

This repository is public for **transparency and security auditing**.

- 🐛 **Bug reports** → Open an Issue
- 🔒 **Security disclosures** → Contact maintainer directly
- 🔀 **Pull Requests** → Not accepted (fork freely instead)

## Acknowledgements

Built with:
- 🦀 [Tauri](https://tauri.app/) — Desktop framework
- ⚡ [Russh](https://github.com/Russh/russh) — Pure Rust SSH implementation
- 🎨 [Svelte](https://svelte.dev/) — UI framework

---

**Noverterm** — Because your SSH client shouldn't need its own memory allocator.
