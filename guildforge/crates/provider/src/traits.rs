//! The `Provider` trait.

use crate::error::ProviderError;
use crate::resource::{Resource, ResourceAddr, ResourceKind};
use async_trait::async_trait;

/// The provider trait.
///
/// Every external system (Discord, Slack, etc.) is reached through this
/// trait. The engine, planner, executor, and state store never import
/// from `guildforge-provider-discord`. Discord is one implementation.
///
/// See [`ADR-0001`](../../docs/adr/ADR-0001-provider-trait.md) for the
/// full spec, alternatives, and consequences.
///
/// # Idempotency
///
/// Every method must be idempotent:
///
/// - `create` returns the existing resource if one with the same address
///   already exists.
/// - `update` is a no-op if `current == desired`.
/// - `delete` is a no-op if the resource does not exist.
/// - `reorder` is a no-op if the resource is already at `new_position`.
///
/// See [`ADR-0007`](../../docs/adr/ADR-0007-idempotency-ordering.md).
#[async_trait]
pub trait Provider: Send + Sync {
    /// Per-provider error type. The engine erases this at its boundary
    /// via `anyhow::Error::from`.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Read a single resource by address.
    ///
    /// Returns `Ok(None)` if the resource does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the read fails for any reason other
    /// than the resource not existing.
    async fn read(&self, addr: &ResourceAddr) -> Result<Option<Resource>, Self::Error>;

    /// Create a new resource. The returned `Resource` includes
    /// provider-assigned fields (ID, etc.) that were not in `desired`.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the create fails.
    async fn create(&self, desired: &Resource) -> Result<Resource, Self::Error>;

    /// Update an existing resource from `current` to `desired`.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the update fails.
    async fn update(&self, current: &Resource, desired: &Resource)
        -> Result<Resource, Self::Error>;

    /// Delete a resource.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the delete fails.
    async fn delete(&self, current: &Resource) -> Result<(), Self::Error>;

    /// Reorder a resource within its parent (channel within category,
    /// role within guild, etc.).
    ///
    /// Default impl is a no-op for resources that don't support
    /// ordering.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the reorder fails.
    async fn reorder(&self, _addr: &ResourceAddr, _new_position: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Return the list of all resources of `kind` currently present in
    /// the provider.
    ///
    /// Used by `guildforge doctor` (drift detection) and
    /// `guildforge import`.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the list fails.
    async fn list(&self, kind: ResourceKind) -> Result<Vec<Resource>, Self::Error>;

    /// Human-readable provider name (e.g. `"discord"`, `"slack"`).
    fn name(&self) -> &'static str;
}

/// Blanket conversion from any provider error to a [`ProviderError`].
///
/// Provider implementations can `impl From<MyError> for ProviderError`
/// and call `.into()` on their errors when surfacing them through the
/// engine.
pub fn classify_error(e: &dyn std::error::Error) -> ProviderError {
    let msg = e.to_string();
    let lower = msg.to_ascii_lowercase();
    if lower.contains("rate limit") || lower.contains("429") {
        ProviderError::Transient(msg)
    } else if lower.contains("auth") || lower.contains("401") || lower.contains("403") {
        ProviderError::Auth(msg)
    } else if lower.contains("conflict") || lower.contains("409") {
        ProviderError::Conflict(msg)
    } else if lower.contains("timeout")
        || lower.contains("connection")
        || lower.contains("502")
        || lower.contains("503")
        || lower.contains("504")
    {
        ProviderError::Transient(msg)
    } else {
        ProviderError::Permanent(msg)
    }
}
