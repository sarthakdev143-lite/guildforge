# Cross-crate integration tests.

> Lives at the workspace root so it can depend on every crate. Per-crate
> integration tests live under `<crate>/tests/`.

This directory is intentionally empty in Phase 0. Integration tests are
added phase by phase per [`docs/TESTING.md`](../docs/TESTING.md).

## Planned tests

### Phase 1 — Config layer

- `tests/config_round_trip.rs` — parse every example, serialize back, assert byte-stable YAML.
- `tests/validate_examples.rs` — `guildforge validate examples/*.yaml` exits 0.
- `tests/validate_broken_examples.rs` — `guildforge validate examples/broken/*.yaml` exits non-zero with expected codes.

### Phase 3 — Plan & apply

- `tests/plan_determinism.rs` — same `(config, state)` produces byte-identical plans.
- `tests/apply_idempotency.rs` — apply twice → second is no-op.
- `tests/apply_rollback.rs` — crash mid-apply → state rolled back.
- `tests/doctor_drift.rs` — out-of-band edit → doctor detects.

### Phase 4 — Import / export

- `tests/import_export_round_trip.rs` — import → export → byte-identical.

## Conventions

- Tests run with `GUILDFORGE_NO_NETWORK=1` so no test can hit real Discord.
- State files use `tempfile::NamedTempFile` per test.
- Snapshot output via `insta` lives in `tests/snapshots/`.
