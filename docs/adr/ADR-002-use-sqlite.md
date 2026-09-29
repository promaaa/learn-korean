# ADR-002: Use SQLite via sqlx, with separated content and user data

## Context
The app stores memory states, review logs, XP and normalized content locally. It must be
rollbackable: installing an older release must not destroy user data.

## Decision
A single SQLite database in the platform app-data directory, accessed with `sqlx`. Embedded,
append-only migrations. Before applying pending migrations the app writes a consistent backup
(`VACUUM INTO`) to `backups/v<previous-version>-before-schema-<n>.db` (the release that last wrote the database is kept in `PRAGMA user_version`). Bundled content tables are
re-seeded from the packs on start; user tables are keyed by stable content ids.

## Alternatives considered
- JSON files: no transactions or queries.
- Embedded key-value stores: no relational queries for sessions and statistics.

## Consequences
- Released migrations are immutable; schema changes are additive by default.
- Rolling back a release = reinstall old version + restore the matching backup.
