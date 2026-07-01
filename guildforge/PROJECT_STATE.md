# Project State

> Living snapshot of where GuildForge is right now. Updated after every merged
> milestone. Forthcoming work lives in [`TASKS.md`](./TASKS.md); the long-range
> plan lives in [`ROADMAP.md`](./ROADMAP.md).

## Current Phase

**Phase 0 — Architecture & Foundations**

The Cargo workspace is laid out, the provider trait is specified, the YAML
schema is locked at v1, ADRs are written for every cross-cutting decision, and
the engineering toolchain (rustfmt, clippy, cargo-deny, CI) is configured. No
runtime behavior is implemented yet — every crate compiles to an empty library
so the workspace is green from day one.

| Capability | Status |
|---|---|
| Workspace layout & manifests | ✅ Done |
| Provider trait spec | ✅ Specified (ADR-0001), stubbed in code |
| YAML schema v1 | ✅ Specified ([`docs/SCHEMA.md`](./docs/SCHEMA.md)) |
| State store design | ✅ Specified (ADR-0002), not implemented |
| Planner design | ✅ Specified (ADR-0003), not implemented |
| Error & diagnostics design | ✅ Specified (ADR-0005), not implemented |
| CI pipeline | ✅ Workflow committed, runs `cargo check` |
| `guildforge` binary | ⚠️ Stubbed — `--version` and `--help` only |
| Config parsing | ❌ Not started (Phase 1) |
| Validation | ❌ Not started (Phase 1) |
| Discord provider | ❌ Not started (Phase 2) |
| Planner | ❌ Not started (Phase 3) |
| Executor | ❌ Not started (Phase 3) |
| Dashboard | ❌ Not started (Phase 5) |

## Build & Test Status

The workspace is expected to compile cleanly with `cargo check --workspace` and
pass `cargo fmt --check` and `cargo clippy -- -D warnings` from this commit
forward. CI enforces all three on every push.

## Known Gaps

- No `LICENSE-MIT` / `LICENSE-APACHE` files committed yet (Phase 0 follow-up).
- No example configs beyond `examples/company.yaml` (more added in Phase 1).
- Dashboard is an empty directory with a placeholder README (Phase 5).
- `cargo-deny` config exists but is not yet enforced in CI (Phase 0 follow-up).

## Compatibility Promises

None yet. The project is pre-alpha. Anything may change without notice until
v0.1.0 is tagged, at which point the YAML schema and CLI commands become
semver-stable per the policy in [`ROADMAP.md`](./ROADMAP.md).
