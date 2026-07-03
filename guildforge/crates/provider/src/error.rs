//! Provider error type.

use thiserror::Error;

/// Errors that a [`Provider`](crate::Provider) can return.
///
/// The associated `Error` type on `Provider` allows each provider to
/// expose its own typed errors. The engine erases the type at its
/// boundary via `anyhow::Error::from`.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Transient error; retry may succeed (network blip, 5xx, 429 after
    /// middleware).
    #[error("transient: {0}")]
    Transient(String),

    /// Permanent error; do not retry (4xx other than 429).
    #[error("permanent: {0}")]
    Permanent(String),

    /// Race condition; retry once after a short delay (409 Conflict).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Authentication failed; abort entire apply (401, 403).
    #[error("auth: {0}")]
    Auth(String),

    /// The provider was asked to operate on a resource kind it does not
    /// support.
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// The provider returned a response that could not be parsed.
    #[error("decode: {0}")]
    Decode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_does_not_panic() {
        for e in [
            ProviderError::Transient("net".into()),
            ProviderError::Permanent("404".into()),
            ProviderError::Conflict("race".into()),
            ProviderError::Auth("bad token".into()),
            ProviderError::Unsupported("emoji".into()),
            ProviderError::Decode("bad json".into()),
        ] {
            let _ = format!("{e}");
        }
    }
}
