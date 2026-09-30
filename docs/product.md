# Product

A keyboard-first Korean trainer that opens instantly (Super + Z), runs a short session, and gets
out of the way (Esc).

## Principles

- **Speed of a TUI, richness of a GUI.** Everything is playable with `1 2 3 4`, `Space`, `Enter`,
  `H J K L`, `R` (replay audio), `F` (focus), `Tab` (next screen) and `Esc`. Shortcuts use
  physical key positions, so they work the same on AZERTY. Mouse is optional.
- **Conversation over vocabulary, vocabulary first.** The unit of learning is a sentence used in
  a context (small talk, restaurants…). Its words are learned first, as word cards; the sentence
  follows once they are known (see Focus).
- **Hangul only.** No romanization anywhere in the UI (ADR-004).
- **Real media.** Native Korean typography, real photos, real audio.
- **Spaced repetition.** Every (item, skill) pair has its own memory state scheduled by FSRS.

## Screens

`Tab` cycles through the screens, `Shift + Tab` goes back: review → Typing Gym → Hangul → stats →
review. The top bar shows the current one. The review session keeps its place while away; coming
back resumes it (or restarts it, if another device's progress was merged meanwhile).

## Games

| Game | Skill | Interaction |
| --- | --- | --- |
| Listening | comprehension | hear/read Korean, pick the meaning (1–4) |
| Reply | response | pick the natural answer to a Korean line |
| Build | production | assemble the sentence from shuffled chunks |
| Typing Gym (`Tab`) | typing | type Korean on a dubeolsik keyboard with live hints |

After answering, every Korean word on the card (the line, Reply options, the placed Build chunks)
is dotted underlined; hovering it shows its English, e.g. 했어요? → "did (하다)". The arrow keys
(or `H J K L`) show them without the mouse: ← → word by word in reading order, ↑ ↓ to the nearest
word on the row above or below.

### Typing Gym

A touch-typing course, like typingclub.com, then free practice. The list works like the Hangul
primer's: `↑ ↓` (or `J K`) and `Enter`; lesson 1 is always open, each next one opens once the
previous is passed.

1. **Keyboard course**: 69 lessons that add a key pair at a time, from the home row out: ㄹ ㅓ
   (index fingers on F and J), ㅇ ㅏ, ㄴ ㅣ, ㅎ ㅗ, ㅁ, then the top row (ㄱ ㅕ, ㄷ ㅑ, ㅅ ㅛ, ㅈ ㅐ,
   ㅂ ㅔ), the bottom row (ㅍ ㅡ, ㅠ ㅜ, ㅊ `,`, ㅌ `.`, ㅋ) and the Shift doubles (ㄲ ㅆ, ㄸ ㅉ ㅃ,
   ㅒ ㅖ). Each stage drills its new keys alone as lone jamo (ㄹㄹ ㅓㅓㅓ), then in syllables
   (러 럴러, borrowing learned vowels or consonants when the stage has none), then twice mixed with
   every key learned before, half the keys still new. Hangul composes as it is typed, so groups
   are either lone consonants, lone vowels that do not combine, or real syllables. Every line is
   random, generated again on each attempt, and types every new key. The summary shows speed,
   accuracy and mistakes; **90 %** accuracy passes. `Enter` opens the next lesson after a pass and
   retries otherwise; `Backspace` returns to the list. Passes are saved and synced.
2. **Free practice**, the last row: rounds of 8 lines from unlocked packs, items already met
   first, short to long.

Leaving the screen mid-round (`Tab`) keeps the round: coming back resumes it. Keys are read by
physical position (`KeyboardEvent.code`), so the standard 2-set (dubeolsik) layout works on any
hardware layout (QWERTY, AZERTY…) without an OS Korean IME. The on-screen keyboard highlights the
next key and Shift, names the finger, and flashes wrong keys; the line composes live like an IME
(ㅎ → 하 → 한). Strict mode: a wrong key counts as an error and does not advance. Speed (words and
keystrokes per minute), accuracy and streak update on every key.

### Hangul primer

For total beginners, before the first card: 10 lessons that introduce the jamo a few at a time
(basic vowels with ㅇ, the consonants in three steps, aspirated consonants, vowels with an extra
stroke, vowels with ㅣ added, tense consonants, final consonants with the compound finals, compound
vowels) and drill them on the Typing Gym's keyboard.

1. **Lessons**: `↑ ↓` (or `J K`) and `Enter`. Lesson 1 is always open; each next one opens once
   the previous one is passed. Locked lessons are dimmed.
2. **Intro card** for each new jamo: the jamo, its Hangul name (기역, 니은, 아…) and an example
   word with its English, spoken aloud. `← →` (`H L`) go through the lesson's jamo, `R` replays,
   `Enter` starts typing, `Backspace` goes back to the list.
3. **Drill**: the lesson's lines, from the lone new jamo to syllables to real words, typed like in
   the Typing Gym (strict mode, live composition, next key highlighted). Each line is spoken when
   it appears. Words show their English; syllable drills show how they are built instead
   (ㄱ + ㅏ = 가, ㄱ + ㅏ + ㄴ = 간; a lone jamo shows its name). Every letter key types, so the
   other shortcuts wait for the end of the round.
4. **Summary**: accuracy over the whole round. **90 %** passes the lesson. `Enter` opens the next
   lesson after a pass and retries otherwise; `Backspace` returns to the list.

Passes are saved and synced across devices. Leaving the screen mid-lesson (`Tab`) keeps the round:
coming back resumes it.

## Focus

`F` switches what sessions draw from (saved, applied to a new session at once):

| Focus | Cards |
| --- | --- |
| words only | word cards |
| words, then sentences (default) | word cards, and every sentence whose words are all known |
| everything | every card, as soon as it is due or next in line |

A word is known once its listening card has left FSRS's short learning steps (a correct answer
again in a later session, or an easy first answer). A sentence waiting for its words is not
served, even when due; its words not seen yet are introduced in its place, so the words taught
first are the ones the next sentences need. Words that are forgotten (lapsed) pause their
sentences until relearned. Lemmas without a word card (particles, the copula 이다) never block a
sentence.

## Progression

- **XP**: 10 per correct answer, +5 when easy, +5 the first time a card is answered, multiplied by
  the combo (+10 % per consecutive correct answer, up to ×2). A miss still earns 2.
- **Levels**: level *n* → *n + 1* costs 100 + 50 (*n* − 1) XP. Packs declare an `unlock_level`.
- **Daily streak**: consecutive days with at least one correct answer (local time).

## Stats

One screen, refreshed each time it is opened or the window is summoned. Days are local days, as
for the streak.

- **Activity**: answers per day over the last 26 weeks, as a calendar heatmap (4 shades, relative
  to the busiest day).
- **Totals**: answers, accuracy, answer time, practice days (with a correct answer) and streak.
- **Retention** per skill, over 30 days and all time: the share of reviews remembered, where a
  review is an answer to a card last answered on an earlier day (same-day repeats are learning,
  not recall). `—` until there is a review.
- **Words**: word cards known (the Focus rule), learning (seen, not known yet) and unseen, out of
  every bundled pack; sentences whose words are all known, out of all sentences.
- **Due**: cards due on each of the next 14 days; today includes overdue cards.
- **Time**: minutes spent answering on each of the last 30 days.
