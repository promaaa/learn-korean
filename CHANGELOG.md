# Changelog

All notable changes are documented here. Versions follow [Semantic Versioning](https://semver.org/);
before 1.0, a minor bump is a new feature and a patch bump is a fix.

## [Unreleased]

### Added
- Intro cards: a word or sentence never seen before is first shown with its English (and note), spoken aloud, instead of being asked straight away; its question comes a few cards later. Answers in the session of the intro are rated Good at most, so a word is known only after a correct answer in a later session (`Session::seen`, `session_seen`).
- Typing Gym keyboard course, like typingclub.com: 69 lessons teach touch typing a key pair at a time, from the home row out (ㄹ ㅓ on F and J, then ㅇ ㅏ, ㄴ ㅣ, ㅎ ㅗ, ㅁ, the top row, the bottom row with `,` `.`, then the Shift doubles). Each stage drills its new keys alone as lone jamo, then in syllables, then twice mixed with every key learned before; lines are random and new on every attempt. 90 % accuracy passes a lesson and opens the next; passes are `settings` rows `keyboard_passed/<lesson id>` and sync. The rounds of words from your packs stay as free practice at the end of the list, and a round left with `Tab` resumes (`korean_core::typing::keyboard`; `korean_db::hangul` is now `korean_db::lessons`).
- Progress sync between computers through a shared folder (MEGA, Syncthing...): each device writes `<device>.db` (reviews, XP, settings) when the window hides or loses focus, and merges the others' at start and on every Super+Z. Enabled by `sync.json` in the app config directory, written by `LEARN_KOREAN_SYNC_DIR=... scripts/install-omarchy.sh` (ADR-006, migration 0008: `settings.updated_at`, `sync_peers`).
- FSRS parameters optimized from your own review log, as in Anki: refitted at most daily in the background when the log grew, kept only if they predict your answers better, synced between devices (`fsrs` setting). Memory states are replayed from the log whenever the parameters or the merged log change.
- Anki's session rules: at most 20 new cards per day (across sessions and devices), learning cards first (learned ahead up to 20 minutes), then review cards least likely to be recalled first, and one card per item per session so siblings don't prime each other.
- Stats screen: a 26-week activity heatmap, totals (answers, accuracy, time, practice days, streak), true retention per skill over 30 days and all time (reviews after at least a day), known / learning / unseen words and sentences ready, cards due over the next 14 days and minutes per day over the last 30. Computed from the review log in `korean_core::stats`; no migration.
- Screens: `Tab` / `Shift + Tab` cycle review → Typing Gym → Hangul → stats, with the current screen shown in the top bar.
- Hangul primer (Hangul screen) for total beginners: 10 lessons (`content/hangul/primer.json`) introduce the 40 letters (compound vowels included) and 8 common compound finals, each with an intro card (the jamo, its Hangul name and a spoken example word), then drill them on the Typing Gym keyboard from the lone jamo to syllables (hinted by their assembly, ㄱ + ㅏ = 가) to real words. A round at 90 % accuracy passes the lesson and opens the next; passes are `settings` rows, one per lesson, so they sync (no migration). A validator checks that no line uses a jamo before its lesson.
- `tools/pack-gen`, the content authoring CLI: `draft` asks an LLM (`PACK_GEN_LLM`, default `claude -p`) for new items of a pack from a scenario brief, with the format rules, existing lines, carded lemmas and glossed forms in the prompt, into `content/<pack>/draft.json`; `check [--draft <file>]` validates the packs and glossaries on disk with the draft merged in, one precise error per line and a non-zero exit code; `review` accepts, skips or edits each draft item in `$EDITOR` into `pack.json` and its glossary. Workflow in `docs/content.md`.
- Seven content packs, 383 sentences and 315 word cards, each word card placed before its first sentence: **Numbers & time** (level 2: native vs Sino-Korean numbers, counters, clock time, dates, prices), **Getting around** (level 2: directions, subway, bus, taxi, KTX), **Shopping** (level 3), **Phone calls** (level 4), **Doctor & pharmacy** (level 4), **At work** (level 5: 해요체 and 합니다체, titles, 회식) and **K-drama Korean** (level 5: casual 반말 lines with when to use them).

### Changed
- Scheduling follows Anki's FSRS implementation: FSRS-6 through `fsrs` (the library Anki uses) instead of `rs-fsrs` (FSRS-5); learning steps 1 min / 10 min and a 10 min relearning step; review intervals in whole days starting at 4 a.m. local time, fuzzed with Anki's ranges and ordered Hard < Good < Easy (ADR-003).
- Glossaries are per pack (`content/<pack>/glossary.json`, replacing `content/glossary.json`): a word form's gloss lives in the first pack in bundle order whose lines use it, so packs can be authored in parallel. The 1100 existing glosses were split by owner pack; validation also rejects a gloss in a pack other than the owner. The word-card rule is now `validate_word_cards` (`content/validate.rs`), shared by the content test and pack-gen.

### Fixed
- macOS: the global shortcut is **Ctrl+Option+Z**; Super+Z was Command+Z and took Undo from every app.
- macOS: Ctrl+Option+Z uses the key labelled Z in the current keyboard layout (it used the QWERTY position, so on AZERTY only Ctrl+Option+W worked).
- macOS: after `Esc`, clicking the Dock icon or opening the app again (Finder, Spotlight) shows the window, and hiding it hands the keyboard back to the previous app instead of an invisible one.

## [0.6.0]

### Added
- Focus (`F`): sessions draw from **words only**, **words, then sentences** (default) or **everything**; the choice is saved (migration 0007, `settings` table). Outside "everything", a sentence is served only once every word of it is known (its word card graduated to FSRS review); its words not seen yet are taught first, in its place.
- 190 word cards so every word of every sentence can be learned on its own first: new words in starter, small-talk and restaurants (placed just before the first sentence using them) and a new **Everyday words** pack for the legacy sentences. Sentence `lexemes` now list every content word (legacy: new curation `usage.lexemes` field); a test requires a word card for each, grammar (이다, 에 있다, 하고, 만) excepted.
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
