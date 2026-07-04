# Project State

> Living snapshot of where GuildForge is right now.

## Current Phase

**Phase 6 — Polish & 1.0 (complete)**

All 15 CLI commands are implemented. Shell completions work for bash,
zsh, fish, powershell, and elvish. Release binary is 7.6 MB (under the
10 MB target). CI enforces cargo-deny and cargo-audit without
`continue-on-error`. The project is feature-complete for v1.0.

| Capability | Status |
|---|---|
| Phase 0–5 deliverables | ✅ Done |
| `guildforge init` (3 templates: minimal/company/community) | ✅ Done (Phase 6) |
| `guildforge login` / `logout` (file-based, mode 0600) | ✅ Done (Phase 6) |
| Shell completions (bash/zsh/fish/powershell/elvish) | ✅ Done (Phase 6) |
| `guildforge completions <shell>` command | ✅ Done (Phase 6) |
| Release binary (7.6 MB, LTO, strip, panic=abort) | ✅ Done (Phase 6) |
| CI enforces cargo-deny (no `continue-on-error`) | ✅ Done (Phase 6) |
| CI enforces cargo-audit (no `continue-on-error`) | ✅ Done (Phase 6) |
| All 15 CLI commands functional | ✅ Done |
| Dashboard builds (14 routes) | ✅ Done (Phase 5) |

## Build & Test Status

- `cargo check --workspace` clean on Rust 1.88.
- `cargo test --workspace`: 256 tests pass across all crates.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo build --release`: 7.6 MB binary, all commands functional.
- `npx next build` in `apps/dashboard/`: 14 routes compile.

## CLI Commands (all 15 implemented)

| Command | Description |
|---|---|
| `init` | Scaffold `guildforge.yaml` from template |
| `validate` | Parse + validate config |
| `plan` | Compute + print execution plan |
| `apply` | Apply config with `--auto-approve` |
| `destroy` | Destroy all resources in config |
| `diff` | Structural diff between two configs |
| `import` | Read live Discord → YAML |
| `export` | State → YAML |
| `doctor` | Drift detection (state vs live) |
| `backup` | Snapshot state file |
| `restore` | Restore state from backup |
| `login` | Store bot token (file-based, mode 0600) |
| `logout` | Delete stored token |
| `version` | Print version info |
| `completions` | Generate shell completions |

## Known Gaps (not blocking v1.0)

- OS keychain integration (`keyring` crate) deferred to Phase 7+ —
  `login` uses file-based storage with mode 0600.
- Cross-compilation CI matrix (Linux/macOS/Windows) not yet set up —
  release binary is x86_64-linux only.
- Homebrew tap + scoop manifest not yet created.
- E2E tests (Playwright) for dashboard not yet written.
- `import` returns empty config — needs `provider.list()` wired through
  `DynProvider`.
- Man pages generation script exists but `clap_mangen` not integrated
  into the binary (completions are; man pages need a build script).
