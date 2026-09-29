-- User data. Content lives in separate, re-seedable tables; user tables reference content by
-- stable text ids and never cascade from content.

-- One row per answered exercise. Append-only history used for statistics and for optimizing the
-- scheduler later.
CREATE TABLE review_log (
    id          INTEGER PRIMARY KEY,
    item_id     TEXT    NOT NULL,
    skill       TEXT    NOT NULL,
    correct     INTEGER NOT NULL CHECK (correct IN (0, 1)),
    rating      INTEGER NOT NULL CHECK (rating BETWEEN 1 AND 4),
    elapsed_ms  INTEGER NOT NULL CHECK (elapsed_ms >= 0),
    answered_at INTEGER NOT NULL -- unix milliseconds
) STRICT;

CREATE INDEX review_log_item_skill ON review_log (item_id, skill);
CREATE INDEX review_log_answered_at ON review_log (answered_at);
