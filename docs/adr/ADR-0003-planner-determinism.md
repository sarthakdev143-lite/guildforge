# ADR-0003: Planner Determinism

- **Status**: Accepted
- **Date**: 2026-07-01
- **Deciders**: founding eng
- **Tags**: planner, determinism, plan-output

## Context

A `guildforge plan` run produces an `ExecutionPlan`: a list of operations
that, when executed, will reconcile current state with the desired
config. This plan is shown to the user (or, in CI, written to a file and
reviewed in a PR).

For plans to be reviewable, they must be **deterministic**: the same
`(config, state)` pair must always produce the exact same plan,
byte-for-byte. If plans varied between runs, reviewers couldn't trust
their review, and CI checks would be flaky.

This is the single most important property of the planner. Every other
design decision (resource addressing, sorting, hash equality, partial
updates) flows from it.

## Decision

### Determinism contract

Given:
- A `Config` C (parsed from YAML)
- A `CurrentState` S (read from SQLite)

The function `plan(C, S) -> ExecutionPlan` MUST satisfy:

1. **Pure**: no I/O, no randomness, no reading of system clock, no
   reading of environment variables.
2. **Total**: terminates for every `(C, S)` and produces either an
   `ExecutionPlan` or a `PlannerError`. Never panics.
3. **Deterministic**: `plan(C1, S1) == plan(C2, S2)` whenever
   `C1 == C2 && S1 == S2`. Equality on `ExecutionPlan` is structural
   equality (derived `PartialEq`).
4. **Byte-stable serialization**: `serde_json::to_string(&plan)` is
   stable across versions of `serde_json` (we pin the version in
   `Cargo.lock`) and across runs.

### Resource addressing

Every resource has a canonical `ResourceAddr` of the form
`<kind>/<path>` where `<path>` is a `/`-separated list of names. Examples:

- `role/Admin`
- `category/COMPANY`
- `channel/COMPANY/announcements`
- `channel/_top/general` (top-level channel; `_top` is a reserved segment)
- `overwrite/COMPANY/announcements/role:Admin`
- `webhook/COMPANY/announcements/CI Notifier`
- `tag/COMPANY/help/Question`

Addresses are case-sensitive. Name normalization (lowercasing for
uniqueness checks) happens at validation time and is recorded in the
`Config` struct; the planner uses the canonical case from `Config`.

### Resource ordering

`ExecutionPlan.operations` is a `Vec<Operation>` sorted as follows:

1. **By topological level**: dependencies first. Level 0 = roles and
   server settings. Level 1 = categories. Level 2 = channels. Level 3 =
   permission overwrites, webhooks, forum tags, invites.
2. **Within a level, by `(kind, addr)`**: lexicographic on the tuple.
3. **Operations on the same resource**: in the order `Create` → `Update`
   → `Reorder` → `Delete` (a single resource never has more than one
   operation per plan; this rule is for safety).

Topological levels are computed from a static dependency graph in
`crates/planner/src/dependencies.rs`. The graph is hand-curated and
covers all resource kinds. Adding a new resource kind requires adding
its dependency edges here.

### Diff algorithm

For each resource address A that appears in C, S, or both:

```
if A in C and A not in S:
    emit Create(C[A])
elif A in S and A not in C:
    emit Delete(S[A])
else:  # A in both
    if C[A] == S[A]:
        emit Noop(S[A])
    elif position_changed(C[A], S[A]):
        emit Reorder(A, C[A].position)
    elif content_changed(C[A], S[A]):
        emit Update(S[A], C[A])
    # else: Noop (already emitted above)
```

`content_changed` compares the `content_hash` field (blake3 of the JSON
serialization of the resource). This is faster than structural equality
and naturally stable.

`position_changed` compares the `position` field independently, so that
a pure reorder doesn't trigger an `Update`.

### Equality and hashing

Every `Resource` variant derives `Serialize, Deserialize, PartialEq, Eq,
Hash`. The `Hash` is **not** used for diffing (we use `content_hash`
instead) but is used for `HashSet` operations in the planner.

`PartialEq` is structural and derived. We do NOT implement custom
`PartialEq` for any resource type. If two resources differ in any field,
they are not equal — period. This forces the provider to populate every
field on read so that round-tripping through state doesn't cause spurious
updates.

### Plan rendering

