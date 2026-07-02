# Architecture

> Living architecture document for GuildForge. Read this first. Updated
> whenever a structural change lands; if a change is significant enough, it
> also gets an [ADR](./docs/adr/).

## 1. Purpose

GuildForge is **Terraform for Discord**. You write a single YAML file declaring
the desired state of a Discord guild — roles, categories, channels, permission
overwrites, webhooks, forum tags, ordering — and GuildForge computes the
minimal diff against the live guild and applies it safely, idempotently, and
with a full audit trail.

The product is open-source, single-binary, local-state-first, and provider-
pluggable. Discord is the first provider; Slack, Mattermost, and MS Teams are
planned for later phases. The engine must never know which provider it is
talking to.

## 2. Design Principles

1. **Declarative, not imperative.** The YAML describes *what* the guild should
   look like. GuildForge figures out *how* to get there.
2. **Idempotent.** Running `apply` twice produces the same end state and the
   second run is a no-op.
3. **Deterministic.** The same config + same live state always produces the
   same plan, byte-for-byte. This makes plans reviewable in PRs.
4. **Provider-pluggable.** Discord lives behind a trait. The engine, planner,
   executor, and state store are provider-agnostic.
5. **Local-first.** State lives in a single SQLite file by default. Remote
   backends are a Phase 7+ concern.
6. **Safe by default.** Destructive operations (`destroy`, channel deletes)
   require explicit confirmation. Partial failures do not corrupt state.
7. **Observable.** Every operation emits structured `tracing` logs. Plans and
   applies are reproducible from logs alone.
8. **Single static binary.** The CLI ships as one Rust binary. No runtime
   dependencies, no Docker required.

## 3. High-Level Component Diagram

```
                            ┌─────────────────────────────┐
   guildforge CLI  ───────▶ │          engine             │
   (clap, apps/cli)         │  (orchestrates workflow)    │
                            └──────────────┬──────────────┘
                                           │
        ┌──────────────────┬───────────────┼────────────────┬──────────────────┐
        ▼                  ▼               ▼                ▼                  ▼
   ┌─────────┐       ┌──────────┐    ┌──────────┐    ┌────────────┐     ┌──────────┐
   │ parser  │──────▶│validation│───▶│ planner  │───▶│  executor  │────▶│ provider │
   └─────────┘       └──────────┘    └──────────┘    └────────────┘     └────┬─────┘
        ▲                                                                  │
        │                                                                  ▼
   ┌─────────┐                                                      ┌─────────────┐
   │ config  │                                                      │ Discord HTTP│
   │ (types) │                                                      └─────────────┘
   └─────────┘

                            ┌─────────────────────────────┐
                            │           state             │
                            │  (SQLite, read by planner,  │
                            │   written by executor)      │
                            └─────────────────────────────┘
```

## 4. The Pipeline

A `guildforge apply config.yaml` invocation flows through the pipeline below.
Every stage is fallible; failures produce a `miette` diagnostic and the
process exits non-zero.

### Stage 1 — Parse

`crates/parser` reads the YAML file and produces a strongly-typed
`Config` object (defined in `crates/config`). Spans are preserved so that
later stages can emit diagnostics that point to the exact line and column.

- YAML deserialization is via `serde_yaml`.
- Unknown keys are errors, not silent drops.
- The parser does **no** semantic validation — that is Stage 2.

### Stage 2 — Validate

`crates/validation` runs a battery of semantic checks on the `Config`:

- All role / category / channel references resolve.
- No duplicate names within a namespace (role names, channel names within a
  category, etc.).
- Colors are valid Discord colors (hex or named).
- Permission strings are valid Discord permission names.
- Hierarchy is sane (categories reference channels that exist, no cycles in
  any future `depends_on` graph).
- Discord API limits are respected (max 250 roles, max 500 channels per guild,
  max 50 categories, etc.).
- Channel type matches category membership (voice channels can be in a
  category; forum channels can be in a category; categories cannot be nested).

Diagnostics carry file spans and are rendered with `miette` for nice
multi-line error output.

### Stage 3 — Plan

`crates/planner` compares the desired `Config` against the **current state**
(read from SQLite via `crates/state`) and produces an `ExecutionPlan` — a
deterministic, topologically-ordered list of `Operation`s.

Each operation is one of:

| Symbol | Meaning |
|---|---|
| `+` | Create a resource that exists in config but not in state. |
| `~` | Update a resource that exists in both but has changed fields. |
| `-` | Delete a resource that exists in state but not in config. |
| `>` | Reorder a resource (position changed). |
| `=` | No change. |

Planner determinism is governed by [ADR-0003](./docs/adr/ADR-0003-planner-determinism.md).
The same `(config, state)` pair always produces the same plan, byte-for-byte.

The planner never talks to Discord. It only reads state. Drift detection
(comparing state to live) is a separate `guildforge doctor` workflow.

### Stage 4 — Execute

`crates/executor` walks the `ExecutionPlan` in topological order, invoking the
appropriate `Provider` method for each operation. The executor is responsible
for:

