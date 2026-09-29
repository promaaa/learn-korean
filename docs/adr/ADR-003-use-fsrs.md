# ADR-003: Schedule reviews with FSRS

## Context
Each (learning item, skill) pair needs its own review schedule. SM-2 (Anki's historical default)
is simple but less accurate than modern models.

## Decision
Use FSRS (Free Spaced Repetition Scheduler). The scheduler lives in `korean-core` behind a
`Scheduler` API; game answers are converted to FSRS ratings (Again/Hard/Good/Easy) from
correctness and response time. A simpler interval ladder is acceptable until FSRS lands.

## Alternatives considered
- SM-2: well known, but worse retention/effort trade-off.
- Leitner boxes: too coarse for mixed skills.

## Consequences
- Memory state stores stability, difficulty, due date and review counts.
- Review logs are kept so parameters can be optimized later.
