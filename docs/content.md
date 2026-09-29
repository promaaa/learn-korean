# Content packs

Bundled content lives in `content/<pack-id>/pack.json`, is embedded in the binary
(`korean_core::content::bundled_packs`) and re-seeded into the `content_*` tables whenever a pack
changes. User progress references items by id only, so editing a pack never loses progress,
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
| `lexemes` | dictionary forms in Hangul; sentences list every content word (see below), words their own lemma |

### Lexemes and word cards

A sentence's `lexemes` list every content word of the line as a dictionary form (`매운` → `맵다`,
`제` → `저`): nouns, pronouns, verbs, adjectives, adverbs, numerals, counters, determiners and
interjections. The session planner only serves a sentence once each of its lexemes that has a word
card is known, pulling missing word cards in just before it. So every such lexeme must have a word
card (an item of kind `word` whose `lexemes` contain it) in some bundled pack, placed in the earliest
pack that uses it, before its first sentence. Grammar — particles, the copula `이다`, patterns such as
`에 있다` — is exempt: those lemmas may appear in `lexemes` but get no card and are listed in the
`GRAMMAR` const of the `every_sentence_lexeme_has_a_word_card` test in
`crates/korean-core/src/content/bundled.rs`, which fails with the offending lemma and sentence id.

Words needed only by legacy sentences live in the `everyday-words` pack, bundled right after
`legacy`, ordered by first use in the legacy deck.

## Skills

Derived from the data, never authored:

- `listening`: every item.
- `response`: items with `replies`.
- `build`: sentences with at least 3 chunks.

Two further rules span all packs: item ids are globally unique, and the same Korean line may not
appear twice (a trailing `.` does not make a new line; `?` does: 괜찮아요 vs 괜찮아요?).

## Glossary

`content/glossary.json` maps every word form used by any pack (item Korean and replies, split on
spaces, `? . , ! ~` stripped) to the English shown when the learner hovers it after answering:

```json
{ "주말에": "on the weekend", "했어요": "did (하다)", "친구를": "friend (object)" }
```

Glosses are at most 40 characters: meaning of that form, particle role in parentheses when it adds
information, and the dictionary form after conjugated verbs and adjectives. A form with different
meanings across lines joins them with ` / ` (`이`: "two / this"). Validation fails when a word has
no gloss or a gloss is used by no line, so adding a pack line means adding its new words here.

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

Each `usage` sentence of a curation entry may carry `lexemes`: the other content words of that
sentence (dictionary forms). The built sentence's lexemes are the entry's head lemma (when it is
Hangul) followed by these, deduplicated.
