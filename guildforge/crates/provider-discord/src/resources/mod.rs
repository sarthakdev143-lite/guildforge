//! Per-resource-type CRUD operations.
//!
//! Each resource type lives in its own module. This module exposes the
//! top-level dispatch functions that the [`DiscordProvider`](crate::DiscordProvider)
//! implementation calls into.

pub mod channel;
pub mod forum_tag;
pub mod invite;
pub mod overwrite;
pub mod role;
pub mod webhook;
pub mod welcome;

use crate::error::DiscordError;
use crate::DiscordProvider;
use guildforge_provider::{Resource, ResourceAddr, ResourceKind};
use guildforge_shared::ResourceId;

/// Dispatch a read by address.
pub async fn read(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<Resource>, DiscordError> {
    let kind = parse_addr_kind(addr)?;
    match kind {
        ResourceKind::Role => role::read(provider, addr)
            .await
            .map(|o| o.map(Resource::Role)),
        ResourceKind::Category => channel::read_category(provider, addr)
            .await
            .map(|o| o.map(Resource::Category)),
        ResourceKind::Channel => channel::read(provider, addr)
            .await
            .map(|o| o.map(Resource::Channel)),
        ResourceKind::PermissionOverwrite => overwrite::read(provider, addr)
            .await
            .map(|o| o.map(Resource::PermissionOverwrite)),
        ResourceKind::Webhook => webhook::read(provider, addr)
            .await
            .map(|o| o.map(Resource::Webhook)),
        ResourceKind::Invite => invite::read(provider, addr)
            .await
            .map(|o| o.map(Resource::Invite)),
        ResourceKind::ForumTag => forum_tag::read(provider, addr)
            .await
            .map(|o| o.map(Resource::ForumTag)),
        ResourceKind::WelcomeScreen => welcome::read(provider, addr)
            .await
            .map(|o| o.map(Resource::WelcomeScreen)),
        ResourceKind::ServerGuide => welcome::read_server_guide(provider, addr)
            .await
            .map(|o| o.map(Resource::ServerGuide)),
    }
}

/// Dispatch a create by resource kind.
pub async fn create(
    provider: &DiscordProvider,
    desired: &Resource,
) -> Result<Resource, DiscordError> {
    match desired {
        Resource::Role(r) => role::create(provider, r).await.map(Resource::Role),
        Resource::Category(r) => channel::create_category(provider, r)
            .await
            .map(Resource::Category),
        Resource::Channel(r) => channel::create(provider, r).await.map(Resource::Channel),
        Resource::PermissionOverwrite(r) => overwrite::create(provider, r)
            .await
            .map(Resource::PermissionOverwrite),
        Resource::Webhook(r) => webhook::create(provider, r).await.map(Resource::Webhook),
        Resource::Invite(r) => invite::create(provider, r).await.map(Resource::Invite),
        Resource::ForumTag(r) => forum_tag::create(provider, r).await.map(Resource::ForumTag),
        Resource::WelcomeScreen(r) => welcome::create(provider, r)
            .await
            .map(Resource::WelcomeScreen),
        Resource::ServerGuide(r) => welcome::create_server_guide(provider, r)
            .await
            .map(Resource::ServerGuide),
    }
}

/// Dispatch an update by resource kind.
pub async fn update(
    provider: &DiscordProvider,
    current: &Resource,
    desired: &Resource,
) -> Result<Resource, DiscordError> {
    if current.addr() != desired.addr() {
        return Err(DiscordError::Unsupported(format!(
            "update: address mismatch (current={}, desired={})",
            current.addr(),
            desired.addr()
        )));
    }
    match (current, desired) {
        (Resource::Role(c), Resource::Role(d)) => {
            role::update(provider, c, d).await.map(Resource::Role)
        }
        (Resource::Category(c), Resource::Category(d)) => channel::update_category(provider, c, d)
            .await
            .map(Resource::Category),
        (Resource::Channel(c), Resource::Channel(d)) => {
            channel::update(provider, c, d).await.map(Resource::Channel)
        }
        (Resource::PermissionOverwrite(c), Resource::PermissionOverwrite(d)) => {
            overwrite::update(provider, c, d)
                .await
                .map(Resource::PermissionOverwrite)
        }
        (Resource::Webhook(c), Resource::Webhook(d)) => {
            webhook::update(provider, c, d).await.map(Resource::Webhook)
        }
        (Resource::Invite(c), Resource::Invite(d)) => {
            invite::update(provider, c, d).await.map(Resource::Invite)
        }
        (Resource::ForumTag(c), Resource::ForumTag(d)) => forum_tag::update(provider, c, d)
            .await
            .map(Resource::ForumTag),
        (Resource::WelcomeScreen(_), Resource::WelcomeScreen(d)) => welcome::update(provider, d)
            .await
            .map(Resource::WelcomeScreen),
        (Resource::ServerGuide(_), Resource::ServerGuide(d)) => {
            welcome::update_server_guide(provider, d)
                .await
                .map(Resource::ServerGuide)
        }
        _ => Err(DiscordError::Unsupported(format!(
            "update: cannot update {:?} to {:?}",
            current.kind(),
            desired.kind()
        ))),
    }
}

/// Dispatch a delete by resource kind.
pub async fn delete(provider: &DiscordProvider, current: &Resource) -> Result<(), DiscordError> {
    match current {
        Resource::Role(r) => role::delete(provider, r).await,
        Resource::Category(r) => channel::delete_channel(provider, r.id).await,
        Resource::Channel(r) => channel::delete_channel(provider, r.id).await,
        Resource::PermissionOverwrite(r) => overwrite::delete(provider, r).await,
        Resource::Webhook(r) => webhook::delete(provider, r).await,
        Resource::Invite(r) => invite::delete(provider, r).await,
        Resource::ForumTag(r) => forum_tag::delete(provider, r).await,
        Resource::WelcomeScreen(_) => welcome::delete(provider).await,
        Resource::ServerGuide(_) => welcome::delete_server_guide(provider).await,
    }
}

/// Dispatch a reorder by address.
pub async fn reorder(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
    new_position: u32,
) -> Result<(), DiscordError> {
    let kind = parse_addr_kind(addr)?;
    match kind {
        ResourceKind::Role => role::reorder(provider, addr, new_position).await,
        ResourceKind::Category | ResourceKind::Channel => {
            channel::reorder(provider, addr, new_position).await
        }
        _ => Err(DiscordError::Unsupported(format!(
            "reorder: not supported for {kind:?}"
        ))),
    }
}

/// Dispatch a list by resource kind.
pub async fn list(
    provider: &DiscordProvider,
    kind: ResourceKind,
) -> Result<Vec<Resource>, DiscordError> {
    match kind {
        ResourceKind::Role => {
            let roles = role::list(provider).await?;
            Ok(roles.into_iter().map(Resource::Role).collect())
        }
        ResourceKind::Category => {
            let cats = channel::list_categories(provider).await?;
            Ok(cats.into_iter().map(Resource::Category).collect())
        }
        ResourceKind::Channel => {
            let chans = channel::list(provider).await?;
            Ok(chans.into_iter().map(Resource::Channel).collect())
        }
        ResourceKind::PermissionOverwrite => Ok(vec![]), // listed via parent channel
        ResourceKind::Webhook => {
            let ws = webhook::list(provider).await?;
            Ok(ws.into_iter().map(Resource::Webhook).collect())
        }
        ResourceKind::Invite => {
            let is = invite::list(provider).await?;
            Ok(is.into_iter().map(Resource::Invite).collect())
        }
        ResourceKind::ForumTag => Ok(vec![]), // listed via parent channel
        ResourceKind::WelcomeScreen => {
            let r = welcome::read(provider, &ResourceId::new("welcome_screen")).await?;
            Ok(r.into_iter().map(Resource::WelcomeScreen).collect())
        }
        ResourceKind::ServerGuide => {
            let r = welcome::read_server_guide(provider, &ResourceId::new("server_guide")).await?;
            Ok(r.into_iter().map(Resource::ServerGuide).collect())
        }
    }
}

/// Parse the resource kind from the first segment of an address.
fn parse_addr_kind(addr: &ResourceAddr) -> Result<ResourceKind, DiscordError> {
    let s = addr.as_str();
    let head = s.split('/').next().unwrap_or("");
    match head {
        "role" => Ok(ResourceKind::Role),
        "category" => Ok(ResourceKind::Category),
        "channel" => Ok(ResourceKind::Channel),
        "overwrite" => Ok(ResourceKind::PermissionOverwrite),
        "webhook" => Ok(ResourceKind::Webhook),
        "invite" => Ok(ResourceKind::Invite),
        "tag" => Ok(ResourceKind::ForumTag),
        "welcome_screen" => Ok(ResourceKind::WelcomeScreen),
        "server_guide" => Ok(ResourceKind::ServerGuide),
        _ => Err(DiscordError::Unsupported(format!(
            "unknown address kind: {s}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_addr_kind_works() {
        assert_eq!(
            parse_addr_kind(&ResourceId::new("role/Admin")).unwrap(),
            ResourceKind::Role
        );
        assert_eq!(
            parse_addr_kind(&ResourceId::new("channel/general")).unwrap(),
            ResourceKind::Channel
        );
        assert!(parse_addr_kind(&ResourceId::new("bogus/x")).is_err());
    }
}
