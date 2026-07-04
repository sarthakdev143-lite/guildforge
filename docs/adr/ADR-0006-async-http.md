# ADR-0006: Async Runtime & HTTP (Tokio + Reqwest)

- **Status**: Accepted
- **Date**: 2026-07-01
- **Deciders**: founding eng
- **Tags**: runtime, http, async, rate-limiting

## Context

GuildForge talks to Discord over HTTPS. The Discord REST API has:

- A **global rate limit** of 50 requests/second per bot.
- **Per-route rate limits** that vary by endpoint (e.g. 5/2s for channel
  modifications, 10/10s for webhook creation).
- **`429 Too Many Requests`** responses with `Retry-After` headers
  (seconds for global; per-bucket for route-specific).
- **`X-RateLimit-*` headers** on every response: `Limit`, `Remaining`,
  `Reset` (epoch seconds with millisecond precision), `Bucket` (hash of
  the route).

GuildForge needs to:

1. Be fast enough that applying a 500-channel guild doesn't take 30
   minutes.
2. Respect rate limits without getting the bot banned.
3. Retry transient failures (5xx, network) with backoff.
4. Not retry permanent failures (4xx other than 429).
5. Time out individual requests so a hung connection doesn't stall an
   apply forever.
6. Stream logs so the user sees progress.
7. Be cancelable — Ctrl-C should stop an apply cleanly, not leave
   Discord in a half-applied state.

## Decision

### Runtime: Tokio

```toml
# Cargo.toml
tokio = { version = "1", features = ["full"] }
```

`tokio` is the de-facto Rust async runtime. Every async crate we use
(`reqwest`, `sqlx`, `wiremock`) targets Tokio. There is no real
alternative.

`features = ["full"]` for v1; we'll trim to specific features (`rt`,
`rt-multi-thread`, `macros`, `net`, `time`, `sync`, `fs`) in Phase 6
for binary size.

The CLI uses a multi-threaded runtime (4 threads by default, capped at
8). The dashboard uses a single-threaded runtime per request (Next.js
API routes already handle concurrency).

### HTTP client: Reqwest

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

- `rustls-tls` (not `native-tls`): pure-Rust TLS, no OpenSSL dep, easier
  cross-compilation, smaller binary. Trade-off: relies on
  `rustls-native-certs` to find the system root store, which is slightly
  less reliable than `native-tls` on some Linux distros. Acceptable.
- `default-features = false`: drops `tokio-util` features we don't need.
- One shared `reqwest::Client` per `DiscordProvider` instance (connection
  pooling, HTTP/2 multiplexing).

### Rate-limit middleware

Custom Tower-style middleware in
`crates/provider-discord/src/client/rate_limit.rs`. Layers, in order
(outer to inner):

```
┌────────────────────────────────────────────────────────────┐
│  Retry layer (3 retries, exp backoff + jitter)             │
├────────────────────────────────────────────────────────────┤
│  Timeout layer (30s default, 5min for uploads)             │
├────────────────────────────────────────────────────────────┤
│  Rate-limit layer (per-bucket + global)                    │
├────────────────────────────────────────────────────────────┤
│  Auth header layer (adds Authorization: Bot <token>)       │
├────────────────────────────────────────────────────────────┤
│  reqwest::Client (connection pool, HTTP/2)                 │
└────────────────────────────────────────────────────────────┘
```

Rate-limit layer internals:

```rust
// sketch — full impl in crates/provider-discord/src/client/rate_limit.rs
struct RateLimitLayer {
    buckets: DashMap<BucketHash, BucketState>,
    global: Arc<GlobalState>,
}

struct BucketState {
    limit: u32,
    remaining: AtomicU32,
    reset_at: Mutex<Instant>,
}

// On every request:
// 1. Compute route hash (e.g. "channels/:id" → hash)
// 2. Look up bucket state
// 3. If remaining == 0 and now < reset_at: sleep until reset_at
// 4. If global limit hit: sleep until global reset
// 5. Send request
// 6. Update bucket state from X-RateLimit-* response headers
// 7. If 429: read Retry-After, sleep, retry (the retry layer above
//    does NOT see 429s; they're handled here)
```

The layer uses `dashmap` for concurrent bucket state (the engine may
issue concurrent requests in the executor).

### Retry strategy

Retry layer retries on:

- Network errors (`reqwest::Error::is_connect`, `is_timeout`).
- HTTP 5xx (server errors).
- HTTP 429 is handled by the rate-limit layer below, not here.

Does NOT retry on:

- HTTP 4xx other than 429 (client errors: 400 Bad Request, 403
  Forbidden, 404 Not Found, etc.).
- HTTP 2xx (success).

Retry policy:

- Up to 3 attempts (initial + 2 retries).
- Exponential backoff: 1s, 2s, 4s.
- Jitter: ±25% of the delay, to avoid thundering herd.
- Total retry budget per request: 10s.

### Cancellation

The CLI installs a `tokio::signal::ctrl_c` handler that cancels the
engine's main future. The engine uses a `CancellationToken` from the
`tokio_util` crate; every long operation checks it.

On cancellation:

1. Stop issuing new requests.
2. Wait for in-flight requests to complete (don't abandon mid-write to
   Discord — that leaves Discord in an inconsistent state).
3. Roll back the state transaction.
4. Release the state lock.
5. Print a "canceled, no changes committed" message and exit 130.

### Concurrency in the executor

