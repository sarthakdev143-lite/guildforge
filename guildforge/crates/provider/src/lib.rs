//! The `Provider` trait and shared resource types.
//!
//! This is the **single most important crate** for extensibility. The
//! engine, planner, executor, and state store never import from
//! `guildforge-provider-discord`; they import from here. Discord is one
//! implementation.
//!
//! See [`ADR-0001`](../../docs/adr/ADR-0001-provider-trait.md) for the
//! full rationale, alternatives, and consequences.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::uninlined_format_args)]

pub mod error;
pub mod resource;
pub mod traits;

pub use error::ProviderError;
pub use resource::{
    CategoryResource, ChannelResource, ChannelType, ForumTagResource, InviteResource,
    OverwriteKind, PermissionOverwriteResource, Resource, ResourceAddr, ResourceKind, RoleResource,
    ServerGuideResource, WebhookResource, WelcomeScreenChannel, WelcomeScreenResource,
};
pub use traits::Provider;

// Re-export shared primitives for convenience.
pub use guildforge_shared::{Hash, ResourceId, Snowflake};