- **Ordering.** Resources are applied in dependency order. Roles before
  channels. Categories before channels within them. Channels before permission
  overwrites on them.
- **Retry & backoff.** Transient HTTP failures (5xx, 429, network) are
  retried with exponential backoff and jitter. Permanent failures (4xx other
  than 429) are not retried.
- **Rate limiting.** The provider enforces Discord's per-route and global
  rate limits; the executor does not need to.
- **Partial failures.** If an operation fails permanently, the resource is
  marked `tainted` in state, the executor continues with the rest of the plan
  (where dependencies allow), and a non-zero exit is returned at the end.
- **Idempotency.** Every operation is idempotent at the provider level. A
  retry that hits "already exists" is treated as success.

### Stage 5 — State

`crates/state` persists the result of every successful operation to a local
SQLite file (default: `./guildforge.db`, overridable with `--state-file`).
State is the source of truth for the next `plan` run. State schema is
versioned with migrations; old state files auto-upgrade on first open.

State locking is via a SQLite transaction plus a file lock to prevent
concurrent `apply` runs from corrupting state. See
[ADR-0002](./docs/adr/ADR-0002-state-store.md).

## 5. The Provider Abstraction

The single most important architectural decision in GuildForge is the
`Provider` trait. The engine, planner, executor, and state store never
import from `crates/provider-discord`. They import from `crates/provider`,
which defines the trait and the shared resource types. Discord is one
implementation.

```rust
// crates/provider/src/lib.rs (sketch — see ADR-0001 for full spec)
#[async_trait]
pub trait Provider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn read(&self, addr: &ResourceAddr) -> Result<Option<Resource>, Self::Error>;
    async fn create(&self, desired: &Resource) -> Result<Resource, Self::Error>;
    async fn update(&self, current: &Resource, desired: &Resource) -> Result<Resource, Self::Error>;
    async fn delete(&self, current: &Resource) -> Result<(), Self::Error>;
    async fn reorder(&self, addr: &ResourceAddr, new_position: u32) -> Result<(), Self::Error>;
}
```

Full rationale, alternatives, and consequences in
[ADR-0001](./docs/adr/ADR-0001-provider-trait.md).

## 6. Crate Dependency Rules

```
            ┌─────────────┐
            │   shared    │   ← no deps, no I/O
            └──────┬──────┘
                   │
   ┌───────────────┼─────────────────┐
   ▼               ▼                 ▼
┌──────┐       ┌────────┐       ┌─────────┐
│config│       │logging │       │ provider│   ← trait only
└──┬───┘       └────────┘       └────┬────┘
   │                                  │
   ▼                                  │
┌──────┐                              │
│parser│                              │
└──┬───┘                              │
   │                                  │
   ▼                                  │
┌──────────┐                          │
│validation│                          │
└──┬───────┘                          │
   │                                  │
   ▼                                  ▼
┌──────┐                          ┌──────────────────┐
│state │ ◀─────────────────────── │ provider-discord │
└──┬───┘                          └──────────────────┘
   │                                  │
   ▼                                  │
┌─────────┐                            │
│ planner │ ◀──────────────────────────┘
└──┬──────┘
   │
   ▼
┌──────────┐
│ executor │
└──┬───────┘
   │
   ▼
┌────────┐
│ engine │
└──┬─────┘
   │
   ▼
┌──────┐
│ cli  │
└──────┘
```

Hard rules:

- `shared` depends on nothing.
- `provider` (trait) depends only on `shared`.
- `provider-discord` depends on `provider`, `shared`, and external crates only.
- `engine` depends on `parser`, `validation`, `planner`, `executor`, `state`,
  `provider`, `config`, `logging`. **Never** on `provider-discord`.
- `cli` depends on `engine` and `provider-discord` (to wire the concrete
  provider in at startup).
- No crate depends on `cli`.
- No crate depends on `engine` except `cli`.
- No back-links. The graph above is a DAG.

Per-crate responsibilities are documented in
[`docs/CRATE_LAYOUT.md`](./docs/CRATE_LAYOUT.md).

## 7. State Lifecycle

```
        ┌──────────────┐
        │  config.yaml │  (desired state, declared)
        └──────┬───────┘
               │
               ▼
        ┌──────────────┐
        │    Config    │  (parsed + validated)
        └──────┬───────┘
               │
               ▼
   ┌─────────────────────┐         ┌──────────────────┐
   │   ExecutionPlan     │ ◀─────  │  current state   │  (from SQLite)
   └──────────┬──────────┘         └──────────────────┘
              │
              ▼
   ┌─────────────────────┐         ┌──────────────────┐
   │     executor        │ ──────▶ │   applied state  │  (intermediate)
   └──────────┬──────────┘         └──────────────────┘
              │
              ▼
        ┌──────────────┐
        │  new state   │  (committed to SQLite)
        └──────────────┘
```

State holds the **authoritative record of what GuildForge last applied**.
Live Discord state may diverge (via out-of-band UI edits); `guildforge doctor`
detects this. State is *not* a cache — it is the contract between plan runs.

