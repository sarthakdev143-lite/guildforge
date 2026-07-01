# ADR-0005: Error Model (Anyhow + ThisError + miette)

- **Status**: Accepted
- **Date**: 2026-07-01
- **Deciders**: founding eng
- **Tags**: errors, diagnostics, ux

## Context

Rust has three popular error-handling approaches, each with tradeoffs:

1. **`thiserror`** — typed enum errors with `#[derive(Error)]`. Great for
   libraries; callers can match on variants. Verbose for application code.
2. **`anyhow`** — type-erased errors with context chaining. Great for
   application code; terrible for matching.
3. **`miette`** — diagnostic-centric errors with spans, labels, source
   context. Designed for compilers and CLI tools that consume user-authored
   input.

GuildForge has both library-style code (every crate below `engine`) and
application-style code (`engine`, `cli`). It also consumes user-authored
input (YAML configs) where span-aware diagnostics are essential.

We need a coherent strategy that:

- Lets library callers match on typed errors when they need to.
- Lets the engine chain errors ergonomically without exploding in size.
- Produces beautiful diagnostics for YAML / CLI-arg errors.
- Never leaks the bot token in error messages.
- Produces lowercase, no-trailing-period messages with enough context to
  debug without re-running.

## Decision

### Three-layer error strategy

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: Library crates (config, parser, validation, ...)  │
│  Typed errors via thiserror.                                │
│  Each crate has its own Error enum.                         │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼  (auto-convert via From)
┌─────────────────────────────────────────────────────────────┐
│  Layer 2: Engine crate                                      │
│  anyhow::Result for ergonomic chaining.                     │
│  .context("...") at every boundary.                         │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 3: CLI                                                │
│  miette for rendering.                                       │
│  anyhow errors rendered as plain chain reports.             │
│  Typed errors with spans rendered as rich diagnostics.       │
└─────────────────────────────────────────────────────────────┘
```

### Layer 1: Typed errors (library crates)

Every library crate has an `Error` enum:

```rust
// crates/parser/src/error.rs
use miette::SourceSpan;
use thiserror::Error;

#[derive(Debug, Error, miette::Diagnostic)]
pub enum ParseError {
    #[error("invalid YAML: {message}")]
    #[diagnostic(code(parser::invalid_yaml), help("check syntax at line {line}"))]
    InvalidYaml {
        message: String,
        line: usize,
        #[source_code]
        source: String,
        #[label("here")]
        span: SourceSpan,
    },

    #[error("unknown field `{field}` in `{context}`")]
    #[diagnostic(code(parser::unknown_field), help("see docs/SCHEMA.md for valid fields"))]
    UnknownField {
        field: String,
        context: String,
        #[source_code]
        source: String,
        #[label("unknown field")]
        span: SourceSpan,
    },

    // ... etc.
}
```

Rules:

- Every variant has a stable `code(...)` (e.g. `parser::invalid_yaml`).
  Codes are part of the public API and never renumbered.
- Every variant has a `help(...)` message with a concrete next step
  ("see docs/SCHEMA.md for valid fields", "did you mean to declare
  this role in the `roles:` block?", etc.).
- Variants that originate from user input have `#[source_code]` and
  `#[label]` for miette to render.
- Messages are lowercase, no trailing period, no "Error:" prefix (miette
  adds severity).
- Variants that wrap an external error (e.g. `IoError(#[from] std::io::Error)`)
  use `#[from]` for ergonomic `?`.

### Layer 2: Engine (anyhow)

The engine crate uses `anyhow::Result<T>` everywhere. It never re-exposes
typed errors from lower layers; it converts them to `anyhow::Error` via
`?` (which uses the `From` impls that `thiserror` generates).

Every boundary adds context:

```rust
// crates/engine/src/lib.rs
pub fn apply(&self, path: &Path, auto_approve: bool) -> Result<ExecutionReport> {
    let config = self.validate(path)
        .with_context(|| format!("validating config at {}", path.display()))?;
    let plan = self.plan_inner(&config)
        .context("computing execution plan")?;
    if !auto_approve {
        self.prompt_confirm(&plan).context("user aborted")?;
    }
    let report = self.execute(&plan).context("executing plan")?;
    Ok(report)
}
```

Rules:

- `.context()` is added at every public method boundary.
- Context messages are lowercase, no trailing period, present-progressive
  ("validating config at X", not "validate config at X").
- `anyhow` is never used inside library crates — only in `engine` and `cli`.
- The engine never constructs an `anyhow::anyhow!("...")` directly except
  for `bail!` on truly exceptional paths. All errors originate from typed
  sources.

### Layer 3: CLI (miette)

The CLI installs `miette` as the diagnostic handler:

```rust
// apps/cli/src/main.rs
fn main() -> miette::Result<()> {
    logging::init_from_env().into_diagnostic()?;
    let args = Args::parse();
    run(args).map_err(|e: anyhow::Error| miette::Report::new(e))?;
    Ok(())
}
```

`miette::Report` automatically renders:

- Typed errors (Layer 1) with spans, labels, source context, help text.
- `anyhow` chain errors (Layer 2) as a chain of "Caused by:" messages.

Output colors are respected (`--no-color`, `NO_COLOR`).

### Token redaction

Every error message that includes a `reqwest::Error` or HTTP request
context is scrubbed. The `provider-discord` HTTP client uses a custom
`tracing` span that redacts the `Authorization` header before logging.

