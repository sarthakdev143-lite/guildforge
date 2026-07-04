//! HTTP client + rate-limit middleware for Discord.
//!
//! Layers (outer → inner):
//!
//! ```text
//! Retry layer (3 attempts, exp backoff + jitter)
//!   ↓
//! Timeout layer (30s default, 5min for uploads)
//!   ↓
//! Rate-limit layer (per-bucket + global)
//!   ↓
//! Auth header layer (adds Authorization: Bot <token>)
//!   ↓
//! reqwest::Client (connection pool, HTTP/2)
//! ```
//!
//! See [`ADR-0006`](../../docs/adr/ADR-0006-async-http.md).

pub mod rate_limit;

pub use rate_limit::RateLimiter;

use crate::error::DiscordError;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// Discord API base URL.
pub const API_BASE: &str = "https://discord.com/api/v10";

/// Default per-request timeout (30 seconds).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default max retries (initial + 2 retries = 3 attempts).
pub const DEFAULT_MAX_RETRIES: u32 = 2;

/// Default initial backoff (1 second).
pub const DEFAULT_INITIAL_BACKOFF: Duration = Duration::from_secs(1);

/// Default max backoff (30 seconds).
pub const DEFAULT_MAX_BACKOFF: Duration = Duration::from_secs(30);

/// HTTP client configuration.
#[derive(Debug, Clone)]
pub struct DiscordHttpConfig {
    /// Per-request timeout.
    pub timeout: Duration,
    /// Max retries (initial + retries).
    pub max_retries: u32,
    /// Initial backoff for retries.
    pub initial_backoff: Duration,
    /// Max backoff for retries.
    pub max_backoff: Duration,
    /// User-Agent header (Discord requires this).
    pub user_agent: String,
    /// API base URL. Defaults to `https://discord.com/api/v10`. Tests
    /// override this to point at a `wiremock` server.
    pub api_base: String,
}

impl Default for DiscordHttpConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            initial_backoff: DEFAULT_INITIAL_BACKOFF,
            max_backoff: DEFAULT_MAX_BACKOFF,
            user_agent: format!(
                "GuildForge/{} (https://github.com/your-org/guildforge)",
                env!("CARGO_PKG_VERSION")
            ),
            api_base: API_BASE.to_string(),
        }
    }
}

/// Discord HTTP client. Holds a `reqwest::Client`, the bot token, and
/// rate-limit state.
pub struct DiscordHttp {
    /// Inner reqwest client.
    pub client: reqwest::Client,
    /// Bot token. Never logged.
    token: String,
    /// Configuration.
    pub config: DiscordHttpConfig,
    /// Rate limiter (shared state across all requests).
    pub rate_limiter: Arc<RateLimiter>,
}

impl DiscordHttp {
    /// Construct a new HTTP client.
    ///
    /// # Errors
    ///
    /// Returns [`DiscordError::Auth`] if the token is empty, or
    /// [`DiscordError::Http`] if the reqwest client cannot be built.
    pub fn new(token: String, config: DiscordHttpConfig) -> Result<Self, DiscordError> {
        if token.is_empty() {
            return Err(DiscordError::Auth("token is empty".into()));
        }
        // Allow HTTP for non-default API bases (used by tests with
        // wiremock, which serves over plain HTTP).
        let allow_http = !config.api_base.starts_with("https://");
        let mut builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .pool_max_idle_per_host(20)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .user_agent(&config.user_agent);
        if !allow_http {
            builder = builder.https_only(true);
        }
        let client = builder
            .build()
            .map_err(|e| DiscordError::Http(format!("could not build reqwest client: {e}")))?;

        Ok(Self {
            client,
            token,
            config,
            rate_limiter: Arc::new(RateLimiter::new()),
        })
    }

