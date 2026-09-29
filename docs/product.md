# Product

A keyboard-first Korean trainer that opens instantly (Super + Z), runs a short session, and gets
out of the way (Esc).

## Principles

- **Speed of a TUI, richness of a GUI.** Everything is playable with `1 2 3 4`, `Space`, `Enter`,
  `H J K L`, `R` (replay audio), `Tab` (Typing Gym) and `Esc`. Shortcuts use physical key
  positions, so they work the same on AZERTY. Mouse is optional.
- **Conversation over vocabulary.** The unit of learning is a sentence used in a context
  (small talk, restaurants…). Single words feed sentences, images and typing drills.
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

### Typing Gym

Rounds of 8 lines from unlocked packs, items already met first, short to long. Keys are read by
physical position (`KeyboardEvent.code`), so the standard 2-set (dubeolsik) layout works on any
hardware layout (QWERTY, AZERTY…) without an OS Korean IME. The on-screen keyboard highlights the
next key and Shift, names the finger, and flashes wrong keys; the line composes live like an IME
(ㅎ → 하 → 한). Strict mode: a wrong key counts as an error and does not advance. Speed (words and
keystrokes per minute), accuracy and streak update on every key.

## Progression

- **XP**: 10 per correct answer, +5 when easy, +5 the first time a card is answered, multiplied by
  the combo (+10 % per consecutive correct answer, up to ×2). A miss still earns 2.
- **Levels**: level *n* → *n + 1* costs 100 + 50 (*n* − 1) XP. Packs declare an `unlock_level`.
- **Daily streak**: consecutive days with at least one correct answer (local time).
