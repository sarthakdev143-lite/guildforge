# Crate Layout

> Per-crate responsibilities, public API surface, and dependency rules for the
> GuildForge Cargo workspace. The authoritative dependency graph is in
> [`ARCHITECTURE.md` §6](../ARCHITECTURE.md); this document is the per-crate
> deep-dive.

## Workspace Root

`Cargo.toml` at the workspace root declares:

- `[workspace] members = [ "apps/cli", "crates/*" ]`
- `[workspace.package]` — shared metadata: version, edition, license, authors, repository
- `[workspace.dependencies]` — pinned versions of shared deps so every crate uses the same `tokio`, `serde`, `reqwest`, etc.
- `[profile.release]` — LTO, strip, single static binary optimizations

`apps/dashboard` (Next.js) is intentionally **not** a Cargo member; it lives
alongside but is managed by pnpm.

## Crate Catalog

### `crates/shared`

**Purpose**: primitives with no dependencies and no I/O. The lowest layer;
every other crate may depend on this.

**Public surface**:

- `ResourceId` — newtype around a stable string identifier (e.g.
  `discord://guild/role/Admin`).
- `Snowflake` — newtype around a Discord snowflake (u64) with parsing and
  display.
- `Hash` — wrapper around a blake3 hash for content-addressing resource
  states (used by planner for diff determinism).
- `Time` — `chrono`/`time` wrappers for consistent timestamps in state and
  logs.
- `IdempotencyKey` — generated per-operation, persisted with state for
  retry safety.

**Allowed deps**: `blake3`, `chrono`, `serde`, `thiserror`. Nothing else.

**Forbidden**: any I/O crate (`tokio`, `reqwest`, `sqlx`).

---

### `crates/logging`

**Purpose**: `tracing` initialization. Tiny crate so every binary (CLI,
tests, future dashboard backend) initializes logging the same way.

**Public surface**:

- `init(level, format, no_color) -> Result`
- `init_from_env() -> Result` — reads `GUILDFORGE_LOG_*` env vars

**Allowed deps**: `tracing`, `tracing_subscriber`, `shared`, `anyhow`.

---

### `crates/config`

**Purpose**: strongly-typed serde models for every key in the YAML schema
([`docs/SCHEMA.md`](./SCHEMA.md)). No parsing, no validation — just types.

**Public surface**:

- `Config` — root struct
- `Server`, `Role`, `Category`, `Channel`, `ChannelType`, `Permission`,
  `PermissionOverwrite`, `Webhook`, `Invite`, `ForumTag`, `WelcomeScreen`,
  `ServerGuide`, `Ordering`
- `Color` — enum of named / hex / rgb / default
- `PermissionName` — enum of all Discord permissions (see SCHEMA.md §10)

**Allowed deps**: `serde`, `serde_yaml`, `shared`, `thiserror`.

**Rules**:

- Every field is documented with a doc comment.
- `#[serde(deny_unknown_fields)]` on every struct.
- Optional fields are `Option<T>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- No `HashMap<String, Value>` anywhere — every mapping is a typed struct.

---

### `crates/parser`

**Purpose**: read a YAML file, produce a `Config`. Track spans for
diagnostics.

**Public surface**:

- `parse(text: &str) -> Result<Config, ParseError>`
- `parse_file(path: &Path) -> Result<Config, ParseError>`
- `ParseError` — includes span (line, col, end_line, end_col) for `miette`

**Allowed deps**: `serde_yaml`, `config`, `shared`, `thiserror`, `miette`.

**Rules**:

- Parser does NO semantic validation. Syntax only. References, uniqueness,
  and API limits are validated in `crates/validation`.
- Spans are tracked via `serde_yaml`'s `Location` API.

---

### `crates/validation`

**Purpose**: semantic validation of a `Config`. Produces a list of
diagnostics with stable codes (V001-V075 per SCHEMA.md §5).

**Public surface**:

- `validate(config: &Config) -> Result<(), Vec<Diagnostic>>`
- `Diagnostic` — code, severity, message, span, hints
- `Severity` — `Error` / `Warning`

**Allowed deps**: `config`, `shared`, `thiserror`, `miette`.

**Rules**:

- Pure function. No I/O, no async.
- Every check has a stable error code. Codes are part of the public API and
  must never be renumbered.
- Validator returns ALL errors, not just the first. Users should see every
  problem in one pass.

---

### `crates/state`

**Purpose**: SQLite-backed state store. Read by planner, written by executor.

**Public surface**:

- `Store` — the main type. `Store::open(path) -> Result<Store>`
- `Store::begin() -> Result<Transaction>` — acquires file lock + begins SQL tx
- `Transaction::current() -> Result<CurrentState>`
- `Transaction::commit(new_state) -> Result<()>`
- `Transaction::rollback() -> Result<()>`
- `Lock` — RAII guard; released on drop
- `CurrentState`, `ResourceRecord` — typed state contents

**Allowed deps**: `sqlx` (sqlite), `tokio`, `config`, `shared`, `thiserror`, `anyhow`.

**Schema**:

```sql
CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT);
  -- holds 'schema_version', 'last_applied_at', etc.