`render(plan: &ExecutionPlan, format: Format) -> String` is also pure
and deterministic. Output formats:

- `text`: aligned columns, sorted by address, with `+ ~ - > =` symbols
- `json`: stable JSON schema, keys sorted alphabetically (via
  `serde_json::value::to_string_pretty` with `BTreeMap`)
- `sarif`: SARIF 2.1.0 for GitHub Code Scanning
- `markdown`: markdown table for PR comments

JSON output is the canonical machine-readable format and is part of the
public API from v0.1.0.

### No hidden state

The planner does NOT read environment variables, system clock, process
ID, machine hostname, or anything else that varies between runs. The
only inputs are the `Config` and `CurrentState` arguments.

## Alternatives Considered

### C1: Hash-based diffing without stable ordering

Rejected. Without stable ordering, two runs could produce the same set
of operations in different orders. The set is equal but the `Vec` is
not, breaking byte-stable serialization. Reviewers would see different
diffs for the same input.

### C2: Time-based ordering (most-recently-changed first)

Rejected. Violates determinism. Also useless for review — reviewers want
to see "what's being created", not "what changed most recently".

### C3: Provider-driven diffing

Rejected. Pushes diffing into the provider, which means the plan cannot
be computed without contacting the provider. Breaks `plan --refresh=false`
and breaks PR-review workflows where the provider isn't available.

### C4: Order operations by YAML declaration order

Tempting but wrong. YAML declaration order is a presentation choice, not
a semantic property. The same config written with channels reordered in
YAML would produce a different plan, even though the desired state is
identical. We use topological + lexicographic ordering instead.

### C5: Field-level diffs

Rejected for v1. The `Update` operation carries `(current, desired)`
pairs; the provider figures out which fields to PATCH. Field-level diffs
in the plan output would be more readable but require every resource
type to implement a `diff` method. Defer to v2 if reviewers ask for it.

## Consequences

### Becomes easier

- Plan snapshot tests are trivial: feed `(config, state)`, snapshot
  `serde_json::to_string(&plan)`.
- PR review: a plan diff in a PR is meaningful because it only changes
  if the inputs change.
- CI: `guildforge plan --format json` can be diffed against a checked-in
  plan file. Drift in the plan = drift in intent.
- Reproducibility: any user can reproduce any plan run by re-running
  with the same `config` and `state`.

### Becomes harder

- New resource kinds must be added to the static dependency graph. Easy
  to forget; the planner has a test that every `ResourceKind` variant
  appears in the graph.
- Providers must populate every field on read, or round-tripping through
  state causes spurious updates. This is a real burden on
  `provider-discord` and is the most common source of bugs.
- JSON output is now part of the public API. Changing the JSON schema
  requires a major version bump. Documented in
  [`docs/CLI_REFERENCE.md`](../CLI_REFERENCE.md).

### New constraints

- The planner is pure: no I/O, no async, no env, no clock. Code review
  enforces this; a `#[forbid(unsafe_code)]` and grep for `tokio::`,
  `std::env::`, `std::time::SystemTime` in `crates/planner/src/` is part
  of CI.
- The dependency graph is hand-curated. Adding a resource kind requires
  updating `crates/planner/src/dependencies.rs` AND its test.
- The `Resource` enum's `PartialEq` is derived and final. Custom
  equality for any variant is forbidden.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Provider omits a field on read → spurious updates | Provider conformance test suite verifies every field is populated |
| New resource kind forgotten in dependency graph | Test `all_kinds_in_graph` in planner |
| `serde_json` version bump changes key ordering | Pin `serde_json` in `Cargo.lock`; release notes call out JSON schema changes |
| User edits state file manually → non-deterministic plans | State file is SQLite; manual edits are discouraged; `doctor` detects tampering via `content_hash` |
| `content_hash` algorithm changes → all plans change | `blake3` is final; if we ever change, bump schema version and provide a migration |

## References

- [Terraform plan determinism](https://developer.hashicorp.com/terraform/internals/architecture)
- [blake3](https://crates.io/crates/blake3)
- Related: [ADR-0001](./ADR-0001-provider-trait.md) (Resource type),
  [ADR-0002](./ADR-0002-state-store.md) (state is planner input),
  [ADR-0007](./ADR-0007-idempotency-ordering.md) (executor consumes plan)
