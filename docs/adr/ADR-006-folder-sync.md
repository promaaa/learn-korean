# ADR-006: Sync progress through a shared folder, one snapshot per device

## Context
The learner uses several computers, one at a time, that already share a synced folder (MEGA,
Syncthing, Dropbox...). The app keeps running hidden on every one of them, so their processes are
alive concurrently even though only one is in use.

## Decision
The live database stays local (ADR-002). Each device writes only its own snapshot,
`<sync dir>/<device>.db`: a small SQLite file with the user tables (`review_log`, `review_states`,
`xp_events`, `settings`) and a `snapshot` row (device, schema version, export time). It is written
to a dot file then renamed when the window is hidden or loses focus, if progress changed. Every
other device's snapshot is merged when the app starts and each time the window is summoned:

- `review_log`, `xp_events`: union; a row is identified by card and time.
- `review_states`: the most recent `last_review_at` wins.
- `settings`: the most recent `updated_at` wins (migration 0008).
- `sync_peers` (local, never exported) records the last snapshot merged per device.

Snapshots from another schema version are skipped until both devices run the same release. The
folder is configured in `<app config dir>/sync.json` (`{ "dir": "..." }`); without it sync is off.

## Alternatives considered
- The database itself in the synced folder: two running processes and a sync tool copying WAL
  files independently corrupt SQLite; the tool would also create conflict copies.
- One shared snapshot: several writers, conflict copies, lost updates when a sync arrives late.
- A sync server over the network: a service to run for data a shared folder already moves.

## Consequences
- Merging is idempotent and order-independent: late, duplicate or out-of-order snapshots are safe.
- Progress made on a device reaches the others once that device hides or unfocuses the window and
  the folder has synced; a machine switched off while the window was focused shares on next use.
- Answers on two devices before either syncs are both kept; each card takes the latest review.
