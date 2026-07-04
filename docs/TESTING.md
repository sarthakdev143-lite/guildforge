# Testing Strategy

> How GuildForge is tested, from unit tests through end-to-end CLI tests.
> Coverage targets, fixture conventions, and the test pyramid.

## Test Pyramid

```
                  ┌───────────┐
                  │    E2E    │   ← CLI invocations against real Discord (live)
                  └───────────┘
                ┌───────────────┐
                │  Integration  │   ← cross-crate, in-process, mocked HTTP
                └───────────────┘
              ┌─────────────────────┐
              │      Snapshot       │   ← plan output, exported YAML, diagnostics
              └─────────────────────┘
            ┌───────────────────────────┐
            │         Unit              │   ← per-function, in-file
            └───────────────────────────┘
```

Volume should decrease as you go up. Most tests are unit tests; E2E is the
smallest layer.

## Layer 1 — Unit Tests

**Location**: `#[cfg(test)] mod tests { ... }` at the bottom of every
source file in every crate.

**What to test**:

- Every public function.
- Every edge case mentioned in a doc comment.
- Every error path with a stable error code or error variant.
- Every parser/serializer round-trip.
- Every validator rule (V001-V075) has at least one positive and one
  negative test.

**What NOT to test in unit tests**:

- Cross-crate behavior (use integration tests).
- File I/O (use integration tests with `tempfile`).
- Network (use mock layer in integration tests).

**Conventions**:

- Test names: `function_name_condition_expected_result`. Example:
  `parse_unknown_top_level_key_returns_error_with_span`.
- One assertion per test when feasible. Multi-assertion tests group by
  "given-when-then" blocks separated by blank lines.
- Use `pretty_assertions::assert_eq!` for any struct comparison so diffs
  are readable.
- Use `insta` for any string/JSON output that is large enough that inline
  comparison is painful.

## Layer 2 — Snapshot Tests

**Location**: `tests/snapshots/` at the crate root.

