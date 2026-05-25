# Noverterm

> A modern, lightweight SSH terminal client built with Tauri & Rust.

[![License](https://img.shields.io/badge/License-BSL_1.1-5f3dc4)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-blue)]()
[![Tech](https://img.shields.io/badge/Tech-Tauri%202%20%2B%20Rust-orange)]()

## Why Noverterm?

Most modern terminal clients are wrapped in Electron, consuming hundreds of MBs of RAM for a simple SSH session. 

Noverterm is built on **Tauri 2** and **Rust**, delivering a native-grade experience with a fraction of the memory footprint. It combines a clean, developer-focused UI with robust SSH management, port forwarding, and cloud-ready architecture.

## Features

- 🔐 **Secure SSH Management** – Password & SSH key authentication, host fingerprint verification, and known hosts manager.
- 🖥️ **Multi-Session Tabs** – Organize connections with tabs, duplicate sessions, and instant local terminals.
- 🔄 **Port Forwarding** – Native SSH tunneling with saved presets.
- 📝 **Snippets** – Save and execute frequent commands across active sessions.
- ☁️ **Cloud Sync (Roadmap)** – Securely sync connections, keys, and settings across devices.
- 🎨 **Modern UI** – Dark-themed, responsive interface with fine-grained terminal controls (search, copy-paste, keyboard shortcuts).
- ⚡ **Lightweight & Fast** – ~10x smaller memory footprint than Electron-based alternatives.

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Frontend** | Svelte 5, Vite, TailwindCSS |
| **Desktop Shell** | Tauri 2 |
| **Native Core** | Rust (Russh, Tokio, Axum) |
| **Backend API** | PostgreSQL + Diesel ORM |
| **Architecture** | Monorepo (`desktop` / `shared` / `backend`) |

## Installation

### macOS (Apple Silicon)
Download the latest `.dmg` from the [Releases](https://github.com/Noverwork/noverterm/releases) page.

### Build from Source
```bash
# Prerequisites: Node.js, Rust, cargo-make
cargo make frontend:install
cargo make tauri:build
```
The bundled app and `.dmg` will be generated in `target/aarch64-apple-darwin/release/bundle/`.

## License & Contribution Policy

Noverterm is **Source Available** under the **Business Source License 1.1 (BSL 1.1)**.

- ✅ **Free for personal & internal company use.**
- 🚫 **Not for commercial resale or managed hosting.**
- 📅 **Auto-converts to MIT License on 2029-05-25.**

⚠️ **This repository is public for transparency and security auditing. We do not accept external Pull Requests.** Please fork the repository if you wish to modify or distribute your own version.

For bug reports or security disclosures, please open an Issue or contact the maintainer directly.

---

Built with 🦀 and ⚡ by Noverwork
