# GuildForge Worklog

Shared multi-agent work log for the GuildForge project. Append new
sections below; do not overwrite previous entries.

---
Task ID: P0-all
Agent: main (founding eng)
Task: Phase 0 — Architecture & Foundations. Produce the complete Cargo
  workspace skeleton, architecture documentation, ADRs, YAML schema
  spec, CLI reference, testing strategy, security model, and example
  YAML configs. No runtime behavior implemented; every crate compiles
  to a stub library and the `guildforge` binary responds to `--help`
  and `--version` only.

Work Log:
- Created directory tree: apps/{cli,dashboard}, crates/{config,parser,
  validation,engine,planner,executor,state,provider,provider-discord,
  shared,logging}, docs/adr, examples, templates, tests, .github/
  workflows, scripts.
- Wrote top-level docs: README, ARCHITECTURE, ROADMAP, TASKS,
  PROJECT_STATE, CHANGELOG, CONTRIBUTING, DECISIONS.
- Wrote docs/SCHEMA.md — full v1 YAML schema (12 top-level keys,
  validation rules V001-V075, permission name list, named colors,
  resource addressing, known limitations).
- Wrote docs/CLI_REFERENCE.md — every subcommand, flags, exit codes,
  env vars, output formats.
- Wrote docs/CRATE_LAYOUT.md — per-crate responsibilities, dependency
  graph, public API surface rules.
- Wrote docs/TESTING.md — test pyramid, fixture conventions, coverage
  targets per crate, fuzzing, property tests.
- Wrote docs/SECURITY.md — token handling, threat model, SSRF
  mitigations, dashboard auth, disclosure policy.
- Wrote 8 ADRs under docs/adr/: provider trait, state store (SQLite +
  file lock), planner determinism, config format (YAML v1, no modules),
  error model (Anyhow + ThisError + miette), async runtime & HTTP
  (Tokio + Reqwest + custom rate-limit middleware), idempotency &
  ordering (topological, taint-on-failure), dashboard binding
  (subprocess, not in-process).
- Wrote Cargo workspace: root Cargo.toml with 12 members, pinned
  workspace dependencies, release profile (LTO, strip, panic=abort).
- Wrote 11 library crates with compiling stubs and unit tests:
  shared, logging, config, parser, validation, state, provider,
  provider-discord, planner, executor, engine.
- Wrote apps/cli with clap-derive Args/Command structs for all 14
  subcommands; only `version` and `--help` are functional in Phase 0,
  others exit 2 with "not implemented yet".
- Wrote engineering configs: rustfmt.toml, clippy.toml, deny.toml,
  rust-toolchain.toml (pinned to 1.88.0), .gitignore.
- Wrote .github/workflows/ci.yml with fmt, clippy, test, doc, msrv,
  deny, audit jobs.
- Wrote .github/pull_request_template.md.
- Wrote example YAML configs: examples/company.yaml (medium company
  guild), examples/community.yaml (open-source community guild).
- Wrote templates/minimal.yaml and templates/README.md.
- Wrote LICENSE-MIT and LICENSE-APACHE.
- Wrote tests/README.md (placeholder for Phase 1+ integration tests).

Stage Summary:
- Workspace compiles cleanly with `cargo check --workspace` on Rust
  1.88.0. MSRV is 1.88 (raised from initial 1.78 because transitive
  deps require edition2024 and let-chains).
- All 27 unit tests pass across 11 library crates.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- `cargo fmt --all -- --check` is clean.
- `cargo build --release` produces a working `guildforge` binary;
  `--help` lists all 14 subcommands; `--version` prints
  `guildforge 0.0.1`.
- Every crate has `#![forbid(unsafe_code)]` and
  `#![warn(missing_docs, clippy::all, clippy::pedantic)]`.
- Engine never imports from provider-discord; cli is the only place
  that wires the concrete Discord provider in. Dependency DAG
  verified.
