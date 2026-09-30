# learn-korean

Keyboard-first Korean learning app. Press **Super + Z**, play a short session, press **Esc**.

Tauri 2 · Rust · Svelte 5 · TypeScript · SQLite · FSRS.

## Install

- **Omarchy / Hyprland**: `scripts/install-omarchy.sh` builds the app, installs it in `~/.local/bin`,
  binds **Super + Z** and starts it hidden at login ([details](docs/omarchy.md)). Set
  `LEARN_KOREAN_SYNC_DIR` to a shared folder (MEGA, Syncthing...) to
  [sync progress between computers](docs/omarchy.md#sync-between-computers).
- **macOS from source**: `scripts/install-macos.sh` builds `learn-korean.app` into `/Applications`,
  starts it hidden at login (LaunchAgent) and restarts it; **Ctrl + Option + Z** shows it. The same
  `LEARN_KOREAN_SYNC_DIR` (and optional `LEARN_KOREAN_DEVICE`) enables sync.
- **Other platforms**: bundles for Linux and macOS are attached to each
  [GitHub Release](https://github.com/promaaa/learn-korean/releases).

## Keys

| Key | Action |
| --- | --- |
| `Super + Z` (macOS: `Ctrl + Option + Z`) | show / hide the window |
| `1`–`4` | answer (physical digit row, also on AZERTY) |
| `J` / `K`, `Enter` | move between options, choose |
| `H` / `L`, `Space`, `Backspace`, `Enter` | build game: move, place a chunk, undo, check |
| arrows or `H` `J` `K` `L` | after answering: show the English of each Korean word |
| `Space` | next card |
| `R` | replay audio |
| `J` / `K`, `Enter`, `Backspace` | Typing Gym: choose a lesson, start / next, back to the lessons |
| `J` / `K`, `H` / `L`, `Enter`, `Backspace` | Hangul primer: choose a lesson, browse its new letters, start / next, back to the lessons |
| `F` | focus: words only → words, then sentences → everything |
| `Tab` / `Shift + Tab` | next / previous screen: review → Typing Gym → Hangul → stats |
| `Esc` | hide |

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
- [Content packs and authoring them with pack-gen](docs/content.md)
- [Architecture decision records](docs/adr/)
- [Omarchy / Hyprland integration (Super+Z)](docs/omarchy.md)
- [Contributing: Git workflow, commits, migrations, releases](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)
