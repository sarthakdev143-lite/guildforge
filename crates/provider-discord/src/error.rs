//! Discord-specific error type.

use thiserror::Error;

/// Discord provider error.
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
        /// Response body (truncated to 1 KiB; never includes the token).
        body: String,
    },

    /// Rate limited. Should not surface from the HTTP layer (handled by
    /// middleware); exposed for tests.
    #[error("rate limited")]
    RateLimited,

    /// Bot token is missing or invalid.
    #[error("auth: {0}")]
    Auth(String),

    /// Configuration error (missing env var, invalid guild ID, etc.).
    #[error("config: {0}")]
    Config(String),

    /// Response could not be parsed.
    #[error("decode: {0}")]
    Decode(String),

    /// The provider was asked to operate on a resource kind it does not
    /// support (e.g. AutoMod rules, emojis in v1).
    #[error("unsupported: {0}")]
    Unsupported(String),
}

impl From<reqwest::Error> for DiscordError {
    fn from(e: reqwest::Error) -> Self {
        Self::Http(format!("{e}"))
    }
}

impl From<serde_json::Error> for DiscordError {
    fn from(e: serde_json::Error) -> Self {
        Self::Decode(format!("{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_does_not_panic() {
        for e in [
            DiscordError::Http("net".into()),
            DiscordError::Discord {
                status: 404,
                body: "nope".into(),
            },
            DiscordError::RateLimited,
            DiscordError::Auth("bad".into()),
            DiscordError::Config("missing".into()),
            DiscordError::Decode("bad json".into()),
            DiscordError::Unsupported("emoji".into()),
        ] {
            let _ = format!("{e}");
        }
    }
}