CREATE TABLE resources (
  addr TEXT PRIMARY KEY,            -- e.g. "role/Admin"
  kind TEXT NOT NULL,               -- "role" / "channel" / etc.
  provider TEXT NOT NULL,           -- "discord"
  data TEXT NOT NULL,               -- JSON blob of the resource
  content_hash TEXT NOT NULL,       -- blake3 of `data`
  tainted INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL
);

CREATE TABLE migrations (
  id INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL,
  plan_hash TEXT NOT NULL,
  summary TEXT NOT NULL             -- "+3 ~1 -0 >2"
);
```

**Rules**:

- File lock via `fs2::FileExt::try_lock_exclusive` on a sidecar `.lock` file.
- On open, run migrations if `schema_version` < current.
- Never auto-delete state; `destroy` is an explicit command.

---

### `crates/provider`

**Purpose**: the `Provider` trait and shared resource types. The single
most important crate for extensibility.

**Public surface**:

- `Provider` trait — see [ADR-0001](./adr/ADR-0001-provider-trait.md) for full spec
- `Resource` — enum of all known resource kinds
- `ResourceKind` — `Role` / `Category` / `Channel` / `PermissionOverwrite` / `Webhook` / `Invite` / `ForumTag` / `WelcomeScreen` / `ServerGuide`
- `ResourceAddr` — typed address
- `ProviderError` — typed error enum

**Allowed deps**: `async-trait`, `serde`, `shared`, `thiserror`.

**Rules**:

- No `reqwest`, no `tokio` runtime, no Discord-specific types.
- The trait must be implementable for any chat/collaboration platform that
  has the concept of "channels" and "permissions".

---

### `crates/provider-discord`

**Purpose**: Discord implementation of `Provider`.

**Public surface**:

- `DiscordProvider::new(token, http_client) -> Self`
- `DiscordProvider` implements `Provider`
- `DiscordProvider::from_env() -> Result<Self>` — reads `GUILDFORGE_BOT_TOKEN` or token file

**Allowed deps**: `provider`, `shared`, `reqwest`, `tokio`, `serde`, `serde_json`, `thiserror`, `async-trait`, `tracing`.

**Internal modules**:

- `client/` — low-level HTTP wrapper, rate-limit middleware, retry
- `resources/` — per-resource-type CRUD: `role.rs`, `channel.rs`, `forum.rs`, `webhook.rs`, etc.
- `error.rs` — `DiscordError` enum

**Rules**:

- Every public function has both a mock test (wiremock) and, where feasible,
  a live test gated behind `--features live-discord`.
- Rate-limit handling is in `client/rate_limit.rs`. It is the single place
  that knows about Discord's per-route buckets.
- Never log the bot token. Never include it in error messages.

---

### `crates/planner`

**Purpose**: compute a deterministic `ExecutionPlan` from `(Config, CurrentState)`.

**Public surface**:

- `Planner::new() -> Self`
- `Planner::plan(config: &Config, state: &CurrentState) -> ExecutionPlan`
- `ExecutionPlan` — list of `Operation` in topological order
- `Operation` — `Create(Resource)` / `Update(current, desired)` / `Delete(Resource)` / `Reorder(addr, new_pos)` / `Noop(Resource)`
- `render(plan: &ExecutionPlan, format: Format) -> String`

**Allowed deps**: `config`, `state`, `shared`, `thiserror`, `serde`, `serde_json`.

**Rules**:

- Pure function. No I/O, no async.
- Deterministic per [ADR-0003](./adr/ADR-0003-planner-determinism.md). The
  same `(config, state)` always produces byte-identical plans.
- Plan output is sorted by `(kind, addr)` within each topological level.

---

### `crates/executor`

**Purpose**: walk an `ExecutionPlan` in topological order, invoke the
provider, and persist state changes.

**Public surface**:

- `Executor::new(provider: Arc<dyn Provider>, state: Store) -> Self`
- `Executor::execute(plan: &ExecutionPlan) -> Result<ExecutionReport>`
- `ExecutionReport` — list of per-op results, summary counts

**Allowed deps**: `provider`, `state`, `shared`, `planner`, `tokio`, `tracing`, `thiserror`, `anyhow`.

**Rules**:

- Async. Uses `tokio`.
- Every operation is wrapped in retry-with-backoff for transient errors.
- Partial failures mark the resource `tainted` and continue with the next
  independent operation.
- State is committed in a single transaction at the end; if commit fails,
  state is rolled back (but live Discord may have changed — this is logged
  loudly).

---

### `crates/engine`

**Purpose**: workflow orchestration. The CLI calls into `engine`; engine
calls into everything else.

**Public surface**:

- `Engine::new(provider: Arc<dyn Provider>, state_path: PathBuf) -> Result<Self>`
- `Engine::validate(path: &Path) -> Result<Config>`
- `Engine::plan(path: &Path) -> Result<ExecutionPlan>`
- `Engine::apply(path: &Path, auto_approve: bool) -> Result<ExecutionReport>`
- `Engine::destroy(path: &Path, auto_approve: bool) -> Result<ExecutionReport>`
- `Engine::doctor() -> Result<DriftReport>`

**Allowed deps**: every other crate EXCEPT `provider-discord`. The concrete
provider is injected at construction.

**Rules**:

- Engine never imports from `provider-discord`. That wiring happens in
  `apps/cli`.
- Engine owns the state lock lifecycle.
- Engine emits structured `tracing` events at every stage; logs alone are
  sufficient to reconstruct what happened.

---

### `apps/cli`

**Purpose**: the `guildforge` binary. Thin shell over `engine`.

**Public surface**: the `guildforge` executable. Not a library.

**Allowed deps**: every crate, including `provider-discord`. This is the
**only** place that knows about the concrete Discord provider.

**Structure**:

```
apps/cli/
├── Cargo.toml
├── src/
│   ├── main.rs           # entry, clap parsing, dispatches to commands
│   ├── commands/
│   │   ├── init.rs
│   │   ├── validate.rs
│   │   ├── plan.rs
│   │   ├── apply.rs
│   │   ├── destroy.rs
│   │   ├── diff.rs
│   │   ├── import.rs
│   │   ├── export.rs
│   │   ├── doctor.rs
│   │   ├── backup.rs
│   │   ├── restore.rs
│   │   ├── login.rs
│   │   ├── logout.rs
│   │   └── version.rs
│   ├── args.rs           # clap derive structs
│   ├── output.rs         # rendering helpers
│   └── prompt.rs         # interactive yes/no
└── tests/
    └── cli_integration.rs
