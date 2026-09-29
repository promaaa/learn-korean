-- Bundled content. These tables are owned by the app and re-seeded from the packs whenever a
-- pack's content hash changes. User tables reference items by id and are never touched by
-- seeding.

CREATE TABLE content_packs (
    id           TEXT PRIMARY KEY NOT NULL,
    version      INTEGER NOT NULL,
    title        TEXT NOT NULL,
    description  TEXT NOT NULL,
    unlock_level INTEGER NOT NULL,
    position     INTEGER NOT NULL,
    hash         TEXT NOT NULL
) STRICT;

CREATE TABLE content_items (
    id       TEXT PRIMARY KEY NOT NULL,
    pack_id  TEXT NOT NULL REFERENCES content_packs (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    kind     TEXT NOT NULL,
    korean   TEXT NOT NULL,
    english  TEXT NOT NULL,
    -- Full item as JSON (distractors, replies, chunks, image...), the source of truth for games.
    data     TEXT NOT NULL
) STRICT;

CREATE INDEX content_items_pack ON content_items (pack_id, position);
