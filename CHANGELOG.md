# Changelog

All notable changes are documented here. Versions follow [Semantic Versioning](https://semver.org/);
before 1.0, a minor bump is a new feature and a patch bump is a fix.

## [Unreleased]

### Added
- Overlay application shell: undecorated always-on-top window, hidden with `Esc`, shown with Super+Z.
- `--toggle`, `--show`, `--hidden` command-line flags forwarded to the running instance (single instance).
- Global Super+Z shortcut on macOS/X11; Hyprland bind, window rule and autostart via `scripts/install-omarchy.sh`.
- Shared keyboard vocabulary (`1-9`, `Space`, `Enter`, `H J K L`, `R`, `Esc`) for every screen.
- Frontend errors forwarded to the Rust log.

## [0.1.0]

### Added
- Repository bootstrap: Cargo workspace, Tauri 2 client with Svelte 5 + TypeScript + Vite, pnpm workspace.
- Tooling: rustfmt, clippy, svelte-check, ESLint, Vitest.
- CI (frontend checks, Rust fmt/clippy/test, production build, Conventional Commit PR titles).
- Release workflow building Linux and macOS bundles with `tauri-action`.
- Architecture, product and contribution docs; ADR-001 to ADR-005.
