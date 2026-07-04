//! Convert a [`Config`] into a set of desired [`Resource`]s.
//!
//! This is the bridge between the user-facing YAML schema (which uses
//! names, not IDs) and the provider's typed [`Resource`] enum (which
//! uses addresses).
//!
//! # Resource addressing
//!
//! | Resource | Address format |
//! |---|---|
//! | Role | `role/<name>` |
//! | Category | `category/<name>` |
//! | Channel (top-level) | `channel/<name>` |
//! | Channel (in category) | `channel/<category>/<name>` |
//! | Webhook | `webhook/<channel>/<name>` |
//! | Invite | `invite/<channel>/<code>` (we don't know the code ahead of time) |

use crate::PlannerError;
use guildforge_config::{
    ChannelType, Color, Config, NamedColor, OverwriteKind as ConfigOverwriteKind,
};
use guildforge_provider::{
    CategoryResource, ChannelResource, ChannelType as ProviderChannelType, ForumTagResource,
    InviteResource, OverwriteKind, PermissionOverwriteResource, Resource, RoleResource,
    WebhookResource,
};
use guildforge_shared::ResourceId;

/// Convert a [`Config`] into a `Vec<Resource>` representing the desired
/// state.
///
/// # Errors
///
/// Returns [`PlannerError`] if the config has an invalid reference
/// (should have been caught by validation, but we double-check).
pub fn config_to_resources(config: &Config) -> Result<Vec<Resource>, PlannerError> {
    let mut resources = Vec::new();

    // Build a name → ID map for categories and channels as we go, so
    // that child resources (permissions, webhooks, forum tags) can
    // reference them. In config, references are by name; in the
    // Resource model, references would be by Snowflake ID — but we
    // don't know IDs until the provider creates the resources. For
    // planning purposes, we use addresses (names) as references.

    // ---- Roles ----
    for role in &config.roles {
        let addr = ResourceId::new(format!("role/{}", role.name));
        let color = color_to_u32(&role.color);
        let perms = permissions_to_bitfield(&role.permissions);
        let resource = RoleResource {
            addr,
            id: None,
            name: role.name.clone(),
            color,
            hoist: role.hoist.unwrap_or(false),
            mentionable: role.mentionable.unwrap_or(false),
            permissions: perms,
            position: role.position.unwrap_or(0),
            unicode_emoji: role.unicode_emoji.clone(),
        };
        resources.push(Resource::Role(resource));
    }

    // ---- Categories ----
    for cat in &config.categories {
        let addr = ResourceId::new(format!("category/{}", cat.name));
        let resource = CategoryResource {
            addr,
            id: None,
            name: cat.name.clone(),
            position: 0,
            nsfw: false,
            permission_overwrites: vec![],
        };
        resources.push(Resource::Category(resource));

        // Inline channels within this category
        for ch in &cat.channels {
            let chan_addr = ResourceId::new(format!("channel/{}/{}", cat.name, ch.name));
            let resource = channel_to_resource(ch, chan_addr, Some(&cat.name))?;
            resources.push(resource);
        }
    }

    // ---- Top-level channels ----
    for ch in &config.channels {
        let chan_addr = if let Some(cat) = &ch.category {
            ResourceId::new(format!("channel/{}/{}", cat, ch.name))
        } else {
            ResourceId::new(format!("channel/_top/{}", ch.name))
        };
        let resource = channel_to_resource(ch, chan_addr, ch.category.as_deref())?;
        resources.push(resource);
    }

    // ---- Permission overwrites (full form) ----
    for ow in &config.permission_overwrites {
        let addr = ResourceId::new(format!(
            "overwrite/{}/{}:{}",
            ow.channel,
            match ow.kind {
                ConfigOverwriteKind::Role => "role",
                ConfigOverwriteKind::Member => "member",
            },
            ow.target
        ));
        // We don't know channel_id / target_id at plan time (they're
        // Discord Snowflakes assigned at creation). Use 0 as a
        // placeholder; the executor resolves names to IDs before
        // calling the provider.
        let resource = PermissionOverwriteResource {
            addr,
            id: None,
            channel_id: guildforge_shared::Snowflake::new(0),
            target_id: guildforge_shared::Snowflake::new(0),
            kind: match ow.kind {
                ConfigOverwriteKind::Role => OverwriteKind::Role,
                ConfigOverwriteKind::Member => OverwriteKind::Member,
            },
            allow: permissions_to_bitfield(&ow.allow),
            deny: permissions_to_bitfield(&ow.deny),
        };
        resources.push(Resource::PermissionOverwrite(resource));
    }

    // ---- Webhooks ----
    for wh in &config.webhooks {
        let addr = ResourceId::new(format!("webhook/{}/{}", wh.channel, wh.name));
        let resource = WebhookResource {
            addr,
            id: None,
            name: wh.name.clone(),
            channel_id: guildforge_shared::Snowflake::new(0),
            url: None,
            avatar: wh.avatar.clone(),
        };
        resources.push(Resource::Webhook(resource));
    }

    // ---- Invites ----
    for inv in &config.invites {
        // We don't know the invite code ahead of time; use the channel
        // name as a placeholder address.
        let addr = ResourceId::new(format!("invite/{}", inv.channel));
        let resource = InviteResource {
            addr,
            code: String::new(), // assigned by Discord
            channel_id: guildforge_shared::Snowflake::new(0),
            max_age: inv.max_age.unwrap_or(86400),
            max_uses: inv.max_uses.unwrap_or(0),
            temporary: inv.temporary.unwrap_or(false),
            unique: inv.unique.unwrap_or(false),
            uses: 0,
        };
        resources.push(Resource::Invite(resource));
    }

    // ---- Forum tags ----
    for (chan, tags) in &config.forum_tags {
        for tag in tags {
            let addr = ResourceId::new(format!("tag/{}/{}", chan, tag.name));
            let resource = ForumTagResource {
                addr,
                id: None,
                name: tag.name.clone(),
                moderated: tag.moderated.unwrap_or(false),
                emoji: tag.emoji.clone(),
            };
            resources.push(Resource::ForumTag(resource));
        }
    }

    Ok(resources)
}