## 8. Lifecycle of a Single `apply` Invocation

1. **CLI parses args.** `clap` validates flags; `--state-file`, `--provider`,
   `--token-file`, `--auto-approve` are read.
2. **Engine acquires state lock.** SQLite transaction + file lock. If the
   lock is held, exit with a clear message.
3. **Engine reads current state** from SQLite.
4. **Engine parses config** via `parser`.
5. **Engine validates config** via `validation`. If invalid, emit diagnostics,
   release lock, exit non-zero.
6. **Engine asks planner** for an `ExecutionPlan` given `(config, current_state)`.
7. **Engine renders plan** to the terminal (or JSON / SARIF if `--format`).
8. **If not `--auto-approve`, prompt the user.** Plan can be aborted here
   without committing anything.
9. **Engine hands plan to executor.** Executor walks the plan in topo order,
   invoking the provider for each op. Every successful op updates the
   in-memory state; every failure marks the resource tainted and continues.
10. **Engine commits the in-memory state** to SQLite in a single transaction.
11. **Engine releases the lock.**
12. **CLI prints summary** (`+ N created, ~ M updated, - K deleted, > P reordered`)
    and exits 0 if no failures, non-zero otherwise.

## 9. Error Model

Three layers, per [ADR-0005](./docs/adr/ADR-0005-error-model.md):

- **Library crates** (`config`, `parser`, `validation`, `state`, `planner`,
  `executor`, `provider`, `provider-discord`): typed errors via `thiserror::Error`.
  Each crate has its own `Error` enum. Conversions are explicit.
- **Engine and CLI**: `anyhow::Result` for ergonomic error chaining. The
  engine never re-exposes typed errors to the CLI; it converts them to
  `anyhow::Error` at the crate boundary.
- **Diagnostics**: `miette` is used to attach spans, labels, and source
  context to errors that originate from user input (YAML, CLI args).
  Library errors that originate from the engine itself (e.g. state lock
  failure) are rendered as plain `anyhow` chain reports.

Error messages are lowercase, no trailing punctuation, include enough context
to debug without re-running.

## 10. Security Model

- The Discord bot token is the only long-lived secret. It is **never** written
  to state, logs, or plan output.
- `guildforge login` stores the token in the OS keychain (Phase 6) or, until
  then, in `~/.config/guildforge/token` with mode 0600.
- The dashboard never sends the token to the browser; it stores it server-side
  encrypted and proxies Discord API calls.
- State files do not contain tokens. They contain resource IDs, names, and
  configuration values only.
- See [`docs/SECURITY.md`](./docs/SECURITY.md) for the full threat model.

## 11. Performance Targets

| Operation | Target | Notes |
|---|---|---|
| `validate` on 100-channel guild | < 50 ms | pure CPU |
| `plan` on 500-channel guild | < 1 s | depends on state size |
| `apply` on 500-channel guild (no changes) | < 2 s | doctor-style read |
| `apply` on 500-channel guild (all new) | < 60 s | bound by Discord rate limits |
| Binary cold start | < 20 ms | no heavy init |
| Binary size | < 10 MB (stripped, release) | |

## 12. Extensibility Points

The architecture defines these explicit extension points; everything else is
frozen:

1. **New providers** — implement the `Provider` trait in a new
   `provider-<name>` crate. Wire it into the CLI in `apps/cli/src/main.rs`.
   No engine changes.
2. **New resource types** — add a variant to `ResourceKind` in
   `crates/provider`, add corresponding serde model in `crates/config`,
   add planner support, add provider implementations. Each step is localized.
3. **New state backends** — abstract `state::Store` behind a trait (Phase 7+).
   SQLite is the only implementation for v1.
4. **New output formats** — the planner emits an intermediate `ExecutionPlan`
   struct; new renderers (JSON, SARIF, HTML) are pure functions over that
   struct.
5. **Policy hooks** — `validation` already runs semantic checks; a future
   policy engine (OPA/Rego) plugs in here without touching the planner.

## 13. Non-Goals (Phase 1.0)

- Message content management. We manage *structure*, not messages.
- Long-lived bot process. GuildForge is a CLI, not a daemon.
- Gateway event handling. Read-only REST only.
- AutoMod rule CRUD (Discord API limitation; documented in
  [`docs/SCHEMA.md`](./docs/SCHEMA.md)).
- Multi-tenant dashboard auth. Single-user dashboard in v1.

## 14. Glossary

- **Config** — the parsed, validated `Config` struct from a YAML file.
- **State** — the SQLite record of what GuildForge last applied.
- **Plan** — the deterministic diff between config and state.
- **Provider** — an implementation of the `Provider` trait; Discord is one.
- **Resource** — a single addressable unit (role, channel, etc.).
- **ResourceAddr** — a stable identifier for a resource within a provider
  (e.g. `discord://guild/role/Admin`).
- **Tainted** — a resource whose last apply failed; will be recreated on next
  apply.
- **Drift** — divergence between state and live Discord, caused by
  out-of-band edits.
