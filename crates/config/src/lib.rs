//! Strongly-typed serde models for every key in the `GuildForge` YAML
//! schema.
//!
//! This crate contains ONLY types. Parsing lives in
//! [`guildforge_parser`], semantic validation lives in
//! [`guildforge_validation`]. See [`docs/SCHEMA.md`](../../docs/SCHEMA.md)
//! for the authoritative schema specification.
//!
//! # Rules
//!
//! - Every struct has `#[serde(deny_unknown_fields)]`.
//! - Every optional field is `Option<T>` with
//!   `#[serde(default, skip_serializing_if = "Option::is_none")]`.
//! - No `HashMap<String, Value>` anywhere — every mapping is a typed struct.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

pub mod channel;
pub mod forum;
pub mod invite;
pub mod ordering;
pub mod permission;
pub mod role;
pub mod server;
pub mod webhook;
pub mod welcome;

pub use channel::{
    AnnouncementChannel, Category, Channel, ChannelType, ForumChannelFields,
    StageVoiceChannelFields, TextChannelFields, VoiceChannelFields,
};
pub use forum::{ForumTag, ForumTagMap};
pub use invite::Invite;
pub use ordering::Ordering;
pub use permission::{
    OverwriteKind, PermissionBlock, PermissionMap, PermissionOverwrite, PermissionShorthand,
};
pub use role::{Color, NamedColor, Role};
pub use server::{
    AfkTimeout, DefaultNotifications, ExplicitContentFilter, Server, SystemChannelFlag,
    VerificationLevel,
};
pub use webhook::Webhook;
pub use welcome::{ServerGuide, WelcomeScreen, WelcomeScreenChannel};

use serde::{Deserialize, Serialize};

/// The current schema version this crate understands.
pub const SCHEMA_VERSION: u32 = 1;

/// Root config struct, corresponding to the top-level YAML mapping.
///
/// See [`docs/SCHEMA.md` §3](../../docs/SCHEMA.md) for the spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Optional schema version. If present and > 1, parser rejects.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "_schema_version"
    )]
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

impl Config {
    /// Returns `true` if the config has no roles, categories, channels,
    /// webhooks, invites, or forum tags (i.e. only `server` is set).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.roles.is_empty()
            && self.categories.is_empty()
            && self.channels.is_empty()
            && self.permissions.is_empty()
            && self.permission_overwrites.is_empty()
            && self.webhooks.is_empty()
            && self.invites.is_empty()
            && self.forum_tags.is_empty()
            && self.welcome_screen.is_none()
            && self.server_guide.is_none()
    }

    /// Returns a `Vec` of references to every channel declaration,
    /// including those nested inside categories.
    #[must_use]
    pub fn all_channels(&self) -> Vec<&Channel> {
        let mut out: Vec<&Channel> =
            Vec::with_capacity(self.channels.len() + self.categories.len() * 4);
        out.extend(self.channels.iter());
        for c in &self.categories {
            out.extend(c.channels.iter());
        }
        out
    }

    /// Returns a `Vec` of references to every category declaration.
    #[must_use]
    pub fn all_categories(&self) -> Vec<&Category> {
        self.categories.iter().collect()
    }

    /// Returns a `Vec` of references to every role declaration.
    #[must_use]
    pub fn all_roles(&self) -> Vec<&Role> {
        self.roles.iter().collect()
    }
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
        assert!(cfg.is_empty());
    }

    #[test]
    fn unknown_top_level_key_rejected() {
        let yaml = "server:\n  name: Test\nbogus: true\n";
        let result: Result<Config, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn channel_type_snake_case() {
        let ct: ChannelType = serde_yaml::from_str("text").unwrap();
        assert_eq!(ct, ChannelType::Text);
    }

    #[test]
    fn all_channels_includes_nested() {
        let yaml = "\
server:
  name: Test
categories:
  - name: CAT1
    channels:
      - name: c1
        type: text
      - name: c2
        type: text
channels:
  - name: top1
    type: text
";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        let names: Vec<&str> = cfg.all_channels().iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["top1", "c1", "c2"]);
    }
}
