# ADR-0007: Idempotency & Ordering

- **Status**: Accepted
- **Date**: 2026-07-01
- **Deciders**: founding eng
- **Tags**: executor, idempotency, partial-failure, ordering

## Context

The executor's job is to walk an `ExecutionPlan` in order and call the
provider. Real-world execution is messy:

- Network requests fail transiently and need retry.
- Some operations depend on others (a channel can't be created before
  its parent category exists).
- A single failure mid-apply shouldn't corrupt the whole guild.
- Running `apply` twice should be a no-op the second time.
- Discord itself is eventually consistent: a `POST /channels` returns
  201 immediately, but the channel may not be visible to subsequent
  calls for ~1 second.
- Ctrl-C should leave Discord in a sane state.

This ADR specifies how the executor handles these concerns. It builds
on [ADR-0001](./ADR-0001-provider-trait.md) (Provider trait),
[ADR-0003](./ADR-0003-planner-determinism.md) (plan is deterministic),
and [ADR-0006](./ADR-0006-async-http.md) (HTTP stack).

## Decision

### Topological ordering

The planner emits operations in topological order (see ADR-0003). The
executor walks that order. It does NOT re-order operations.

Dependency edges (from `crates/planner/src/dependencies.rs`):

```
server settings  (no deps)
roles             (no deps)
categories        (depend on roles, for permission overwrites)
channels          (depend on categories, for parent; on roles, for overwrites)
permission overwrites (depend on channels and roles)
webhooks          (depend on channels)
forum tags        (depend on forum channels)
invites           (depend on channels)
welcome screen    (depends on channels)
server guide      (depends on channels)
```

The executor processes levels in order: level 0 (server, roles) → level
1 (categories) → level 2 (channels) → level 3 (overwrites, webhooks,
tags, invites, welcome, guide).

Within a level, operations are processed **sequentially by default**.
Concurrent execution within a level is opt-in via `--max-concurrency=N`
(default 1 for safety, max 8). When concurrent, operations on the same
**parent** are still sequential (e.g. two channels in the same category
are sequential; two channels in different categories can be concurrent).

### Idempotency at the provider level

Every provider CRUD method is idempotent:

