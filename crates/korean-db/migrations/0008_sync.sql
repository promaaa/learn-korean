-- Progress sync between devices (ADR-006).

-- A setting's change time, so the latest choice made on any device wins.
ALTER TABLE settings ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0; -- unix milliseconds

-- Local bookkeeping, never exported: the last snapshot merged from each other device.
CREATE TABLE sync_peers (
    device      TEXT PRIMARY KEY NOT NULL,
    exported_at INTEGER NOT NULL -- unix milliseconds, as written in the peer's snapshot
) STRICT;
