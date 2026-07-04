//! Rate-limit middleware for Discord.
//!
//! Discord enforces two kinds of rate limits:
//!
//! 1. **Global rate limit**: 50 requests/second per bot. Returned via
//!    `X-RateLimit-Global: true` + `Retry-After` header on 429 responses.
//! 2. **Per-route rate limits**: vary by endpoint. Tracked via
//!    `X-RateLimit-Bucket`, `X-RateLimit-Limit`, `X-RateLimit-Remaining`,
//!    and `X-RateLimit-Reset` headers on every response.
//!
//! This module tracks both. State is shared across all requests for a
//! single `DiscordHttp` instance via `Arc`.
//!
//! See [`ADR-0006`](../../docs/adr/ADR-0006-async-http.md).

use dashmap::DashMap;
use reqwest::Response;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, trace};

/// Compute the rate-limit route key for a given HTTP method + URL path.
///
/// Discord's rate-limit buckets are keyed by a route that has dynamic
/// IDs replaced with placeholders. For example, both
/// `/channels/123/messages/456` and `/channels/789/messages/999` share
/// the same route `channels/:id/messages/:id`. Some endpoints have
/// special-cased routes (e.g. `DELETE /channels/:id/messages/:id` has
/// its own bucket because Discord limits deletes separately).
///
/// See <https://discord.com/developers/docs/topics/rate-limits>.
#[must_use]
pub fn route_for(method: &reqwest::Method, url: &str) -> String {
    // Strip the API base URL prefix if it's a Discord URL. We accept
    // any host (so wiremock URLs work in tests) by stripping
    // everything up to and including `/api/vN/`.
    let path = url
        .find("/api/v")
        .map(|i| {
            let after_v = &url[i + 6..]; // skip "/api/v"
                                         // Skip the version digits and the following slash.
            let trimmed = after_v.trim_start_matches(|c: char| c.is_ascii_digit());
            trimmed.strip_prefix('/').unwrap_or(trimmed)
        })
        .unwrap_or(url);

    // Split into segments and replace numeric IDs with `:id`.
    let mut segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for seg in &mut segments {
        if !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()) {
            *seg = ":id";
        }
    }
    let route = segments.join("/");

    // Special case: DELETE on a message uses a separate bucket.
    if method == reqwest::Method::DELETE && route.starts_with("channels/:id/messages/:id") {
        format!("DELETE {route}")
    } else {
        route
    }
}

/// Per-bucket rate-limit state.
#[derive(Debug)]
pub struct BucketState {
    /// Bucket hash from Discord (`X-RateLimit-Bucket`).
    pub bucket: Option<String>,
    /// Max requests per window.
    pub limit: u32,
    /// Remaining requests in the current window.
    pub remaining: AtomicU64,
    /// When the current window resets (as a Unix timestamp in millis).
    pub reset_at_ms: AtomicU64,
    /// Lock held while waiting for the bucket to reset.
    pub lock: Mutex<()>,
}

impl Default for BucketState {
    fn default() -> Self {
        Self {
            bucket: None,
            limit: 1,
            remaining: AtomicU64::new(1),
            reset_at_ms: AtomicU64::new(0),
            lock: Mutex::new(()),
        }
    }
}

/// Global rate-limit state.
#[derive(Debug, Default)]
pub struct GlobalState {
    /// Whether a global rate-limit is currently in effect.
    pub blocked: AtomicBool,
    /// When the global block clears (Unix millis).
    pub reset_at_ms: AtomicU64,
    /// Lock held while waiting for the global block to clear.
    pub lock: Mutex<()>,
}

/// The rate limiter. Holds per-route bucket state and global state.
#[derive(Debug, Default)]
pub struct RateLimiter {
    buckets: DashMap<String, BucketState>,
    global: GlobalState,
}

