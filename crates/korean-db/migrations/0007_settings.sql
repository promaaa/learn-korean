-- Learner preferences, one row per key. User data: never touched by content seeding.
CREATE TABLE settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
) STRICT;
