# Tasks

> Live backlog. Updated whenever work starts, finishes, or is re-prioritized.
> Items are grouped by milestone; `[next]` marks the next-up task in each group.

## Conventions

- Every task has an ID of the form `P{phase}-{seq}` (e.g. `P1-003`).
- Status: `todo` / `doing` / `done` / `blocked` / `dropped`.
- "Blocked" tasks must link to the blocker in `Blocker`.
- "Dropped" tasks must link to the rationale in `DECISIONS.md`.

---

## Phase 0 — Architecture & Foundations

| ID | Task | Status | Notes |
|---|---|---|---|
| P0-001 | Author README + project pitch | done | `README.md` |
| P0-002 | Author ARCHITECTURE.md | done | `ARCHITECTURE.md` |
| P0-003 | Author YAML schema spec | done | `docs/SCHEMA.md` |
| P0-004 | Author CLI reference | done | `docs/CLI_REFERENCE.md` |
| P0-005 | Author crate layout doc | done | `docs/CRATE_LAYOUT.md` |
| P0-006 | Author testing strategy | done | `docs/TESTING.md` |
| P0-007 | Author security model | done | `docs/SECURITY.md` |
| P0-008 | Author ADR-0001 provider trait | done | `docs/adr/ADR-0001-provider-trait.md` |
| P0-009 | Author ADR-0002 state store | done | `docs/adr/ADR-0002-state-store.md` |
| P0-010 | Author ADR-0003 planner determinism | done | `docs/adr/ADR-0003-planner-determinism.md` |
| P0-011 | Author ADR-0004 config format | done | `docs/adr/ADR-0004-config-format.md` |
| P0-012 | Author ADR-0005 error model | done | `docs/adr/ADR-0005-error-model.md` |
| P0-013 | Author ADR-0006 async http | done | `docs/adr/ADR-0006-async-http.md` |
| P0-014 | Author ADR-0007 idempotency & ordering | done | `docs/adr/ADR-0007-idempotency-ordering.md` |
| P0-015 | Author ADR-0008 dashboard binding | done | `docs/adr/ADR-0008-dashboard-binding.md` |
| P0-016 | Commit Cargo workspace skeleton | done | 11 crates + cli app |
| P0-017 | Commit rustfmt + clippy + deny configs | done | `rustfmt.toml`, `clippy.toml`, `deny.toml` |
| P0-018 | Commit CI workflow | done | `.github/workflows/ci.yml` |
| P0-019 | Commit example YAML configs | done | `examples/company.yaml`, `examples/community.yaml` |
| P0-020 | Author CONTRIBUTING + DECISIONS + PROJECT_STATE | done | top-level docs |
| P0-021 | Commit LICENSE-MIT + LICENSE-APACHE | todo | blocked on legal sign-off |
| P0-022 | Wire cargo-deny into CI | todo | post-Phase-0 hardening |

---

## Phase 1 — Config Layer

| ID | Task | Status | Notes |
|---|---|---|---|
| P1-001 | Implement `shared` crate: `ResourceId`, `Hash`, time helpers | todo | [next] |
| P1-002 | Implement `logging` crate: `tracing_subscriber` init | todo | |
| P1-003 | Implement `config` crate: serde models for all schema keys | todo | depends P1-001 |
| P1-004 | Implement `parser` crate: YAML → `Config` with span tracking | todo | depends P1-003 |
| P1-005 | Implement `validation` crate: semantic validators + diagnostics | todo | depends P1-004 |
| P1-006 | Wire `guildforge validate <file>` end-to-end in `apps/cli` | todo | depends P1-005 |
| P1-007 | Snapshot tests for every example in `examples/` | todo | depends P1-006 |
| P1-008 | Property-based tests for parser round-trip | todo | `proptest` |
| P1-009 | ≥90% line coverage in `config`, `parser`, `validation` | todo | `cargo-tarpaulin` |
| P1-010 | Document public API with `cargo doc` publish step in CI | todo | |

---

## Phase 2 — Discord Provider

| ID | Task | Status | Notes |
|---|---|---|---|
| P2-001 | Implement `provider` crate: `Provider` trait, `Resource`, `ResourceAddr` | todo | [next] after Phase 1 |
| P2-002 | Implement `provider-discord` HTTP client wrapper | todo | reqwest + tokio |
| P2-003 | Implement rate-limit middleware (global + per-route) | todo | honor `X-RateLimit-*` + `Retry-After` |
| P2-004 | Implement Role CRUD | todo | |
| P2-005 | Implement Category CRUD | todo | channels of type `guild_category` |
| P2-006 | Implement Text / Voice / Forum channel CRUD | todo | |
| P2-007 | Implement permission overwrite CRUD | todo | |
| P2-008 | Implement channel reordering (`modify_channel_positions`) | todo | |
| P2-009 | Implement Webhook CRUD | todo | |
| P2-010 | Implement Invite listing + create + revoke | todo | |
| P2-011 | Implement Forum tag CRUD | todo | |
| P2-012 | Implement Welcome Screen + Server Guide read/update | todo | API-permitting |
| P2-013 | Document unsupported features list | todo | `docs/DISCORD_LIMITATIONS.md` |
| P2-014 | Mock HTTP test layer (wiremock-rs) | todo | |
| P2-015 | Live test harness behind `live-discord` feature | todo | requires bot token |
| P2-016 | Provider trait conformance test suite | todo | every provider must pass |

