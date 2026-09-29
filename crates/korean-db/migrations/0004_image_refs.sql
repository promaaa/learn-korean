-- Photo references resolved per search query. Metadata only: image bytes are never stored
-- (ADR-005). A row with NULL image_url records that nothing suitable was found.
CREATE TABLE image_refs (
    query       TEXT PRIMARY KEY NOT NULL,
    provider    TEXT,
    image_url   TEXT,
    source_url  TEXT,
    attribution TEXT,
    resolved_at INTEGER NOT NULL -- unix milliseconds
) STRICT;
