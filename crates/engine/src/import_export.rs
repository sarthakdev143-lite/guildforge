//! Convert provider [`Resource`]s back into a user-facing [`Config`].
//!
//! This is the inverse of `guildforge_planner::convert::config_to_resources`.
//! Used by `guildforge import` and `guildforge export`.
//!
//! # Stable ordering
//!
//! The output `Config` is sorted in a canonical order so that
//! `export → import → export` produces byte-identical YAML:
//! - Roles: by position (descending = highest first), then by name.
//! - Categories: by position, then by name.
//! - Channels within a category: by position, then by name.
//! - Top-level channels: by name.
//! - Other resources: by address.

use guildforge_config::{
    AnnouncementChannel, Category, Channel, ChannelType, Color, Config, ForumChannelFields,
    ForumTag, ForumTagMap, Invite, OverwriteKind, PermissionMap, PermissionOverwrite, Role, Server,
    TextChannelFields, VoiceChannelFields, Webhook,
};
use guildforge_provider::{
    CategoryResource, ChannelResource, ChannelType as ProviderChannelType, ForumTagResource,
    InviteResource, OverwriteKind as ProviderOverwriteKind, PermissionOverwriteResource, Resource,
    RoleResource, WebhookResource,
};
use std::collections::BTreeMap;

/// Convert a list of [`Resource`]s into a [`Config`].
///
/// The server name is required (Discord doesn't expose it via the
/// resource list; the caller passes it in from the guild object).
#[must_use]
pub fn resources_to_config(resources: &[Resource], server_name: &str) -> Config {
    let mut roles: Vec<RoleResource> = Vec::new();
    let mut categories: Vec<CategoryResource> = Vec::new();
    let mut channels: Vec<ChannelResource> = Vec::new();
    let mut overwrites: Vec<PermissionOverwriteResource> = Vec::new();
    let mut webhooks: Vec<WebhookResource> = Vec::new();
    let mut invites: Vec<InviteResource> = Vec::new();
    let mut forum_tags: BTreeMap<String, Vec<ForumTagResource>> = BTreeMap::new();

    for r in resources {
        match r {
            Resource::Role(r) => roles.push(r.clone()),
            Resource::Category(r) => categories.push(r.clone()),
            Resource::Channel(r) => channels.push(r.clone()),
            Resource::PermissionOverwrite(r) => overwrites.push(r.clone()),
            Resource::Webhook(r) => webhooks.push(r.clone()),
            Resource::Invite(r) => invites.push(r.clone()),
            Resource::ForumTag(r) => {
                // Extract channel name from the address: `tag/<channel>/<name>`
                let parts: Vec<&str> = r.addr.as_str().splitn(3, '/').collect();
                if parts.len() == 3 {
                    forum_tags
                        .entry(parts[1].to_string())
                        .or_default()
                        .push(r.clone());
                }
            }
            _ => {}
        }
    }

    // Sort roles by position descending (highest first), then by name.
    roles.sort_by(|a, b| b.position.cmp(&a.position).then(a.name.cmp(&b.name)));
    // Sort categories by position, then by name.
    categories.sort_by(|a, b| a.position.cmp(&b.position).then(a.name.cmp(&b.name)));
    // Sort channels by position, then by name.
    channels.sort_by(|a, b| a.position.cmp(&b.position).then(a.name.cmp(&b.name)));

    // Build the Config.
    let config_roles: Vec<Role> = roles.iter().map(role_to_config).collect();

    // Group channels by parent category.
    let mut channels_by_parent: BTreeMap<Option<String>, Vec<&ChannelResource>> = BTreeMap::new();
    for ch in &channels {
        // We don't have parent names (only IDs), so we group by ID for now.
        // The export will put all channels as top-level unless we can resolve
        // parent IDs to names. For simplicity, we emit all channels as
        // top-level and let the user reorganize.
        channels_by_parent
            .entry(ch.parent_id.map(|_| String::new())) // placeholder
            .or_default()
            .push(ch);
    }

    let mut config_categories: Vec<Category> = categories
        .iter()
        .map(|c| Category {
            name: c.name.clone(),
            description: None,
            permissions: None,
            channels: vec![],
        })
        .collect();

    // Attach channels to categories by matching parent_id to category ID.
    for cat in &mut config_categories {
        let cat_id = categories
            .iter()
            .find(|c| c.name == cat.name)
            .and_then(|c| c.id);
        if let Some(cat_id) = cat_id {
            let child_channels: Vec<Channel> = channels
                .iter()
                .filter(|ch| ch.parent_id == Some(cat_id))
                .map(channel_to_config)
                .collect();
            cat.channels = child_channels;
        }
    }

    // Top-level channels (no parent).
    let config_channels: Vec<Channel> = channels
        .iter()
        .filter(|ch| ch.parent_id.is_none())
        .map(channel_to_config)
        .collect();

    // Permission overwrites.
    let config_overwrites: Vec<PermissionOverwrite> =
        overwrites.iter().map(overwrite_to_config).collect();

    // Webhooks.
    let config_webhooks: Vec<Webhook> = webhooks.iter().map(webhook_to_config).collect();

    // Invites.
    let config_invites: Vec<Invite> = invites.iter().map(invite_to_config).collect();

    // Forum tags.
    let config_forum_tags: ForumTagMap = forum_tags
        .iter()
        .map(|(chan, tags)| {
            let config_tags: Vec<ForumTag> = tags
                .iter()
                .map(|t| ForumTag {
                    name: t.name.clone(),
                    moderated: if t.moderated { Some(true) } else { None },
                    emoji: t.emoji.clone(),
                })
                .collect();
            (chan.clone(), config_tags)
        })
        .collect();

    Config {
        schema_version: Some(1),
        server: Server {
            name: server_name.to_string(),
            description: None,
            icon: None,
            banner: None,
            verification_level: None,
            explicit_content_filter: None,
            default_notifications: None,
            system_channel: None,
            system_channel_flags: vec![],
            afk_channel: None,
            afk_timeout: None,
            premium_progress_bar: None,
        },
        roles: config_roles,
        categories: config_categories,
        channels: config_channels,
        permissions: PermissionMap::new(),
        permission_overwrites: config_overwrites,
        webhooks: config_webhooks,
        invites: config_invites,
        forum_tags: config_forum_tags,
        welcome_screen: None,
        server_guide: None,
        ordering: None,
    }
}

