# Product

A keyboard-first Korean trainer that opens instantly (Super + Z), runs a short session, and gets
out of the way (Esc).

## Principles

- **Speed of a TUI, richness of a GUI.** Everything is playable with `1 2 3 4`, `Space`, `Enter`,
  `H J K L`, `R` (replay audio), `F` (focus), `Tab` (Typing Gym) and `Esc`. Shortcuts use
  physical key positions, so they work the same on AZERTY. Mouse is optional.
- **Conversation over vocabulary, vocabulary first.** The unit of learning is a sentence used in
  a context (small talk, restaurants…). Its words are learned first, as word cards; the sentence
  follows once they are known (see Focus).
- **Hangul only.** No romanization anywhere in the UI (ADR-004).
- **Real media.** Native Korean typography, real photos, real audio.
- **Spaced repetition.** Every (item, skill) pair has its own memory state scheduled by FSRS.

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

Rounds of 8 lines from unlocked packs, items already met first, short to long. Keys are read by
physical position (`KeyboardEvent.code`), so the standard 2-set (dubeolsik) layout works on any
hardware layout (QWERTY, AZERTY…) without an OS Korean IME. The on-screen keyboard highlights the
next key and Shift, names the finger, and flashes wrong keys; the line composes live like an IME
(ㅎ → 하 → 한). Strict mode: a wrong key counts as an error and does not advance. Speed (words and
keystrokes per minute), accuracy and streak update on every key.

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