```

---

### `apps/dashboard` (Phase 5)

**Purpose**: Next.js 16 web UI. Lives in this directory but is **not** a
Cargo member; managed by pnpm.

See [ADR-0008](./adr/ADR-0008-dashboard-binding.md) for the binding model
(dashboard shells out to the CLI, does not embed the engine in-process).

## Dependency Rules (Summary)

```
shared          ← (nothing)
logging         ← shared
config          ← shared
parser          ← config, shared
validation      ← config, shared
state           ← config, shared
provider        ← shared
planner         ← config, state, shared
provider-discord ← provider, shared
executor        ← provider, state, planner, shared
engine          ← parser, validation, planner, executor, state, config, provider, logging, shared
cli             ← engine, provider-discord, logging, shared
```

**Hard rules**:

1. No back-links. The graph is a DAG.
2. No crate imports from `cli`.
3. No crate except `cli` and `provider-discord` imports from `provider-discord`.
4. `shared` imports from nothing.
5. `provider` (trait) imports from `shared` only.
6. Circular dependencies are forbidden; `cargo` enforces this but reviewers
   must catch attempts to绕过 it.

## Public API Stability

For v0.x: no crate's public API is stable. For v1.0: `config`, `parser`,
`validation`, `planner`, `provider` become semver-stable. `state`,
`executor`, `engine` are not stable (internal). `provider-discord` follows
its own version. `cli` is stable across patch versions per
[CLI_REFERENCE.md](./CLI_REFERENCE.md).
