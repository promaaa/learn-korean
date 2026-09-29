# ADR-003: Schedule reviews with FSRS

## Context
Each (learning item, skill) pair needs its own review schedule. SM-2 (Anki's historical default)
is simple but less accurate than modern models.

## Decision
Use FSRS (Free Spaced Repetition Scheduler) through the reference Rust implementation `rs-fsrs`
(open-spaced-repetition, MIT): default FSRS-5 weights, 90 % requested retention, short-term
learning steps. The scheduler lives in `korean_core::scheduler::review`; game answers are converted
to FSRS ratings (Again/Hard/Good/Easy) from correctness and response time
(`korean_core::scoring`). An interval ladder shipped first (schema 3); schema 5 converts its rungs
into FSRS stability so existing progress keeps its meaning.

## Alternatives considered
- SM-2: well known, but worse retention/effort trade-off.
- Leitner boxes: too coarse for mixed skills.

## Consequences
- Memory state stores phase, stability, difficulty, due date, scheduled days and review counts.
- Review logs are kept so parameters can be optimized later.
