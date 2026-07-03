//! Role CRUD operations against Discord.
//!
//! API reference: <https://discord.com/developers/docs/resources/guild#roles>

use crate::error::DiscordError;
use crate::DiscordProvider;
use guildforge_provider::{ResourceAddr, RoleResource};
use guildforge_shared::{ResourceId, Snowflake};
use serde::{Deserialize, Serialize};

// ===========================================================================
// Discord API types (mirror the wire format)
// ===========================================================================

/// Discord API role object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordRole {
    /// Role ID.
    pub id: String,
    /// Role name.
    pub name: String,
    /// Color as integer (0xRRGGBB; 0 = default).
    pub color: u32,
    /// Whether the role is hoisted.
    pub hoist: bool,
    /// Icon hash (Discord-assigned; we don't manage raw icon bytes in v1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Unicode emoji icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<String>,
    /// Position.
    pub position: u32,
    /// Permissions bitfield (as string per Discord API).
    pub permissions: String,
    /// Whether the role is mentionable.
    pub mentionable: bool,
    /// Whether the role is managed by integration/bot.
    #[serde(default)]
    pub managed: bool,
    /// Whether the role is bot-managed.
    #[serde(default)]
    pub tags: Option<DiscordRoleTags>,
}

/// Role tags (for bot/integration roles).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordRoleTags {
    /// Bot ID, if this role is a bot role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<String>,
    /// Integration ID, if integration-managed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_id: Option<String>,
}

/// Payload for `POST /guilds/:id/roles`.
#[derive(Debug, Serialize)]
struct CreateRolePayload<'a> {
    name: &'a str,
    color: u32,
    hoist: bool,
    mentionable: bool,
    permissions: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    unicode_emoji: Option<&'a str>,
}

/// Payload for `PATCH /guilds/:id/roles/:role_id`.
#[derive(Debug, Serialize)]
struct ModifyRolePayload<'a> {
    name: &'a str,
    color: u32,
    hoist: bool,
    mentionable: bool,
    permissions: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    unicode_emoji: Option<&'a str>,
}

/// Payload for `PATCH /guilds/:id` (role positions).
#[derive(Debug, Serialize)]
struct ModifyRolePositionsPayload<'a> {
    id: &'a str,
    position: u32,
}

// ===========================================================================
// Conversions
// ===========================================================================

fn permissions_to_string(p: u64) -> String {
    p.to_string()
}

fn permissions_from_string(s: &str) -> u64 {
    s.parse::<u64>().unwrap_or(0)
}

fn parse_role_id(s: &str) -> Option<Snowflake> {
    s.parse::<u64>().ok().map(Snowflake::new)
}

/// Parse a role address of form `role/<name>` into the role name.
fn parse_role_addr(addr: &ResourceAddr) -> Result<String, DiscordError> {
    let s = addr.as_str();
    s.strip_prefix("role/")
        .map(String::from)
        .ok_or_else(|| DiscordError::Unsupported(format!("not a role address: {s}")))
}

// ===========================================================================
// CRUD
// ===========================================================================

