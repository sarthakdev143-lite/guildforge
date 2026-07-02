# Contributing to GuildForge

Thanks for your interest in contributing. This document is the short form; the
long form (governance, release process, security disclosure) lives in
[`docs/`](./docs/) and is linked below.

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct)
1:1. Be kind, be patient, assume good intent, and remember that maintainers are
volunteers.

## Before You Start

1. Read [`ARCHITECTURE.md`](./ARCHITECTURE.md) — you need to understand the
   parser → validator → planner → executor pipeline before touching code.
2. Read the relevant [ADR](./docs/adr/) for the area you're working in. ADRs
   record *why* things are the way they are; if you want to change the *what*,
   you'll likely need a new ADR.
3. Check [`TASKS.md`](./TASKS.md) — if a task is `todo`, comment on the
   tracking issue to claim it. If your idea isn't listed, open an issue first
   so we can agree on scope before code is written.

## Getting Set Up

Requirements:

- Rust 1.78+ (pinned in [`rust-toolchain.toml`](./rust-toolchain.toml))
- Node.js 20+ and pnpm 9+ (only if you're touching `apps/dashboard`)
- SQLite 3.34+ (only for live provider tests)
- A Discord bot token (only for `--features live-discord` tests; never required
  for the default test suite)

```bash
git clone https://github.com/your-org/guildforge
cd guildforge
cargo check --workspace
cargo test --workspace
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

All four commands must pass before a PR is mergeable.

## Workflow

1. **Issue first.** Open an issue describing what you want to change and why.
   Wait for a maintainer to acknowledge before opening a PR — unless the change
   is a typo, docs fix, or trivial bug fix.
2. **Branch from `main`.** Use a descriptive name:
   `feat/plan-diff-renderer`, `fix/role-color-parsing`, `docs/schema-typos`.
3. **One concern per PR.** A PR that touches the parser *and* the planner
   *and* the dashboard will be sent back. Split it.
4. **Tests are mandatory.** Every behavior change ships with tests. Bug fixes
   ship with a regression test that fails before the fix. See
   [`docs/TESTING.md`](./docs/TESTING.md) for the test pyramid.
5. **Update the changelog.** Add an entry under `[Unreleased]` in
   [`CHANGELOG.md`](./CHANGELOG.md) under the appropriate subsection.
6. **Update docs if you touch behavior.** Schema changes update
   [`docs/SCHEMA.md`](./docs/SCHEMA.md). New CLI flags update
   [`docs/CLI_REFERENCE.md`](./docs/CLI_REFERENCE.md). Cross-cutting decisions
   get a new ADR.
7. **Squash-merge.** We squash on merge. Your PR title becomes the commit
   subject — keep it under 72 chars, imperative mood (`add forum tag CRUD`).

## Coding Standards

These are enforced by CI; failing any of them blocks merge.

- `cargo fmt --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- No `unwrap()` / `expect()` / `panic!()` outside `#[cfg(test)]`. Use `?` and
  typed errors.
- No global mutable state. If you think you need it, open an ADR.
- Every public item has a doc comment with an example where reasonable.
- Tests live in the same file as the code (`#[cfg(test)] mod tests`) for unit
  tests, or in `tests/` for integration tests. See
  [`docs/TESTING.md`](./docs/TESTING.md) for the boundary.
- Error messages are lowercase, no trailing punctuation, and include enough
  context to debug without re-running. See
  [ADR-0005](./docs/adr/ADR-0005-error-model.md).

## Commit & PR Style

- Subject line: imperative, ≤72 chars, no period.
- Body: wrap at 80, explain *why* not *what*.
- Reference issues: `Closes #123`, `Refs #456`.
- PR description template lives in `.github/pull_request_template.md`.

Example:

```
Add forum tag CRUD to Discord provider

Forum channels support up to 20 tags that gate which posts can be
created. Until now GuildForge could create forum channels but not
their tags, making forum configs effectively read-only.

Closes #142
```

## Architecture Decision Records

Any change that affects cross-cutting structure, public API, or future
extensibility gets an ADR. ADRs are numbered sequentially, live in
[`docs/adr/`](./docs/adr/), and use the format described in
[`DECISIONS.md`](./DECISIONS.md). Once accepted, an ADR is *never edited
in place* — supersession is a new ADR that links to the old one.

## Testing

See [`docs/TESTING.md`](./docs/TESTING.md). The short version:

- Unit tests: every public function.
- Integration tests: per-crate, in `tests/`.
- Snapshot tests: parser output, plan output, exported YAML.
- CLI tests: `assert_cmd` + `predicates`.
- Provider tests: `wiremock`-based mocks by default; live tests behind
  `--features live-discord`.
- Coverage target: ≥90% for `config`, `parser`, `validation`, `planner`,
  `executor`, `state`, `provider`, `provider-discord`.

## Security Vulnerabilities

**Do not open a public issue for security vulnerabilities.** Email
`security@guildforge.dev` (TBD) with a description and repro. We respond
within 72 hours and coordinate disclosure per [`docs/SECURITY.md`](./docs/SECURITY.md).

## Licensing

Contributions are accepted under the same dual MIT/Apache-2.0 license as the
project. You retain copyright; we record it in the contributors file. No CLA.

## Recognition

All contributors are listed in `AUTHORS.md` (TBD). Significant contributions
are called out in release notes.
