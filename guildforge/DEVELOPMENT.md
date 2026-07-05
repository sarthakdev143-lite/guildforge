# Development Setup

> How to set up a local development environment for GuildForge.

## Prerequisites

| Tool | Version | Required for |
|---|---|---|
| Rust | 1.88+ (pinned in `rust-toolchain.toml`) | All Rust development |
| Node.js | 20+ | Dashboard development |
| npm | 10+ | Dashboard dependencies |
| SQLite | 3.34+ (bundled via `libsqlite3-sys`) | State store (automatic) |
| Discord bot token | — | Live testing (optional) |

## Getting Started

### 1. Clone and build

```bash
git clone https://github.com/your-org/guildforge
cd guildforge
cargo check --workspace
cargo test --workspace
```

### 2. Verify code quality

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
```

All four must pass before a PR is mergeable. CI enforces all of them.

### 3. Build the release binary

```bash
cargo build --release -p guildforge
./target/release/guildforge --version
```

The release binary is ~7.6 MB (LTO, strip, panic=abort).

### 4. Run the CLI locally

```bash
# Scaffold a config
./target/debug/guildforge init --template minimal

# Validate
./target/debug/guildforge validate guildforge.yaml

# Plan (requires bot token for live Discord)
GUILDFORGE_BOT_TOKEN=your_token ./target/debug/guildforge plan guildforge.yaml
```

### 5. Dashboard development

```bash
cd apps/dashboard
npm install
npm run dev   # http://localhost:3000
```

The dashboard shells out to the `guildforge` binary, which must be on
your `PATH`. For development, add the debug build:

```bash
export PATH="$PWD/target/debug:$PATH"
```

### 6. Run benchmarks

```bash
cargo bench -p guildforge-planner --bench planner_bench
```

Results are saved to `target/criterion/`. The 500-channel guild plan
target is < 1 second (actual: ~245 µs).

### 7. Generate man pages

```bash
cargo run --bin guildforge-manpages -- --output assets/man
man assets/man/guildforge.1
```

### 8. Generate shell completions

```bash
./target/release/guildforge completions bash > assets/completions/bash/_guildforge
./target/release/guildforge completions zsh > assets/completions/zsh/_guildforge
./target/release/guildforge completions fish > assets/completions/fish/_guildforge
```

Or use the script:

```bash
python3 scripts/generate-completions.py --output-dir assets/completions
```

## Project Structure

```
guildforge/
├── apps/
│   ├── cli/              # `guildforge` binary (15 commands)
│   └── dashboard/        # Next.js 16 dashboard (14 routes)
├── crates/
│   ├── config/           # Strongly-typed YAML models (40 tests)
│   ├── parser/           # YAML → Config (9 tests + 10 fuzz tests)
│   ├── validation/       # Semantic validation (26 tests)
│   ├── engine/           # Workflow orchestrator (17 tests + 3 E2E)
│   ├── planner/          # Deterministic diff (26 tests + 7 property)
│   ├── executor/         # Plan executor (6 tests)
│   ├── state/            # SQLite state store (12 tests)
│   ├── provider/         # Provider trait (10 tests + 10 conformance)
│   ├── provider-discord/ # Discord implementation (39 tests + 14 mock)
│   ├── shared/           # Cross-crate primitives (18 tests)
│   └── logging/          # Tracing init (5 tests)
├── docs/                 # Architecture, ADRs, schema, CLI ref
├── docs-site/            # mdbook documentation site
├── examples/             # Runnable example configs
├── templates/            # Starter configs for `guildforge init`
├── packaging/            # Homebrew formula + scoop manifest
├── scripts/              # Completion generation script
├── tests/                # Cross-crate integration tests
└── .github/workflows/    # CI + Release workflows
```

## Testing Strategy

| Layer | Location | Tool | Count |
|---|---|---|---|
| Unit | `#[cfg(test)] mod tests` | `cargo test` | ~200 |
| Integration | `tests/` per crate | `cargo test` | ~50 |
| Property | `tests/` per crate | `proptest` | ~24 |
| Mock HTTP | `provider-discord/tests/` | `wiremock` | 14 |
| E2E pipeline | `engine/tests/` | custom | 3 |
| Conformance | `provider/tests/` | custom | 10 |
| Benchmarks | `planner/benches/` | `criterion` | 8 |

Run everything:

```bash
cargo test --workspace
cargo bench -p guildforge-planner
```

## Live Testing (optional)

For testing against real Discord:

1. Create a bot at <https://discord.com/developers/applications>
2. Grant it `Administrator` permissions in a throwaway guild
3. Store the token:

```bash
guildforge login
# or
echo "YOUR_TOKEN" | guildforge login
```

4. Run the full pipeline:

```bash
guildforge init --template company
guildforge validate guildforge.yaml
guildforge plan guildforge.yaml
guildforge apply --auto-approve guildforge.yaml
guildforge doctor
guildforge destroy --auto-approve guildforge.yaml
```

**Never run live tests against a guild with real users.**

## CI

GitHub Actions runs 7 jobs on every push and PR:

| Job | Purpose |
|---|---|
| `fmt` | `cargo fmt --check` |
| `clippy` | `cargo clippy -D warnings` |
| `test` | `cargo test` on Linux, macOS, Windows |
| `docs` | `cargo doc` with `-D warnings` |
| `deny` | `cargo deny` (license + advisory check) |
| `audit` | `cargo audit` (known vulnerabilities) |
| `bench` | Benchmark regression check (PR only) |

The `release` workflow triggers on `git tag v*` and builds 5 platform
targets.

## Adding a New Resource Type

1. Add a variant to `ResourceKind` in `crates/provider/src/resource.rs`
2. Add a corresponding `Resource` variant with typed fields
3. Add serde models in `crates/config/src/` (new module or extend existing)
4. Add conversion logic in `crates/planner/src/convert.rs`
5. Add validation rules in `crates/validation/src/lib.rs`
6. Implement CRUD in `crates/provider-discord/src/resources/`
7. Add to the dependency graph in `crates/planner/src/diff.rs`
8. Write tests for each layer
9. Update `docs/SCHEMA.md`

## Adding a New Provider

1. Create `crates/provider-<name>/` with `Cargo.toml`
2. Implement the `Provider` trait (see `crates/provider/tests/conformance.rs`)
3. Wire it into `apps/cli/src/main.rs` at startup
4. Add mock tests using `wiremock` or similar
5. Add live tests behind `--features live-<name>`

## Debugging Tips

### State file inspection

```bash
sqlite3 guildforge.db "SELECT addr, kind, tainted FROM resources ORDER BY addr"
sqlite3 guildforge.db "SELECT * FROM migrations_log ORDER BY id DESC LIMIT 10"
```

### Verbose logging

```bash
GUILDFORGE_LOG_LEVEL=debug guildforge plan guildforge.yaml
GUILDFORGE_LOG_LEVEL=trace GUILDFORGE_LOG_FORMAT=json guildforge apply --auto-approve guildforge.yaml
```

### Force-refresh state from live

```bash
guildforge doctor   # shows drift
# To absorb drift into state (accept out-of-band changes):
# (not yet implemented — use `guildforge apply` to reconcile)
```

### Clean state

```bash
rm guildforge.db guildforge.db.lock
guildforge apply --auto-approve guildforge.yaml
```
