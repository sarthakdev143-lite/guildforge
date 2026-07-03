//! Categories and channels. See
//! [`docs/SCHEMA.md` §3.3, §3.4](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

use crate::permission::PermissionShorthand;

/// A category declaration. Categories are channel groups; in Discord they
/// are themselves channels of type `guild_category`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// Category name, 0-100 chars, unique within guild (case-insensitive).
    pub name: String,

    /// Category description, max 120 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Inline shorthand permission block applied to this category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionShorthand>,

    /// Inline list of channels under this category.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<Channel>,
}

/// A channel declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    /// Channel name, 1-100 chars.
    pub name: String,

    /// Channel type.
    #[serde(rename = "type")]
    pub kind: ChannelType,

    /// Parent category name, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Channel topic, max 1024 chars (text/forum only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,

    /// Whether the channel is NSFW.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,

    /// Slowmode delay in seconds, 0-21600 (text/forum only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slowmode: Option<u32>,

    /// Inline shorthand permission block applied to this channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionShorthand>,

    /// Type-specific fields for text channels.
    #[serde(flatten)]
    pub text: Option<TextChannelFields>,

    /// Type-specific fields for voice channels.
    #[serde(flatten)]
    pub voice: Option<VoiceChannelFields>,

    /// Type-specific fields for stage voice channels.
    #[serde(flatten)]
    pub stage: Option<StageVoiceChannelFields>,

    /// Type-specific fields for forum channels.
    #[serde(flatten)]
    pub forum: Option<ForumChannelFields>,

    /// Type-specific fields for announcement channels.
    #[serde(flatten)]
    pub announcement: Option<AnnouncementChannel>,
}

/// Text-channel-specific fields.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextChannelFields {
    /// Default auto-archive duration for threads in this channel
    /// (60, 1440, 4320, 10080 minutes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<u32>,
}

/// Voice-channel-specific fields.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceChannelFields {
    /// Bitrate, 8000-384000.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,

    /// User limit, 0-99 (0 = unlimited).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u32>,

    /// RTC region override (deprecated by Discord; kept for compat).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<String>,

    /// Video quality mode (auto=1, full=2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<u32>,
}

/// Stage-voice-channel-specific fields.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageVoiceChannelFields {
    /// Topic shown on the stage (max 120 chars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// Forum-channel-specific fields.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForumChannelFields {
    /// List of tag names that must exist on the channel (max 20).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_tags: Vec<String>,

    /// Default reaction emoji (unicode only; custom emoji not supported
    /// in v1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_reaction_emoji: Option<String>,

    /// Default sort order for posts. `0` = latest activity, `1` = creation
    /// date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sort_order: Option<u32>,

    /// Default forum layout view. `0` = list, `1` = gallery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<u32>,
}

/// Announcement-channel-specific fields (placeholder for future
/// extensibility; announcement channels share text channel fields).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnouncementChannel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    placeholder: Option<()>,
}

/// Discord channel types supported by `GuildForge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl ChannelType {
    /// Returns the Discord API integer code for this channel type.
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

    /// Returns `true` if this channel type can hold messages.
    #[must_use]
    pub const fn is_text_like(self) -> bool {
        matches!(self, Self::Text | Self::Announcement | Self::Forum)
    }

    /// Returns `true` if this channel type is a voice variant.
    #[must_use]
    pub const fn is_voice_like(self) -> bool {
        matches!(self, Self::Voice | Self::StageVoice)
    }

    /// Returns `true` if this channel type supports a `topic` field.
    #[must_use]
    pub const fn supports_topic(self) -> bool {
        matches!(
            self,
            Self::Text | Self::Forum | Self::StageVoice | Self::Announcement
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_channel_parses() {
        let yaml = "name: general\ntype: text\n";
        let c: Channel = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.name, "general");
        assert_eq!(c.kind, ChannelType::Text);
        assert!(c.category.is_none());
    }

    #[test]
    fn channel_with_category_parses() {
        let yaml = "name: general\ntype: text\ncategory: COMPANY\ntopic: Hello\n";
        let c: Channel = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.category.as_deref(), Some("COMPANY"));
        assert_eq!(c.topic.as_deref(), Some("Hello"));
    }

    #[test]
    fn voice_channel_parses() {
        let yaml = "name: voice-1\ntype: voice\nbitrate: 64000\nuser_limit: 10\n";
        let c: Channel = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.kind, ChannelType::Voice);
        assert_eq!(c.voice.as_ref().and_then(|v| v.bitrate), Some(64000));
        assert_eq!(c.voice.as_ref().and_then(|v| v.user_limit), Some(10));
    }

    #[test]
    fn forum_channel_parses() {
        let yaml = "name: help\ntype: forum\navailable_tags: [question, answered]\n";
        let c: Channel = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.kind, ChannelType::Forum);
        assert_eq!(
            c.forum
                .as_ref()
                .map(|f| f.available_tags.clone())
                .unwrap_or_default(),
            vec!["question".to_string(), "answered".to_string()]
        );
    }

    #[test]
    fn channel_type_serde_snake_case() {
        let yaml = "voice";
        let ct: ChannelType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ct, ChannelType::Voice);
        assert_eq!(serde_yaml::to_string(&ct).unwrap().trim(), "voice");
    }

    #[test]
    fn channel_type_helpers() {
        assert!(ChannelType::Text.is_text_like());
        assert!(!ChannelType::Voice.is_text_like());
        assert!(ChannelType::Voice.is_voice_like());
        assert!(ChannelType::Text.supports_topic());
        assert!(!ChannelType::Voice.supports_topic());
        assert_eq!(ChannelType::Text.as_discord_code(), 0);
        assert_eq!(ChannelType::Forum.as_discord_code(), 15);
    }

    #[test]
    fn category_with_inline_channels() {
        let yaml = "\
name: COMPANY
description: Company-wide channels.
channels:
  - name: announcements
    type: text
  - name: general
    type: text
";
        let c: Category = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.name, "COMPANY");
        assert_eq!(c.channels.len(), 2);
        assert_eq!(c.channels[0].name, "announcements");
    }

    #[test]
    fn channel_unknown_field_rejected() {
        let yaml = "name: general\ntype: text\nbogus: true\n";
        let r: Result<Channel, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err());
    }
}
