# Architecture

```
learn-korean/
├── apps/client/            Tauri 2 desktop app
│   ├── src/                Svelte 5 + TypeScript UI (rendering + input only)
│   └── src-tauri/          Tauri commands: thin glue between UI and crates
├── crates/                 Rust libraries (added as features land)
│   ├── korean-core/        pure domain logic: scheduling, sessions, scoring, typing
│   ├── korean-db/          SQLite (sqlx), migrations, backups, repositories
│   └── korean-providers/   network providers behind traits: TTS, images
├── content/                bundled content packs (JSON), validated in CI
├── tools/anki-import/      legacy Anki CSV importer
└── docs/                   product, architecture, ADRs
```

## Boundaries

- **All learning logic lives in `korean-core`** (next card, answer scoring, memory update, XP,
  unlocks). Svelte components never compute scheduling or scoring; this keeps a future mobile UI a
  new front end rather than a rewrite.
- `korean-core` performs no I/O. `korean-db` and `korean-providers` do I/O and depend on
  `korean-core` types, never the other way round.
- The Tauri crate wires state (DB pool, providers) and exposes commands. It contains no domain rules.
- Providers are traits (`TtsProvider`, `ImageProvider`). No other code knows which concrete
  provider is in use, so swapping one is a one-line change in the wiring.

## Data

Three levels of content (see ADR-002):

1. **Raw** — imported source rows, immutable.
2. **Lexeme** — normalized dictionary entry (lemma, surface, gloss, part of speech, register).
3. **Learning item** — what the games teach: a sentence or word with context and skills.

User data (memory states, review log, XP) is stored separately from bundled content and keyed by
stable item ids, so content packs can be re-seeded without touching progress. Pack format and
validation rules: [content.md](content.md).

## Media

Images and audio are fetched by Rust (`reqwest`) and handed to the UI as bytes held in memory.
Only metadata (provider, URLs, attribution) is persisted; no media files are written (ADR-005).