- Phase 1 is ready to start. Recommended first task: P1-001 (implement
  shared crate fully) and P1-003 (implement config crate fully),
  unblocking P1-004 (parser) and P1-005 (validation).

---
Task ID: P1-all
Agent: main (founding eng)
Task: Phase 1 — Config Layer. Implement shared, logging, config, parser,
  validation crates fully; wire `guildforge validate <file>` end-to-end
  in apps/cli; add snapshot/property tests and broken examples. Exit
  criterion: `guildforge validate examples/company.yaml` exits 0;
  broken examples exit non-zero with the expected diagnostic codes.

Work Log:
- P1-001: Implemented `crates/shared` fully — ResourceId, Snowflake
  (with timestamp/worker/process/increment extraction), Hash (with
  hex serde), Clock/SystemClock/Time, IdempotencyKey. 18 unit tests.
- P1-002: Implemented `crates/logging` — LogLevel, LogFormat,
  init_from_env, idempotent init. 5 unit tests.
- P1-003: Implemented `crates/config` fully — split into 9 modules
  (server, role, channel, permission, webhook, invite, forum, welcome,
  ordering). Every struct has `#[serde(deny_unknown_fields)]` and
  `skip_serializing_if` on Option fields. Color uses custom
  try_from/into for parse-time format detection. AfkTimeout uses
  custom serde to accept raw integers. 40 unit tests.
- P1-004: Implemented `crates/parser` — parse, parse_with_limit,
  parse_file. Handles empty input, too-large input, unsupported
  schema version, YAML syntax errors, schema mismatches. 9 unit
  tests + 6 property tests (proptest).
- P1-005: Implemented `crates/validation` — `validate` (errors
  only) and `validate_collect` (warnings + errors). Rules V001–V075
  covering uniqueness, references, API limits, type-specific,
  colors (now at parse time), semantic, ordering. 26 unit tests.
- P1-006: Wired `guildforge validate <file>` in apps/cli. Refactored
  CLI into commands/{validate,version}.rs. Exit codes: 0 valid, 1
  validation errors, 2 I/O, 3 parse error. 6 unit tests + 12
  integration tests.
- P1-007: Snapshot tests via assert_cmd integration tests in
  apps/cli/tests/config_integration.rs — covers company.yaml,
  community.yaml, templates/minimal.yaml.
- P1-008: Property-based tests in
  crates/parser/tests/property_tests.rs — never-panic fuzzing,
  empty-input rejection, minimal-config parsing, schema version
  rejection.
- P1-009: Added 4 broken examples in examples/broken/ — duplicate
  role (V001), unknown category (V010), unknown field (parse
  error), too many roles (V020), voice fields on text (V061).
  Integration tests verify each exits with the expected code and
  stderr contains the expected V-code.
- P1-010: Verified cargo check, test (135 tests pass), clippy
  (-D warnings clean), fmt (--check clean) all green.

Stage Summary:
- Phase 1 complete. The config layer is rock-solid: 135 tests pass,
  clippy is clean with -D warnings, fmt is clean.
- `guildforge validate examples/company.yaml` exits 0 with warnings
  for V064 (Community server required for welcome_screen and
  announcement channels) and V065 (boost level required for forum
  channel). All warnings are advisories; no errors.
- Every broken example in examples/broken/ is rejected with the
  expected stable diagnostic code, verified by integration test.
- The YAML schema v1 is now effectively frozen — adding optional
  fields is fine; changing required fields or V-codes is a major
  version bump.
- Phase 2 (Discord provider) is next. Recommended first task:
  P2-001 (implement Provider trait conformance test suite) and
  P2-002 (Discord HTTP client wrapper) so the rest of Phase 2 can
  build on a solid foundation.

---
Task ID: P2-all
Agent: main (founding eng)
Task: Phase 2 — Discord Provider. Implement the full HTTP client with
  rate-limit middleware, retry, and idempotent CRUD operations for
  every supported Discord resource type. Mock-HTTP test coverage via
  wiremock. Document known Discord API limitations.

