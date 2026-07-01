# Design Decisions

> Index of significant design decisions for GuildForge. Each entry points to a
> full Architecture Decision Record (ADR) in [`docs/adr/`](./docs/adr/).
>
> ADRs are numbered sequentially (`ADR-NNNN`) and never edited in place. To
> change a decision, write a *new* ADR that supersedes the old one and update
> the status of the old ADR to `Superseded by ADR-NNNN`.

## ADR Format

Every ADR follows this template:

```markdown
# ADR-NNNN: Title

- **Status**: Proposed | Accepted | Deprecated | Superseded by ADR-NNNN
- **Date**: YYYY-MM-DD
- **Deciders**: names
- **Tags**: area tags (e.g. `provider`, `state`, `planner`)

## Context

What is the issue we're facing? What forces are at play?

## Decision

What we decided to do. Be specific — include trait signatures, file paths,
or concrete examples where relevant.

## Alternatives Considered

For each alternative: what is it, why didn't we pick it, and what would it
take to revisit?

## Consequences

What becomes easier? What becomes harder? What new constraints do we accept?

## Risks & Mitigations

What could go wrong? How do we detect it? How do we recover?

## References

Links to external docs, related ADRs, prior art.
```

## Decision Index

| ADR | Title | Status | Area |
|---|---|---|---|
| [ADR-0001](./docs/adr/ADR-0001-provider-trait.md) | Provider trait: async, typed resources, CRUD lifecycle | Accepted | provider |
| [ADR-0002](./docs/adr/ADR-0002-state-store.md) | SQLite + file lock for local state | Accepted | state |
| [ADR-0003](./docs/adr/ADR-0003-planner-determinism.md) | Deterministic diff via canonical resource ordering | Accepted | planner |
| [ADR-0004](./docs/adr/ADR-0004-config-format.md) | YAML v1, no modules or variables | Accepted | config |
| [ADR-0005](./docs/adr/ADR-0005-error-model.md) | Anyhow + ThisError + miette for diagnostics | Accepted | errors |
| [ADR-0006](./docs/adr/ADR-0006-async-http.md) | Tokio + Reqwest, per-route rate-limit middleware | Accepted | runtime |
| [ADR-0007](./docs/adr/ADR-0007-idempotency-ordering.md) | Topological apply, taint-on-failure, idempotent CRUD | Accepted | executor |
| [ADR-0008](./docs/adr/ADR-0008-dashboard-binding.md) | Dashboard shells out to CLI, no in-process engine | Accepted | dashboard |

## Future ADRs (proposed, not yet written)

| Title | When |
|---|---|
| Module & variable system | Phase 7+ — when config complexity demands it |
| Remote state backends | Phase 7+ — when multi-user workflows demand it |
| Policy engine (OPA/Rego) | Phase 7+ — when enterprise governance demand appears |
| Provider protocol v2 (streaming) | Only if a future provider needs streaming semantics |
| Multi-guild templating | Phase 7+ — after first non-Discord provider exists |

## How to Propose a New ADR

1. Copy `docs/adr/_template.md` to `docs/adr/ADR-NNNN-short-slug.md` where
   `NNNN` is the next free number.
2. Fill in every section. "Alternatives Considered" must list at least two
   real alternatives — if you can't think of any, you haven't thought hard
   enough.
3. Open a PR with `Status: Proposed`.
4. A maintainer schedules a review meeting (or async comment period).
5. Once accepted, the PR is merged with `Status: Accepted` and the decision
   is recorded above.
