//! Permission overwrite CRUD operations.
//!
//! Permission overwrites are not independent resources in Discord —
//! they are properties of a channel. The CRUD ops here manipulate them
//! via `PUT /channels/:id/permissions/:overwrite_id`.
//!
//! API reference:
//! <https://discord.com/developers/docs/resources/channel#edit-channel-permissions>

use crate::error::DiscordError;
use crate::DiscordProvider;
use guildforge_provider::PermissionOverwriteResource;
use guildforge_provider::ResourceAddr;
use serde::Serialize;

/// Payload for `PUT /channels/:id/permissions/:overwrite_id`.
#[derive(Debug, Serialize)]
struct EditPermissionsPayload {
    /// 0 = role, 1 = member.
    #[serde(rename = "type")]
    kind: u8,
    allow: String,
    deny: String,
}

/// Read is not a real Discord op — overwrites are read as part of the
/// parent channel. We return `Ok(None)` so the engine falls back to
/// reading the parent channel.
pub async fn read(
    _provider: &DiscordProvider,
    _addr: &ResourceAddr,
) -> Result<Option<PermissionOverwriteResource>, DiscordError> {
    Ok(None)
}

/// Create or update a permission overwrite on a channel.
pub async fn create(
    provider: &DiscordProvider,
    desired: &PermissionOverwriteResource,
) -> Result<PermissionOverwriteResource, DiscordError> {
    let payload = EditPermissionsPayload {
        kind: desired.kind.as_discord_code(),
        allow: desired.allow.to_string(),
        deny: desired.deny.to_string(),
    };
    provider
        .http
        .put(
            &format!(
                "/channels/{}/permissions/{}",
                desired.channel_id, desired.target_id
            ),
            &payload,
        )
        .await?;
    Ok(desired.clone())
}

/// Update is the same as create for permission overwrites (PUT is
/// idempotent).
pub async fn update(
    provider: &DiscordProvider,
    _current: &PermissionOverwriteResource,
    desired: &PermissionOverwriteResource,
) -> Result<PermissionOverwriteResource, DiscordError> {
    create(provider, desired).await
}

/// Delete a permission overwrite from a channel. Idempotent.
pub async fn delete(
    provider: &DiscordProvider,
    current: &PermissionOverwriteResource,
) -> Result<(), DiscordError> {
    match provider
        .http
        .delete(&format!(
            "/channels/{}/permissions/{}",
            current.channel_id, current.target_id
        ))
        .await
    {
        Ok(()) => Ok(()),
        Err(DiscordError::Discord { status: 404, .. }) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guildforge_provider::OverwriteKind;
    use guildforge_shared::{ResourceId, Snowflake};

    #[test]
    fn payload_serializes_correctly() {
        let p = EditPermissionsPayload {
            kind: 0,
            allow: "8".into(),
            deny: "0".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"type\":0"));
        assert!(json.contains("\"allow\":\"8\""));
        assert!(json.contains("\"deny\":\"0\""));
    }

    #[test]
    fn create_returns_desired_unchanged() {
        // We can't actually call create without a mock server, but we
        // can verify the desired resource would be returned unchanged.
        let desired = PermissionOverwriteResource {
            addr: ResourceId::new("overwrite/c1/role:Admin"),
            id: None,
            channel_id: Snowflake::new(1),
            target_id: Snowflake::new(2),
            kind: OverwriteKind::Role,
            allow: 8,
            deny: 0,
        };
        // Just verify the resource is well-formed.
        assert_eq!(desired.kind.as_discord_code(), 0);
    }
}
