<div align="center">

# Noverterm

**A native SSH client for macOS and Windows. Built with Tauri 2 and Rust. No Electron.**

[![Release](https://img.shields.io/github/v/release/noverwork/noverterm)](https://github.com/noverwork/noverterm/releases)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-blue)]()
[![License](https://img.shields.io/badge/License-MIT-5f3dc4)](LICENSE)

[Download](https://github.com/noverwork/noverterm/releases) · [Website](https://noverwork.github.io/noverterm/) · [Report a bug](https://github.com/noverwork/noverterm/issues)

</div>

> _Screenshots coming soon._

Noverterm puts SSH sessions, SFTP, port forwarding, keys, and snippets in one fast window. It uses about 50MB of memory, compared to 500MB+ for Electron-based SSH clients. All data stays in a local SQLite database. No account, no cloud sync.

## Highlights

- **Lightweight** — Tauri 2 + Rust, ~50MB memory footprint
- **Tabs for everything** — many SSH sessions, plus a local terminal, side by side
- **TUI-ready** — Kitty keyboard protocol, IME input, and paste handling for vim, tmux, and htop
- **Dual-pane SFTP** — local or any saved host on each side, drag and drop between them
- **Local-first** — connections, keys, and snippets never leave your machine

## Features

### Terminal

- Multiple SSH sessions in tabs; duplicate a session or open a local terminal instantly
- Password and SSH key authentication
- Host fingerprint verification and a known hosts manager
- Dropped connections reconnect when you switch back to the tab
- Immediate output on idle sessions, bounded binary streaming, ordered input with visible write failures
- Negotiated Kitty keyboard protocol with modifier, repeat, and release events
- In-terminal search

### SFTP file transfer

- Dual-pane browser: local or any host in each pane
- Drag and drop, multi-select, recursive folder transfer
- Transfer progress and conflict resolution
- Create, rename, and delete files and folders

### Port forwarding

- Local SSH tunnels saved as presets
- Several ports per preset

### Snippets

- Save frequent commands and run them in active sessions
- Pin snippets to the sidebar as one-click launchers with a custom icon

### Connections and keys

- Organize connections into folders
- Reach private servers through jump hosts (like `ssh -J`), chained up to five hops; terminal, SFTP, and port forwarding all tunnel through them
- Import and manage SSH keys

### App

- Built-in updater from GitHub Releases
- Dark theme, keyboard shortcuts, context menus

## Download

| Platform | Package |
|----------|---------|
| macOS (Apple Silicon) | `.dmg` |
| Windows | `.exe` installer |

Get the latest version from [Releases](https://github.com/noverwork/noverterm/releases). Noverterm checks for updates on its own after install.

## Quick Start

1. Open Noverterm and click **New connection**.
2. Enter the host, port, and user. Choose password or SSH key authentication.
3. Connect. On first connect, verify the host fingerprint.
4. Open more sessions in tabs, or open a **Local Terminal**.
5. Use the sidebar for SFTP, port forwards, SSH keys, and snippets.

### Connect through a jump host

1. Save the bastion as a normal connection.
2. Create a connection for the private server. Enter its address as the bastion sees it, such as `10.0.1.5`.
3. Under **Jump host**, select the bastion and save.

Connecting asks you to verify the bastion's fingerprint first, then the target's. Each host signs in with its own saved credentials, and your private keys stay on your machine. To chain more hops, give the bastion a jump host of its own. The bastion's SSH server must allow TCP forwarding (`AllowTcpForwarding yes`, the OpenSSH default).

## Keyboard Shortcuts

On Windows, use `Ctrl` in place of `Cmd`.

| Action | Shortcut |
|--------|----------|
| Jump to tab 1–9 | `Cmd+1` … `Cmd+9` |
| Next / previous tab | `Cmd+Shift+]` / `Cmd+Shift+[`, `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| Search in terminal | `Cmd+F` |
| Next / previous match (macOS) | `Cmd+G` / `Cmd+Shift+G` |
| Copy selection | `Cmd+C` (only with a selection, else sent to the shell) |
| Paste | `Cmd+V`, `Ctrl+Shift+V`, `Shift+Insert` |
| Close tab (macOS) | `Cmd+W` |

Keyboard encoding uses the layout information from the WebView. On WebKit, shifted punctuation on non-US layouts may lack its unshifted key value the first time it is typed. The browser-reported key and physical alternate stay available.

## Data and Privacy

- All data is stored in a local SQLite database in the app data folder.
- No account, telemetry, or cloud sync.
- The only network traffic besides your SSH connections is the update check against GitHub Releases.

## Roadmap

- [ ] Linux support
- [ ] Screenshots

## Build from Source

Prerequisites: Node.js 22+, Rust stable, [cargo-make](https://github.com/sagiegurari/cargo-make).

```bash
cargo make frontend:install
cargo make tauri:build
```

The bundled app is in `target/release/bundle/`.

### Development

```bash
cargo make frontend:install   # install UI dependencies
cargo make tauri:dev          # run the app in dev mode
cargo make ci                 # clippy, format check, tests, Svelte check
```

### Tech stack

| Layer | Technology |
|-------|------------|
| UI | Svelte 5, Vite, TailwindCSS |
| Desktop | Tauri 2 |
| SSH | Russh (pure Rust) |
| Async runtime | Tokio |
| Database | SQLite + Diesel |

### Project structure

```
noverterm/
├── packages/
│   ├── desktop/        # Tauri app (Rust + Svelte UI), owns the SQLite database
│   ├── shared/         # Types shared between Rust commands and the UI
│   └── orm/            # Diesel models and schema
├── Cargo.toml          # Rust workspace
└── Makefile.toml       # cargo-make tasks
```

## Contributing

Contributions are welcome. Fork it, hack on it, and send a pull request.

- **Bug reports and ideas:** open an [issue](https://github.com/noverwork/noverterm/issues).
- **Pull requests:** welcome. Run `cargo make ci` before you open one.
- **Security issues:** [report privately](https://github.com/noverwork/noverterm/security/advisories/new). Do not open a public issue.

## License

Noverterm is released under the [MIT License](LICENSE).

## Acknowledgements

- [Tauri](https://tauri.app/) — desktop framework
- [Russh](https://github.com/Eugeny/russh) — pure Rust SSH implementation
- [Svelte](https://svelte.dev/) — UI framework

---

<div align="center">

**Noverterm** — Because your SSH client shouldn't need its own memory allocator.

</div>
