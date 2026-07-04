//! Channel + Category CRUD operations against Discord.
//!
//! API reference:
//! - <https://discord.com/developers/docs/resources/guild#create-guild-channel>
//! - <https://discord.com/developers/docs/resources/channel#modify-channel>
//! - <https://discord.com/developers/docs/resources/channel#deleteclose-channel>
//! - <https://discord.com/developers/docs/resources/guild#modify-guild-channel-positions>

use crate::error::DiscordError;
use crate::DiscordProvider;
use guildforge_provider::{
    CategoryResource, ChannelResource, ChannelType, OverwriteKind, PermissionOverwriteResource,
    ResourceAddr,
};
use guildforge_shared::{ResourceId, Snowflake};
use serde::{Deserialize, Serialize};

// ===========================================================================
// Discord API types
// ===========================================================================

/// Discord API channel object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannel {
    /// Channel ID.
    pub id: String,
    /// Channel type (0=text, 2=voice, 4=category, 5=announcement, 13=stage, 15=forum).
    #[serde(rename = "type")]
    pub kind: u8,
    /// Channel name.
    pub name: String,
    /// Topic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    /// NSFW flag.
    #[serde(default)]
    pub nsfw: bool,
    /// Slowmode delay.
    #[serde(default)]
    pub rate_limit_per_user: u32,
    /// Bitrate (voice).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    /// User limit (voice).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u32>,
    /// Position.
    #[serde(default)]
    pub position: u32,
    /// Parent category ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Permission overwrites.
    #[serde(default)]
    pub permission_overwrites: Vec<DiscordOverwrite>,
    /// Forum tags (forum only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_tags: Vec<DiscordForumTag>,
    /// Default reaction emoji (forum).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_reaction_emoji: Option<DiscordDefaultReaction>,
    /// Default sort order (forum).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sort_order: Option<u32>,
    /// Default forum layout (forum).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<u32>,
}

/// Discord permission overwrite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordOverwrite {
    /// Target ID (role or member).
    pub id: String,
    /// 0 = role, 1 = member.
    #[serde(rename = "type")]
    pub kind: u8,
    /// Allow bitfield (string).
    pub allow: String,
    /// Deny bitfield (string).
    pub deny: String,
}

/// Discord forum tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordForumTag {
    /// Tag ID.
    pub id: String,
    /// Tag name.
    pub name: String,
    /// Moderated flag.
    #[serde(default)]
    pub moderated: bool,
    /// Emoji.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<DiscordForumTagEmoji>,
}

/// Discord forum tag emoji.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordForumTagEmoji {
    /// Emoji name (for unicode).
    pub name: String,
    /// Emoji ID (for custom — not supported in v1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Discord default reaction emoji.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordDefaultReaction {
    /// Emoji ID (custom) or emoji name (unicode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<String>,
    /// Emoji name (unicode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
}

/// Payload for `POST /guilds/:id/channels`.
#[derive(Debug, Serialize)]
struct CreateChannelPayload<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_per_user: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bitrate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_limit: Option<u32>,
}

/// Payload for `PATCH /channels/:id`.
#[derive(Debug, Serialize)]
struct ModifyChannelPayload<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit_per_user: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bitrate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_limit: Option<u32>,
}

/// Payload for `PATCH /guilds/:id` (channel positions).
#[derive(Debug, Serialize)]
struct ModifyPositionsPayload<'a> {
    id: &'a str,
    position: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<&'a str>,
}

// ===========================================================================
// Conversions
// ===========================================================================

fn parse_snowflake(s: &str) -> Option<Snowflake> {
    s.parse::<u64>().ok().map(Snowflake::new)
}