- **`create(desired)`**: if a resource with the same `addr` already
  exists, return the existing one (don't fail). This handles the case
  where a previous `create` succeeded but the response was lost.
- **`update(current, desired)`**: if `current == desired`, return
  `current` unchanged. No API call.
- **`delete(current)`**: if the resource is already gone, return
  `Ok(())`. No error.
- **`reorder(addr, pos)`**: if the resource is already at `pos`, return
  `Ok(())`.

The executor relies on these guarantees. It does NOT pre-check existence
before calling `create` — it just calls `create` and trusts the provider
to handle the duplicate case.

### Retry strategy

The executor wraps every provider call in a retry loop (in addition to
the HTTP-level retry in the provider; see ADR-0006). The executor-level
retry handles provider errors that surface after HTTP retries are
exhausted:

- `ProviderError::RateLimited` → never seen by executor (handled by
  HTTP layer).
- `ProviderError::Transient(msg)` → retry up to 3 times, exp backoff
  (1s, 2s, 4s), then surface to executor.
- `ProviderError::Conflict` → retry once after 500ms (race condition
  with another concurrent op), then surface.
- `ProviderError::Permanent(msg)` → no retry, mark tainted, continue.
- `ProviderError::Auth` → no retry, abort entire apply (token is bad).

### Partial failure: taint, continue, report

When a single operation fails permanently (after retries):

1. The resource is marked `tainted: true` in the in-memory state.
2. The operation's error is recorded in the `ExecutionReport`.
3. The executor continues with the next **independent** operation.
4. Operations that depend on the failed resource are skipped (recorded
   as `skipped: <reason>` in the report).
5. At the end, the executor commits the in-memory state (including
   taints) to SQLite and returns a non-zero exit code.

A tainted resource is one whose state is unknown. On the next `apply`:

- The planner sees the resource is tainted.
- The planner issues a `delete` (best-effort) + `create` for the tainted
  resource. This is the only case where GuildForge uses
  delete-and-recreate instead of in-place update.

Users can manually untaint a resource with `guildforge untaint <addr>`
if they know the resource is actually fine (e.g. they manually verified
it in Discord).

### Eventual consistency

Discord is eventually consistent. After `POST /channels` returns 201,
the channel exists in Discord's storage but may not be visible to
`GET /guilds/:id/channels` for ~1 second.

Mitigations:

- The provider's `create` method verifies the resource is readable
  before returning success. It does a `GET` immediately after the
  `POST`, with up to 3 retries (1s, 2s, 4s) if the GET returns 404.
  If all retries fail, the resource is marked tainted.
- This adds 1 GET per create. Acceptable; Discord's rate limits are
  generous enough.
- For `delete`, no verification. Idempotency means a duplicate delete is
  a no-op, so eventual consistency doesn't matter.

### State commits

The executor does NOT commit state after every operation. It commits
once at the end, in a single SQLite transaction. This means:

- If the process crashes mid-apply, state is rolled back to the
  pre-apply snapshot. Live Discord may have changed (some operations
  succeeded), but state is consistent.
- The next `apply` after a crash re-runs the entire plan. Idempotency
  at the provider level means already-applied operations are no-ops.
- This is the safest trade-off: state is always consistent, at the cost
  of re-running no-ops after a crash.

Trade-off considered and rejected: commit after every operation. This
would let us resume after a crash, but it complicates state (need to
track which ops have been applied) and makes the apply flow harder to
reason about. The idempotency guarantee makes re-running no-ops cheap.

### Cancellation

The CLI installs a Ctrl-C handler that triggers a `CancellationToken`.
The executor checks the token between operations. When canceled:

1. Stop issuing new operations.
2. Wait for in-flight provider calls to complete (don't abandon mid-
   HTTP-request — that could leave Discord in an inconsistent state).
3. Roll back the state transaction.
4. Release the state lock.
5. Print "canceled, no changes committed to state. Live Discord may
   have partial changes; run `guildforge doctor` to verify."
6. Exit 130.

The user is told explicitly that Discord may have partial changes.
`doctor` will detect them and the next `apply` will reconcile.

### Concurrency model

```
                  ┌────────────────────┐
                  │  Executor::execute │
                  └─────────┬──────────┘
                            │
                            ▼
              ┌──────────────────────────┐
              │  For each topological    │
              │  level (in order):       │
              └──────────┬───────────────┘
                         │
                         ▼
              ┌──────────────────────────┐
              │  Group ops in this level │
              │  by parent (category,    │
              │  role, etc.)             │
              └──────────┬───────────────┘
                         │
                         ▼
              ┌──────────────────────────┐
              │  For each parent group:  │
              │  process ops sequentially│
              │  (with retry + cancel    │
              │  check between each op)  │
              └──────────┬───────────────┘
                         │
                         ▼
              ┌──────────────────────────┐
              │  Parent groups processed │
              │  concurrently up to      │
              │  --max-concurrency       │
              └──────────────────────────┘
```

Default `--max-concurrency=1` means everything is sequential. The user
opts into concurrency explicitly. This matches Discord's per-route
limits (most writes are 5/2s per route, so concurrency > 1 only helps
when operations span routes).

### Execution report

```rust
pub struct ExecutionReport {
    pub created: u32,
    pub updated: u32,
    pub deleted: u32,
    pub reordered: u32,
    pub noop: u32,
    pub failed: u32,
    pub skipped: u32,    // skipped due to upstream failure
    pub tainted: u32,
    pub operations: Vec<OperationResult>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: chrono::DateTime<chrono::Utc>,
}

pub enum OperationResult {
    Success { addr: ResourceAddr, op: Operation, duration_ms: u64 },
    Failure { addr: ResourceAddr, op: Operation, error: String, retries: u32 },
    Skipped { addr: ResourceAddr, reason: String },
}
```

The CLI renders this as:

```
Apply complete: +12 ~3 -1 >0 =85

Failures:
  - role/Mod: 403 Forbidden: Missing Permissions (retried 0 times)
  - channel/COMPANY/announcements: skipped (depends on failed role/Mod)

Tainted resources (will be recreated on next apply):
  - role/Mod

Run `guildforge doctor` to verify live state.
```

## Alternatives Considered

### G1: Commit state after every operation

Rejected. Complicates state (need partial-apply tracking), makes the
happy path slower (N SQLite writes instead of 1), and doesn't actually
help — idempotency means re-running no-ops after a crash is cheap.

### G2: Abort on first failure

Rejected. If the user has 100 operations and #5 fails, abandoning #6-#100
wastes the work that could have been done. Taint-and-continue gives the
user a list of failures to fix and applies everything that can be
applied.

### G3: Delete-and-recreate for every update

Rejected. Slow (each delete + create is 2 API calls + verification),
loses Discord-side metadata (message history on a recreated channel is
gone), and violates the principle of minimal change.

### G4: Per-resource locking

Rejected. Adds complexity for no benefit — the state-level exclusive
lock (ADR-0002) already prevents concurrent GuildForge processes, and
within a single process the executor controls ordering.

### G5: Two-phase commit with Discord

Rejected. Discord doesn't support transactions. We can't atomically
create 50 channels; we have to do them one at a time. Idempotency is
our compensation mechanism.

### G6: Optimistic concurrency (compare-and-swap on state version)

Rejected. State is local and locked; no concurrent writers to conflict
with. Optimistic concurrency is for distributed state, which we don't
have in v1.

## Consequences

### Becomes easier

- Idempotency means re-running after a crash is safe and cheap.
- Taint-and-continue gives users actionable failure lists.
- Cancellation is clean: roll back state, exit, let the next `apply`
  reconcile.
- Concurrency is opt-in and bounded; default behavior is sequential and
  safe.

### Becomes harder

- Tainted resources require a `guildforge untaint` command for users who
  want to manually verify and skip recreate. Extra CLI surface.
- Eventual-consistency verification adds a GET per create (acceptable
  cost).
- The execution report is complex; rendering it well is a real UX task.

### New constraints

- Every provider method MUST be idempotent. The provider conformance
  test suite verifies this. A provider that fails idempotency is
  rejected.
- Tainted state is part of the state schema. The `tainted` column in
  `resources` table is non-optional.
- Cancellation must be cooperative. Every long operation checks the
  `CancellationToken`. Code review enforces.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Tainted resource is actually fine, user runs `apply`, recreates it, loses data | `guildforge untaint <addr>` lets user skip recreate; documented in `--help` |
| Eventual consistency causes spurious 404 on verify | Provider retries GET up to 3 times with backoff |
| Concurrent operations hit per-route 429s | Same-parent ops are sequential; cross-parent ops use distinct routes; rate-limit middleware handles residual 429s |
| Cancellation mid-HTTP-request leaves Discord half-written | Wait for in-flight requests before rollback; document residual risk |
| Crash mid-apply leaves state rolled back but Discord half-applied | Next `apply` reconciles via idempotency; `doctor` detects drift |

## References

- [Terraform apply semantics](https://developer.hashicorp.com/terraform/internals/lifecycle)
- [Discord API consistency](https://discord.com/developers/docs/reference)
- [tokio_util CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)
- Related: [ADR-0001](./ADR-0001-provider-trait.md),
  [ADR-0002](./ADR-0002-state-store.md),
  [ADR-0003](./ADR-0003-planner-determinism.md),
  [ADR-0006](./ADR-0006-async-http.md)
