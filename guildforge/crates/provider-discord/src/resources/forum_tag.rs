//! Forum tag CRUD operations against Discord.
//!
//! Forum tags are part of the channel object in Discord. We manipulate
//! them via `PATCH /channels/:id` with a new `available_tags` array.
//!
//! API reference:
//! <https://discord.com/developers/docs/resources/channel#modify-channel>

use crate::error::DiscordError;
use crate::DiscordProvider;
use guildforge_provider::{ForumTagResource, ResourceAddr};
use guildforge_shared::{ResourceId, Snowflake};
use serde::Serialize;

/// Discord API forum tag (re-used from channel module).
pub use crate::resources::channel::DiscordForumTag;

/// Payload for `PATCH /channels/:id` (forum tags only).
#[derive(Debug, Serialize)]
struct ModifyForumTagsPayload<'a> {
    available_tags: Vec<ModifyForumTagPayload<'a>>,
}

#[derive(Debug, Serialize)]
struct ModifyForumTagPayload<'a> {
    name: &'a str,
    moderated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    emoji_name: Option<&'a str>,
}

fn parse_tag_addr(addr: &ResourceAddr) -> Result<(String, String), DiscordError> {
    // Format: `tag/<channel>/<name>`
    let s = addr.as_str();
    let rest = s
        .strip_prefix("tag/")
        .ok_or_else(|| DiscordError::Unsupported(format!("not a tag address: {s}")))?;
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(DiscordError::Unsupported(format!(
            "malformed tag address: {s}"
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

async fn find_forum_channel_by_name(
    provider: &DiscordProvider,
    name: &str,
) -> Result<Option<Snowflake>, DiscordError> {
    let channels: Vec<crate::resources::channel::DiscordChannel> = provider
        .http
        .get(&format!("/guilds/{}/channels", provider.guild_id))
        .await?;
    for ch in channels {
        if ch.kind == 15 && ch.name.eq_ignore_ascii_case(name) {
            return Ok(parse_snowflake(&ch.id));
        }
    }
    Ok(None)
}

fn parse_snowflake(s: &str) -> Option<Snowflake> {
    s.parse::<u64>().ok().map(Snowflake::new)
}

fn tag_to_resource(tag: DiscordForumTag, channel_addr: &str) -> ForumTagResource {
    ForumTagResource {
        addr: ResourceId::new(format!("tag/{channel_addr}/{}", tag.name)),
        id: parse_snowflake(&tag.id),
        name: tag.name,
        moderated: tag.moderated,
        emoji: tag.emoji.map(|e| e.name),
    }
}

/// Read a forum tag by address. Returns `Ok(None)` if the tag or the
/// parent channel doesn't exist.
pub async fn read(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<ForumTagResource>, DiscordError> {
    let (channel_name, tag_name) = parse_tag_addr(addr)?;
    let Some(channel_id) = find_forum_channel_by_name(provider, &channel_name).await? else {
        return Ok(None);
    };
    let ch: crate::resources::channel::DiscordChannel = provider
        .http
        .get(&format!("/channels/{channel_id}"))
        .await?;
    for tag in ch.available_tags {
        if tag.name.eq_ignore_ascii_case(&tag_name) {
            return Ok(Some(tag_to_resource(tag, &channel_name)));
        }
    }
    Ok(None)
}

/// Create a forum tag by adding it to the channel's `available_tags`.
pub async fn create(
    provider: &DiscordProvider,
    desired: &ForumTagResource,
) -> Result<ForumTagResource, DiscordError> {
    let (channel_name, _) = parse_tag_addr(&desired.addr)?;
    let Some(channel_id) = find_forum_channel_by_name(provider, &channel_name).await? else {
        return Err(DiscordError::Discord {
            status: 404,
            body: format!("forum channel `{channel_name}` not found"),
        });
    };
    // Read current tags, append the new one, PATCH.
    let ch: crate::resources::channel::DiscordChannel = provider
        .http
        .get(&format!("/channels/{channel_id}"))
        .await?;
    let mut tags: Vec<ModifyForumTagPayload> = ch
        .available_tags
        .iter()
        .map(|t| ModifyForumTagPayload {
            name: &t.name,
            moderated: t.moderated,
            emoji_name: t.emoji.as_ref().map(|e| e.name.as_str()),
        })
        .collect();
    tags.push(ModifyForumTagPayload {
        name: &desired.name,
        moderated: desired.moderated,
        emoji_name: desired.emoji.as_deref(),
    });
    let payload = ModifyForumTagsPayload {
        available_tags: tags,
    };
    let _ = provider
        .http
        .patch::<crate::resources::channel::DiscordChannel, _>(
            &format!("/channels/{channel_id}"),
            &payload,
        )
        .await?;
    Ok(desired.clone())
}

/// Update a forum tag (replace the matching tag in `available_tags`).
pub async fn update(
    provider: &DiscordProvider,
    _current: &ForumTagResource,
    desired: &ForumTagResource,
) -> Result<ForumTagResource, DiscordError> {
    let (channel_name, tag_name) = parse_tag_addr(&desired.addr)?;
    let Some(channel_id) = find_forum_channel_by_name(provider, &channel_name).await? else {
        return Err(DiscordError::Discord {
            status: 404,
            body: format!("forum channel `{channel_name}` not found"),
        });
    };
    let ch: crate::resources::channel::DiscordChannel = provider
        .http
        .get(&format!("/channels/{channel_id}"))
        .await?;
    let tags: Vec<ModifyForumTagPayload> = ch
        .available_tags
        .iter()
        .map(|t| {
            if t.name.eq_ignore_ascii_case(&tag_name) {
                ModifyForumTagPayload {
                    name: &desired.name,
                    moderated: desired.moderated,
                    emoji_name: desired.emoji.as_deref(),
                }
            } else {
                ModifyForumTagPayload {
                    name: &t.name,
                    moderated: t.moderated,
                    emoji_name: t.emoji.as_ref().map(|e| e.name.as_str()),
                }
            }
        })
        .collect();
    let payload = ModifyForumTagsPayload {
        available_tags: tags,
    };
    let _ = provider
        .http
        .patch::<crate::resources::channel::DiscordChannel, _>(
            &format!("/channels/{channel_id}"),
            &payload,
        )
        .await?;
    Ok(desired.clone())
}

/// Delete a forum tag by removing it from `available_tags`.
pub async fn delete(
    provider: &DiscordProvider,
    current: &ForumTagResource,
) -> Result<(), DiscordError> {
    let (channel_name, tag_name) = parse_tag_addr(&current.addr)?;
    let Some(channel_id) = find_forum_channel_by_name(provider, &channel_name).await? else {
        return Ok(()); // Channel gone; tag is gone too.
    };
    let ch: crate::resources::channel::DiscordChannel = provider
        .http
        .get(&format!("/channels/{channel_id}"))
        .await?;
    let tags: Vec<ModifyForumTagPayload> = ch
        .available_tags
        .iter()
        .filter(|t| !t.name.eq_ignore_ascii_case(&tag_name))
        .map(|t| ModifyForumTagPayload {
            name: &t.name,
            moderated: t.moderated,
            emoji_name: t.emoji.as_ref().map(|e| e.name.as_str()),
        })
        .collect();
    let payload = ModifyForumTagsPayload {
        available_tags: tags,
    };
    let _ = provider
        .http
        .patch::<crate::resources::channel::DiscordChannel, _>(
            &format!("/channels/{channel_id}"),
            &payload,
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_addr_works() {
        let addr = ResourceId::new("tag/help/Question");
        let (c, t) = parse_tag_addr(&addr).unwrap();
        assert_eq!(c, "help");
        assert_eq!(t, "Question");
    }

    #[test]
    fn parse_tag_addr_rejects_no_slash() {
        let addr = ResourceId::new("tag/help");
        assert!(parse_tag_addr(&addr).is_err());
    }

    #[test]
    fn parse_tag_addr_rejects_wrong_prefix() {
        let addr = ResourceId::new("webhook/x");
        assert!(parse_tag_addr(&addr).is_err());
    }
}
