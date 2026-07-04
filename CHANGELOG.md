# Changelog

All notable user-facing changes to GuildForge are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
once `v0.1.0` is tagged. Earlier versions have no stability guarantees.

## [Unreleased]

### Added — Phase 0 (Architecture & Foundations)

- Initial Cargo workspace with 11 library crates and 1 CLI app crate.
- `README.md` with project pitch, quick-start target UX, and repo layout.
- `ARCHITECTURE.md` — living architecture overview, component diagram, data
  flow, lifecycle of a `guildforge apply` invocation.
- `docs/SCHEMA.md` — full v1 YAML schema specification covering `server`,
  `roles`, `categories`, `channels`, `permissions`, `permission_overwrites`,
  `webhooks`, `invites`, `forum_tags`, `welcome_screen`, `server_guide`,
  `ordering`. Includes type tables, examples, and validation rules.
- `docs/CLI_REFERENCE.md` — every CLI subcommand, flags, exit codes, and
  environment variables.
- `docs/CRATE_LAYOUT.md` — per-crate responsibilities, dependency rules,
  public API surface guidelines.
- `docs/TESTING.md` — testing strategy, fixture conventions, coverage targets.
- `docs/SECURITY.md` — token handling, threat model, disclosure policy.
- Eight Architecture Decision Records under `docs/adr/`:
  - ADR-0001: Provider trait
  - ADR-0002: State store (SQLite + file lock)
  - ADR-0003: Planner determinism
  - ADR-0004: Config format (YAML, no modules in v1)
  - ADR-0005: Error model (Anyhow + ThisError + miette)
  - ADR-0006: Async runtime & HTTP (Tokio + Reqwest)
  - ADR-0007: Idempotency & ordering
  - ADR-0008: Dashboard ↔ engine binding (subprocess, not in-process)
- Engineering configs: `rustfmt.toml`, `clippy.toml`, `deny.toml`,
  `rust-toolchain.toml`, `.gitignore`.
- GitHub Actions CI workflow running `cargo fmt --check`,
  `cargo clippy -- -D warnings`, `cargo test --workspace`, and
  `cargo doc --workspace --no-deps`.
- Example YAML configs: `examples/company.yaml`, `examples/community.yaml`.
- Project management files: `PROJECT_STATE.md`, `ROADMAP.md`, `TASKS.md`,
  `CONTRIBUTING.md`, `DECISIONS.md`.

### Known Limitations (carried forward)

- No runtime behavior implemented yet — `guildforge` binary only responds to
  `--help` and `--version`.
- `apps/dashboard` is an empty directory with a placeholder README; the
  Next.js app is scaffolded in Phase 5.
- AutoMod rule CRUD is unsupported because Discord's public bot API does not
  expose it. Tracked in `docs/SCHEMA.md` → "Known Limitations".
- Emoji and integration management are deferred to a future phase.

### Breaking Changes

None. The project has no prior releases.

### Deprecated

None.

### Removed

None.

### Security

No security-relevant changes yet. See `docs/SECURITY.md` for the threat model
and disclosure process once code reaches alpha.