fn channel_to_resource(
    ch: DiscordChannel,
    addr: ResourceAddr,
) -> Result<ChannelResource, DiscordError> {
    let kind = ChannelType::from_discord_code(ch.kind)
        .ok_or_else(|| DiscordError::Decode(format!("unknown channel type: {}", ch.kind)))?;
    let overwrites: Vec<PermissionOverwriteResource> = ch
        .permission_overwrites
        .into_iter()
        .map(|o| PermissionOverwriteResource {
            addr: ResourceId::new(format!(
                "overwrite/{}/{}:{}",
                ch.id,
                if o.kind == 0 { "role" } else { "member" },
                o.id
            )),
            id: None,
            channel_id: parse_snowflake(&ch.id).unwrap_or(Snowflake::new(0)),
            target_id: parse_snowflake(&o.id).unwrap_or(Snowflake::new(0)),
            kind: OverwriteKind::from_discord_code(o.kind).unwrap_or(OverwriteKind::Role),
            allow: o.allow.parse::<u64>().unwrap_or(0),
            deny: o.deny.parse::<u64>().unwrap_or(0),
        })
        .collect();
    Ok(ChannelResource {
        addr,
        id: parse_snowflake(&ch.id),
        name: ch.name,
        kind,
        parent_id: ch.parent_id.and_then(|p| parse_snowflake(&p)),
        topic: ch.topic,
        nsfw: ch.nsfw,
        slowmode: ch.rate_limit_per_user,
        bitrate: ch.bitrate,
        user_limit: ch.user_limit,
        position: ch.position,
        permission_overwrites: overwrites,
        available_tags: vec![], // populated by forum_tag module
        default_reaction_emoji: ch
            .default_reaction_emoji
            .map(|r| r.emoji_name.unwrap_or_default()),
        default_sort_order: ch.default_sort_order,
        default_forum_layout: ch.default_forum_layout,
    })
}

fn category_to_resource(
    ch: DiscordChannel,
    addr: ResourceAddr,
) -> Result<CategoryResource, DiscordError> {
    debug_assert_eq!(ch.kind, 4, "category_to_resource on non-category channel");
    Ok(CategoryResource {
        addr,
        id: parse_snowflake(&ch.id),
        name: ch.name,
        position: ch.position,
        nsfw: ch.nsfw,
        permission_overwrites: ch
            .permission_overwrites
            .into_iter()
            .map(|o| PermissionOverwriteResource {
                addr: ResourceId::new(format!(
                    "overwrite/{}/{}:{}",
                    ch.id,
                    if o.kind == 0 { "role" } else { "member" },
                    o.id
                )),
                id: None,
                channel_id: parse_snowflake(&ch.id).unwrap_or(Snowflake::new(0)),
                target_id: parse_snowflake(&o.id).unwrap_or(Snowflake::new(0)),
                kind: OverwriteKind::from_discord_code(o.kind).unwrap_or(OverwriteKind::Role),
                allow: o.allow.parse::<u64>().unwrap_or(0),
                deny: o.deny.parse::<u64>().unwrap_or(0),
            })
            .collect(),
    })
}

/// Parse a channel address of form `channel/<name>` or
/// `channel/<category>/<name>` or `channel/_top/<name>`.
fn parse_channel_addr(addr: &ResourceAddr) -> Result<String, DiscordError> {
    let s = addr.as_str();
    let rest = s
        .strip_prefix("channel/")
        .ok_or_else(|| DiscordError::Unsupported(format!("not a channel address: {s}")))?;
    // Take the last segment as the name.
    let name = rest.rsplit('/').next().unwrap_or(rest);
    Ok(name.to_string())
}

/// Parse a category address of form `category/<name>`.
fn parse_category_addr(addr: &ResourceAddr) -> Result<String, DiscordError> {
    let s = addr.as_str();
    s.strip_prefix("category/")
        .map(String::from)
        .ok_or_else(|| DiscordError::Unsupported(format!("not a category address: {s}")))
}

// ===========================================================================
// CRUD: Channel
// ===========================================================================

