# Dashboard (Phase 5)

> This directory is intentionally empty in Phase 0. The Next.js 16 + Tailwind
> 4 + shadcn/ui web UI is scaffolded in Phase 5 per
> [`ROADMAP.md`](../../ROADMAP.md).

## Architecture

The dashboard is a Next.js 16 App Router application that shells out to the
`guildforge` CLI binary for every operation. See
[`ADR-0008`](../../docs/adr/ADR-0008-dashboard-binding.md) for the binding
model and the rationale (subprocess vs in-process vs HTTP server).

## Stack (target)

- Next.js 16 (App Router, React Server Components)
- Tailwind CSS 4
- shadcn/ui (Radix primitives + Tailwind)
- TypeScript 5
- pnpm 9
- SQLite shared with CLI state (read-only access via `better-sqlite3`)

## Planned features (Phase 5)

- Login (single-user passphrase; token stored server-side encrypted)
- Server picker
- YAML editor (Monaco) with live validation via `guildforge validate`
- Visual plan viewer (tree + diff)
- Apply with live log streaming (WebSocket → subprocess stdout)
- History view (from state migrations table)
- Template browser

## Security

The bot token NEVER reaches the browser. All Discord API calls are
proxied through Next.js API routes, which spawn the `guildforge` CLI as
a subprocess with the token passed via `GUILDFORGE_BOT_TOKEN` env var.
See [`docs/SECURITY.md`](../../docs/SECURITY.md) for the full threat
model.

## Development (target)

```bash
cd apps/dashboard
pnpm install
pnpm dev   # http://localhost:3000
```

Requires `guildforge` binary on `PATH`. The dashboard setup wizard
(Phase 5) verifies this on first run.

## Not in v1

- Multi-user auth (Phase 7+)
- Remote dashboard (Phase 7+)
- In-process engine (rejected — see ADR-0008)
