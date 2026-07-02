# ADR-0002: State Store (SQLite + File Lock)

- **Status**: Accepted
- **Date**: 2026-07-01
- **Deciders**: founding eng
- **Tags**: state, persistence

## Context

GuildForge needs to remember what it last applied so that the next `plan`
can compute a diff. This is the "state" concept from Terraform.

Requirements:

1. **Persistent across invocations**. State must survive process restarts.
2. **Atomic updates**. A crashed `apply` must not corrupt state.
3. **Concurrent-write protection**. Two `apply` runs at once must not
   corrupt state.
4. **Inspectable**. Users must be able to query state (`guildforge doctor`,
   `guildforge export`).
5. **Local-first**. State lives on the user's machine by default. Remote
   backends are a Phase 7+ concern.
6. **Zero-ops**. No database server to install or manage.
7. **Versioned schema**. Migrations must be auto-applied on open.
8. **Fast enough**. Plan on a 500-channel guild must complete in < 1s;
   state reads dominate.

## Decision

### SQLite via `sqlx`

Single-file SQLite database, default path `./guildforge.db` (overridable
with `--state-file`).

Reasons:

- **Zero-ops**: SQLite is a library, not a server. The state file is a
  regular file the user can back up, copy, or delete.
- **Atomic**: SQLite's WAL mode gives us atomic transactions. A crashed
  `apply` either commits or rolls back; never partial.
- **Inspectable**: Users can open the state file with `sqlite3` or any
  SQLite browser.
- **Fast**: SQLite reads ~100K rows/sec from cold cache; our 500-resource
  state is sub-millisecond to scan.
- **Rust ecosystem**: `sqlx` provides compile-time-checked SQL, async,
  and migrations. Mature, well-maintained, no ORM overhead.
- **Cross-platform**: SQLite ships with every OS. No native deps.

### Schema (sketch — full DDL in `crates/state/migrations/`)

```sql
CREATE TABLE schema_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE resources (
  addr          TEXT PRIMARY KEY,        -- e.g. "role/Admin"
  kind          TEXT NOT NULL,           -- "role"
  provider      TEXT NOT NULL,           -- "discord"
  data          TEXT NOT NULL,           -- JSON blob
  content_hash  TEXT NOT NULL,           -- blake3 of `data`
  tainted       INTEGER NOT NULL DEFAULT 0,
  updated_at    TEXT NOT NULL            -- RFC 3339
);

CREATE INDEX idx_resources_kind ON resources(kind);

CREATE TABLE migrations (
  id          INTEGER PRIMARY KEY,
  applied_at  TEXT NOT NULL,
  plan_hash   TEXT NOT NULL,
  summary     TEXT NOT NULL              -- "+3 ~1 -0 >2"
);

CREATE TABLE drift_snapshots (
  id          INTEGER PRIMARY KEY,
  taken_at    TEXT NOT NULL,
  snapshot    TEXT NOT NULL              -- JSON of all live resources at that time
);
```

### File locking

A sidecar file `./guildforge.db.lock` is created with `fs2::FileExt::try_lock_exclusive`
on open. If the lock is held, `Store::open` returns a `StateError::LockHeld`
with the PID of the holder (read from the lock file's contents).

Lock is held for the duration of an `apply` / `destroy` / `restore` and
released on drop. `plan`, `validate`, `doctor`, `export`, `backup` take a
**shared** lock (multiple readers OK).

On Unix the lock is also released if the process dies (advisory locks via
`flock`). On Windows `LockFileEx` is used; behavior matches.

### Migrations

`sqlx::migrate!` macro embeds SQL migration files at compile time. On
open, `Store::open` runs `migrate!` which applies pending migrations in
order inside a transaction. The `schema_meta` table records the current
schema version.

Migration files live in `crates/state/migrations/` and are named
`<NN>_<description>.sql` (e.g. `01_initial.sql`, `02_add_drift_snapshots.sql`).
Once shipped, a migration is **never edited**; fixes are new migrations.

### Transaction model

```rust
let store = Store::open(path)?;        // acquires shared lock + runs migrations
let tx = store.begin_exclusive()?;     // upgrades to exclusive lock + BEGIN
let current = tx.current()?;           // reads all resources
// ... planner + executor work ...
tx.commit(new_state)?;                 // COMMIT + release exclusive lock
// or tx.rollback()? on failure
```

`begin_exclusive` returns `StateError::LockHeld` if another process holds
the exclusive lock. The CLI translates this to a user-friendly message
with the holder's PID.

## Alternatives Considered

### B1: JSON file

Rejected. JSON has no atomic write, no concurrent-access protection, no
schema, no query capability. We could implement all of these (atomic write
via rename, lock via flock, schema via JSON Schema, query via jq) but
that's just reinventing SQLite poorly.

### B2: Postgres

Rejected. Requires a Postgres server. Violates "zero-ops". Defer to
remote-state backend (Phase 7+).

### B3: sled

Rejected. sled is in maintenance mode as of 2024; the maintainers
recommend not using it for new projects. No good story for inspection
(humans can't read a sled file). Migrations would be ad-hoc.

### B4: redb

Rejected. Pure-Rust, embedded, transactional. Promising but younger than
SQLite, smaller ecosystem, no human-inspectable format. Revisit if SQLite
licensing or cross-compilation ever becomes a problem.

### B5: Multi-file state (one file per resource)

Rejected. Distributes state across hundreds of files. Slow on cold cache,
hard to atomically update, hard to inspect, git-unfriendly. The only
advantage is partial-failure recovery, which SQLite gives us via
transactions anyway.

## Consequences

### Becomes easier

- Atomic apply: a single SQLite transaction wraps the entire state update.
- Inspection: users open `guildforge.db` with any SQLite browser.
- Backup: copy the file. Done.
- CI: `tempfile` per test, no shared state.
- Future remote backends: a `Store` trait abstracts the implementation;
  `RemoteStore` (S3, Postgres) is a Phase 7+ impl.

### Becomes harder

- Binary size: `sqlx` + `libsqlite3-sys` adds ~1 MB to the release binary.
  Acceptable.
- Cross-compilation: `libsqlite3-sys` bundles SQLite by default, which
  adds compile time but no runtime dep. Alternatively, link to system
  SQLite via `--features sqlx/sqlite-unbundled`.
- Schema migrations: every schema change requires a new migration file.
  This is intentional friction but worth noting.

### New constraints

- The state file is a single file. If you want to split state across
  multiple guilds, use separate state files (`--state-file`).
- SQLite is not network-safe. Do not put `guildforge.db` on NFS / SMB.
  Documented in `--help`.
- The `schema_meta` table is the source of truth for schema version. Do
  not edit it manually.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| User puts state file on network drive | Document; refuse to open if `statfs` reports network FS (best-effort) |
| Lock file left behind after crash | Advisory locks are released by the OS on process exit; stale lock files are safe to overwrite |
| Migration fails mid-way | `sqlx::migrate!` wraps each migration in a transaction; partial migration is rolled back |
| State file grows unbounded | `migrations` table is append-only but small; `drift_snapshots` is purged after 30 days via background job on `doctor` |
| Concurrent `apply` from two terminals | Exclusive lock prevents this; second terminal gets `LockHeld` with the first's PID |

## References

- [SQLite docs](https://www.sqlite.org/docs.html)
- [sqlx crate](https://docs.rs/sqlx)
- [fs2 crate](https://docs.rs/fs2)
- Related: [ADR-0007](./ADR-0007-idempotency-ordering.md) (executor commits state),
  [ADR-0003](./ADR-0003-planner-determinism.md) (planner reads state)
