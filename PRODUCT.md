# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

Desktop app for macOS (Apple Silicon) and Windows, rendered in the Tauri 2 webview. The design language is the app's own, not native AppKit/WinUI. Linux support is in progress.

## Users

Backend and DevOps engineers who work across many servers every day. They keep several SSH sessions open in tabs for hours, jump between hosts, move files, and tunnel ports while debugging or operating infrastructure.

## Product Purpose

Noverterm is an SSH terminal client with multi-session tabs, SFTP file transfer, port forwarding, SSH key management, and snippets. Success means an engineer can run their whole day of remote work from one fast window without the client itself costing noticeable memory or attention.

## Positioning

Native and lightweight: built on Tauri 2 and Rust instead of Electron, around 50MB of memory versus 500MB+ for Electron-based SSH clients. Speed and low overhead are the core claim; the UI should never undermine it.

## Operating Context

- Long-lived sessions: many tabs open at once, dropped connections reconnect when the user switches back to the tab.
- Keyboard-heavy use: TUI apps (vim, tmux, htop) with the Kitty keyboard protocol, IME input, and paste handling, including pasting clipboard images into SSH sessions.
- Side tasks next to the terminal: dual-pane SFTP (local or any host per pane), port-forward presets, snippets run against active sessions or pinned to the sidebar.
- All data lives in a local SQLite database in the app data folder; no cloud account or sync.

## Capabilities and Constraints

- Features shipped: SSH (password and key auth, host fingerprint verification, known hosts manager), multi-session tabs with duplicate and local terminal, port forwarding, SFTP, snippets, SSH key manager, connection groups, auto-update from GitHub Releases.
- Stack: Svelte 5, Vite, TailwindCSS in the UI; Rust, Russh, Tokio, Diesel/SQLite behind Tauri commands.
- UI copy is English only.
- Source available under BSL 1.1; pull requests are not accepted.

## Brand Commitments

- Name: Noverterm (by noverwork).
- Voice: direct, slightly dry developer humor ("Because your SSH client shouldn't need its own memory allocator.").
- Existing assets: app icons in `packages/desktop/icons/`, website favicon and OG image in `docs/`.

## Evidence on Hand

- Memory claim (~50MB vs 500MB+) stated in the README; no published benchmark yet.
- No screenshots, testimonials, or user counts exist. Do not invent them.

## Product Principles

1. The terminal is the product. Chrome around it stays out of the way.
2. Never feel heavier than the claim. Interactions must be instant; no decorative motion or weight that contradicts "lightweight".
3. Keyboard first. Every frequent action is reachable without the mouse.
4. Long sessions are the norm. Design for hours of use, many tabs, and recovering from dropped connections.
5. Local and trustworthy. Make host trust, keys, and connection state legible.
