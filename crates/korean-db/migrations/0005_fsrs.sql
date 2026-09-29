-- FSRS memory model (ADR-003). Additive: the ladder column `step` stays (unused from schema 5).
ALTER TABLE review_states ADD COLUMN phase INTEGER NOT NULL DEFAULT 0;
ALTER TABLE review_states ADD COLUMN stability REAL NOT NULL DEFAULT 0;
ALTER TABLE review_states ADD COLUMN difficulty REAL NOT NULL DEFAULT 0;
ALTER TABLE review_states ADD COLUMN scheduled_days INTEGER NOT NULL DEFAULT 0;

-- Convert ladder progress: step 0 was still learning (10 minutes), step n >= 1 waited the rung's
-- interval. Stability starts at that interval so the next FSRS review continues from there.
UPDATE review_states
SET phase = CASE WHEN step = 0 THEN 1 ELSE 2 END,
    scheduled_days = CASE step
        WHEN 0 THEN 0 WHEN 1 THEN 1 WHEN 2 THEN 3 WHEN 3 THEN 7 WHEN 4 THEN 16
        WHEN 5 THEN 35 WHEN 6 THEN 80 ELSE 180 END,
    stability = CASE step
        WHEN 0 THEN 0.5 WHEN 1 THEN 1 WHEN 2 THEN 3 WHEN 3 THEN 7 WHEN 4 THEN 16
        WHEN 5 THEN 35 WHEN 6 THEN 80 ELSE 180 END,
    difficulty = 5.0 + MIN(lapses, 5) * 0.5;