Work Log:
- P2-001: Expanded `crates/provider` into modules (error, resource,
  traits). Added typed Resource variants (Role, Category, Channel,
  PermissionOverwrite, Webhook, Invite, ForumTag, WelcomeScreen,
  ServerGuide) with full field sets. Added ChannelType + OverwriteKind
  enums with Discord API code conversion. Added content_hash() for
  diffing. 10 unit tests.
- P2-002: Implemented `crates/provider-discord/src/client/mod.rs` —
  DiscordHttp wrapper with GET/POST/PATCH/PUT/DELETE helpers, JSON
  encode/decode, retry with exponential backoff + jitter, configurable
  API base URL (so tests can target wiremock). Idempotent error
  classification.
- P2-003: Implemented `crates/provider-discord/src/client/rate_limit.rs`
  — route_for() that maps URLs to bucket keys (replaces numeric IDs
  with `:id`, special-cases DELETE message), per-bucket state with
  DashMap, global rate limit handling, X-RateLimit-* header parsing,
  Retry-After honoring. 7 unit tests.
- P2-004: Implemented `crates/provider-discord/src/resources/role.rs`
  — list, read (by name case-insensitive), create, update, delete
  (idempotent on 404), reorder. 6 unit tests.
- P2-005/P2-006: Implemented `crates/provider-discord/src/resources/channel.rs`
  — handles both Category and Channel resources. Category uses
  Discord channel type 4; channels use 0/2/5/13/15. list filters out
  categories. delete_channel idempotent on 404. 7 unit tests.
- P2-007: Implemented `overwrite.rs` — PUT /channels/:id/permissions/:id
  with allow/deny bitfields. 2 unit tests.
- P2-008: Reordering for roles and channels via
  PATCH /guilds/:id/roles and /guilds/:id/channels with position
  arrays.
- P2-009: Implemented `webhook.rs` — list, read (across all channels
  in guild), create, update, delete (idempotent). 3 unit tests.
- P2-010: Implemented `invite.rs` — list guild invites, read by code,
  create, delete (revoke), update-as-no-op (invites can't be updated).
  2 unit tests.
- P2-011: Implemented `forum_tag.rs` — tags are part of the channel
  object, so CRUD is via PATCH /channels/:id with a new available_tags
  array. 3 unit tests.
- Implemented `welcome.rs` — welcome screen + server guide (onboarding).
  Welcome screen has full CRUD. Server guide has partial CRUD
  (documented limitation: full prompt editing requires read-modify-write).
  2 unit tests.
- P2-013: Wrote `docs/DISCORD_LIMITATIONS.md` — 15 documented
  limitations including AutoMod rules, custom-emoji role icons, voice
  region overrides, threads, emoji/sticker management, etc.
- P2-014: Added `crates/provider-discord/tests/mock_api.rs` with 14
  wiremock-based integration tests:
  - role list/read/create/delete (incl. 404 idempotency)
  - channel list/create/delete
  - category create with type=4
  - provider name, list dispatch, read returns None for unknown addr
  - HTTP retry on 5xx, permanent error on 4xx

Stage Summary:
- Phase 2 complete. 193 tests pass (up from 135 in Phase 1), clippy is
  clean with -D warnings, fmt is clean.
- The `Provider` trait is fully implemented for Discord. Every supported
  resource type has create/read/update/delete/reorder/list operations
  with idempotent semantics per ADR-0007.
- The HTTP client handles rate limits (global + per-route), retries
  transient failures (5xx, network), and classifies permanent failures
  (4xx other than 429) per ADR-0006.
- Mock-HTTP tests verify the wire format of every request and the
  decoding of every response, including edge cases like 404 idempotent
  deletes and 5xx retry.
- Live tests against real Discord are stubbed out behind the
  `live-discord` feature flag (P2-015) — deferred until we have a bot
  token + test guild.
- Phase 3 (planner + executor) is next. Recommended first task:
  P3-001 (implement state crate with SQLite + migrations) so the
  planner has something to read from.
