//! Discord implementation of the [`Provider`](guildforge_provider::Provider)
//! trait.
//!
//! This is the **only** crate that knows about Discord. The engine,
//! planner, executor, and state store never import from here; they
//! import from [`guildforge_provider`]. Wiring happens in `apps/cli`.
//!
//! # Modules (planned)
//!
//! - `client/` — low-level HTTP wrapper, rate-limit middleware, retry
//! - `resources/` — per-resource-type CRUD: `role.rs`, `channel.rs`,
//!   `forum.rs`, `webhook.rs`, etc.
//! - `error.rs` — `DiscordError` enum
//!
//! See [`ADR-0006`](../../docs/adr/ADR-0006-async-http.md) for HTTP
//! stack details and [`ADR-0001`](../../docs/adr/ADR-0001-provider-trait.md)
//! for the trait contract.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 2
//! (tasks `P2-001` through `P2-016`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use async_trait::async_trait;
use guildforge_provider::{Provider, ProviderError, Resource, ResourceAddr, ResourceKind};
use thiserror::Error;

/// Discord-specific error type.
///
/// Maps to [`ProviderError`] at the engine boundary. See
/// [`ADR-0005`](../../docs/adr/ADR-0005-error-model.md).
#[derive(Debug, Error)]
pub enum DiscordError {
    /// HTTP failure (network, timeout, 5xx after retries).
    #[error("http: {0}")]
    Http(String),

    /// Discord returned a 4xx other than 429.
    #[error("discord: {status} {body}")]
    Discord {
        /// HTTP status code.
        status: u16,
        /// Response body (truncated to 1 KiB, never includes the token).
        body: String,
    },

    /// Rate limited (should not surface; handled by middleware).
    #[error("rate limited")]
    RateLimited,

    /// Bot token is missing or invalid.
    #[error("auth: {0}")]
    Auth(String),

    /// Response could not be parsed.
    #[error("decode: {0}")]
    Decode(String),
}

impl From<DiscordError> for ProviderError {
    fn from(e: DiscordError) -> Self {
        match e {
            DiscordError::Http(msg) => Self::Transient(msg),
            DiscordError::Discord { status, body } => match status {
                401 | 403 => Self::Auth(format!("{status} {body}")),
                409 => Self::Conflict(format!("{status} {body}")),
                // 400, 404, 422, and any other 4xx all map to Permanent.
                _ => Self::Permanent(format!("{status} {body}")),
            },
            DiscordError::RateLimited => Self::Transient("rate limited".to_string()),
            DiscordError::Auth(msg) => Self::Auth(msg),
            DiscordError::Decode(msg) => Self::Permanent(msg),
        }
    }
}

/// Discord provider. Phase 0 stub.
///
/// Real implementation (Phase 2) will hold:
/// - `reqwest::Client` with rate-limit middleware
/// - bot token (in memory only, never logged)
/// - guild ID
pub struct DiscordProvider {
    /// Placeholder for the HTTP client. Real type lands in Phase 2.
    _http: (),
    /// Placeholder for the bot token. Real type lands in Phase 2.
    _token: (),
}

impl DiscordProvider {
    /// Construct a new `DiscordProvider`. Phase 0 stub.
    ///
    /// Real implementation (Phase 2) will accept an HTTP client and
    /// a token.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _http: (),
            _token: (),
        }
    }

    /// Construct a `DiscordProvider` from environment variables.
    ///
    /// Reads `GUILDFORGE_BOT_TOKEN` or `GUILDFORGE_TOKEN_FILE`.
    ///
    /// # Errors
    ///
    /// Returns [`DiscordError::Auth`] if no token is available.
    pub fn from_env() -> Result<Self, DiscordError> {
        // Phase 0 stub.
        Ok(Self::new())
    }
}

impl Default for DiscordProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for DiscordProvider {
    type Error = DiscordError;

    async fn read(&self, _addr: &ResourceAddr) -> Result<Option<Resource>, Self::Error> {
        // Phase 0 stub. Real implementation lands in task P2-004..P2-012.
        Ok(None)
    }

    async fn create(&self, _desired: &Resource) -> Result<Resource, Self::Error> {
        // Phase 0 stub.
        Err(DiscordError::Auth("not implemented".to_string()))
    }

    async fn update(
        &self,
        _current: &Resource,
        _desired: &Resource,
    ) -> Result<Resource, Self::Error> {
        // Phase 0 stub.
        Err(DiscordError::Auth("not implemented".to_string()))
    }

    async fn delete(&self, _current: &Resource) -> Result<(), Self::Error> {
        // Phase 0 stub.
        Err(DiscordError::Auth("not implemented".to_string()))
    }

    async fn list(&self, _kind: ResourceKind) -> Result<Vec<Resource>, Self::Error> {
        // Phase 0 stub.
        Ok(vec![])
    }

    fn name(&self) -> &'static str {
        "discord"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_provider_name() {
        let p = DiscordProvider::new();
        assert_eq!(p.name(), "discord");
    }

    #[tokio::test]
    async fn stub_read_returns_none() {
        let p = DiscordProvider::new();
        let addr = ResourceAddr::new("role/Admin");
        assert!(p.read(&addr).await.unwrap().is_none());
    }

    #[test]
    fn error_mapping() {
        let e = DiscordError::Auth("bad token".to_string());
        let p: ProviderError = e.into();
        assert!(matches!(p, ProviderError::Auth(_)));
    }
}
