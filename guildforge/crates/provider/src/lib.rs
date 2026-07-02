//! The `Provider` trait and shared resource types.
//!
//! This is the **single most important crate** for extensibility. The
//! engine, planner, executor, and state store never import from
//! `guildforge-provider-discord`; they import from here. Discord is one
//! implementation.
//!
//! See [`ADR-0001`](../../docs/adr/ADR-0001-provider-trait.md) for the
//! full rationale, alternatives, and consequences.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 2
//! (task `P2-001`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use async_trait::async_trait;
use guildforge_shared::ResourceId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The kind of a resource.
///
/// Adding a new variant requires updating:
/// 1. This enum.
/// 2. The serde tag in [`Resource`].
/// 3. The dependency graph in `crates/planner/src/dependencies.rs`.
/// 4. The provider implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// A role.
    Role,
    /// A category (Discord `guild_category` channel).
    Category,
    /// Any kind of channel (text, voice, forum, etc.).
    Channel,
    /// A permission overwrite on a channel or category.
    PermissionOverwrite,
    /// A webhook.
    Webhook,
    /// An invite.
    Invite,
    /// A forum tag.
    ForumTag,
    /// The guild welcome screen.
    WelcomeScreen,
    /// The server guide / onboarding.
    ServerGuide,
}

/// A typed resource.
///
/// Variants map 1:1 to [`ResourceKind`]. The serde tag is `kind` so
/// JSON/YAML serialization is self-describing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resource {
    /// A role resource. The inner type lands in Phase 2.
    Role(RoleResource),
    /// A category resource. The inner type lands in Phase 2.
    Category(CategoryResource),
    /// A channel resource. The inner type lands in Phase 2.
    Channel(ChannelResource),
    /// A permission overwrite. The inner type lands in Phase 2.
    PermissionOverwrite(PermissionOverwriteResource),
    /// A webhook. The inner type lands in Phase 2.
    Webhook(WebhookResource),
    /// An invite. The inner type lands in Phase 2.
    Invite(InviteResource),
    /// A forum tag. The inner type lands in Phase 2.
    ForumTag(ForumTagResource),
    /// The welcome screen. The inner type lands in Phase 2.
    WelcomeScreen(WelcomeScreenResource),
    /// The server guide. The inner type lands in Phase 2.
    ServerGuide(ServerGuideResource),
}

impl Resource {
    /// Get the kind of this resource.
    #[must_use]
    pub fn kind(&self) -> ResourceKind {
        match self {
            Self::Role(_) => ResourceKind::Role,
            Self::Category(_) => ResourceKind::Category,
            Self::Channel(_) => ResourceKind::Channel,
            Self::PermissionOverwrite(_) => ResourceKind::PermissionOverwrite,
            Self::Webhook(_) => ResourceKind::Webhook,
            Self::Invite(_) => ResourceKind::Invite,
            Self::ForumTag(_) => ResourceKind::ForumTag,
            Self::WelcomeScreen(_) => ResourceKind::WelcomeScreen,
            Self::ServerGuide(_) => ResourceKind::ServerGuide,
        }
    }

    /// Get the address of this resource.
    #[must_use]
    pub fn addr(&self) -> &ResourceId {
        match self {
            Self::Role(r) => &r.addr,
            Self::Category(r) => &r.addr,
            Self::Channel(r) => &r.addr,
            Self::PermissionOverwrite(r) => &r.addr,
            Self::Webhook(r) => &r.addr,
            Self::Invite(r) => &r.addr,
            Self::ForumTag(r) => &r.addr,
            Self::WelcomeScreen(r) => &r.addr,
            Self::ServerGuide(r) => &r.addr,
        }
    }
}

/// Trait-object-compatible address for a resource.
///
/// Addresses are stable strings like `role/Admin` or
/// `channel/COMPANY/announcements`. See
/// [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md) for the
/// canonical address format.
pub type ResourceAddr = ResourceId;

/// A role resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Role name.
    pub name: String,
}

/// A category resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Category name.
    pub name: String,
}

/// A channel resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Channel name.
    pub name: String,
}

/// A permission overwrite resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOverwriteResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
}

/// A webhook resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Webhook name.
    pub name: String,
}

/// An invite resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
}

/// A forum tag resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForumTagResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Tag name.
    pub name: String,
}

/// The welcome screen resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WelcomeScreenResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
}

/// The server guide resource. Full field set lands in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerGuideResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
}

/// Errors that a [`Provider`] can return.
///
/// The associated `Error` type on `Provider` allows each provider to
/// expose its own typed errors. The engine erases the type at its
/// boundary via `anyhow::Error::from`.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Transient error; retry may succeed.
    #[error("transient: {0}")]
    Transient(String),

    /// Permanent error; do not retry.
    #[error("permanent: {0}")]
    Permanent(String),

    /// Race condition; retry once after 500ms.
    #[error("conflict: {0}")]
    Conflict(String),

    /// Authentication failed; abort entire apply.
    #[error("auth: {0}")]
    Auth(String),
}

/// The provider trait.
///
/// Every external system (Discord, Slack, etc.) is reached through this
/// trait. The engine, planner, executor, and state store never import
/// from `guildforge-provider-discord`. Discord is one implementation.
///
/// See [`ADR-0001`](../../docs/adr/ADR-0001-provider-trait.md) for the
/// full spec, alternatives, and consequences.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Per-provider error type.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Read a single resource by address. Returns `Ok(None)` if not present.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the read fails for any reason other
    /// than the resource not existing.
    async fn read(&self, addr: &ResourceAddr) -> Result<Option<Resource>, Self::Error>;

    /// Create a new resource. The returned `Resource` includes
    /// server-assigned fields (ID, etc.) that were not in `desired`.
    ///
    /// Must be idempotent: if a resource with the same `addr` already
    /// exists, return the existing one (don't fail).
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the create fails.
    async fn create(&self, desired: &Resource) -> Result<Resource, Self::Error>;

    /// Update an existing resource from `current` to `desired`.
    ///
    /// Must be idempotent: if `current == desired`, return `current`
    /// unchanged without making an API call.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the update fails.
    async fn update(&self, current: &Resource, desired: &Resource)
        -> Result<Resource, Self::Error>;

    /// Delete a resource. Must be idempotent: deleting a non-existent
    /// resource returns `Ok(())`.
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
    /// `guildforge import` (read existing guild into YAML).
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] if the list fails.
    async fn list(&self, kind: ResourceKind) -> Result<Vec<Resource>, Self::Error>;

    /// Human-readable provider name (e.g. `"discord"`, `"slack"`).
    fn name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_kind_round_trips() {
        let yaml = "role";
        let k: ResourceKind = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(k, ResourceKind::Role);
    }

    #[test]
    fn resource_kind_tag() {
        let r = Resource::Role(RoleResource {
            addr: ResourceAddr::new("role/Admin"),
            name: "Admin".to_string(),
        });
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"kind\":\"role\""));
    }

    #[test]
    fn resource_addr_and_kind() {
        let r = Resource::Category(CategoryResource {
            addr: ResourceAddr::new("category/COMPANY"),
            name: "COMPANY".to_string(),
        });
        assert_eq!(r.kind(), ResourceKind::Category);
        assert_eq!(r.addr().as_str(), "category/COMPANY");
    }
}
