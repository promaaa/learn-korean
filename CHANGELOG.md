# Changelog

All notable changes are documented here. Versions follow [Semantic Versioning](https://semver.org/);
before 1.0, a minor bump is a new feature and a patch bump is a fix.

## [Unreleased]

### Added
- Audio: every card is spoken when it appears; `R` (or the speaker button) replays it.
- `korean-providers` crate with the `TtsProvider` trait: Edge neural voice (`ko-KR-SunHiNeural`) with Google Translate fallback and an in-memory cache; native playback via `rodio`.

## [0.2.0]

### Added
- Listening game: Korean line, four meanings, answer with `1-4` or `J`/`K` + `Enter`, `Space` for the next card; correct/incorrect feedback with the meaning and usage note, streak counter and session progress bar.
- Session summary (answered, accuracy, best streak); summoning the window after a finished session starts a new one, and time spent hidden is not counted as answer time.
- Review scheduler (interval ladder: 10 min → 1 d → 3 d → … → 180 d; lapses reset) per (item, skill) card.
- Answer grading: wrong → Again, correct → Easy/Good/Hard by response time (skill-specific bands).
- Session planning: due cards first (most overdue first), new material round-robin across packs, other skills unlocked after listening; missed cards retried later in the session.
- Schema 3: `review_states`; every answer is written atomically to `review_states` and `review_log`.
- Session commands `session_start`, `session_current`, `session_answer` (answers are checked in Rust, never in the UI).
- Legacy pack ordered as the deck: each word followed by the sentences that use it.
- `anki-import` tool: legacy CSV → immutable raw rows → curated lexemes → `legacy` pack (284 word cards, 213 usage sentences).
- Curation of the 284-word deck with 29 explicit fixes (e.g. Arabic `فقط` → `만`/`밖에`, `딱딱하다` is "hard, rigid", not "difficult"); romanization dropped.
- Validator rule: the same Korean line may not appear in two items.
- `korean-core` crate with the normalized content model (raw entry, lexeme, learning item, pack) and derived skills (listening, response, build).
- Content validator enforcing the pack contract (Hangul-only Korean, 3 distinct distractors, reply counts, chunk integrity, unique ids); bundled packs validated in CI.
- `content/starter` pack: 22 survival phrases.
- Schema 2: re-seedable `content_packs` / `content_items` tables, reseeded only when a pack changes; user history untouched.
- `korean-db` crate: SQLite database in the app data directory (sqlx, WAL), embedded append-only migrations.
- Automatic `VACUUM INTO` backup to `backups/v<previous-version>-before-schema-<n>.db` before pending migrations run.
- Older releases refuse a newer schema with an explanatory screen instead of corrupting data.
- `migrations.lock` test: released migrations cannot be edited.
- Schema 1: append-only `review_log` of every answer.
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
