# Contributing

These rules apply to humans and coding agents alike.

## `main` is always usable

- Never commit directly to `main`. One feature = one branch = one PR.
- Branch names: `feat/<topic>`, `fix/<topic>`, `chore/<topic>`, `docs/<topic>`, `refactor/<topic>`.
- PRs are small, tested, reviewable and revertable. They are **squash merged**: every commit on
  `main` is one complete change, so `git revert <sha>` undoes exactly one feature.
- The PR template (What / Why / Architecture changes / Database changes / Tests / Screenshots /
  Known limitations) must be filled in.

## Conventional Commits

The PR title becomes the squash commit message and is checked in CI.

```
feat: add listening exercise engine
fix: preserve review state after interruption
refactor: isolate FSRS scheduling service
test: add scheduler regression tests
docs: document image provider architecture
chore: configure CI
```

Never `update stuff`, `changes`, `fix`, `working version`.

## Required checks

Every PR must pass (see `.github/workflows/ci.yml`):

| Area | Check |
| --- | --- |
| Frontend | `pnpm check` (svelte-check + tsc), `pnpm lint`, `pnpm test` |
| Rust | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` |
| Integration | migration tests, starter content validation, production build |

Run them locally with:

```sh
pnpm install
pnpm --filter client build   # frontend dist is embedded in the Rust binary
pnpm check && pnpm lint && pnpm test
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

## Database migrations

1. A migration is never modified once released; add a new one instead.
2. Prefer additive migrations (new tables/columns); destructive changes need an ADR.
3. The app backs up the database before applying any pending migration.
4. Every migration is covered by the migration tests.
5. User data (reviews, XP) lives in tables separate from bundled content, which is re-seeded freely.

## Releases

1. Update `CHANGELOG.md` and the version in `Cargo.toml` (`[workspace.package]`) and
   `apps/client/package.json` in the release PR.
2. After merge, tag `main`: `git tag vX.Y.Z && git push origin vX.Y.Z`.
3. `.github/workflows/release.yml` builds Linux and macOS bundles and publishes the GitHub Release.

## Architecture decisions

Significant decisions are recorded in `docs/adr/`. Read them before proposing a rewrite; supersede
an ADR with a new one instead of silently diverging from it.
