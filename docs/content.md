# Content packs

Bundled content lives in `content/<pack-id>/pack.json`, is embedded in the binary
(`korean_core::content::bundled_packs`) and re-seeded into the `content_*` tables whenever a pack
changes. User progress references items by id only, so editing a pack never loses progress —
**but item ids are permanent once released**.

`cargo test -p korean-core` validates every bundled pack (CI: "starter content validation").

## Pack

```json
{
  "id": "starter",
  "version": 1,
  "title": "Starter",
  "description": "Survival phrases.",
  "unlock_level": 1,
  "items": []
}
```

`id` is kebab-case and equals the directory name. `unlock_level` ≥ 1.

## Item

```json
{
  "id": "small-talk/what-did-you-do-weekend",
  "kind": "sentence",
  "korean": "주말에 뭐 했어요?",
  "english": "What did you do on the weekend?",
  "context": "weekend",
  "register": "polite",
  "note": "optional, ≤ 120 chars",
  "lexemes": ["주말", "뭐", "하다"],
  "distractors": ["What are you doing this weekend?", "Where did you go on the weekend?", "Who did you meet on the weekend?"],
  "replies": {
    "good": [{ "korean": "친구를 만났어요.", "english": "I met a friend." }],
    "bad": [
      { "korean": "네, 맛있어요.", "english": "Yes, it's delicious." },
      { "korean": "저는 학생이에요.", "english": "I'm a student." },
      { "korean": "다음 주에 갈 거예요.", "english": "I'll go next week." }
    ]
  },
  "chunks": ["주말에", "뭐", "했어요?"],
  "image": "weekend picnic park"
}
```

| Field | Rule |
| --- | --- |
| `id` | `<pack-id>/<kebab-slug>`, unique across packs, never renamed |
| `kind` | `sentence` or `word` |
| `korean` | Hangul syllables, single spaces, `? . , ! ~`; starts with a syllable; no Latin, digits, romanization or `...` |
| `english` | unique within the pack |
| `context` | kebab-case tag |
| `register` | `polite` (해요체), `formal` (합니다체), `casual` (반말), `neutral` |
| `distractors` | sentences: exactly 3 plausible but wrong meanings, distinct, ≠ `english`; words: optional (else drawn from other items) |
| `replies` | sentences only; `good` 1–3, `bad` exactly 3, all Hangul text with English |
| `chunks` | optional; `chunks.join(" ") == korean`; default is splitting on spaces |
| `image` | optional 1–3 word English photo query |

## Skills

Derived from the data, never authored:

- `listening` — every item.
- `response` — items with `replies`.
- `build` — sentences with at least 3 chunks.

Two further rules span all packs: item ids are globally unique, and the same Korean line may not
appear twice (a trailing `.` does not make a new line; `?` does: 괜찮아요 vs 괜찮아요?).

## Legacy Anki deck

`content/legacy/` holds the three data levels for the original deck:

| File | Level | Produced by |
| --- | --- | --- |
| `source/anki-important-words.csv` | source export | Anki |
| `raw.json` | raw rows, immutable | `anki-import import` |
| `curation.json` | hand-written normalization, one entry per row | editor |
| `lexemes.json` | normalized lexemes | `anki-import build` |
| `pack.json` | `legacy` pack: word cards + usage sentences | `anki-import build` |

```sh
cargo run -p anki-import -- import content/legacy/source/anki-important-words.csv content/legacy/raw.json
cargo run -p anki-import -- build content/legacy
```

The importer refuses to build unless every raw row is curated, and rows with detected problems
(no Hangul, foreign script such as the Arabic `فقط`, `...`/`X` templates, empty rows) carry an
explicit `fix` or `skip`. Grammar patterns and particles get no word card; they are taught through
their usage sentences. A test checks that the committed `raw.json`, `lexemes.json` and `pack.json`
are exactly what the importer produces, so generated files cannot drift from their sources.
