# Project State

> Living snapshot of where GuildForge is right now.

## Current Phase

**Phase 2 — Discord Provider (complete)**

The Discord provider is fully implemented with mock-HTTP test coverage.
The `Provider` trait from Phase 0 is wired up to a working HTTP client
with rate-limit middleware, retry, and idempotent CRUD operations for
every supported Discord resource type.

| Capability | Status |
|---|---|
| Workspace layout & manifests | ✅ Done (Phase 0) |
| Provider trait spec | ✅ Done (Phase 0) + full impl (Phase 2) |
| YAML schema v1 | ✅ Done (Phase 0) |
| State store design | ✅ Specified (ADR-0002), not implemented |
| Planner design | ✅ Specified (ADR-0003), not implemented |
| Error & diagnostics design | ✅ Specified (ADR-0005) |
| CI pipeline | ✅ Workflow committed |
| `guildforge` binary | ✅ `version`, `validate`, `--help` work; others are stubs |
| `shared` crate | ✅ Done (Phase 1) |
| `logging` crate | ✅ Done (Phase 1) |
| `config` crate | ✅ Done (Phase 1) |
| `parser` crate | ✅ Done (Phase 1) |
| `validation` crate | ✅ Done (Phase 1) |
| CLI `validate` command | ✅ Done (Phase 1) |
| `provider` crate (trait + Resource types) | ✅ Done (Phase 2) |
| `provider-discord` HTTP client | ✅ Done (Phase 2) |
| Rate-limit middleware | ✅ Done (Phase 2) |
| Role CRUD | ✅ Done (Phase 2) |
| Category CRUD | ✅ Done (Phase 2) |
| Channel CRUD (text/voice/forum/announcement/stage) | ✅ Done (Phase 2) |
| Permission overwrite CRUD | ✅ Done (Phase 2) |
| Channel reordering | ✅ Done (Phase 2) |
| Webhook CRUD | ✅ Done (Phase 2) |
| Invite list/create/revoke | ✅ Done (Phase 2) |
| Forum tag CRUD | ✅ Done (Phase 2) |
| Welcome screen CRUD | ✅ Done (Phase 2) |
| Server guide CRUD (partial) | ✅ Done (Phase 2) — see [`docs/DISCORD_LIMITATIONS.md`](./docs/DISCORD_LIMITATIONS.md) |
| Mock-HTTP tests (wiremock) | ✅ 14 tests covering role/channel/category + retry + 4xx |
| Live tests | ❌ Not started (gated behind `--features live-discord`) |
| Discord limitations doc | ✅ [`docs/DISCORD_LIMITATIONS.md`](./docs/DISCORD_LIMITATIONS.md) |
| Planner | ❌ Not started (Phase 3) |
| Executor | ❌ Not started (Phase 3) |
| Dashboard | ❌ Not started (Phase 5) |

## Build & Test Status

- `cargo check --workspace` clean on Rust 1.88.
- `cargo test --workspace`: 193 tests pass across all crates.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo build --release` produces a working `guildforge` binary.

## Known Gaps

- `guildforge plan`, `apply`, `destroy`, `doctor`, `import`, `export`,
  `diff`, `backup`, `restore`, `login`, `logout`, `init` are stubs.
- No live Discord integration tests (require bot token; deferred to P2-015).
- Dashboard is an empty directory.
- Server guide (onboarding) full prompt editing is documented as a
  limitation in [`docs/DISCORD_LIMITATIONS.md`](./docs/DISCORD_LIMITATIONS.md).

## Compatibility Promises

The YAML schema and the `validate` command's exit codes are stable from
Phase 1 onward. The `Provider` trait and `Resource` enum are stable from
Phase 2 onward (per [ADR-0001](./docs/adr/ADR-0001-provider-trait.md)).
Other CLI commands, library APIs, and the JSON plan output format are
NOT stable yet.
