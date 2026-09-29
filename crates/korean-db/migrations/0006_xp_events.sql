-- Append-only experience ledger. The total and the level are derived from it.
CREATE TABLE xp_events (
    id        INTEGER PRIMARY KEY,
    earned_at INTEGER NOT NULL, -- unix milliseconds
    amount    INTEGER NOT NULL CHECK (amount >= 0),
    item_id   TEXT    NOT NULL,
    skill     TEXT    NOT NULL
) STRICT;

CREATE INDEX xp_events_earned_at ON xp_events (earned_at);