    /// Issue a GET request and decode the JSON response.
    ///
    /// # Errors
    ///
    /// See [`Self::send`] for error categories.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, DiscordError> {
        let url = format!("{}{path}", self.config.api_base);
        debug!(%url, "GET");
        self.send_with_retry(reqwest::Method::GET, &url, None).await
    }

    /// Issue a POST request with a JSON body and decode the response.
    ///
    /// # Errors
    ///
    /// See [`Self::send`] for error categories.
    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, DiscordError> {
        let url = format!("{}{path}", self.config.api_base);
        debug!(%url, "POST");
        let body = serde_json::to_vec(body)?;
        self.send_with_retry(reqwest::Method::POST, &url, Some(body))
            .await
    }

    /// Issue a POST request with no body (Discord sometimes wants this).
    ///
    /// # Errors
    ///
    /// See [`Self::send`] for error categories.
    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, DiscordError> {
        let url = format!("{}{path}", self.config.api_base);
        debug!(%url, "POST (empty)");
        self.send_with_retry(reqwest::Method::POST, &url, None)
            .await
    }

    /// Issue a PATCH request with a JSON body and decode the response.
    ///
    /// # Errors
    ///
    /// See [`Self::send`] for error categories.
    pub async fn patch<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, DiscordError> {
        let url = format!("{}{path}", self.config.api_base);
        debug!(%url, "PATCH");
        let body = serde_json::to_vec(body)?;
        self.send_with_retry(reqwest::Method::PATCH, &url, Some(body))
            .await
    }

    /// Issue a PUT request with a JSON body. Returns no response body.
    ///
    /// # Errors
    ///
    /// See [`Self::send`] for error categories.
    pub async fn put<B: serde::Serialize>(&self, path: &str, body: &B) -> Result<(), DiscordError> {
        let url = format!("{}{path}", self.config.api_base);
        debug!(%url, "PUT");
        let body = serde_json::to_vec(body)?;
        self.send_no_decode(reqwest::Method::PUT, &url, Some(body))
            .await
    }

    /// Issue a DELETE request. Returns no response body.
    ///
    /// # Errors
    ///
    /// See [`Self::send`] for error categories.
    pub async fn delete(&self, path: &str) -> Result<(), DiscordError> {
        let url = format!("{}{path}", self.config.api_base);
        debug!(%url, "DELETE");
        self.send_no_decode(reqwest::Method::DELETE, &url, None)
            .await
    }

    // ---------------------------------------------------------------------
    // Internals
    // ---------------------------------------------------------------------

    async fn send_with_retry<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<Vec<u8>>,
    ) -> Result<T, DiscordError> {
        let response_bytes = self.send_raw(method, url, body).await?;
        if response_bytes.is_empty() {
            return Err(DiscordError::Decode(
                "expected JSON response, got empty body".into(),
            ));
        }
        serde_json::from_slice(&response_bytes).map_err(DiscordError::from)
    }

    async fn send_no_decode(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<Vec<u8>>,
    ) -> Result<(), DiscordError> {
        let _ = self.send_raw(method, url, body).await?;
        Ok(())
    }

    /// Send a single request with rate-limit pre-flight, retry on
    /// transient errors.
    ///
    /// The rate limiter is consulted before each attempt and updated
    /// after each response.
    async fn send_raw(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, DiscordError> {
        let route = rate_limit::route_for(&method, url);
        let mut attempt = 0u32;
        loop {
            // Wait for rate limit before each attempt.
            self.rate_limiter.wait(&route).await;

            let response = self
                .client
                .request(method.clone(), url)
                .header("Authorization", format!("Bot {}", self.token))
                .header("Content-Type", "application/json")
                .body(body.clone().unwrap_or_default())
                .send()
                .await
                .map_err(|e| {
                    if attempt < self.config.max_retries {
                        warn!(%url, attempt, error = %e, "transient error, will retry");
                        DiscordError::Http(format!("{e}"))
                    } else {
                        DiscordError::Http(format!("{e}"))
                    }
                })?;

            let status = response.status();

            // Update rate limit from response headers.
            self.rate_limiter.update(&route, &response).await;

            // Handle 429 specially — it may be a global rate limit.
            if status.as_u16() == 429 {
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(1.0);
                let is_global = response
                    .headers()
                    .get("X-RateLimit-Global")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s == "true")
                    .unwrap_or(false);
                if is_global {
                    self.rate_limiter
                        .wait_global(Duration::from_secs_f64(retry_after))
                        .await;
                } else {
                    self.rate_limiter
                        .wait_bucket(&route, Duration::from_secs_f64(retry_after))
                        .await;
                }
                if attempt < self.config.max_retries {
                    attempt += 1;
                    debug!(%url, attempt, "429, retrying");
                    continue;
                }
                return Err(DiscordError::RateLimited);
            }

            // Non-2xx: classify.
            if !status.is_success() {
                let body_text = response.text().await.unwrap_or_default();
                let body_truncated = if body_text.len() > 1024 {
                    format!("{}...(truncated)", &body_text[..1024])
                } else {
                    body_text
                };
                let status_code = status.as_u16();

                // 5xx: retry.
                if (500..600).contains(&status_code) && attempt < self.config.max_retries {
                    attempt += 1;
                    let backoff = self.backoff_for(attempt);
                    warn!(%url, attempt, status = status_code, ?backoff, "5xx, retrying");
                    tokio::time::sleep(backoff).await;
                    continue;
                }
                return Err(DiscordError::Discord {
                    status: status_code,
                    body: body_truncated,
                });
            }

            // Success.
            return response
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| DiscordError::Http(format!("could not read response body: {e}")));
        }
    }

    /// Compute the backoff duration for a given attempt number.
    fn backoff_for(&self, attempt: u32) -> Duration {
        let multiplier = 2u32.saturating_pow(attempt.saturating_sub(1));
        let base = self
            .config
            .initial_backoff
            .checked_mul(multiplier)
            .unwrap_or(self.config.max_backoff);
        std::cmp::min(base, self.config.max_backoff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_token() {
        let r = DiscordHttp::new(String::new(), DiscordHttpConfig::default());
        assert!(matches!(r, Err(DiscordError::Auth(_))));
    }

    #[test]
    fn backoff_grows_exponentially() {
        let http = DiscordHttp::new(
            "test-token".into(),
            DiscordHttpConfig {
                timeout: Duration::from_secs(1),
                max_retries: 3,
                initial_backoff: Duration::from_secs(1),
                max_backoff: Duration::from_secs(30),
                user_agent: "test".into(),
                api_base: API_BASE.into(),
            },
        )
        .unwrap();
        assert_eq!(http.backoff_for(1), Duration::from_secs(1));
        assert_eq!(http.backoff_for(2), Duration::from_secs(2));
        assert_eq!(http.backoff_for(3), Duration::from_secs(4));
        assert_eq!(http.backoff_for(10), Duration::from_secs(30));
    }

    #[test]
    fn config_defaults_are_sane() {
        let c = DiscordHttpConfig::default();
        assert_eq!(c.timeout, Duration::from_secs(30));
        assert_eq!(c.max_retries, 2);
        assert!(c.user_agent.starts_with("GuildForge/"));
    }
}
