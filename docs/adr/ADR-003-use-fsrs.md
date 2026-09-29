# ADR-003: Schedule reviews with FSRS, as Anki does

## Context
Each (learning item, skill) pair needs its own review schedule. SM-2 (Anki's historical default)
is simple but less accurate than modern models. Anki's FSRS implementation is the most used and
best studied, so the scheduler follows it rather than inventing its own rules.

## Decision
Use FSRS-6 through `fsrs` (fsrs-rs, open-spaced-repetition, BSD-3), the library Anki itself uses,
with Anki's defaults (`korean_core::scheduler`):

- 90 % desired retention, 36 500-day maximum interval.
- Learning steps 1 min and 10 min, one 10 min relearning step after a lapse; Hard on the first
  step waits between the two, Easy graduates at once. The current step is stored in
  `review_states.step` (the old ladder column).
- Review intervals in whole days from FSRS, ordered Hard < Good < Easy, fuzzed with Anki's
  ranges and seeded per card and review. A learner day starts at 4 a.m. local time.
- Parameters optimized from the learner's own review log (Anki's training items: every answer
  given on a later day than the card's previous one). A fit replaces the current parameters only
  if it predicts the answers better (log loss), so with few reviews the defaults stay. The app
  refits at most daily, in the background, when the log grew; the parameters sync as the `fsrs`
  setting.
- Memory states (stability, difficulty) are replayed from the review log whenever parameters
  or the merged log may change; due dates are kept.
- Sessions follow Anki's queues: (re)learning cards first, including those due within the
  20-minute learn-ahead limit, then review cards by lowest retrievability; 20 new cards per day;
  one card per item per session so a sibling does not prime the other.

Game answers are converted to FSRS ratings (Again/Hard/Good/Easy) from correctness and response
time (`korean_core::scoring`); the optimizer learns how these ratings relate to recall. An
interval ladder shipped first (schema 3); schema 5 converted its rungs into FSRS stability, and the
replay now recomputes them from the log.

## Alternatives considered
- SM-2: well known, but worse retention/effort trade-off.
- Leitner boxes: too coarse for mixed skills.
- `rs-fsrs` (used until 0.6): FSRS-5 scheduling only, no optimizer, its own learning steps.

## Consequences
- Memory state stores phase, step, stability, difficulty, due date, scheduled days and review
  counts; the review log is the source of truth they are replayed from.
- The first sessions use the default parameters; scheduling becomes personal as reviews accumulate.
