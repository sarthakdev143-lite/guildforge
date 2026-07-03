//! Resource types shared across all providers.
//!
//! Every provider implementation must populate these structs faithfully
//! — omitting fields causes spurious diffs in the planner. See
//! [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md).

use guildforge_shared::{Hash, ResourceId, Snowflake};
use serde::{Deserialize, Serialize};

/// Trait-object-compatible address for a resource.
pub type ResourceAddr = ResourceId;

/// The kind of a resource.
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
    /// A role resource.
    Role(RoleResource),
    /// A category resource.
    Category(CategoryResource),
    /// A channel resource.
    Channel(ChannelResource),
    /// A permission overwrite resource.
    PermissionOverwrite(PermissionOverwriteResource),
    /// A webhook resource.
    Webhook(WebhookResource),
    /// An invite resource.
    Invite(InviteResource),
    /// A forum tag resource.
    ForumTag(ForumTagResource),
    /// The welcome screen resource.
    WelcomeScreen(WelcomeScreenResource),
    /// The server guide resource.
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
    pub fn addr(&self) -> &ResourceAddr {
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

    /// Compute the content hash of this resource for diffing.
    ///
    /// The hash is taken over the canonical JSON serialization of the
    /// inner resource (excluding the `addr` field, which is identity,
    /// not content). See
    /// [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md).
    #[must_use]
    pub fn content_hash(&self) -> Hash {
        // Serialize the inner resource (not the wrapping enum) so the
        // `kind` tag doesn't affect the hash.
        let Ok(json) = serde_json::to_string(self) else {
            return Hash::of(b"<serialize-error>");
        };
        Hash::of(json.as_bytes())
    }
}

// ===========================================================================
// Role
// ===========================================================================

/// A role resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleResource {
    /// Stable resource address (e.g. `role/Admin`).
    pub addr: ResourceAddr,
    /// Discord-assigned role ID.
    pub id: Option<Snowflake>,
    /// Role name (1-100 chars).
    pub name: String,
    /// Role color as RGB integer (0xRRGGBB). `0` means default.
    pub color: u32,
    /// Whether the role is hoisted (shown separately in member list).
    pub hoist: bool,
    /// Whether the role is mentionable.
    pub mentionable: bool,
    /// Permissions bitfield.
    pub permissions: u64,
    /// Position (higher = more authority). Discord assigns this.
    pub position: u32,
    /// Optional unicode emoji icon.
    pub unicode_emoji: Option<String>,
}

impl RoleResource {
    /// Construct a new role with sane defaults.
    #[must_use]
    pub fn new(addr: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            addr: ResourceAddr::new(addr.into()),
            id: None,
            name: name.into(),
            color: 0,
            hoist: false,
            mentionable: false,
            permissions: 0,
            position: 0,
            unicode_emoji: None,
        }
    }
}

// ===========================================================================
// Category
// ===========================================================================

/// A category resource (a Discord `guild_category` channel).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Discord-assigned channel ID.
    pub id: Option<Snowflake>,
    /// Category name (0-100 chars).
    pub name: String,
    /// Position within the guild.
    pub position: u32,
    /// Whether the category is NSFW.
    pub nsfw: bool,
    /// Permission overwrites on this category (one entry per role/member).
    pub permission_overwrites: Vec<PermissionOverwriteResource>,
}

impl CategoryResource {
    /// Construct a new category with sane defaults.
    #[must_use]
    pub fn new(addr: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            addr: ResourceAddr::new(addr.into()),
            id: None,
            name: name.into(),
            position: 0,
            nsfw: false,
            permission_overwrites: vec![],
        }
    }
}

// ===========================================================================
// Channel
// ===========================================================================

/// Discord channel types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Standard text channel.
    Text,
    /// Voice channel.
    Voice,
    /// Forum channel.
    Forum,
    /// Announcement channel.
    Announcement,
    /// Stage voice channel.
    StageVoice,
}

impl ChannelType {
    /// Convert to the Discord API integer code.
    #[must_use]
    pub const fn as_discord_code(self) -> u8 {
        match self {
            Self::Text => 0,
            Self::Voice => 2,
            Self::Forum => 15,
            Self::Announcement => 5,
            Self::StageVoice => 13,
        }
    }

    /// Convert from a Discord API integer code.
    #[must_use]
    pub const fn from_discord_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Text),
            2 => Some(Self::Voice),
            5 => Some(Self::Announcement),
            13 => Some(Self::StageVoice),
            15 => Some(Self::Forum),
            _ => None,
        }
    }
}

