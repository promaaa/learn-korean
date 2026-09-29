# ADR-004: No romanization

## Context
The legacy Anki deck pairs every word with a romanization. Romanization slows down Hangul reading
and fixes wrong pronunciation habits.

## Decision
Romanization is never shown. The importer keeps it only inside the immutable raw record; normalized
lexemes and learning items have no romanization. Pronunciation is taught with audio.

## Alternatives considered
- Optional toggle: invites dependency; adds UI and data complexity.

## Consequences
- Beginners rely on audio (TTS) and the Typing Gym to learn Hangul.