fn channel_to_resource(
    ch: &guildforge_config::Channel,
    addr: ResourceId,
    _category: Option<&str>,
) -> Result<Resource, PlannerError> {
    let kind = match ch.kind {
        ChannelType::Text => ProviderChannelType::Text,
        ChannelType::Voice => ProviderChannelType::Voice,
        ChannelType::Forum => ProviderChannelType::Forum,
        ChannelType::Announcement => ProviderChannelType::Announcement,
        ChannelType::StageVoice => ProviderChannelType::StageVoice,
    };
    let resource = ChannelResource {
        addr,
        id: None,
        name: ch.name.clone(),
        kind,
        parent_id: None, // resolved at apply time
        topic: ch.topic.clone(),
        nsfw: ch.nsfw.unwrap_or(false),
        slowmode: ch.slowmode.unwrap_or(0),
        bitrate: ch.voice.as_ref().and_then(|v| v.bitrate),
        user_limit: ch.voice.as_ref().and_then(|v| v.user_limit),
        position: 0,
        permission_overwrites: vec![],
        available_tags: ch
            .forum
            .as_ref()
            .map(|f| {
                f.available_tags
                    .iter()
                    .map(|name| ForumTagResource {
                        addr: ResourceId::new("tag/_unknown"),
                        id: None,
                        name: name.clone(),
                        moderated: false,
                        emoji: None,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        default_reaction_emoji: ch
            .forum
            .as_ref()
            .and_then(|f| f.default_reaction_emoji.clone()),
        default_sort_order: ch.forum.as_ref().and_then(|f| f.default_sort_order),
        default_forum_layout: ch.forum.as_ref().and_then(|f| f.default_forum_layout),
    };
    Ok(Resource::Channel(resource))
}

/// Convert a config [`Color`] to a `u32` RGB value.
fn color_to_u32(color: &Option<Color>) -> u32 {
    let Some(color) = color else {
        return 0;
    };
    match color {
        Color::Default => 0,
        Color::Named(named) => named_to_u32(*named),
        Color::Hex(s) => parse_hex(s).unwrap_or(0),
        Color::Rgb(s) => parse_rgb(s).unwrap_or(0),
    }
}

fn named_to_u32(n: NamedColor) -> u32 {
    match n {
        NamedColor::Default => 0,
        NamedColor::White => 0xFF_FFFF,
        NamedColor::Black => 0x00_0000,
        NamedColor::DarkGray => 0x42_4242,
        NamedColor::LighterGray => 0x99_ABA4,
        NamedColor::DarkerGray => 0x1E_1F22,
        NamedColor::LightGray => 0xCC_DCDD,
        NamedColor::VeryDarkGray => 0x12_1316,
        NamedColor::Red => 0xE7_4C3C,
        NamedColor::DarkRed => 0x99_2D22,
        NamedColor::Orange => 0xE6_7E22,
        NamedColor::DarkOrange => 0xA8_4300,
        NamedColor::Gold => 0xF1_C40F,
        NamedColor::DarkGold => 0x9B_6300,
        NamedColor::Yellow => 0xFE_E75A,
        NamedColor::DarkYellow => 0xC2_7C0E,
        NamedColor::Green => 0x2E_CC71,
        NamedColor::DarkGreen => 0x1F_8B4C,
        NamedColor::Teal => 0x1D_BC9C,
        NamedColor::DarkTeal => 0x11_806A,
        NamedColor::Blue => 0x34_98DB,
        NamedColor::DarkBlue => 0x20_6069,
        NamedColor::Purple => 0x9B_59B6,
        NamedColor::DarkPurple => 0x71_3BAF,
        NamedColor::Magenta => 0xE9_1E63,
        NamedColor::DarkMagenta => 0xAD_1457,
        NamedColor::LightPink => 0xFF_80AB,
        NamedColor::DarkPink => 0xF4_8FB1,
    }
}

fn parse_hex(s: &str) -> Option<u32> {
    let s = s.strip_prefix('#').or_else(|| s.strip_prefix("0x"))?;
    u32::from_str_radix(s, 16).ok()
}

fn parse_rgb(s: &str) -> Option<u32> {
    let inner = s.strip_prefix("rgb(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
    if parts.len() != 3 {
        return None;
    }
    let r: u32 = parts[0].parse().ok()?;
    let g: u32 = parts[1].parse().ok()?;
    let b: u32 = parts[2].parse().ok()?;
    Some((r << 16) | (g << 8) | b)
}

/// Permission name → bitfield mapping.
///
/// See <https://discord.com/developers/docs/topics/permissions>.
fn permission_bit(name: &str) -> Option<u64> {
    Some(match name {
        "create_instant_invite" => 1 << 0,
        "kick_members" => 1 << 1,
        "ban_members" => 1 << 2,
        "administrator" => 1 << 3,
        "manage_channels" => 1 << 4,
        "manage_guild" => 1 << 5,
        "add_reactions" => 1 << 6,
        "view_audit_log" => 1 << 7,
        "priority_speaker" => 1 << 8,
        "stream" => 1 << 9,
        "view_channel" | "read_messages" => 1 << 10,
        "send_messages" => 1 << 11,
        "send_tts_messages" => 1 << 12,
        "manage_messages" => 1 << 13,
        "embed_links" => 1 << 14,
        "attach_files" => 1 << 15,
        "read_message_history" => 1 << 16,
        "mention_everyone" => 1 << 17,
        "use_external_emojis" => 1 << 18,
        "view_guild_insights" => 1 << 19,
        "connect" => 1 << 20,
        "speak" => 1 << 21,
        "mute_members" => 1 << 22,
        "deafen_members" => 1 << 23,
        "move_members" => 1 << 24,
        "use_vad" => 1 << 25,
        "change_nickname" => 1 << 26,
        "manage_nicknames" => 1 << 27,
        "manage_roles" => 1 << 28,
        "manage_webhooks" => 1 << 29,
        "manage_emojis_and_stickers" => 1 << 30,
        "use_application_commands" => 1 << 31,
        "request_to_speak" => 1 << 32,
        "manage_events" => 1 << 33,
        "manage_threads" => 1 << 34,
        "create_public_threads" => 1 << 35,
        "create_private_threads" => 1 << 36,
        "use_external_stickers" => 1 << 37,
        "send_messages_in_threads" => 1 << 38,
        "use_embedded_activities" => 1 << 39,
        "moderate_members" => 1 << 40,
        "view_creator_monetization_analytics" => 1 << 41,
        "use_soundboard" => 1 << 42,
        _ => return None,
    })
}

/// Convert a list of permission names to a bitfield.
fn permissions_to_bitfield(names: &[String]) -> u64 {
    let mut bits: u64 = 0;
    for name in names {
        if let Some(bit) = permission_bit(name) {
            bits |= bit;
        }
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_config_converts_to_no_resources() {
        let cfg: Config = serde_yaml::from_str("server:\n  name: Test\n").unwrap();
        let resources = config_to_resources(&cfg).unwrap();
        assert!(resources.is_empty());
    }

    #[test]
    fn role_converts_with_color_and_permissions() {
        let cfg: Config = serde_yaml::from_str(
            "server:\n  name: Test\nroles:\n  - name: Admin\n    color: red\n    permissions: [administrator]\n",
        )
        .unwrap();
        let resources = config_to_resources(&cfg).unwrap();
        assert_eq!(resources.len(), 1);
        if let Resource::Role(r) = &resources[0] {
            assert_eq!(r.name, "Admin");
            assert_eq!(r.color, 0xE7_4C3C); // named red
            assert_eq!(r.permissions, 1 << 3); // administrator bit
        } else {
            panic!("expected Role");
        }
    }

    #[test]
    fn hex_color_converts() {
        let cfg: Config = serde_yaml::from_str(
            "server:\n  name: Test\nroles:\n  - name: R\n    color: \"#FF5733\"\n",
        )
        .unwrap();
        let resources = config_to_resources(&cfg).unwrap();
        if let Resource::Role(r) = &resources[0] {
            assert_eq!(r.color, 0xFF_5733);
        }
    }

    #[test]
    fn channel_converts_with_correct_type() {
        let cfg: Config = serde_yaml::from_str(
            "server:\n  name: Test\nchannels:\n  - name: c1\n    type: text\n  - name: v1\n    type: voice\n    bitrate: 64000\n",
        )
        .unwrap();
        let resources = config_to_resources(&cfg).unwrap();
        assert_eq!(resources.len(), 2);
        if let Resource::Channel(c) = &resources[0] {
            assert_eq!(c.kind, ProviderChannelType::Text);
        }
        if let Resource::Channel(c) = &resources[1] {
            assert_eq!(c.kind, ProviderChannelType::Voice);
            assert_eq!(c.bitrate, Some(64_000));
        }
    }

    #[test]
    fn category_with_inline_channels() {
        let cfg: Config = serde_yaml::from_str(
            "server:\n  name: Test\ncategories:\n  - name: CAT\n    channels:\n      - name: c1\n        type: text\n",
        )
        .unwrap();
        let resources = config_to_resources(&cfg).unwrap();
        assert_eq!(resources.len(), 2); // 1 category + 1 channel
        assert!(matches!(resources[0], Resource::Category(_)));
        assert!(matches!(resources[1], Resource::Channel(_)));
    }

    #[test]
    fn permission_bit_known() {
        assert_eq!(permission_bit("administrator"), Some(1 << 3));
        assert_eq!(permission_bit("send_messages"), Some(1 << 11));
        assert_eq!(permission_bit("bogus"), None);
    }

    #[test]
    fn permissions_to_bitfield_accumulates() {
        let bits =
            permissions_to_bitfield(&["administrator".to_string(), "send_messages".to_string()]);
        assert_eq!(bits, (1 << 3) | (1 << 11));
    }
}
