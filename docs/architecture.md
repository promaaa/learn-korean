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

1. **Raw**: imported source rows, immutable.
2. **Lexeme**: normalized dictionary entry (lemma, surface, gloss, part of speech, register).
3. **Learning item**: what the games teach: a sentence or word with context and skills.

User data (memory states, review log, XP) is stored separately from bundled content and keyed by
stable item ids, so content packs can be re-seeded without touching progress. Pack format and
validation rules: [content.md](content.md).

Progress syncs between devices through a shared folder: each device writes a snapshot of its user
tables, `<device>.db`, and merges the others' (ADR-006, `korean_db::sync`).

## Media

Images and audio are fetched by Rust (`reqwest`) and handed to the UI as bytes held in memory.
Only metadata (provider, URLs, attribution) is persisted; no media files are written (ADR-005).

### Speech

`korean_providers::tts::TtsProvider` is the only interface the app knows. The default chain is
`Cached<Fallback[EdgeTts, GoogleTranslateTts]>`: Microsoft Edge's neural voice
(`ko-KR-SunHiNeural`, 10 % slower), falling back to Google Translate's voice, with the last 128
syntheses kept in memory. Playback happens natively in Rust (`rodio`) on a dedicated audio thread,
so it does not depend on the WebView's codecs. Swapping the voice service means adding a provider
and changing `tts::default_provider()`; nothing else changes.

### Images

`korean_providers::images::ImageProvider` searches open-licensed photos for an item's `image`
query: Openverse first, Wikimedia Commons second (`FirstMatch`). The resolved reference
(`provider`, `image_url`, `source_url`, `attribution`) is cached in `image_refs`; queries that found
nothing are retried after a week. Bytes are downloaded by Rust into a small in-memory cache and
served to the WebView through the `kimg://localhost/<item id>` protocol with `Cache-Control:
no-store`, so no image is written to disk and the WebView never hotlinks (no CORS/hotlink issues).
The attribution is always shown under the photo.