The executor applies operations **sequentially within a topological
level** but **concurrently across independent resources** at the same
level. For example, creating 50 channels in the same category is
sequential (Discord's per-route limit would reject concurrent creates
anyway), but creating channels in different categories is concurrent
(up to a configurable max, default 4).

Concurrency is bounded by a `tokio::sync::Semaphore` with permit count
= 4 (default, override with `--max-concurrency`).

### Timeouts

- Per-request: 30s default. Override with `--http-timeout`.
- File uploads (server icon, role icon): 5min.
- Total apply time: unbounded (user can Ctrl-C).

### Connection pooling

`reqwest::Client` is configured with:

- `pool_max_idle_per_host(20)`
- `pool_idle_timeout(Duration::from_secs(90))`
- `tcp_keepalive(Duration::from_secs(60))`
- `http2_keep_alive_interval(Duration::from_secs(30))`

This is enough to keep HTTP/2 connections warm without leaking sockets.

## Alternatives Considered

### F1: `hyper` directly

Rejected. `reqwest` is a thin wrapper over `hyper` that adds TLS,
JSON, redirects, cookies, and pooling. Re-implementing this is a
distraction. `reqwest`'s API is stable and well-documented.

### F2: `isahc` / `ureq` / `attohttpc`

Rejected. `isahc` is unmaintained as of 2024. `ureq` and `attohttpc`
are blocking-only; we need async for concurrency.

### F3: `hyper` + `rustls` without `reqwest`

Rejected. Same as F1 with extra steps. We'd reimplement half of
`reqwest`.

### F4: `async-std` instead of `tokio`

Rejected. The Rust ecosystem has converged on Tokio. `async-std` is
viable but every other crate we use (`reqwest`, `sqlx`, `wiremock`)
targets Tokio natively; using `async-std` would force compat shims
(`async_compat`) everywhere.

### F5: `native-tls` instead of `rustls-tls`

Rejected. `native-tls` links to OpenSSL on Linux, Secure Transport on
macOS, and SChannel on Windows. This triples our cross-compilation
matrix and bloats CI. `rustls-tls` is pure Rust, links to a single
`ring` or `aws-lc-rs` crypto backend, and cross-compiles trivially.

### F6: Use Discord gateway (WebSocket) instead of REST

Rejected for v1. The gateway is for real-time events (new messages,
member joins, etc.). GuildForge is a batch CLI, not a long-lived bot.
REST is simpler and is what every Terraform-style tool does.

### F7: Manual rate-limit tracking without middleware

Rejected. Spreads rate-limit logic across every call site.
Centralizing in middleware means one place to test, one place to
update when Discord changes the rules.

### F8: `governor` crate for rate limiting

Considered. `governor` is a nice generic rate-limiter but doesn't
model Discord's per-bucket state machine. We'd end up wrapping it
anyway. A custom middleware is ~150 lines and matches Discord's model
exactly.

## Consequences

### Becomes easier

- Concurrency: Tokio's `spawn` + `Semaphore` gives us bounded
  concurrency in ~20 lines.
- Cancellation: `CancellationToken` is clean and ergonomic.
- Testing: `wiremock` mocks the HTTP layer; tests are deterministic.
- Cross-compilation: pure-Rust deps mean `cargo build --target` just
  works.

### Becomes harder

- Binary size: Tokio + reqwest + rustls adds ~3 MB to the release
  binary. Acceptable for v1; trim in Phase 6.
- Compile time: Tokio is a big crate. Incremental builds are fine;
  clean builds take ~30s longer than a minimal project.
- Rate-limit middleware is custom code. Bugs here are subtle (off-by-
  one on `reset_at`, wrong bucket hash). Mitigation: extensive unit
  tests with mocked clocks.

### New constraints

- All async code uses `tokio`. No `async-std`, no `smol`, no `embassy`.
  Clippy lint `disallowed_types` enforces.
- All HTTP goes through `reqwest`. No `hyper` directly, no `isahc`, no
  `ureq`. Same lint.
- TLS is `rustls-tls`. No `native-tls`. Same lint.
- Rate-limit logic is in `crates/provider-discord/src/client/rate_limit.rs`
  ONLY. No other module knows about Discord's buckets.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Discord changes rate-limit headers | Middleware reads headers defensively; missing headers fall back to conservative defaults |
| Bot gets banned for hitting 429s too often | Middleware sleeps on every 429; tests verify the sleep happens before retry; live test runs `apply` against a test guild and asserts no 429s in logs |
| `reqwest` 0.13 breaks API | Pin minor version; bump deliberately with a migration PR |
| Tokio runtime panics on shutdown | Use `tokio::runtime::Runtime::shutdown_timeout` to give in-flight ops 5s to complete |
| Cancellation leaves Discord half-applied | Wait for in-flight requests before rolling back state; document that a canceled apply may need `doctor` to verify |
| HTTP/2 stream limits block requests | `pool_max_idle_per_host(20)` is enough; monitor in live tests |

## References

- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
- [Reqwest docs](https://docs.rs/reqwest)
- [Discord rate limit docs](https://discord.com/developers/docs/topics/rate-limits)
- [rustls](https://docs.rs/rustls)
- [tower middleware](https://docs.rs/tower)
- Related: [ADR-0001](./ADR-0001-provider-trait.md) (Provider trait is async),
  [ADR-0007](./ADR-0007-idempotency-ordering.md) (executor uses this HTTP stack)
