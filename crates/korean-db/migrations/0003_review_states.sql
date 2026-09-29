-- Memory state of each (item, skill) card. User data: keyed by stable content ids, no foreign key
-- to the re-seedable content tables.
CREATE TABLE review_states (
    item_id        TEXT    NOT NULL,
    skill          TEXT    NOT NULL,
    step           INTEGER NOT NULL,
    due_at         INTEGER NOT NULL, -- unix milliseconds
    reps           INTEGER NOT NULL,
    lapses         INTEGER NOT NULL,
    last_review_at INTEGER NOT NULL, -- unix milliseconds
    PRIMARY KEY (item_id, skill)
) STRICT;

CREATE INDEX review_states_due ON review_states (due_at);