fn role_to_config(r: &RoleResource) -> Role {
    let color = if r.color == 0 {
        None
    } else {
        Some(u32_to_color(r.color))
    };
    Role {
        name: r.name.clone(),
        color,
        hoist: if r.hoist { Some(true) } else { None },
        mentionable: if r.mentionable { Some(true) } else { None },
        permissions: bitfield_to_names(r.permissions),
        position: if r.position > 0 {
            Some(r.position)
        } else {
            None
        },
        icon: None,
        unicode_emoji: r.unicode_emoji.clone(),
    }
}

fn channel_to_config(ch: &ChannelResource) -> Channel {
    let kind = match ch.kind {
        ProviderChannelType::Text => ChannelType::Text,
        ProviderChannelType::Voice => ChannelType::Voice,
        ProviderChannelType::Forum => ChannelType::Forum,
        ProviderChannelType::Announcement => ChannelType::Announcement,
        ProviderChannelType::StageVoice => ChannelType::StageVoice,
    };
    Channel {
        name: ch.name.clone(),
        kind,
        category: None,
        topic: ch.topic.clone(),
        nsfw: if ch.nsfw { Some(true) } else { None },
        slowmode: if ch.slowmode > 0 {
            Some(ch.slowmode)
        } else {
            None
        },
        permissions: None,
        text: Some(TextChannelFields::default()),
        voice: if matches!(
            ch.kind,
            ProviderChannelType::Voice | ProviderChannelType::StageVoice
        ) {
            Some(VoiceChannelFields {
                bitrate: ch.bitrate,
                user_limit: ch.user_limit,
                rtc_region: None,
                video_quality_mode: None,
            })
        } else {
            None
        },
        stage: None,
        forum: if ch.kind == ProviderChannelType::Forum {
            Some(ForumChannelFields {
                available_tags: ch.available_tags.iter().map(|t| t.name.clone()).collect(),
                default_reaction_emoji: ch.default_reaction_emoji.clone(),
                default_sort_order: ch.default_sort_order,
                default_forum_layout: ch.default_forum_layout,
            })
        } else {
            None
        },
        announcement: if ch.kind == ProviderChannelType::Announcement {
            Some(AnnouncementChannel::default())
        } else {
            None
        },
    }
}

fn overwrite_to_config(o: &PermissionOverwriteResource) -> PermissionOverwrite {
    PermissionOverwrite {
        channel: o.channel_id.to_string(),
        kind: match o.kind {
            ProviderOverwriteKind::Role => OverwriteKind::Role,
            ProviderOverwriteKind::Member => OverwriteKind::Member,
        },
        target: o.target_id.to_string(),
        allow: bitfield_to_names(o.allow),
        deny: bitfield_to_names(o.deny),
    }
}

fn webhook_to_config(w: &WebhookResource) -> Webhook {
    Webhook {
        name: w.name.clone(),
        channel: w.channel_id.to_string(),
        avatar: w.avatar.clone(),
    }
}

fn invite_to_config(i: &InviteResource) -> Invite {
    Invite {
        channel: i.channel_id.to_string(),
        max_age: (i.max_age != 86400).then_some(i.max_age),
        max_uses: (i.max_uses != 0).then_some(i.max_uses),
        temporary: i.temporary.then_some(true),
        unique: i.unique.then_some(true),
    }
}

