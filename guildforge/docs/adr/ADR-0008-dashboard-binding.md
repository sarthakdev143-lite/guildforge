# ADR-0008: Dashboard ↔ Engine Binding (Subprocess, Not In-Process)

- **Status**: Accepted
- **Date**: 2026-07-01
- **Deciders**: founding eng
- **Tags**: dashboard, architecture, binding

## Context

GuildForge ships two surfaces:

1. The **CLI** (`guildforge`), a single Rust binary.
2. The **Dashboard**, a Next.js 16 web app (Phase 5).

The dashboard must expose every CLI capability: validate, plan, apply,
destroy, doctor, import, export, history. The question is how the
dashboard talks to the engine.

Options:

1. **Subprocess**: dashboard shells out to the `guildforge` binary for
   every operation.
2. **In-process (Rust)**: compile the engine into a WebAssembly module
   and run it in Next.js (Node.js).
3. **In-process (JS rewrite)**: rewrite the engine in TypeScript.
4. **Rust HTTP server**: wrap the engine in an `axum` HTTP server; the
   dashboard calls it over HTTP.
5. **gRPC**: same as 4 but with gRPC instead of REST.
6. **FFI**: bind Rust to Node.js via `napi-rs`.

This ADR picks one and justifies the choice.

## Decision

### Subprocess (Option 1)

The dashboard shells out to the `guildforge` binary for every operation.

Concretely:

- The dashboard's Next.js API routes call `execFile('guildforge', [...args])`.
- Output is captured as JSON (`--format json`).
- Long operations (`apply`, `destroy`) stream output over WebSocket by
  spawning the subprocess with `stdio: ['ignore', 'pipe', 'pipe']` and
  forwarding lines to the WebSocket client.
- The bot token is stored server-side (encrypted at rest) and passed to
  the subprocess via `GUILDFORGE_BOT_TOKEN` env var. The browser never
  sees the token.
- State file path is configured per-dashboard-instance and passed to
  every subprocess via `--state-file`.

```typescript
// apps/dashboard/app/api/plan/route.ts (sketch)
import { spawn } from 'node:child_process';
import { env } from 'node:process';

export async function POST(req: Request) {
  const { yamlPath } = await req.json();
  const proc = spawn('guildforge', ['plan', yamlPath, '--format', 'json'], {
    env: { ...env, GUILDFORGE_BOT_TOKEN: await getTokenFromKeychain() },
  });
  // stream proc.stdout to response...
}
```

### Why subprocess

#### Pros

1. **Single source of truth.** The CLI is the engine. The dashboard is
   a UI on top. Bug fixes in the engine apply to both surfaces
   automatically. No risk of drift between "what the CLI does" and
   "what the dashboard does".
2. **No second runtime.** The Rust engine runs in its own process; no
   FFI boundary, no WASM quirks, no `napi-rs` build complexity.
3. **Token isolation.** The token lives in the dashboard server's
   keychain and is passed via env var to short-lived subprocesses. It
   never touches the Node.js event loop beyond the `spawn` call. The
   browser never sees it.
4. **Stable contract.** The CLI's `--format json` output is a stable
   API (per [ADR-0003](./ADR-0003-planner-determinism.md)). The
   dashboard consumes that API; the engine internals can change freely.
5. **Trivial deployment.** One binary on the host. The dashboard
   requires only Node.js + the `guildforge` binary on `PATH`.
6. **No CORS / no port conflicts.** The dashboard is the only server.
   No second TCP port for an engine API.
7. **No async-runtime mismatch.** Rust's Tokio and Node.js's event
   loop don't have to coordinate. Each process owns its own runtime.
8. **Crash isolation.** A panic in the engine takes down the
   subprocess, not the dashboard. The dashboard reports the failure
   and stays up.
9. **Memory isolation.** The engine allocates memory in its own
   process; the dashboard's memory footprint stays small.

#### Cons (and mitigations)

1. **Subprocess startup overhead.** ~20 ms per `guildforge` invocation
   on Linux (cold). For `validate`, this dominates. Mitigation: for
   high-frequency operations (live YAML validation in the editor), the
   dashboard can run a long-lived `guildforge serve` subprocess that
   accepts JSON-RPC over stdin/stdout. This is an optional optimization
   for Phase 5.2; Phase 5.1 ships plain subprocess.
2. **Streaming complexity.** WebSocket forwarding of subprocess stdout
   is real code. Mitigation: well-tested Node.js patterns; ~50 lines
   per route.
3. **No shared in-memory state.** The dashboard can't cache the parsed
   `Config` in Rust memory across requests. Mitigation: the dashboard
   caches the JSON output of `plan` in Node.js memory; re-running
   `plan` is cheap (state read + pure compute).
4. **Process management.** The dashboard must track and clean up
   subprocesses. Mitigation: `execa` or `nanotarstreams` Node.js
   libraries handle this cleanly; we use `execa` for its superior
   child-process ergonomics.
5. **Windows path issues.** `guildforge` must be on `PATH`. Mitigation:
   dashboard installer adds it; documented in setup guide.

## Alternatives Considered

### H1: In-process (WASM)

Compile `crates/engine` to `wasm32-wasi`, run in Node.js via `wasmtime`
or `wasmer`.

