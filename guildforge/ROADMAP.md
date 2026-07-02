# Roadmap

> Long-range plan for GuildForge, from pre-alpha through 2.0. Driven by product
> value, not by feature count — each milestone must be *useful on its own*
> before the next one starts. Milestones are time-boxed to two engineering weeks
> by default; if a milestone overruns, it is re-scoped, not extended.

## Stability Policy

| Version | Stability |
|---|---|
| `0.0.x` | None. Anything may break. YAML schema, CLI, library APIs all mutable. |
| `0.x.0` | CLI commands are stable within a minor. YAML schema is stable within a minor. Library APIs (`crates/*`) are not stable. |
| `1.0.0` | CLI and YAML schema are stable across majors. Library APIs follow semver. Breaking schema changes require `guildforge migrate`. |
| `2.0.0` | Reserved for a future schema redesign or provider-protocol v2. |

## Milestones

### Phase 0 — Architecture & Foundations ✅

**Goal**: lock every cross-cutting decision before any logic is written.

- Cargo workspace layout
- Provider trait spec
- YAML schema v1 spec
- State store design
- Planner determinism design
- Error & diagnostics design
- CI skeleton
- 8 ADRs

**Exit criteria**: workspace compiles, all docs committed, ADRs reviewed.

### Phase 1 — Config Layer

**Goal**: take a YAML file, turn it into a fully-validated, strongly-typed
`Config` object, or produce a precise diagnostic explaining why it's invalid.

Crates: `config`, `parser`, `validation`, `shared`, `logging`.

- Serde models for every schema key
- YAML parser with span tracking
- Semantic validation (refs resolve, no dupes, hierarchy sane, API limits respected)
- `miette`-powered diagnostics with file:line:col
- Snapshot tests for every example in `examples/`
- `guildforge validate <file>` end-to-end

**Exit criteria**: `guildforge validate examples/company.yaml` exits 0;
`guildforge validate examples/broken/*.yaml` exits non-zero with a precise
diagnostic. ≥90% line coverage in the three crates above.

### Phase 2 — Discord Provider

**Goal**: a working `DiscordProvider` that can read and write every supported
Discord resource type through the public REST API.

Crates: `provider` (trait), `provider-discord`.

- `Provider` trait + shared resource types
- Discord REST client (reqwest + tokio)
- Rate-limit handling (global + per-route buckets, retry-after honoring)
- CRUD ops: roles, categories, text/voice/forum channels, permission overwrites,
  webhooks, invites, forum tags, channel reordering
- Mock HTTP layer for tests (Wiremock-rs)
- Live test harness behind `--features live-discord`
- Documented list of unsupported features (AutoMod rules, server guide, etc.)

**Exit criteria**: every CRUD op has both a mock test and (when token available)
a live test. Rate-limit behavior verified against Discord's documented buckets.

### Phase 3 — Planner & Executor

**Goal**: compute deterministic diffs and apply them safely.

Crates: `planner`, `executor`, `engine`, `state`.

- Resource graph (DAG) construction
- Deterministic diff (`+`, `~`, `-`, `>`, `=`)
- Topological execution order
- Idempotent apply with retry on transient failures
- Partial-failure rollback (mark tainted, continue, report)
- SQLite state store with locking
- `guildforge plan`, `guildforge apply`, `guildforge destroy`
- `guildforge doctor` (drift detection)

**Exit criteria**: full plan/apply/destroy cycle on `examples/company.yaml`
against a real Discord guild produces the expected end state and a clean
state file. Drift detection correctly flags out-of-band UI edits.

### Phase 4 — Import / Export / Diff

**Goal**: round-trip an existing guild through GuildForge.

- `guildforge import <guild-id>` reads live guild, emits YAML
- `guildforge export` emits YAML from state with stable ordering
- `guildforge diff <a.yaml> <b.yaml>` shows structural diff
- `guildforge backup` / `guildforge restore` snapshot state
- Snapshot tests for round-trip stability

**Exit criteria**: importing a moderately complex guild (50+ channels) and
re-exporting produces byte-identical YAML. Diff output is git-friendly.

### Phase 5 — Dashboard

**Goal**: full web UI exposing every CLI capability.

Stack: Next.js 16 (App Router, RSC), Tailwind 4, shadcn/ui, SQLite (shared with
CLI state).

- Login (token stored server-side, never sent to client)
- Server picker
- YAML editor with schema validation
- Visual plan viewer (tree + diff)
- Apply with live log streaming
- History (from state migrations table)
- Template browser

**Exit criteria**: every CLI workflow is reproducible from the dashboard. Live
apply log streams over WebSocket. No token ever reaches the browser.

### Phase 6 — Polish & 1.0

**Goal**: production-grade release.

- `cargo install guildforge` works on Linux, macOS, Windows
- Cross-compiled binaries + Homebrew formula + scoop manifest
- `guildforge login` uses OS keychain (keyring crate)
- Man pages (`guildforge.1`)
- Shell completions (bash, zsh, fish, powershell)
- Performance: `plan` on 500-channel guild < 1s
- Security review + threat-model pass
- Public docs site (mkdocs or mdbook)
- 1.0 release blog post + announcement

### Phase 7+ — Future

- Modules (compose multiple YAML files)
- Variables & outputs (Terraform-parity config)
- Slack / Teams / Mattermost providers
- Emoji & integration support (when Discord API allows)
- Remote state backends (S3, GCS, Postgres)
- Policy engine (OPA/Rego integration for permission governance)
- Multi-guild templating (deploy the same shape to N guilds)

## Versioning Milestones

| Tag | When |
|---|---|
| `v0.1.0` | End of Phase 1 — `validate` works |
| `v0.2.0` | End of Phase 2 — Discord provider usable as a library |
| `v0.3.0` | End of Phase 3 — plan/apply/destroy against real Discord |
| `v0.4.0` | End of Phase 4 — import/export round-trip |
| `v0.5.0` | End of Phase 5 — dashboard parity with CLI |
| `v1.0.0` | End of Phase 6 — production-ready |

## Deferment & Non-Goals

The following are explicitly **out of scope** for v1.0 and will not be added
even if requested:

- **Message content management.** GuildForge manages *structure*, not content.
  Backing up or migrating messages is a different product.
- **Bot hosting.** GuildForge is a CLI/dashboard that talks to the Discord REST
  API. It does not run a long-lived bot process or handle gateway events.
- **AutoMod rule CRUD.** Discord does not expose AutoMod rules through the
  public bot API as of 2024-12. We will document this and revisit if the API
  opens up. See [`docs/SCHEMA.md`](./docs/SCHEMA.md) "Known Limitations".
- **Voice channel region overrides.** Deprecated by Discord; out of scope.
