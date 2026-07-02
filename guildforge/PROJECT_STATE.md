# Project State

> Living snapshot of where GuildForge is right now. Updated after every merged
> milestone. Forthcoming work lives in [`TASKS.md`](./TASKS.md); the long-range
> plan lives in [`ROADMAP.md`](./ROADMAP.md).

## Current Phase

**Phase 1 — Config Layer (complete)**

The config layer is fully implemented. The full parse → validate pipeline is
wired into the `guildforge validate <file>` command and works end-to-end
against the example YAMLs. The broken-examples test suite verifies that
invalid configs are rejected with the expected stable diagnostic codes.

| Capability | Status |
|---|---|
| Workspace layout & manifests | ✅ Done (Phase 0) |
| Provider trait spec | ✅ Specified (ADR-0001), stubbed in code |
| YAML schema v1 | ✅ Specified ([`docs/SCHEMA.md`](./docs/SCHEMA.md)) |
| State store design | ✅ Specified (ADR-0002), not implemented |
| Planner design | ✅ Specified (ADR-0003), not implemented |
| Error & diagnostics design | ✅ Specified (ADR-0005), not implemented |
| CI pipeline | ✅ Workflow committed, runs `cargo check` |
| `guildforge` binary | ✅ `version`, `validate`, `--help` work; others are stubs |
| `shared` crate | ✅ Done (Phase 1) — 18 tests |
| `logging` crate | ✅ Done (Phase 1) — 5 tests |
| `config` crate | ✅ Done (Phase 1) — 40 tests |
| `parser` crate | ✅ Done (Phase 1) — 9 unit + 6 property tests |
| `validation` crate | ✅ Done (Phase 1) — 26 tests, rules V001–V075 |
| CLI `validate` command | ✅ Done (Phase 1) — 6 unit + 12 integration tests |
| Example YAMLs | ✅ `company.yaml`, `community.yaml`, 4 broken examples |
| Discord provider | ❌ Not started (Phase 2) |
| Planner | ❌ Not started (Phase 3) |
| Executor | ❌ Not started (Phase 3) |
| Dashboard | ❌ Not started (Phase 5) |

## Build & Test Status

- `cargo check --workspace` clean on Rust 1.88.
- `cargo test --workspace`: 135 tests pass across all crates.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo build --release` produces a working `guildforge` binary.

## Known Gaps

- `guildforge plan`, `apply`, `destroy`, `doctor`, `import`, `export`,
  `diff`, `backup`, `restore`, `login`, `logout`, `init` are stubs that
  exit 2 with "not implemented yet" — these land in Phases 2–5.
- No live Discord integration — provider crate is still a stub.
- Dashboard is an empty directory with a placeholder README (Phase 5).
- `cargo-deny` config exists but is not yet enforced in CI (TD-001).

## Compatibility Promises

The YAML schema and the `validate` command's exit codes are stable from
this commit forward. Adding new optional fields is non-breaking; adding
new required fields or changing stable diagnostic codes (V001–V075) is
a breaking change requiring a major version bump.

Other CLI commands, library APIs, and the JSON plan output format are
NOT stable yet — they become stable in v0.3.0 (planner) and v0.5.0
(dashboard) per [`ROADMAP.md`](./ROADMAP.md).