Code review checklist (also in [`docs/SECURITY.md`](../SECURITY.md)):

- [ ] No error message includes the bot token.
- [ ] No error message includes a full URL with query params (which
      sometimes contain tokens).
- [ ] No `Debug` impl on a secret type prints the secret.

### Error message style guide

- Lowercase first letter.
- No trailing period.
- No "Error:" prefix (miette adds severity).
- Include the failing input where possible: `unknown field `foo` in
  `server`` not just `unknown field`.
- Suggest a fix when possible: `did you mean `color`?` (when there's a
  close match).
- Link to docs when relevant: `see docs/SCHEMA.md §3.4 for valid channel
  types`.

### No `unwrap()` / `expect()` / `panic!()` outside tests

Enforced by clippy (`clippy::unwrap_used`, `clippy::expect_used`,
`clippy::panic`). Tests may use them.

The only exception is `tokio::spawn`'s future, which must propagate
panics to the join handle. Even there, prefer `Result` over `panic!`.

### Exit codes

The CLI maps error categories to exit codes per
[`docs/CLI_REFERENCE.md`](../CLI_REFERENCE.md):

| Code | Category |
|---|---|
| 0 | Success |
| 1 | Soft failure (validation, plan-has-changes, partial-apply) |
| 2 | User error (file not found, invalid args) |
| 3 | State error |
| 4 | Provider error |
| 5 | User aborted |
| 6 | Lock held |

The engine's typed errors map to these via a `From<Error> for ExitCode`
impl in `apps/cli`.

## Alternatives Considered

### E1: `anyhow` everywhere

Rejected. Loses typed error matching. The executor needs to match on
`ProviderError::RateLimited` to retry, `ProviderError::Forbidden` to
give up, `ProviderError::NotFound` to convert to a delete. With `anyhow`,
this requires `downcast_ref` which is brittle and undocumented.

### E2: `thiserror` everywhere

Rejected. Verbose at the engine layer. Every `?` needs a `From` impl or
a `.map_err()`. Context chaining is manual. The engine has ~20 public
methods; manually wrapping each error in a `thiserror` enum bloats the
codebase by 30% with no caller benefit (the engine's callers are the CLI
and tests, neither of which matches on engine errors).

### E3: `snafu` instead of `thiserror`

Rejected. `snafu` is excellent but less widely known. `thiserror` is the
de-facto standard and is what every Rust contributor already knows. The
marginal benefit of `snafu`'s positional generics doesn't justify the
onboarding cost.

### E4: `miette` everywhere (no `anyhow`)

Rejected. `miette::Result` is `Result<T, miette::Report>`. `Report` is
type-erased, so we lose the typed-error benefits at Layer 1. `miette` is
better used as a renderer at the boundary, not as the error type
throughout.

### E5: Custom error hierarchy (boxed trait objects)

Rejected. Reinvents `anyhow` poorly. No.

### E6: `failure` crate

Rejected. `failure` is deprecated. The maintainer recommends `anyhow` +
`thiserror`.

## Consequences

### Becomes easier

- Library callers can match on error variants (executor matches on
  `ProviderError::RateLimited`).
- Engine code is ergonomic: `?` + `.context()` chains naturally.
- CLI output is beautiful: miette renders multi-line diagnostics with
  source spans for YAML errors.
- Error codes are stable: `parser::unknown_field` is part of the public
  API and never changes.
- Token redaction is centralized in the provider HTTP client.

### Becomes harder

- Three crates to learn (`thiserror`, `anyhow`, `miette`). Onboarding
  cost is real but bounded.
- Every library crate has an `Error` enum. Boilerplate, but
  `thiserror::Error` derive makes it small.
- Engine errors are `anyhow::Error`, which loses the typed code at the
  engine boundary. Acceptable: the engine's callers don't need typed
  errors, they just print and exit.

### New constraints

- `anyhow` is banned in library crates (`config`, `parser`, `validation`,
  `state`, `planner`, `provider`, `provider-discord`). Clippy enforces
  via `clippy::disallowed_types`.
- `thiserror` is banned in `engine` and `cli`. Engine uses `anyhow`;
  CLI uses `miette::Report`. Same clippy lint, scoped per-crate.
- Every error variant has a stable code. Codes are documented in
  [`docs/SCHEMA.md`](../SCHEMA.md) for validator errors and in
  per-crate `error.rs` for others.
- Every error message is lowercase, no trailing period. CI greps error
  messages for violations.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| `miette` API churn breaks rendering | Pin `miette` minor version; bump deliberately |
| Error codes collide across crates | Namespace by crate name (`parser::...`, `validation::...`) |
| Token leaks via error chain | Redaction in provider HTTP client; CI grep for the token in test output |
| User can't easily extract typed error from `anyhow::Error` | The engine exposes a `EngineError` enum at its boundary for the few cases where the CLI needs to match (e.g. `LockHeld`) |

## References

- [anyhow](https://docs.rs/anyhow)
- [thiserror](https://docs.rs/thiserror)
- [miette](https://docs.rs/miette)
- [Rust API Guidelines — error handling](https://rust-lang.github.io/api-guidelines/error.html)
- Related: [ADR-0004](./ADR-0004-config-format.md) (config errors need spans),
  [`docs/SECURITY.md`](../SECURITY.md) (token redaction)