**Library**: [`insta`](https://crates.io/crates/insta).

**What to snapshot**:

- Parser output for every YAML in `examples/`.
- Validator diagnostics for every YAML in `examples/broken/`.
- Planner output for `(config, state)` pairs from fixture state files.
- Exporter output (must round-trip cleanly).
- CLI `--help` text for every subcommand (catches accidental UX changes).

**Conventions**:

- Snapshot files live in `tests/snapshots/` and are committed to git.
- `cargo insta review` is part of the PR workflow; CI rejects unreviewed
  snapshots.
- Snapshot filenames: `<crate>__<function>__<case>.snap`.

## Layer 3 — Integration Tests

**Location**: `tests/` at each crate root. Cross-crate integration tests
live in `/tests/` at the workspace root.

**Libraries**:

- [`wiremock`](https://crates.io/crates/wiremock) for HTTP mocking
  (Discord provider tests).
- [`tempfile`](https://crates.io/crates/tempfile) for state file fixtures.
- [`assert_cmd`](https://crates.io/crates/assert_cmd) for CLI tests.
- [`predicates`](https://crates.io/crates/predicates) for CLI output
  assertions.

**What to test**:

- `crates/parser` integration: read every example YAML, parse it, assert
  structural equality against a hand-written `Config`.
- `crates/validation` integration: read every `examples/broken/*.yaml`,
  assert that validation produces exactly the expected diagnostic codes.
- `crates/planner` integration: given a fixture config + fixture state,
  assert the plan matches a snapshot.
- `crates/provider-discord` integration: spin up a `wiremock` server,
  point the provider at it, exercise every CRUD op against mocked
  endpoints, assert request shape and response handling.
- `crates/executor` integration: feed a plan against a mock provider,
  assert state transitions and retry behavior.
- `crates/engine` integration: end-to-end `validate → plan → apply`
  against a mock provider, with a temp SQLite state file.

**Mock data**: `tests/fixtures/` at the workspace root. JSON files
mirroring Discord API responses, organized by resource type. Used by
`provider-discord` tests.

## Layer 4 — CLI Tests

**Location**: `apps/cli/tests/`.

**Library**: `assert_cmd` + `predicates`.

**What to test**:

- Every subcommand responds to `--help` with non-empty output and exit 0.
- `guildforge version` exits 0 with expected format.
- `guildforge validate examples/company.yaml` exits 0.
- `guildforge validate examples/broken/*.yaml` exits non-zero with
  expected error message substrings.
- `guildforge plan examples/company.yaml --format json` produces valid
  JSON matching the schema.
- `guildforge apply --auto-approve examples/company.yaml` against a mock
  provider succeeds and writes state.
- `guildforge doctor` against clean state exits 0; against drifted state
  exits 1.

**Environment**:

- `GUILDFORGE_NO_NETWORK=1` is always set in CLI tests so no test can
  accidentally hit real Discord.
- `GUILDFORGE_STATE_FILE` points to a `tempfile` per test.
- Token is never required for default tests; `--features live-discord`
  gates the live test suite.

## Layer 5 — Live Tests (gated)

**Location**: `crates/provider-discord/tests/live/` and
`apps/cli/tests/live/`.

**Gating**: `#[cfg(feature = "live-discord")]`. Not run by default.

**Requirements to run**:

- `GUILDFORGE_BOT_TOKEN` env var set to a valid bot token.
- `GUILDFORGE_LIVE_GUILD_ID` env var set to a guild ID the bot has admin
  permissions in.
- A throwaway guild used exclusively for these tests. **Never run live
  tests against a guild with real users.**

**What to test**:

- Every CRUD op against real Discord.
- Rate-limit handling under load.
- Idempotency: apply twice → second is no-op.
- Drift detection: apply, manually edit a channel via direct API call,
  run `doctor`, expect drift reported.
- Round-trip: `import` a guild, `export`, byte-compare to a known-good
  YAML.

**CI**: live tests are NOT run in default CI. They run nightly via a
scheduled workflow against a dedicated test guild. The bot token is stored
as a GitHub Actions secret.

## Coverage

**Tool**: [`cargo-tarpaulin`](https://crates.io/crates/cargo-tarpaulin)
or [`cargo-llvm-cov`](https://crates.io/crates/cargo-llvm-cov) (preferred;
faster).

**Targets** (enforced in CI from Phase 1 onward):

| Crate | Min coverage |
|---|---|
| `shared` | 95% |
| `config` | 95% |
| `parser` | 95% |
| `validation` | 95% |
| `state` | 90% |
| `provider` (trait) | 90% |
| `provider-discord` | 85% (live tests not counted) |
| `planner` | 95% |
| `executor` | 85% |
| `engine` | 80% |
| `logging` | 70% |
| `apps/cli` | 70% |

Coverage is reported on every PR via `codecov` (or `coveralls`). A PR that
drops coverage by more than 2 percentage points is blocked.

## Property-Based Testing

**Library**: [`proptest`](https://crates.io/crates/proptest).

**Where**:

- `crates/parser`: generate arbitrary `Config` values, serialize to YAML,
  parse back, assert round-trip equality.
- `crates/planner`: generate arbitrary `(config, state)` pairs, assert
  that `plan` is deterministic (same input → same output).
- `crates/state`: generate arbitrary resource sets, write to SQLite,
  read back, assert equality.

Property tests run in CI as part of the default test suite.

## Fuzzing

**Library**: [`cargo-fuzz`](https://crates.io/crates/cargo-fuzz).

**Targets**:

- `parser::parse` — fuzz with arbitrary bytes, must never panic.
- `validation::validate` — fuzz with arbitrary `Config`s generated by
  proptest, must never panic.

Fuzzing runs nightly in CI for 10 minutes per target. Crashes are filed
as issues and block release.

## Test Data Management

- `tests/fixtures/` — JSON fixtures (Discord API responses, sample configs).
- `tests/snapshots/` — `insta` snapshots, committed.
- `tests/state/` — sample SQLite state files for planner/executor tests.
- `tests/yaml/` — sample YAML configs for parser tests (also see `examples/`).

Fixtures are versioned alongside the code. If a Discord API response shape
changes, the fixture is updated in the same PR that updates the provider
code.

## Performance Tests

**Library**: [`criterion`](https://crates.io/crates/criterion).

**Benchmarks**:

- `parser::parse` on a 500-channel YAML.
- `planner::plan` on a 500-channel config + 500-resource state.
- `executor::execute` on a 100-op plan against a mock provider.

Benchmarks run on every PR and compared against the `main` branch. A 10%
regression blocks merge; the author must either fix the regression or
justify it in the PR description.

## Test Naming Conventions

```
<crate>/tests/<area>_<scenario>.rs
```

Examples:

- `crates/parser/tests/yaml_round_trip.rs`
- `crates/validation/tests/references_resolve.rs`
- `crates/provider-discord/tests/role_crud.rs`
- `crates/planner/tests/determinism.rs`
- `apps/cli/tests/validate_command.rs`

## CI Workflow

```yaml
# .github/workflows/ci.yml (sketch — see committed file)
jobs:
  fmt:        cargo fmt --check
  clippy:     cargo clippy --workspace --all-targets -- -D warnings
  test:       cargo test --workspace
  coverage:   cargo llvm-cov --workspace --fail-under-lines 85
  doc:        cargo doc --workspace --no-deps
  msrv:       cargo +1.78.0 check --workspace
  deny:       cargo deny check
  audit:      cargo audit
```

All jobs must pass for `main` to be green. PRs must pass all jobs to merge.

## Test Anti-Patterns (Forbidden)

- **No `#[ignore]` without a tracking issue.** Ignored tests are
  forgotten tests.
- **No `sleep()` in tests.** Use deterministic waits (channels, oneshot,
  condition variables).
- **No tests that depend on test execution order.** Every test is
  independent.
- **No tests that mutate shared global state.** Use `tempfile` per test.
- **No tests that hit the network without a mock.** CI must be offline-capable.
- **No tests that hardcode absolute paths.** Use `env!("CARGO_MANIFEST_DIR")`.
- **No tests that depend on the current time.** Inject a `Clock` trait.
