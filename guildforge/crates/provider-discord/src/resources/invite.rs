//! Invite operations against Discord.
//!
//! API reference:
//! <https://discord.com/developers/docs/resources/invite>

use crate::error::DiscordError;
use crate::DiscordProvider;
use guildforge_provider::{InviteResource, ResourceAddr};
use guildforge_shared::{ResourceId, Snowflake};
use serde::{Deserialize, Serialize};

/// Discord API invite object (partial — we only care about the fields
/// we manage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordInvite {
    /// Invite code.
    pub code: String,
    /// Channel the invite is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<DiscordInviteChannel>,
    /// Max age in seconds.
    #[serde(default)]
    pub max_age: u32,
    /// Max uses.
    #[serde(default)]
    pub max_uses: u32,
    /// Temporary flag.
    #[serde(default)]
    pub temporary: bool,
    /// Unique flag.
    #[serde(default)]
    pub unique: bool,
    /// Current uses.
    #[serde(default)]
    pub uses: u32,
}

/// Discord invite channel reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordInviteChannel {
    /// Channel ID.
    pub id: String,
    /// Channel name.
    pub name: String,
}

/// Payload for `POST /channels/:id/invites`.
#[derive(Debug, Serialize)]
struct CreateInvitePayload {
    max_age: u64,
    max_uses: u32,
    temporary: bool,
    unique: bool,
}

fn parse_snowflake(s: &str) -> Option<Snowflake> {
    s.parse::<u64>().ok().map(Snowflake::new)
}

fn invite_to_resource(inv: DiscordInvite) -> InviteResource {
    let channel_id = inv
        .channel
        .as_ref()
        .and_then(|c| parse_snowflake(&c.id))
        .unwrap_or(Snowflake::new(0));
    InviteResource {
        addr: ResourceId::new(format!("invite/{}", inv.code)),
        code: inv.code,
        channel_id,
        max_age: u64::from(inv.max_age),
        max_uses: inv.max_uses,
        temporary: inv.temporary,
        unique: inv.unique,
        uses: inv.uses,
    }
}

/// Read an invite by code.
pub async fn read(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<InviteResource>, DiscordError> {
    let code = addr
        .as_str()
        .strip_prefix("invite/")
        .ok_or_else(|| DiscordError::Unsupported(format!("not an invite address: {}", addr)))?;
    match provider
        .http
        .get::<DiscordInvite>(&format!("/invites/{code}"))
        .await
    {
        Ok(inv) => Ok(Some(invite_to_resource(inv))),
        Err(DiscordError::Discord { status: 404, .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Create an invite on the desired channel.
pub async fn create(
    provider: &DiscordProvider,
    desired: &InviteResource,
) -> Result<InviteResource, DiscordError> {
    let payload = CreateInvitePayload {
        max_age: desired.max_age,
        max_uses: desired.max_uses,
        temporary: desired.temporary,
        unique: desired.unique,
    };
    let inv: DiscordInvite = provider
        .http
        .post(
            &format!("/channels/{}/invites", desired.channel_id),
            &payload,
        )
        .await?;
    Ok(invite_to_resource(inv))
}

/// Invites can't be updated — they can only be revoked (deleted) and
/// re-created. We treat this as unsupported.
pub async fn update(
    _provider: &DiscordProvider,
    _current: &InviteResource,
    desired: &InviteResource,
) -> Result<InviteResource, DiscordError> {
    // Return desired unchanged; planner treats this as no-op when
    // current == desired. For real changes, the planner should emit
    // Delete + Create instead.
    Ok(desired.clone())
}

/// Delete (revoke) an invite. Idempotent.
pub async fn delete(
    provider: &DiscordProvider,
    current: &InviteResource,
) -> Result<(), DiscordError> {
    match provider
        .http
        .delete(&format!("/invites/{}", current.code))
        .await
    {
        Ok(()) => Ok(()),
        Err(DiscordError::Discord { status: 404, .. }) => Ok(()),
        Err(e) => Err(e),
    }
}

/// List all invites in the guild.
pub async fn list(provider: &DiscordProvider) -> Result<Vec<InviteResource>, DiscordError> {
    let invites: Vec<DiscordInvite> = provider
        .http
        .get(&format!("/guilds/{}/invites", provider.guild_id))
        .await?;
    Ok(invites.into_iter().map(invite_to_resource).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invite_to_resource_decodes() {
        let inv = DiscordInvite {
            code: "abc123".into(),
            channel: Some(DiscordInviteChannel {
                id: "1".into(),
                name: "general".into(),
            }),
            max_age: 86400,
            max_uses: 10,
            temporary: false,
            unique: true,
            uses: 3,
        };
        let r = invite_to_resource(inv);
        assert_eq!(r.code, "abc123");
        assert_eq!(r.channel_id, Snowflake::new(1));
        assert_eq!(r.max_age, 86400);
        assert_eq!(r.uses, 3);
    }

    #[test]
    fn create_payload_serializes() {
        let p = CreateInvitePayload {
            max_age: 86400,
            max_uses: 10,
            temporary: false,
            unique: true,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"max_age\":86400"));
        assert!(json.contains("\"unique\":true"));
    }
}
