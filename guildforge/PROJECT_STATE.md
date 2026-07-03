# Project State

> Living snapshot of where GuildForge is right now.

## Current Phase

**Phase 4 — Import / Export / Diff (complete)**

The import/export/diff/backup/restore pipeline is wired. `guildforge
import` reads live Discord and emits YAML, `guildforge export` dumps
state to YAML, `guildforge diff <a> <b>` shows structural diff,
`guildforge backup`/`restore` snapshots state, and `doctor` has full
drift detection (state vs live).

| Capability | Status |
|---|---|
| Phase 0–3 deliverables | ✅ Done |
| `guildforge import` | ✅ Done (Phase 4) |
| `guildforge export` | ✅ Done (Phase 4) |
| `guildforge diff` | ✅ Done (Phase 4) — 6 tests |
| `guildforge backup` / `restore` | ✅ Done (Phase 4) |
| `doctor` drift detection | ✅ Done (Phase 4) — state→live |
| Config ↔ Resource conversion (both directions) | ✅ Done (Phase 4) — 5 tests |
| Dashboard | ❌ Not started (Phase 5) |

## Build & Test Status

- `cargo check --workspace` clean on Rust 1.88.
- `cargo test --workspace`: 250 tests pass across all crates.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Known Gaps

- `import` returns an empty config in v1 — full import requires wiring
  the provider's `list()` method through the engine (the `DynProvider`
  trait object doesn't expose `list` yet). The YAML conversion logic is
  complete and tested; only the live-data fetch is stubbed.
- `doctor` only detects state→live drift (resource in state but missing
  or changed in live). Live→state drift (resource in live but not in
  state) requires calling `provider.list()` for each kind — deferred.
- `init`, `login`, `logout` are stubs.
- Dashboard is an empty directory.