---

## Phase 3 — Planner & Executor

| ID | Task | Status | Notes |
|---|---|---|---|
| P3-001 | Implement `state` crate: SQLite schema + migrations | todo | [next] after Phase 2 |
| P3-002 | Implement state locking (file-lock on macOS/Linux, op-lock on Windows) | todo | |
| P3-003 | Implement `planner` crate: resource graph construction | todo | |
| P3-004 | Implement deterministic diff algorithm | todo | ADR-0003 |
| P3-005 | Implement plan renderer (text + JSON + SARIF) | todo | |
| P3-006 | Implement `executor` crate: topological apply | todo | |
| P3-007 | Implement retry, backoff, partial-failure, taint | todo | ADR-0007 |
| P3-008 | Implement `engine` crate: workflow orchestration | todo | parser→validate→plan→execute→state |
| P3-009 | Wire `guildforge plan` / `apply` / `destroy` | todo | |
| P3-010 | Implement `guildforge doctor` (drift detection) | todo | compare state vs live |
| P3-011 | End-to-end test: apply `examples/company.yaml` to real guild | todo | live |
| P3-012 | End-to-end test: apply twice → second is no-op | todo | idempotency |
| P3-013 | End-to-end test: out-of-band UI edit → `doctor` detects | todo | drift |

---

## Phase 4 — Import / Export / Diff

| ID | Task | Status | Notes |
|---|---|---|---|
| P4-001 | Implement `guildforge import <guild-id>` | todo | |
| P4-002 | Implement `guildforge export` with stable ordering | todo | |
| P4-003 | Implement `guildforge diff <a> <b>` | todo | |
| P4-004 | Implement `guildforge backup` / `restore` | todo | |
| P4-005 | Round-trip snapshot tests: import → export → byte-identical | todo | |
| P4-006 | Git-friendly YAML formatting rules (consistent quoting, ordering) | todo | |

---

## Phase 5 — Dashboard

| ID | Task | Status | Notes |
|---|---|---|---|
| P5-001 | Scaffold Next.js 16 app with Tailwind 4 + shadcn/ui | todo | |
| P5-002 | Token storage: server-side encrypted, never sent to client | todo | ADR-0008 |
| P5-003 | Login + session | todo | |
| P5-004 | Server picker | todo | |
| P5-005 | YAML editor with monaco + schema validation | todo | |
| P5-006 | Plan viewer (tree + diff) | todo | |
| P5-007 | Apply with WebSocket log streaming | todo | |
| P5-008 | History view from state migrations table | todo | |
| P5-009 | Template browser | todo | |
| P5-010 | E2E tests with Playwright | todo | |

---

## Phase 6 — Polish & 1.0

| ID | Task | Status | Notes |
|---|---|---|---|
| P6-001 | Cross-compile Linux/macOS/Windows binaries in CI | todo | |
| P6-002 | Homebrew tap + scoop manifest | todo | |
| P6-003 | OS keychain integration (`guildforge login`) | todo | `keyring` crate |
| P6-004 | Man pages | todo | `clap_mangen` |
| P6-005 | Shell completions | todo | `clap_complete` |
| P6-006 | Performance: 500-channel `plan` < 1s | todo | |
| P6-007 | Security review | todo | |
| P6-008 | Public docs site | todo | mdbook or mkdocs |
| P6-009 | 1.0 release | todo | 🎉 |

---

## Tech Debt & Hardening (cross-phase)

| ID | Task | Status | Notes |
|---|---|---|---|
| TD-001 | Add `cargo-deny` to CI | todo | |
| TD-002 | Add `cargo-audit` to CI | todo | |
| TD-003 | Add SBOM generation (cyclonedx) to release | todo | |
| TD-004 | Add dependency auto-bump PRs (dependabot) | todo | |
| TD-005 | Add coverage gate (≥90%) to CI | todo | after Phase 1 |
| TD-006 | Add `msrv` verification to CI | todo | |
| TD-007 | Add fuzzing for parser | todo | `cargo-fuzz` |
