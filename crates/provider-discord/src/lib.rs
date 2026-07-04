//! Discord implementation of the [`Provider`](guildforge_provider::Provider)
//! trait.
//!
//! This is the **only** crate that knows about Discord. The engine,
//! planner, executor, and state store never import from here; they
//! import from [`guildforge_provider`]. Wiring happens in `apps/cli`.
//!
//! # Modules
//!
//! - `client/` — HTTP wrapper, rate-limit middleware, retry.
//! - `resources/` — Per-resource-type CRUD: `role.rs`, `channel.rs`,
//!   `webhook.rs`, etc.
//! - `error.rs` — `DiscordError` enum.
//!
//! See [`ADR-0006`](../../docs/adr/ADR-0006-async-http.md) for the HTTP
//! stack and [`ADR-0001`](../../docs/adr/ADR-0001-provider-trait.md)
//! for the trait contract.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![allow(
    clippy::uninlined_format_args,
    clippy::unused_async,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::option_map_unit_fn,
    clippy::inconsistent_digit_grouping,
    clippy::unnecessary_wraps,
    clippy::missing_errors_doc,
    clippy::match_same_arms,
    clippy::doc_markdown,
    clippy::map_unwrap_or,
    clippy::unreadable_literal
)]

pub mod client;
pub mod error;
pub mod resources;

use async_trait::async_trait;
pub use client::{DiscordHttp, DiscordHttpConfig};
pub use error::DiscordError;
use guildforge_provider::{Provider, ProviderError, Resource, ResourceAddr, ResourceKind};
use guildforge_shared::Snowflake;
use std::sync::Arc;

/// Discord provider.
///
/// Holds an HTTP client (with rate-limit middleware) and the bot token.
/// The token is in memory only; never logged.
pub struct DiscordProvider {
    /// The guild ID this provider operates on.
    pub guild_id: Snowflake,
    /// HTTP client wrapper.
    pub http: Arc<DiscordHttp>,
}

impl DiscordProvider {
    /// Construct a new `DiscordProvider`.
    #[must_use]
    pub fn new(guild_id: Snowflake, http: Arc<DiscordHttp>) -> Self {
        Self { guild_id, http }
    }

    /// Construct from environment variables.
    ///
    /// Reads `GUILDFORGE_BOT_TOKEN` (or `GUILDFORGE_TOKEN_FILE` for a
    /// file path) and `GUILDFORGE_GUILD_ID`.
    ///
    /// # Errors
    ///
    /// Returns [`DiscordError::Auth`] if no token is available, or
    /// [`DiscordError::Config`] if the guild ID is missing or invalid.
    pub fn from_env() -> Result<Self, DiscordError> {
        let token = read_token_from_env()?;
        let guild_id = read_guild_id_from_env()?;
        let http = DiscordHttp::new(token, DiscordHttpConfig::default())?;
        Ok(Self::new(guild_id, Arc::new(http)))
    }

    /// Construct from explicit token + guild ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be constructed.
    pub fn with_token(token: String, guild_id: Snowflake) -> Result<Self, DiscordError> {
        let http = DiscordHttp::new(token, DiscordHttpConfig::default())?;
        Ok(Self::new(guild_id, Arc::new(http)))
    }
}

fn read_token_from_env() -> Result<String, DiscordError> {
    if let Ok(t) = std::env::var("GUILDFORGE_BOT_TOKEN") {
        if !t.is_empty() {
            return Ok(t);
        }
    }
    if let Ok(p) = std::env::var("GUILDFORGE_TOKEN_FILE") {
        let token = std::fs::read_to_string(&p)
            .map_err(|e| DiscordError::Auth(format!("could not read token file {p}: {e}")))?
            .trim()
            .to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }
    Err(DiscordError::Auth(
        "no bot token: set GUILDFORGE_BOT_TOKEN or GUILDFORGE_TOKEN_FILE".into(),
    ))
}

fn read_guild_id_from_env() -> Result<Snowflake, DiscordError> {
    let s = std::env::var("GUILDFORGE_GUILD_ID")
        .map_err(|_| DiscordError::Config("GUILDFORGE_GUILD_ID not set".into()))?;
    let v: u64 = s.parse().map_err(|_| {
        DiscordError::Config(format!("GUILDFORGE_GUILD_ID `{s}` is not a valid u64"))
    })?;
    Ok(Snowflake::new(v))
}

impl From<DiscordError> for ProviderError {
    fn from(e: DiscordError) -> Self {
        match e {
            DiscordError::Http(msg) => Self::Transient(msg),
            DiscordError::Discord { status, body } => match status {
                401 | 403 => Self::Auth(format!("{status} {body}")),
                409 => Self::Conflict(format!("{status} {body}")),
                // 400, 404, 422, 5xx, and any other 4xx all map to Permanent.
                _ => Self::Permanent(format!("{status} {body}")),
            },
            DiscordError::RateLimited => Self::Transient("rate limited".into()),
            DiscordError::Auth(msg) => Self::Auth(msg),
            DiscordError::Config(msg) => Self::Permanent(msg),
            DiscordError::Decode(msg) => Self::Permanent(msg),
            DiscordError::Unsupported(msg) => Self::Permanent(msg),
        }
    }
}

#[async_trait]
impl Provider for DiscordProvider {
    type Error = DiscordError;

    async fn read(&self, addr: &ResourceAddr) -> Result<Option<Resource>, Self::Error> {
        resources::read(self, addr).await
    }

    async fn create(&self, desired: &Resource) -> Result<Resource, Self::Error> {
        resources::create(self, desired).await
    }

    async fn update(
        &self,
        current: &Resource,
        desired: &Resource,
    ) -> Result<Resource, Self::Error> {
        resources::update(self, current, desired).await
    }

    async fn delete(&self, current: &Resource) -> Result<(), Self::Error> {
        resources::delete(self, current).await
    }

    async fn reorder(&self, addr: &ResourceAddr, new_position: u32) -> Result<(), Self::Error> {
        resources::reorder(self, addr, new_position).await
    }

    async fn list(&self, kind: ResourceKind) -> Result<Vec<Resource>, Self::Error> {
        resources::list(self, kind).await
    }

    fn name(&self) -> &'static str {
        "discord"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_mapping_auth() {
        let e = DiscordError::Auth("bad token".into());
        let p: ProviderError = e.into();
        assert!(matches!(p, ProviderError::Auth(_)));
    }

    #[test]
    fn error_mapping_404_permanent() {
        let e = DiscordError::Discord {
            status: 404,
            body: "Not Found".into(),
        };
        let p: ProviderError = e.into();
        assert!(matches!(p, ProviderError::Permanent(_)));
    }

    #[test]
    fn error_mapping_429_transient() {
        let e = DiscordError::RateLimited;
        let p: ProviderError = e.into();
        assert!(matches!(p, ProviderError::Transient(_)));
    }

    #[test]
    fn from_env_without_token_returns_err() {
        // Clear any env vars that might leak from the test runner.
        std::env::remove_var("GUILDFORGE_BOT_TOKEN");
        std::env::remove_var("GUILDFORGE_TOKEN_FILE");
        let r = DiscordProvider::from_env();
        assert!(matches!(r, Err(DiscordError::Auth(_))));
    }
}
