# Changelog

All notable changes are documented here. Versions follow [Semantic Versioning](https://semver.org/);
before 1.0, a minor bump is a new feature and a patch bump is a fix.

## [Unreleased]

### Added
- Word glosses: after answering, hover any Korean word on the card (the line, Reply options, the Build answer) to see its English, or browse the words with the arrow keys (← → word by word, ↑ ↓ row by row). `content/glossary.json` glosses all 998 word forms of the bundled packs; validation requires every word to have a gloss and every gloss to be used.

## [0.5.2]

### Fixed
- The WebView talks to Rust over Tauri's `ipc:` protocol again (the CSP blocked it, forcing the slower postMessage fallback, noticeable per keystroke in the Typing Gym).

### Added
- README: install instructions and key reference.

## [0.5.1]

### Fixed
- Time spent in the Typing Gym no longer counts as answer time for the waiting review card (it was graded Hard), and summoning the window while the gym is open no longer plays the hidden card.
- A slow speech synthesis can no longer play an older line over the current card.
- The card no longer re-mounts when answering (photo flicker, repeated fly-in); a card retried immediately animates in again.
- The Typing Gym starts one round per visit (it started two), and a double `Enter` no longer skips a line.
- A photo deleted upstream (403/404/410) is forgotten and searched again instead of failing forever.

## [0.5.0]

### Added
- Typing Gym (`Tab`): type Korean lines on an on-screen 2-set (dubeolsik) keyboard that highlights the next key, Shift and finger, with live IME-style composition, strict error counting, WPM, keystrokes per minute, accuracy and streak; rounds of 8 lines from unlocked content, known items first.
- `korean_core::typing`: dubeolsik layout on physical keys, jamo decomposition, 2-set composition automaton, drill state machine.

### Fixed
- Shortcuts use physical key positions: the digit row answers on AZERTY keyboards (where it types `& é " '` without Shift).

## [0.4.0]

### Changed
- Legacy usage sentences that duplicated small-talk lines were moved to the small-talk pack (validator: one card per Korean line).
- Scheduling now uses FSRS (`rs-fsrs`, FSRS-5 default weights, 90 % retention, 1/5/10-minute learning steps) instead of the interval ladder.

### Added
- `restaurants` pack (unlocked at level 3): 60 sentences for a Korean restaurant or café visit (seating, ordering, spice level, allergies, refills, paying, takeout, café orders) with honorific staff lines and 30 reply exercises, plus 14 dish/utensil word cards with photos.
- `small-talk` pack (level 1): 81 everyday conversation lines (greetings, introductions, weather, weekend, hobbies, work, family, feelings, plans, compliments, leaving), 59 with replies for the reply game.
- XP and levels: 10 XP per correct answer (+5 easy, +5 first review, combo up to ×2), progressively longer levels, daily streak; level, XP bar and streak in the title bar, `+XP` after each answer, level-up banner listing unlocked packs.
- Packs unlock by level: sessions draw only from packs whose `unlock_level` is reached.
- Schema 6: append-only `xp_events` ledger, written in the same transaction as the review.
- Schema 5: FSRS columns on `review_states` (phase, stability, difficulty, scheduled days); ladder progress is converted in place (backup first).
- Feedback shows when the card comes back ("next review in 3 days").

## [0.3.0]

### Added
- Build game: the English meaning is shown, assemble the Korean sentence from shuffled chunks (`1-9` or `H`/`L` + `Space` to place, `Backspace` to undo, `Enter` to check); the sentence is spoken after answering.
- Every skill (listening, response, build) is now scheduled; sentences with 3 to 9 chunks can be built.
- Reply game: someone says a Korean line to you (spoken), pick the natural Korean answer among four; after answering, every option is translated and the right answer is spoken back.
- Photos on cards with an `image` query: Openverse, then Wikimedia Commons; attribution shown under every photo.
- Schema 4: `image_refs` (provider, image URL, source URL, attribution per query). Image bytes are kept in memory and served through the `kimg://` protocol, never written to disk.
- Audio: every card is spoken when it appears; `R` (or the speaker button) replays it.
- `korean-providers` crate with the `TtsProvider` trait: Edge neural voice (`ko-KR-SunHiNeural`) with Google Translate fallback and an in-memory cache; native playback via `rodio`.

### Changed
- Window height 720 px to fit the photo above the card.

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
