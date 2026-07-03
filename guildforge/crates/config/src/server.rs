//! Guild-level settings. See [`docs/SCHEMA.md` §3.1](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

/// Guild-level settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    /// Guild name, 2-100 chars.
    pub name: String,

    /// Guild description, max 120 chars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Path to a PNG/JPEG icon file, max 256 KiB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Path to a PNG banner file, max 1 MiB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,

    /// Verification level required to join.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_level: Option<VerificationLevel>,

    /// Explicit content filter level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explicit_content_filter: Option<ExplicitContentFilter>,

    /// Default notification setting for new channels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_notifications: Option<DefaultNotifications>,

    /// Name of a text channel to use as the system channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_channel: Option<String>,

    /// Flags controlling which notifications the system channel receives.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system_channel_flags: Vec<SystemChannelFlag>,

    /// Name of a voice channel to use as the AFK channel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub afk_channel: Option<String>,

    /// AFK timeout in seconds (60, 300, 900, 1800, or 3600).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub afk_timeout: Option<AfkTimeout>,

    /// Whether to display the premium (Nitro) progress bar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium_progress_bar: Option<bool>,
}

/// Verification level required to join the guild.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationLevel {
    /// Unrestricted.
    None,
    /// Must have a verified email.
    Low,
    /// Must be registered for ≥5 minutes.
    Medium,
    /// Must be a member of the guild for ≥10 minutes.
    High,
    /// Must have a verified phone number.
    VeryHigh,
}

/// Explicit content filter level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplicitContentFilter {
    /// Disabled.
    Disabled,
    /// Filter members without roles.
    MembersWithoutRoles,
    /// Filter all members.
    All,
}

/// Default notification setting for new channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultNotifications {
    /// Notify on all messages.
    AllMessages,
    /// Notify only on @mentions.
    OnlyMentions,
}

/// Allowed AFK timeout values (seconds).
///
/// Deserializes from the raw integer (60, 300, 900, 1800, 3600).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfkTimeout {
    /// 60 seconds.
    Seconds60,
    /// 300 seconds (5 minutes).
    Seconds300,
    /// 900 seconds (15 minutes).
    Seconds900,
    /// 1800 seconds (30 minutes).
    Seconds1800,
    /// 3600 seconds (1 hour).
    Seconds3600,
}

impl AfkTimeout {
    /// Convert to seconds.
    #[must_use]
    pub const fn as_seconds(self) -> u64 {
        match self {
            Self::Seconds60 => 60,
            Self::Seconds300 => 300,
            Self::Seconds900 => 900,
            Self::Seconds1800 => 1800,
            Self::Seconds3600 => 3600,
        }
    }

    /// Convert from seconds, returning `None` if the value is not one of
    /// the allowed durations.
    #[must_use]
    pub const fn from_seconds(seconds: u64) -> Option<Self> {
        match seconds {
            60 => Some(Self::Seconds60),
            300 => Some(Self::Seconds300),
            900 => Some(Self::Seconds900),
            1800 => Some(Self::Seconds1800),
            3600 => Some(Self::Seconds3600),
            _ => None,
        }
    }
}

impl serde::Serialize for AfkTimeout {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(self.as_seconds())
    }
}

impl<'de> serde::Deserialize<'de> for AfkTimeout {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let v = u64::deserialize(d)?;
        Self::from_seconds(v).ok_or_else(|| {
            Error::custom(format!(
                "invalid afk_timeout {v}: must be one of 60, 300, 900, 1800, 3600"
            ))
        })
    }
}

/// System channel flag — controls which notifications the system channel
/// receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemChannelFlag {
    /// Suppress member join notifications.
    SuppressJoinNotifications,
    /// Suppress server boost notifications.
    SuppressPremiumSubscriptions,
    /// Suppress guild setup tips.
    SuppressGuildReminderNotifications,
    /// Hide a welcome sticker from the join message reply.
    HideWelcomeScreen,
    /// Suppress member join sticker replies.
    SuppressJoinNotificationReplies,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_server_parses() {
        let yaml = "name: Test\n";
        let s: Server = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(s.name, "Test");
        assert!(s.description.is_none());
    }

    #[test]
    fn full_server_parses() {
        let yaml = "\
name: Test
description: Hello
verification_level: medium
explicit_content_filter: all
default_notifications: only_mentions
system_channel: general
afk_timeout: 300
premium_progress_bar: true
";
        let s: Server = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(s.verification_level, Some(VerificationLevel::Medium));
        assert_eq!(s.afk_timeout, Some(AfkTimeout::Seconds300));
        assert_eq!(s.premium_progress_bar, Some(true));
    }

    #[test]
    fn unknown_field_rejected() {
        let yaml = "name: Test\nbogus: true\n";
        let r: Result<Server, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err());
    }

    #[test]
    fn afk_timeout_seconds_round_trip() {
        assert_eq!(AfkTimeout::Seconds60.as_seconds(), 60);
        assert_eq!(AfkTimeout::Seconds3600.as_seconds(), 3600);
        assert_eq!(AfkTimeout::from_seconds(300), Some(AfkTimeout::Seconds300));
        assert_eq!(AfkTimeout::from_seconds(100), None);
    }

    #[test]
    fn afk_timeout_rejects_invalid() {
        let yaml = "name: Test\nafk_timeout: 100\n";
        let r: Result<Server, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err());
    }
}
