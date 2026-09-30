# Content packs

Bundled content lives in `content/<pack-id>/`: `pack.json` (the items) and `glossary.json` (the
English of its words, see [Glossary](#glossary)). The packs listed in `BUNDLED`
(`crates/korean-core/src/content/bundled.rs`, in unlock order) are embedded in the binary
(`korean_core::content::bundled_packs`) and re-seeded into the `content_*` tables whenever a pack
changes. User progress references items by id only, so editing a pack never loses progress,
**but item ids are permanent once released**.

`cargo test -p korean-core` validates every bundled pack (CI: "starter content validation");
`cargo run -p pack-gen -- check <pack-id>` runs the same rules on the files on disk (see
[Authoring with pack-gen](#authoring-with-pack-gen)).

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
`GRAMMAR` const of `crates/korean-core/src/content/validate.rs`. The rule is `validate_word_cards`
there, run by the bundled content test and `pack-gen check`; it reports the lemma and sentence id.

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

Each pack's `glossary.json` maps word forms (item Korean and replies, split on spaces, `? . , ! ~`
stripped) to the English shown when the learner hovers them after answering:

```json
{ "주말에": "on the weekend", "했어요": "did (하다)", "친구를": "friend (object)" }
```

A form's gloss lives in the glossary of its **owner: the first pack in bundle order whose lines use
it**. A pack glosses only the forms no earlier pack uses, so packs can be written in parallel
without sharing a file; the app merges all glossaries (`bundled_glossary`).

Glosses are at most 40 characters: meaning of that form, particle role in parentheses when it adds
information, and the dictionary form after conjugated verbs and adjectives. A form with different
meanings across lines joins them with ` / ` in its owner's glossary (`이`: "two / this").
Validation (`validate_glossary`) fails when a form has no gloss in its owner pack, a form is glossed
in another pack (the message names the owner) or a gloss is used by no line. So adding a pack line
means glossing its new words in that pack; a line added to an earlier pack can make it the owner of
a form a later pack glosses, and the gloss moves (`pack-gen review` moves it).

## Hangul primer

`content/hangul/primer.json` holds the primer's lessons, in teaching order. It is embedded in the
binary (`korean_core::typing::primer::bundled_primer`), is not a pack and has no items or cards.

```json
{
  "lessons": [
    {
      "id": "consonants-1",
      "title": "First consonants",
      "jamo": [
        { "jamo": "ㄱ", "name": "기역", "example": { "korean": "고기", "english": "meat" } }
      ],
      "lines": [
        { "korean": "ㄱ ㄴ ㄷ ㄹ" },
        { "korean": "가 나 다 라" },
        { "korean": "고기", "english": "meat" }
      ]
    }
  ]
}
```

| Field | Rule |
| --- | --- |
| `id` | kebab-case, unique; **permanent once released** (passes are stored by lesson id) |
| `title` | English, not empty |
| `jamo` | at least one; each a compatibility jamo (compounds such as ㅘ or ㄺ included) introduced by no other lesson, and typed in at least one of the lesson's lines |
| `jamo[].name` | its Hangul name: 기역, 니은, 쌍기역, 리을기역; vowels are named by their sound (아, 와) |
| `jamo[].example` | a common word containing the jamo, with its English |
| `lines` | at least one, from the lone new jamo to syllables to real words |
| `lines[].korean` | typeable on the 2-set layout and shown back as written: single spaces, no lone jamo the IME would merge (`ㄱㅏ` types as 가) |
| `lines[].english` | words: the meaning. Syllable drills omit it; the UI shows the assembly instead (ㄱ + ㅏ = 가, computed by `Primer::assembly`) |

The key rule: **every jamo of a line was introduced in this lesson or an earlier one**, counting
each keystroke (과 needs ㄱ, ㅗ and ㅏ) and the compound vowel or final itself (과 also needs ㅘ,
닭 needs ㄺ). Examples follow the same rule from the second lesson on; the first lesson's six vowels
and ㅇ make too few words (none with ㅓ or ㅡ), so its examples (어머니, 우유, 그림) may use later
jamo: they are heard and read, never typed. Compound finals are introduced in the final-consonant
lesson, so that lesson has new jamo to present. `Primer::validate` checks all of this; the
`bundled_primer_is_valid` test (`crates/korean-core/src/typing/primer.rs`) runs it on the file.

Drill targets are `hangul/<lesson id>/<line number>` (from 1). Passes are `settings` rows
`hangul_passed/<lesson id>` (`korean_db::hangul`), one per lesson so that sync, which keeps the
latest row per key, merges passes from several devices.

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

## Authoring with pack-gen

`tools/pack-gen` drafts items with an LLM, checks them against every rule above and lets a human
accept them one by one. It reads `content/` and this file from the repository it was built in, at
run time. For each pack: draft → check → review → bundle.

1. **Draft** new items:

   ```sh
   cargo run -p pack-gen -- draft cafe --brief "ordering at a café, sizes and takeout" --count 12 \
     --title "Café" --description "Ordering coffee and tea." --level 4   # last line: new packs only
   ```

   The prompt holds this file (minus the tool sections), the pack's existing lines, the lemmas that
   already have a word card and the forms already glossed. It runs `PACK_GEN_LLM` (default
   `claude -p`) with the prompt on stdin and expects `{"items": [...], "glossary": {...}}`, alone
   or in a fenced code block. The items are appended to `content/<pack-id>/draft.json` (git-ignored;
   it also keeps a new pack's title, description and level), then checked. An answer that does not
   parse is saved to a temporary file whose path is printed.

2. **Check** until it prints `OK`:

   ```sh
   cargo run -p pack-gen -- check --draft content/cafe/draft.json cafe
   ```

   It loads the bundled packs and their glossaries from disk in bundle order (a pack not bundled
   yet comes last), merges the draft into its pack and runs `validate_packs`, `validate_glossary`
   and `validate_word_cards`. Each error is one line `<item id | pack glossary "form">: <message>`,
   marked `(draft)` when it comes from the draft; a draft gloss of a form some pack already glosses
   is reported rather than merged. Exit code: 0 valid, 1 content errors, 2 unreadable file (the
   message gives the file, line and column). Without `--draft` it checks the pack as it is.

3. **Review** in a terminal:

   ```sh
   cargo run -p pack-gen -- review cafe
   ```

   Each draft item is shown with its distractors, replies, chunks, lexemes, image and the gloss of
   each of its words with the glossary holding it. Type `a` accept, `s` skip (discard), `e` edit the
   item's JSON in `$VISUAL` / `$EDITOR`, or `q` quit, then Enter. Accepted items are appended to
   `pack.json` (created for a new pack) and their new glosses added to the pack's `glossary.json`,
   or moved there from a later pack that owned them until now. Handled items leave the draft, which
   is deleted once empty. The check runs at the end.

4. **Bundle** a new pack: add its id to `BUNDLED` in `crates/korean-core/src/content/bundled.rs` at
   its place in unlock order (check and review print this reminder while it is missing), then run
   `cargo test -p korean-core`.