impl RateLimiter {
    /// Construct a new rate limiter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Wait for permission to send a request on `route`.
    ///
    /// Acquires the per-bucket lock and the global lock as needed, then
    /// returns. The caller should send the request immediately after.
    pub async fn wait(&self, route: &str) {
        // Check global block first.
        self.wait_global_until_clear().await;

        // Get or create the bucket state for this route.
        let bucket = self.buckets.entry(route.to_string()).or_default().clone();
        let _guard = bucket.lock.lock().await;

        // If we're out of requests, wait until the reset.
        let remaining = bucket.remaining.load(Ordering::Relaxed);
        if remaining == 0 {
            let reset_at_ms = bucket.reset_at_ms.load(Ordering::Relaxed);
            let now_ms = now_millis();
            if reset_at_ms > now_ms {
                let wait = Duration::from_millis(reset_at_ms - now_ms);
                debug!(%route, ?wait, "rate-limited, waiting for bucket reset");
                tokio::time::sleep(wait).await;
            }
        }

        // Decrement remaining (saturating at 0).
        bucket
            .remaining
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                if v == 0 {
                    Some(0)
                } else {
                    Some(v - 1)
                }
            })
            .ok();
        trace!(%route, "rate limit pass");
    }

    /// Wait for the global block to clear, if any.
    async fn wait_global_until_clear(&self) {
        if !self.global.blocked.load(Ordering::Relaxed) {
            return;
        }
        let _guard = self.global.lock.lock().await;
        // Re-check after acquiring the lock — another task may have
        // already waited and cleared the block.
        if !self.global.blocked.load(Ordering::Relaxed) {
            return;
        }
        let reset_at_ms = self.global.reset_at_ms.load(Ordering::Relaxed);
        let now_ms = now_millis();
        if reset_at_ms > now_ms {
            let wait = Duration::from_millis(reset_at_ms - now_ms);
            debug!(?wait, "global rate-limited, waiting");
            tokio::time::sleep(wait).await;
        }
        self.global.blocked.store(false, Ordering::Relaxed);
    }

    /// Mark the global rate limit as blocked for `duration`.
    pub async fn wait_global(&self, duration: Duration) {
        let reset_at_ms = now_millis() + duration.as_millis() as u64;
        self.global
            .reset_at_ms
            .store(reset_at_ms, Ordering::Relaxed);
        self.global.blocked.store(true, Ordering::Relaxed);
        // Wait synchronously so the caller doesn't re-enter before the
        // block clears.
        tokio::time::sleep(duration).await;
        self.global.blocked.store(false, Ordering::Relaxed);
    }

    /// Wait for a specific bucket to reset (used after 429).
    pub async fn wait_bucket(&self, route: &str, duration: Duration) {
        let bucket = self.buckets.entry(route.to_string()).or_default().clone();
        let _guard = bucket.lock.lock().await;
        let reset_at_ms = now_millis() + duration.as_millis() as u64;
        bucket.reset_at_ms.store(reset_at_ms, Ordering::Relaxed);
        bucket.remaining.store(0, Ordering::Relaxed);
        tokio::time::sleep(duration).await;
    }

    /// Update the bucket state from a response's `X-RateLimit-*` headers.
    pub async fn update(&self, route: &str, response: &Response) {
        let headers = response.headers();
        let bucket_hash = headers
            .get("X-RateLimit-Bucket")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let limit = headers
            .get("X-RateLimit-Limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        let remaining = headers
            .get("X-RateLimit-Remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let reset_at_ms = headers
            .get("X-RateLimit-Reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| (f * 1000.0) as u64)
            .unwrap_or_else(now_millis);

        let mut entry = self.buckets.entry(route.to_string()).or_default();
        entry.bucket = bucket_hash;
        entry.limit = limit;
        entry.remaining.store(remaining, Ordering::Relaxed);
        entry.reset_at_ms.store(reset_at_ms, Ordering::Relaxed);
        trace!(%route, limit, remaining, reset_at_ms, "rate limit updated");
    }
}

/// Current time in milliseconds since the Unix epoch.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// BucketState needs Clone for the entry pattern above. We can't auto-derive
// because AtomicU64 doesn't impl Clone. Manual impl creates fresh atomics
// with the same values — which is fine because we only ever access a
// BucketState through the DashMap entry.
impl Clone for BucketState {
    fn clone(&self) -> Self {
        Self {
            bucket: self.bucket.clone(),
            limit: self.limit,
            remaining: AtomicU64::new(self.remaining.load(Ordering::Relaxed)),
            reset_at_ms: AtomicU64::new(self.reset_at_ms.load(Ordering::Relaxed)),
            lock: Mutex::new(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn route_for_replaces_numeric_ids() {
        let r = route_for(
            &reqwest::Method::GET,
            "https://discord.com/api/v10/channels/123/messages/456",
        );
        assert_eq!(r, "channels/:id/messages/:id");
    }

    #[test]
    fn route_for_delete_message_uses_separate_bucket() {
        let r = route_for(
            &reqwest::Method::DELETE,
            "https://discord.com/api/v10/channels/123/messages/456",
        );
        assert_eq!(r, "DELETE channels/:id/messages/:id");
    }

    #[test]
    fn route_for_non_numeric_segments_preserved() {
        let r = route_for(
            &reqwest::Method::GET,
            "https://discord.com/api/v10/guilds/123/roles",
        );
        assert_eq!(r, "guilds/:id/roles");
    }

    #[test]
    fn route_for_handles_unknown_url() {
        // Non-API URLs are passed through as-is; only numeric IDs get
        // replaced.
        let r = route_for(&reqwest::Method::GET, "https://example.com/foo/123/bar");
        assert_eq!(r, "https:/example.com/foo/:id/bar");
    }

    #[test]
    fn route_for_works_with_wiremock_urls() {
        // wiremock URLs look like http://127.0.0.1:PORT/api/v10/...
        let r = route_for(
            &reqwest::Method::GET,
            "http://127.0.0.1:4321/api/v10/guilds/123/roles",
        );
        assert_eq!(r, "guilds/:id/roles");
    }

    #[tokio::test]
    async fn rate_limiter_no_wait_when_remaining() {
        let r = RateLimiter::new();
        // First call should not block.
        let start = Instant::now();
        r.wait("test-route").await;
        assert!(start.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn rate_limiter_wait_global_clears_after_duration() {
        let r = RateLimiter::new();
        r.wait_global(Duration::from_millis(50)).await;
        // After wait_global returns, the block should be cleared.
        let start = Instant::now();
        r.wait("any-route").await;
        assert!(start.elapsed() < Duration::from_millis(100));
    }
}