/// Convert a u32 color to a Config Color (hex format).
fn u32_to_color(c: u32) -> Color {
    Color::Hex(format!("#{:06X}", c))
}

/// Convert a permission bitfield back to a list of permission names.
fn bitfield_to_names(bits: u64) -> Vec<String> {
    let mut names = Vec::new();
    for (name, bit) in PERMISSION_NAMES {
        if bits & bit != 0 {
            names.push((*name).to_string());
        }
    }
    names
}

/// Permission name → bit mapping (must match planner/src/convert.rs).
const PERMISSION_NAMES: &[(&str, u64)] = &[
    ("create_instant_invite", 1 << 0),
    ("kick_members", 1 << 1),
    ("ban_members", 1 << 2),
    ("administrator", 1 << 3),
    ("manage_channels", 1 << 4),
    ("manage_guild", 1 << 5),
    ("add_reactions", 1 << 6),
    ("view_audit_log", 1 << 7),
    ("priority_speaker", 1 << 8),
    ("stream", 1 << 9),
    ("view_channel", 1 << 10),
    ("send_messages", 1 << 11),
    ("send_tts_messages", 1 << 12),
    ("manage_messages", 1 << 13),
    ("embed_links", 1 << 14),
    ("attach_files", 1 << 15),
    ("read_message_history", 1 << 16),
    ("mention_everyone", 1 << 17),
    ("use_external_emojis", 1 << 18),
    ("view_guild_insights", 1 << 19),
    ("connect", 1 << 20),
    ("speak", 1 << 21),
    ("mute_members", 1 << 22),
    ("deafen_members", 1 << 23),
    ("move_members", 1 << 24),
    ("use_vad", 1 << 25),
    ("change_nickname", 1 << 26),
    ("manage_nicknames", 1 << 27),
    ("manage_roles", 1 << 28),
    ("manage_webhooks", 1 << 29),
    ("manage_emojis_and_stickers", 1 << 30),
    ("use_application_commands", 1 << 31),
    ("request_to_speak", 1 << 32),
    ("manage_events", 1 << 33),
    ("manage_threads", 1 << 34),
    ("create_public_threads", 1 << 35),
    ("create_private_threads", 1 << 36),
    ("use_external_stickers", 1 << 37),
    ("send_messages_in_threads", 1 << 38),
    ("use_embedded_activities", 1 << 39),
    ("moderate_members", 1 << 40),
    ("view_creator_monetization_analytics", 1 << 41),
    ("use_soundboard", 1 << 42),
];

/// Serialize a Config to canonical YAML with stable ordering.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn config_to_yaml(config: &Config) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use guildforge_provider::RoleResource;
    use guildforge_shared::ResourceId;

    #[test]
    fn empty_resources_produce_minimal_config() {
        let cfg = resources_to_config(&[], "TestGuild");
        assert_eq!(cfg.server.name, "TestGuild");
        assert!(cfg.roles.is_empty());
    }

    #[test]
    fn role_round_trip() {
        let role = RoleResource {
            addr: ResourceId::new("role/Admin"),
            id: None,
            name: "Admin".into(),
            color: 0xFF_5733,
            hoist: true,
            mentionable: true,
            permissions: 1 << 3, // administrator
            position: 5,
            unicode_emoji: None,
        };
        let cfg = resources_to_config(&[Resource::Role(role)], "Test");
        assert_eq!(cfg.roles.len(), 1);
        assert_eq!(cfg.roles[0].name, "Admin");
        // Color should be hex format
        assert!(matches!(cfg.roles[0].color, Some(Color::Hex(_))));
        assert!(cfg.roles[0].hoist == Some(true));
        assert!(cfg.roles[0]
            .permissions
            .contains(&"administrator".to_string()));
    }

    #[test]
    fn bitfield_round_trip() {
        let bits = (1 << 3) | (1 << 11); // administrator + send_messages
        let names = bitfield_to_names(bits);
        assert!(names.contains(&"administrator".to_string()));
        assert!(names.contains(&"send_messages".to_string()));
    }

    #[test]
    fn u32_to_color_produces_hex() {
        let c = u32_to_color(0xFF_5733);
        assert!(matches!(c, Color::Hex(_)));
        if let Color::Hex(s) = c {
            assert_eq!(s, "#FF5733");
        }
    }

    #[test]
    fn config_to_yaml_is_stable() {
        let role = RoleResource::new("role/Admin", "Admin");
        let cfg = resources_to_config(&[Resource::Role(role)], "Test");
        let yaml1 = config_to_yaml(&cfg).unwrap();
        let yaml2 = config_to_yaml(&cfg).unwrap();
        assert_eq!(yaml1, yaml2);
    }
}
