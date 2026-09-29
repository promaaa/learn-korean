# learn-korean

Keyboard-first Korean learning app. Press **Super + Z**, play a short session, press **Esc**.

Tauri 2 · Rust · Svelte 5 · TypeScript · SQLite · FSRS.

## Development

Requirements: Rust (stable), Node 24+, pnpm, and the
[Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/) (`webkit2gtk-4.1`).

```sh
pnpm install
pnpm dev          # run the app with hot reload
pnpm build        # production bundles
```

Checks: see [CONTRIBUTING.md](CONTRIBUTING.md).

## Documentation

- [Product](docs/product.md)
- [Architecture](docs/architecture.md)
- [Architecture decision records](docs/adr/)
- [Omarchy / Hyprland integration (Super+Z)](docs/omarchy.md)
- [Contributing: Git workflow, commits, migrations, releases](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)