/// Read a role by address. Returns `Ok(None)` if the role does not exist.
pub async fn read(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<RoleResource>, DiscordError> {
    let name = parse_role_addr(addr)?;
    let roles: Vec<DiscordRole> = provider
        .http
        .get(&format!("/guilds/{}/roles", provider.guild_id))
        .await?;

    for role in roles {
        if role.name.eq_ignore_ascii_case(&name) {
            return Ok(Some(role_to_resource(role, addr.clone())));
        }
    }
    Ok(None)
}

/// Create a new role.
pub async fn create(
    provider: &DiscordProvider,
    desired: &RoleResource,
) -> Result<RoleResource, DiscordError> {
    let perms_str = permissions_to_string(desired.permissions);
    let payload = CreateRolePayload {
        name: &desired.name,
        color: desired.color,
        hoist: desired.hoist,
        mentionable: desired.mentionable,
        permissions: &perms_str,
        unicode_emoji: desired.unicode_emoji.as_deref(),
    };
    let role: DiscordRole = provider
        .http
        .post(&format!("/guilds/{}/roles", provider.guild_id), &payload)
        .await?;
    Ok(role_to_resource(role, desired.addr.clone()))
}

/// Update a role from `current` to `desired`.
pub async fn update(
    provider: &DiscordProvider,
    current: &RoleResource,
    desired: &RoleResource,
) -> Result<RoleResource, DiscordError> {
    // Idempotency: if no field changed, return current.
    if current == desired {
        return Ok(current.clone());
    }
    let id = current.id.ok_or_else(|| {
        DiscordError::Unsupported("update: role has no Discord ID (not yet created?)".into())
    })?;
    let perms_str = permissions_to_string(desired.permissions);
    let payload = ModifyRolePayload {
        name: &desired.name,
        color: desired.color,
        hoist: desired.hoist,
        mentionable: desired.mentionable,
        permissions: &perms_str,
        unicode_emoji: desired.unicode_emoji.as_deref(),
    };
    let role: DiscordRole = provider
        .http
        .patch(
            &format!("/guilds/{}/roles/{}", provider.guild_id, id),
            &payload,
        )
        .await?;
    Ok(role_to_resource(role, desired.addr.clone()))
}

/// Delete a role. Idempotent: deleting a missing role returns Ok.
pub async fn delete(
    provider: &DiscordProvider,
    current: &RoleResource,
) -> Result<(), DiscordError> {
    let Some(id) = current.id else {
        // No ID means the role never existed; treat as already deleted.
        return Ok(());
    };
    match provider
        .http
        .delete(&format!("/guilds/{}/roles/{}", provider.guild_id, id))
        .await
    {
        Ok(()) => Ok(()),
        Err(DiscordError::Discord { status: 404, .. }) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Reorder a role by setting its position.
pub async fn reorder(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
    new_position: u32,
) -> Result<(), DiscordError> {
    let Some(role) = read(provider, addr).await? else {
        return Ok(()); // Nothing to reorder.
    };
    let Some(id) = role.id else {
        return Ok(());
    };
    let id_str = id.to_string();
    let payload = vec![ModifyRolePositionsPayload {
        id: &id_str,
        position: new_position,
    }];
    provider
        .http
        .patch(&format!("/guilds/{}/roles", provider.guild_id), &payload)
        .await
        .map(|_: Vec<DiscordRole>| ())
}

/// List all roles in the guild.
pub async fn list(provider: &DiscordProvider) -> Result<Vec<RoleResource>, DiscordError> {
    let roles: Vec<DiscordRole> = provider
        .http
        .get(&format!("/guilds/{}/roles", provider.guild_id))
        .await?;
    Ok(roles
        .into_iter()
        .map(|r| {
            let name = r.name.clone();
            role_to_resource(r, ResourceId::new(format!("role/{name}")))
        })
        .collect())
}

// ===========================================================================
// Conversion helpers
// ===========================================================================

fn role_to_resource(role: DiscordRole, addr: ResourceAddr) -> RoleResource {
    RoleResource {
        addr,
        id: parse_role_id(&role.id),
        name: role.name,
        color: role.color,
        hoist: role.hoist,
        mentionable: role.mentionable,
        permissions: permissions_from_string(&role.permissions),
        position: role.position,
        unicode_emoji: role.unicode_emoji,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_string_round_trip() {
        assert_eq!(permissions_to_string(0), "0");
        assert_eq!(permissions_to_string(8), "8"); // administrator
        assert_eq!(permissions_from_string("8"), 8);
        assert_eq!(permissions_from_string("bogus"), 0);
    }

    #[test]
    fn parse_role_addr_works() {
        let addr = ResourceId::new("role/Admin");
        assert_eq!(parse_role_addr(&addr).unwrap(), "Admin");
    }

    #[test]
    fn parse_role_addr_rejects_other_kinds() {
        let addr = ResourceId::new("channel/general");
        assert!(parse_role_addr(&addr).is_err());
    }

    #[test]
    fn role_to_resource_round_trip() {
        let discord_role = DiscordRole {
            id: "123".into(),
            name: "Admin".into(),
            color: 0xFF5733,
            hoist: true,
            icon: None,
            unicode_emoji: Some("X".into()),
            position: 5,
            permissions: "8".into(),
            mentionable: true,
            managed: false,
            tags: None,
        };
        let r = role_to_resource(discord_role, ResourceId::new("role/Admin"));
        assert_eq!(r.id, Some(Snowflake::new(123)));
        assert_eq!(r.name, "Admin");
        assert_eq!(r.color, 0xFF5733);
        assert!(r.hoist);
        assert!(r.mentionable);
        assert_eq!(r.permissions, 8);
        assert_eq!(r.position, 5);
    }

    #[test]
    fn payload_serializes_correctly() {
        let p = CreateRolePayload {
            name: "Admin",
            color: 0xFF0000,
            hoist: true,
            mentionable: true,
            permissions: "8",
            unicode_emoji: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"name\":\"Admin\""));
        assert!(json.contains("\"color\":16711680")); // 0xFF0000
        assert!(json.contains("\"permissions\":\"8\""));
        assert!(!json.contains("unicode_emoji"));
    }
}
