//! Strongly-typed serde models for every key in the `GuildForge` YAML
//! schema.
//!
//! This crate contains ONLY types. Parsing lives in
//! [`guildforge-parser`], semantic validation lives in
//! [`guildforge-validation`]. See [`docs/SCHEMA.md`](../../docs/SCHEMA.md)
//! for the authoritative schema specification.
//!
//! # Rules
//!
//! - Every struct has `#[serde(deny_unknown_fields)]`.
//! - Every optional field is `Option<T>` with
//!   `#[serde(default, skip_serializing_if = "Option::is_none")]`.
//! - No `HashMap<String, Value>` anywhere — every mapping is a typed struct.
//!
//! Phase 0: this crate is a stub. Full model implementation lands in
//! Phase 1 (task `P1-003`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use serde::{Deserialize, Serialize};

/// Root config struct, corresponding to the top-level YAML mapping.
///
/// See [`docs/SCHEMA.md` §3](../../docs/SCHEMA.md) for the spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Optional schema version. If present and > 1, parser rejects.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "_schema_version")]
    pub schema_version: Option<u32>,

    /// Guild-level settings. Required.
    pub server: Server,

    /// Optional role declarations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Role>,

    /// Optional category declarations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<Category>,

    /// Optional channel declarations (top-level or via `category` ref).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<Channel>,

    /// Optional shorthand permission block, keyed by channel name.
    #[serde(default, skip_serializing_if = "PermissionMap::is_empty")]
    pub permissions: PermissionMap,

    /// Optional full-form permission overwrites.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permission_overwrites: Vec<PermissionOverwrite>,

    /// Optional webhooks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub webhooks: Vec<Webhook>,

    /// Optional invites.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub invites: Vec<Invite>,

    /// Optional forum tags, keyed by forum channel name.
    #[serde(default, skip_serializing_if = "ForumTagMap::is_empty")]
    pub forum_tags: ForumTagMap,

    /// Optional welcome screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub welcome_screen: Option<WelcomeScreen>,

    /// Optional server guide / onboarding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_guide: Option<ServerGuide>,

    /// Optional explicit ordering overrides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordering: Option<Ordering>,
}

/// Guild-level settings. See [`docs/SCHEMA.md` §3.1](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    /// Guild name, 2-100 chars.
    pub name: String,
    /// Guild description, max 120 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // TODO(P1-003): remaining fields per SCHEMA.md §3.1.
}

/// Role declaration. See [`docs/SCHEMA.md` §3.2](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    /// Role name, 1-100 chars, unique within guild.
    pub name: String,
    /// Role color (named, hex, rgb, or default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Whether the role is hoisted (shown separately in member list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hoist: Option<bool>,
    /// Whether the role can be mentioned by users.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<bool>,
    /// List of permission names granted by this role.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
}

/// Category declaration. See [`docs/SCHEMA.md` §3.3](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// Category name, 0-100 chars, unique within guild.
    pub name: String,
    /// Category description, max 120 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Inline channels under this category.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<Channel>,
}

/// Channel declaration. See [`docs/SCHEMA.md` §3.4](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    /// Channel name, 1-100 chars.
    pub name: String,
    /// Channel type (text, voice, forum, etc.).
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// Parent category name, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Channel topic, max 1024 chars (text/forum only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// Discord channel types supported by `GuildForge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Standard text channel.
    Text,
    /// Voice channel.
    Voice,
    /// Forum channel (requires boost level 1+).
    Forum,
    /// Announcement channel (requires Community).
    Announcement,
    /// Stage voice channel (requires Community).
    StageVoice,
}

/// Color representation. See [`docs/SCHEMA.md` §3.2](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Color {
    /// Named color (`red`, `blue`, etc.).
    Named(String),
    /// Hex color (`#RRGGBB` or `0xRRGGBB`).
    Hex(String),
    /// Default theme-dependent color.
    Default,
}

/// Shorthand permission block, keyed by channel name.
pub type PermissionMap = std::collections::BTreeMap<String, PermissionBlock>;

/// Shorthand permission block applied to a single channel or category.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionBlock {
    /// Roles that can read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read: Vec<String>,
    /// Roles that can write.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write: Vec<String>,
    /// Roles that can manage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manage: Vec<String>,
}

/// Full-form permission overwrite. See [`docs/SCHEMA.md` §3.7](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionOverwrite {
    /// Target channel or category name.
    pub channel: String,
    /// Whether the target is a role or a member.
    #[serde(rename = "type")]
    pub kind: OverwriteKind,
    /// Role name or member ID.
    pub target: String,
    /// Permissions to allow.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    /// Permissions to deny.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
}

/// Kind of permission overwrite target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverwriteKind {
    /// Role overwrite.
    Role,
    /// Member overwrite.
    Member,
}

/// Webhook declaration. See [`docs/SCHEMA.md` §3.8](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Webhook {
    /// Webhook name, 1-80 chars.
    pub name: String,
    /// Target channel name.
    pub channel: String,
    /// Optional avatar (path or URL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

/// Invite declaration. See [`docs/SCHEMA.md` §3.9](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invite {
    /// Target channel name.
    pub channel: String,
    /// Max age in seconds (0 = never, max 604800).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u64>,
    /// Max uses (0 = unlimited, max 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u32>,
    /// Whether the invite grants temporary membership.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    /// Whether to guarantee a unique code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
}

/// Forum tags, keyed by forum channel name.
pub type ForumTagMap = std::collections::BTreeMap<String, Vec<ForumTag>>;

/// A single forum tag. See [`docs/SCHEMA.md` §3.10](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForumTag {
    /// Tag name, 1-20 chars, unique within channel.
    pub name: String,
    /// Whether posts with this tag require moderation approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderated: Option<bool>,
    /// Optional unicode emoji (custom emoji not supported).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
}

/// Welcome screen. See [`docs/SCHEMA.md` §3.11](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WelcomeScreen {
    /// Whether the welcome screen is enabled.
    pub enabled: bool,
    /// Welcome screen description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Channels featured on the welcome screen (max 5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<WelcomeScreenChannel>,
}

/// A channel featured on the welcome screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WelcomeScreenChannel {
    /// Channel name.
    pub channel: String,
    /// Description shown on the welcome screen.
    pub description: String,
}

/// Server guide / onboarding. See [`docs/SCHEMA.md` §3.12](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerGuide {
    /// Whether the server guide is enabled.
    pub enabled: bool,
    /// Welcome message shown to new members.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub welcome_message: Option<String>,
    /// Recommended channels (max 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_channels: Vec<WelcomeScreenChannel>,
}

/// Explicit ordering overrides. See [`docs/SCHEMA.md` §3.13](../../docs/SCHEMA.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ordering {
    /// Role ordering (top-to-bottom = highest to lowest).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    /// Category ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    /// Per-category channel ordering (and `_top_level` for top-level channels).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<std::collections::BTreeMap<String, Vec<String>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_config_parses() {
        let yaml = "server:\n  name: Test\n";
        let cfg: Config = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(cfg.server.name, "Test");
        assert!(cfg.roles.is_empty());
    }

    #[test]
    fn unknown_top_level_key_rejected() {
        let yaml = "server:\n  name: Test\nbogus: true\n";
        let result: Result<Config, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn channel_type_snake_case() {
        let yaml = "text: x\n";
        let ct: ChannelType = serde_yaml::from_str("text").unwrap();
        assert_eq!(ct, ChannelType::Text);
        let _ = yaml;
    }
}