Rejected. Pros: no subprocess overhead. Cons: WASM doesn't have a
stable story for `reqwest` (need to shim fetch), `sqlx` (need to shim
SQLite), and `tokio` (need single-threaded executor). Each shim is a
maintenance burden. The Rust-to-WASM toolchain for async + networking
is still maturing as of 2026. Revisit in 2 years.

### H2: In-process (JS rewrite)

Reimplement the engine in TypeScript.

Rejected. Doubles the maintenance burden. Bugs drift between surfaces.
Loses the type-safety and performance of Rust. The whole point of
GuildForge is that the engine is Rust; rewriting it in JS defeats the
purpose.

### H3: Rust HTTP server (axum)

Wrap the engine in `axum`, expose REST endpoints, dashboard calls
them.

Rejected for v1. Pros: clean separation, native async. Cons: second
TCP port, second process to manage, second binary to ship, second
auth boundary (dashboard auth + engine auth), second logging pipeline.
The subprocess model gives us the same separation with none of this
overhead.

Revisit if the dashboard ever needs to be remote from the engine
(e.g. hosted dashboard talking to on-prem engine). That's a Phase 7+
concern.

### H4: gRPC

Same as H3 but gRPC.

Rejected. Same cons as H3, plus gRPC in the browser requires
gRPC-Web or Connect, which adds another layer. REST-over-subprocess
is simpler and good enough.

### H5: FFI via `napi-rs`

Compile Rust to a Node.js native addon.

Rejected. FFI is fragile across Node.js versions (N-API stabilizes
this but doesn't eliminate it). Build complexity (each target needs a
separate compile). The engine uses async Tokio; bridging Tokio futures
to Node.js promises is non-trivial. Subprocess sidesteps all of this.

### H6: CLI-as-long-running-server (JSON-RPC over stdin/stdout)

`guildforge serve` enters a long-running mode that reads JSON-RPC
requests from stdin and writes responses to stdout.

This is a **future optimization** on top of the subprocess model, not
an alternative. The dashboard of Phase 5.1 uses plain subprocess
(`guildforge plan`, `guildforge apply`, etc.). If startup overhead
becomes a real UX problem (likely for live YAML validation in the
editor), Phase 5.2 adds `guildforge serve` and the dashboard uses it
for hot paths only. Plain subprocess remains as the fallback.

## Consequences

### Becomes easier

- Dashboard implementation: thin wrappers around `execa`. No engine
  code in JS.
- Engine changes: as long as the CLI's `--format json` contract is
  stable, the dashboard doesn't care about engine internals.
- Deployment: one binary, one Node.js app. No daemon.
- Auth: dashboard handles its own auth; passes token to subprocess via
  env. No second auth boundary.
- Logs: dashboard captures subprocess stdout/stderr and stores it
  alongside the operation record.

### Becomes harder

- Streaming UX is more work than calling an in-process function.
- Subprocess startup overhead is real for high-frequency operations.
  Mitigated by optional `guildforge serve` in Phase 5.2.
- Browser ↔ dashboard ↔ subprocess is two hops. WebSocket bridging is
  ~100 lines per route.

### New constraints

- The CLI's `--format json` output is a stable public API from v0.3.0
  onward (when the first JSON-emitting command ships). Breaking changes
  require a major version bump. This is documented in
  [`docs/CLI_REFERENCE.md`](../CLI_REFERENCE.md).
- The dashboard MUST run on the same host as the `guildforge` binary.
  Remote dashboard (talking to a remote engine) is Phase 7+.
- The dashboard MUST store the bot token server-side only. The browser
  never sees it. Auth between browser and dashboard is a user-chosen
  passphrase (Phase 5.1) or OS-level SSO (Phase 7+).
- Every CLI command that the dashboard uses MUST support `--format
  json` and MUST stream progress to stdout in a stable JSON Lines
  format. This is a new CLI requirement starting in Phase 5.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Subprocess startup overhead hurts UX | `guildforge serve` long-running mode for hot paths (Phase 5.2) |
| User installs dashboard without `guildforge` binary on PATH | Dashboard setup wizard verifies binary presence; offers to download |
| Token leaks via subprocess args (visible in `ps`) | Token passed via env var, never via CLI arg; documented in `docs/SECURITY.md` |
| Subprocess crashes mid-apply, dashboard hangs | Dashboard wraps every subprocess call in a 30-min timeout; reports timeout as failure |
| Concurrent dashboard operations conflict | State file lock (ADR-0002) prevents concurrent applies; dashboard surfaces `LockHeld` as user-friendly message |
| WebSocket disconnects mid-stream | Dashboard tracks operation ID; on reconnect, client polls `/api/operations/:id` for status |

## References

- [Next.js 16 docs](https://nextjs.org/docs)
- [execa](https://github.com/sindresorhus/execa)
- [Node.js child_process](https://nodejs.org/api/child_process.html)
- [Terraform Cloud agent model](https://developer.hashicorp.com/terraform/cloud-docs/agents)
- Related: [ADR-0002](./ADR-0002-state-store.md) (state lock prevents
  concurrent dashboard/CLI conflict),
  [ADR-0005](./ADR-0005-error-model.md) (JSON output is stable contract)
