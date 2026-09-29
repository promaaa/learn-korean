# Changelog

All notable changes are documented here. Versions follow [Semantic Versioning](https://semver.org/);
before 1.0, a minor bump is a new feature and a patch bump is a fix.

## [Unreleased]

## [0.1.0]

### Added
- Repository bootstrap: Cargo workspace, Tauri 2 client with Svelte 5 + TypeScript + Vite, pnpm workspace.
- Tooling: rustfmt, clippy, svelte-check, ESLint, Vitest.
- CI (frontend checks, Rust fmt/clippy/test, production build, Conventional Commit PR titles).
- Release workflow building Linux and macOS bundles with `tauri-action`.
- Architecture, product and contribution docs; ADR-001 to ADR-005.
