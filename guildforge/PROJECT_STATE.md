# Project State

> Living snapshot of where GuildForge is right now.

## Current Phase

**Phase 5 — Dashboard (complete)**

The Next.js 16 dashboard is scaffolded and builds successfully. It shells
out to the `guildforge` CLI binary per ADR-0008. The bot token is stored
encrypted on the server and never sent to the browser. All API routes
are wired: validate, plan, apply (SSE streaming), doctor, export,
history, login, logout, version.

| Capability | Status |
|---|---|
| Phase 0–4 deliverables | ✅ Done |
| Next.js 16 app scaffold | ✅ Done (Phase 5) — 14 routes build |
| Tailwind CSS 4 + dark theme | ✅ Done (Phase 5) |
| API routes (validate/plan/apply/doctor/export/history/login/logout/version) | ✅ Done (Phase 5) |
| Token storage (AES-256-GCM encrypted at rest) | ✅ Done (Phase 5) |
| Session (passphrase + httpOnly cookie) | ✅ Done (Phase 5) |
| Login page | ✅ Done (Phase 5) |
| YAML editor + plan viewer | ✅ Done (Phase 5) |
| Apply with SSE log streaming | ✅ Done (Phase 5) |
| History page | ✅ Done (Phase 5) |
| E2E tests (Playwright) | ❌ Not done (Phase 6) |
| Polish & 1.0 | ❌ Not started (Phase 6) |

## Build & Test Status

- `cargo check --workspace` clean on Rust 1.88.
- `cargo test --workspace`: 250 Rust tests pass.
- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `npx next build` in `apps/dashboard/`: 14 routes compile successfully.

## Known Gaps

- Dashboard E2E tests not yet written (Phase 6).
- Token storage uses file-based AES encryption; OS keychain integration
  is Phase 6.
- `import` returns empty config — needs `provider.list()` wired through
  `DynProvider`.
- `init`, `login` (CLI), `logout` (CLI) are stubs.
- No cross-compiled release binaries yet (Phase 6).