/// A channel resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Discord-assigned channel ID.
    pub id: Option<Snowflake>,
    /// Channel name (1-100 chars).
    pub name: String,
    /// Channel type.
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// Parent category ID, if any.
    pub parent_id: Option<Snowflake>,
    /// Channel topic (text/forum only).
    pub topic: Option<String>,
    /// Whether the channel is NSFW.
    pub nsfw: bool,
    /// Slowmode delay in seconds (0-21600).
    pub slowmode: u32,
    /// Bitrate (voice only, 8000-384000).
    pub bitrate: Option<u32>,
    /// User limit (voice only, 0-99).
    pub user_limit: Option<u32>,
    /// Position within the parent.
    pub position: u32,
    /// Permission overwrites on this channel.
    pub permission_overwrites: Vec<PermissionOverwriteResource>,
    /// Forum tags (forum only).
    pub available_tags: Vec<ForumTagResource>,
    /// Default reaction emoji (forum only).
    pub default_reaction_emoji: Option<String>,
    /// Default sort order (forum only). 0 = latest activity, 1 = creation.
    pub default_sort_order: Option<u32>,
    /// Default forum layout (forum only). 0 = list, 1 = gallery.
    pub default_forum_layout: Option<u32>,
}

impl ChannelResource {
    /// Construct a new text channel with sane defaults.
    #[must_use]
    pub fn new_text(addr: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            addr: ResourceAddr::new(addr.into()),
            id: None,
            name: name.into(),
            kind: ChannelType::Text,
            parent_id: None,
            topic: None,
            nsfw: false,
            slowmode: 0,
            bitrate: None,
            user_limit: None,
            position: 0,
            permission_overwrites: vec![],
            available_tags: vec![],
            default_reaction_emoji: None,
            default_sort_order: None,
            default_forum_layout: None,
        }
    }

    /// Construct a new voice channel with sane defaults.
    #[must_use]
    pub fn new_voice(addr: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            kind: ChannelType::Voice,
            bitrate: Some(64_000),
            user_limit: Some(0),
            ..Self::new_text(addr, name)
        }
    }

    /// Construct a new forum channel with sane defaults.
    #[must_use]
    pub fn new_forum(addr: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            kind: ChannelType::Forum,
            ..Self::new_text(addr, name)
        }
    }
}

// ===========================================================================
// PermissionOverwrite
// ===========================================================================

/// Kind of permission overwrite target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverwriteKind {
    /// Role overwrite.
    Role,
    /// Member overwrite.
    Member,
}

impl OverwriteKind {
    /// Convert to the Discord API integer code (0 = role, 1 = member).
    #[must_use]
    pub const fn as_discord_code(self) -> u8 {
        match self {
            Self::Role => 0,
            Self::Member => 1,
        }
    }

    /// Convert from a Discord API integer code.
    #[must_use]
    pub const fn from_discord_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Role),
            1 => Some(Self::Member),
            _ => None,
        }
    }
}

/// A permission overwrite resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionOverwriteResource {
    /// Stable resource address (e.g.
    /// `overwrite/announcements/role:Admin`).
    pub addr: ResourceAddr,
    /// Discord-assigned overwrite ID.
    pub id: Option<Snowflake>,
    /// Target channel ID.
    pub channel_id: Snowflake,
    /// Target role or member ID.
    pub target_id: Snowflake,
    /// Whether the target is a role or a member.
    #[serde(rename = "type")]
    pub kind: OverwriteKind,
    /// Permissions to allow (bitfield).
    pub allow: u64,
    /// Permissions to deny (bitfield).
    pub deny: u64,
}

// ===========================================================================
// Webhook
// ===========================================================================

/// A webhook resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Discord-assigned webhook ID.
    pub id: Option<Snowflake>,
    /// Webhook name (1-80 chars).
    pub name: String,
    /// Target channel ID.
    pub channel_id: Snowflake,
    /// Webhook URL (contains secret token — never logged).
    pub url: Option<String>,
    /// Avatar hash (Discord-assigned; we never store the raw avatar).
    pub avatar: Option<String>,
}

// ===========================================================================
// Invite
// ===========================================================================

/// An invite resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Invite code (the unique part of discord.gg/<code>).
    pub code: String,
    /// Target channel ID.
    pub channel_id: Snowflake,
    /// Max age in seconds (0 = never).
    pub max_age: u64,
    /// Max uses (0 = unlimited).
    pub max_uses: u32,
    /// Whether the invite grants temporary membership.
    pub temporary: bool,
    /// Whether the invite is unique.
    pub unique: bool,
    /// Current use count (Discord-assigned).
    pub uses: u32,
}

// ===========================================================================
// ForumTag
// ===========================================================================