/// Read a channel by address (across categories and top-level).
pub async fn read(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<ChannelResource>, DiscordError> {
    let name = parse_channel_addr(addr)?;
    let channels: Vec<DiscordChannel> = provider
        .http
        .get(&format!("/guilds/{}/channels", provider.guild_id))
        .await?;
    for ch in channels {
        if ch.kind != 4 && ch.name.eq_ignore_ascii_case(&name) {
            return Ok(Some(channel_to_resource(ch, addr.clone())?));
        }
    }
    Ok(None)
}

/// Create a channel.
pub async fn create(
    provider: &DiscordProvider,
    desired: &ChannelResource,
) -> Result<ChannelResource, DiscordError> {
    let parent_id_str = desired.parent_id.map(|s| s.to_string());
    let payload = CreateChannelPayload {
        name: &desired.name,
        kind: desired.kind.as_discord_code(),
        topic: desired.topic.as_deref(),
        parent_id: parent_id_str.as_deref(),
        nsfw: if desired.nsfw { Some(true) } else { None },
        rate_limit_per_user: if desired.slowmode > 0 {
            Some(desired.slowmode)
        } else {
            None
        },
        bitrate: desired.bitrate,
        user_limit: desired.user_limit,
    };
    let ch: DiscordChannel = provider
        .http
        .post(&format!("/guilds/{}/channels", provider.guild_id), &payload)
        .await?;
    channel_to_resource(ch, desired.addr.clone())
}

/// Update a channel from `current` to `desired`.
pub async fn update(
    provider: &DiscordProvider,
    current: &ChannelResource,
    desired: &ChannelResource,
) -> Result<ChannelResource, DiscordError> {
    if current == desired {
        return Ok(current.clone());
    }
    let id = current
        .id
        .ok_or_else(|| DiscordError::Unsupported("update: channel has no Discord ID".into()))?;
    let parent_id_str = desired.parent_id.map(|s| s.to_string());
    let payload = ModifyChannelPayload {
        name: &desired.name,
        topic: desired.topic.as_deref(),
        parent_id: parent_id_str.as_deref(),
        nsfw: Some(desired.nsfw),
        rate_limit_per_user: Some(desired.slowmode),
        bitrate: desired.bitrate,
        user_limit: desired.user_limit,
    };
    let ch: DiscordChannel = provider
        .http
        .patch(&format!("/channels/{}", id), &payload)
        .await?;
    channel_to_resource(ch, desired.addr.clone())
}

/// Delete a channel by ID. Idempotent.
pub async fn delete_channel(
    provider: &DiscordProvider,
    id: Option<Snowflake>,
) -> Result<(), DiscordError> {
    let Some(id) = id else {
        return Ok(());
    };
    match provider.http.delete(&format!("/channels/{}", id)).await {
        Ok(()) => Ok(()),
        Err(DiscordError::Discord { status: 404, .. }) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Reorder a channel by setting its position.
pub async fn reorder(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
    new_position: u32,
) -> Result<(), DiscordError> {
    let Some(ch) = read(provider, addr).await? else {
        return Ok(());
    };
    let Some(id) = ch.id else {
        return Ok(());
    };
    let id_str = id.to_string();
    let parent_id_str = ch.parent_id.map(|s| s.to_string());
    let payload = vec![ModifyPositionsPayload {
        id: &id_str,
        position: new_position,
        parent_id: parent_id_str.as_deref(),
    }];
    provider
        .http
        .patch(&format!("/guilds/{}/channels", provider.guild_id), &payload)
        .await
        .map(|_: serde_json::Value| ())
}

/// List all non-category channels in the guild.
pub async fn list(provider: &DiscordProvider) -> Result<Vec<ChannelResource>, DiscordError> {
    let channels: Vec<DiscordChannel> = provider
        .http
        .get(&format!("/guilds/{}/channels", provider.guild_id))
        .await?;
    channels
        .into_iter()
        .filter(|c| c.kind != 4)
        .map(|c| channel_to_resource(c, ResourceId::new("channel/_unknown")))
        .collect()
}

// ===========================================================================
// CRUD: Category
// ===========================================================================

/// Read a category by address.
pub async fn read_category(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<CategoryResource>, DiscordError> {
    let name = parse_category_addr(addr)?;
    let channels: Vec<DiscordChannel> = provider
        .http
        .get(&format!("/guilds/{}/channels", provider.guild_id))
        .await?;
    for ch in channels {
        if ch.kind == 4 && ch.name.eq_ignore_ascii_case(&name) {
            return Ok(Some(category_to_resource(ch, addr.clone())?));
        }
    }
    Ok(None)
}

/// Create a category.
pub async fn create_category(
    provider: &DiscordProvider,
    desired: &CategoryResource,
) -> Result<CategoryResource, DiscordError> {
    let payload = CreateChannelPayload {
        name: &desired.name,
        kind: 4, // guild_category
        topic: None,
        parent_id: None,
        nsfw: if desired.nsfw { Some(true) } else { None },
        rate_limit_per_user: None,
        bitrate: None,
        user_limit: None,
    };
    let ch: DiscordChannel = provider
        .http
        .post(&format!("/guilds/{}/channels", provider.guild_id), &payload)
        .await?;
    category_to_resource(ch, desired.addr.clone())
}

/// Update a category from `current` to `desired`.
pub async fn update_category(
    provider: &DiscordProvider,
    current: &CategoryResource,
    desired: &CategoryResource,
) -> Result<CategoryResource, DiscordError> {
    if current == desired {
        return Ok(current.clone());
    }
    let id = current
        .id
        .ok_or_else(|| DiscordError::Unsupported("update: category has no Discord ID".into()))?;
    let payload = ModifyChannelPayload {
        name: &desired.name,
        topic: None,
        parent_id: None,
        nsfw: Some(desired.nsfw),
        rate_limit_per_user: None,
        bitrate: None,
        user_limit: None,
    };
    let ch: DiscordChannel = provider
        .http
        .patch(&format!("/channels/{}", id), &payload)
        .await?;
    category_to_resource(ch, desired.addr.clone())
}

/// List all categories in the guild.
pub async fn list_categories(
    provider: &DiscordProvider,
) -> Result<Vec<CategoryResource>, DiscordError> {
    let channels: Vec<DiscordChannel> = provider
        .http
        .get(&format!("/guilds/{}/channels", provider.guild_id))
        .await?;
    channels
        .into_iter()
        .filter(|c| c.kind == 4)
        .map(|c| category_to_resource(c, ResourceId::new("category/_unknown")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_channel_addr_simple() {
        let addr = ResourceId::new("channel/general");
        assert_eq!(parse_channel_addr(&addr).unwrap(), "general");
    }

    #[test]
    fn parse_channel_addr_with_category() {
        let addr = ResourceId::new("channel/COMPANY/announcements");
        assert_eq!(parse_channel_addr(&addr).unwrap(), "announcements");
    }

    #[test]
    fn parse_channel_addr_top_level() {
        let addr = ResourceId::new("channel/_top/welcome");
        assert_eq!(parse_channel_addr(&addr).unwrap(), "welcome");
    }

    #[test]
    fn parse_category_addr_works() {
        let addr = ResourceId::new("category/COMPANY");
        assert_eq!(parse_category_addr(&addr).unwrap(), "COMPANY");
    }

    #[test]
    fn channel_to_resource_decodes_text() {
        let discord_ch = DiscordChannel {
            id: "123".into(),
            kind: 0,
            name: "general".into(),
            topic: Some("hello".into()),
            nsfw: false,
            rate_limit_per_user: 5,
            bitrate: None,
            user_limit: None,
            position: 1,
            parent_id: Some("456".into()),
            permission_overwrites: vec![],
            available_tags: vec![],
            default_reaction_emoji: None,
            default_sort_order: None,
            default_forum_layout: None,
        };
        let r = channel_to_resource(discord_ch, ResourceId::new("channel/general")).unwrap();
        assert_eq!(r.id, Some(Snowflake::new(123)));
        assert_eq!(r.kind, ChannelType::Text);
        assert_eq!(r.topic.as_deref(), Some("hello"));
        assert_eq!(r.slowmode, 5);
        assert_eq!(r.parent_id, Some(Snowflake::new(456)));
    }

    #[test]
    fn channel_to_resource_rejects_unknown_type() {
        let discord_ch = DiscordChannel {
            id: "1".into(),
            kind: 99,
            name: "x".into(),
            topic: None,
            nsfw: false,
            rate_limit_per_user: 0,
            bitrate: None,
            user_limit: None,
            position: 0,
            parent_id: None,
            permission_overwrites: vec![],
            available_tags: vec![],
            default_reaction_emoji: None,
            default_sort_order: None,
            default_forum_layout: None,
        };
        assert!(channel_to_resource(discord_ch, ResourceId::new("channel/x")).is_err());
    }

    #[test]
    fn create_payload_omits_optional_none() {
        let p = CreateChannelPayload {
            name: "x",
            kind: 0,
            topic: None,
            parent_id: None,
            nsfw: None,
            rate_limit_per_user: None,
            bitrate: None,
            user_limit: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"name\":\"x\""));
        assert!(json.contains("\"type\":0"));
        assert!(!json.contains("topic"));
        assert!(!json.contains("nsfw"));
    }
}