/// A forum tag resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForumTagResource {
    /// Stable resource address.
    pub addr: ResourceAddr,
    /// Discord-assigned tag ID.
    pub id: Option<Snowflake>,
    /// Tag name (1-20 chars).
    pub name: String,
    /// Whether posts with this tag require moderation.
    pub moderated: bool,
    /// Unicode emoji (custom emoji not supported in v1).
    pub emoji: Option<String>,
}

// ===========================================================================
// WelcomeScreen
// ===========================================================================

/// The welcome screen resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WelcomeScreenResource {
    /// Stable resource address (always `welcome_screen`).
    pub addr: ResourceAddr,
    /// Whether the welcome screen is enabled.
    pub enabled: bool,
    /// Welcome screen description.
    pub description: Option<String>,
    /// Channels featured on the welcome screen (max 5).
    pub channels: Vec<WelcomeScreenChannel>,
}

/// A channel featured on the welcome screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WelcomeScreenChannel {
    /// Channel ID.
    pub channel_id: Snowflake,
    /// Description shown on the welcome screen.
    pub description: String,
    /// Emoji shown next to the channel.
    pub emoji: Option<String>,
}

// ===========================================================================
// ServerGuide
// ===========================================================================

/// The server guide / onboarding resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerGuideResource {
    /// Stable resource address (always `server_guide`).
    pub addr: ResourceAddr,
    /// Whether the server guide is enabled.
    pub enabled: bool,
    /// Welcome message shown to new members.
    pub welcome_message: Option<String>,
    /// Recommended channels (max 7).
    pub recommended_channels: Vec<WelcomeScreenChannel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_kind_round_trip() {
        for k in [
            ResourceKind::Role,
            ResourceKind::Category,
            ResourceKind::Channel,
            ResourceKind::PermissionOverwrite,
            ResourceKind::Webhook,
            ResourceKind::Invite,
            ResourceKind::ForumTag,
            ResourceKind::WelcomeScreen,
            ResourceKind::ServerGuide,
        ] {
            let yaml = serde_yaml::to_string(&k).unwrap();
            let k2: ResourceKind = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(k, k2);
        }
    }

    #[test]
    fn resource_addr_and_kind() {
        let r = Resource::Role(RoleResource::new("role/Admin", "Admin"));
        assert_eq!(r.kind(), ResourceKind::Role);
        assert_eq!(r.addr().as_str(), "role/Admin");
    }

    #[test]
    fn content_hash_is_deterministic() {
        let r = RoleResource::new("role/Admin", "Admin");
        let h1 = Resource::Role(r.clone()).content_hash();
        let h2 = Resource::Role(r).content_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_differs_on_change() {
        let r1 = Resource::Role(RoleResource::new("role/Admin", "Admin"));
        let r2 = Resource::Role(RoleResource::new("role/Admin", "Moderator"));
        assert_ne!(r1.content_hash(), r2.content_hash());
    }

    #[test]
    fn channel_type_discord_code_round_trip() {
        for ct in [
            ChannelType::Text,
            ChannelType::Voice,
            ChannelType::Forum,
            ChannelType::Announcement,
            ChannelType::StageVoice,
        ] {
            let code = ct.as_discord_code();
            assert_eq!(ChannelType::from_discord_code(code), Some(ct));
        }
        assert_eq!(ChannelType::from_discord_code(99), None);
    }

    #[test]
    fn overwrite_kind_discord_code_round_trip() {
        for k in [OverwriteKind::Role, OverwriteKind::Member] {
            let code = k.as_discord_code();
            assert_eq!(OverwriteKind::from_discord_code(code), Some(k));
        }
    }

    #[test]
    fn role_resource_defaults() {
        let r = RoleResource::new("role/Admin", "Admin");
        assert_eq!(r.color, 0);
        assert!(!r.hoist);
        assert!(!r.mentionable);
        assert_eq!(r.permissions, 0);
        assert!(r.id.is_none());
    }

    #[test]
    fn channel_resource_constructors() {
        let t = ChannelResource::new_text("channel/c1", "c1");
        assert_eq!(t.kind, ChannelType::Text);
        assert_eq!(t.bitrate, None);

        let v = ChannelResource::new_voice("channel/v1", "v1");
        assert_eq!(v.kind, ChannelType::Voice);
        assert_eq!(v.bitrate, Some(64_000));

        let f = ChannelResource::new_forum("channel/f1", "f1");
        assert_eq!(f.kind, ChannelType::Forum);
    }

    #[test]
    fn resource_serde_round_trip() {
        let r = Resource::Role(RoleResource::new("role/Admin", "Admin"));
        let json = serde_json::to_string(&r).unwrap();
        let r2: Resource = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }
}
