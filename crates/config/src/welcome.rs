//! Welcome screen and server guide. See
//! [`docs/SCHEMA.md` §3.11, §3.12](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

/// Server-wide welcome screen. Requires Community server feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WelcomeScreen {
    /// Whether the welcome screen is enabled.
    pub enabled: bool,

    /// Welcome screen description (max 140 chars).
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

    /// Description shown on the welcome screen (max 90 chars).
    pub description: String,
}

/// Server guide / onboarding. Limited by Discord API; see
/// [`docs/SCHEMA.md` §3.12](../../docs/SCHEMA.md) for known limitations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerGuide {
    /// Whether the server guide is enabled.
    pub enabled: bool,

    /// Welcome message shown to new members (max 1000 chars).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub welcome_message: Option<String>,

    /// Recommended channels (max 7).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_channels: Vec<WelcomeScreenChannel>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_welcome_screen() {
        let yaml = "enabled: true\n";
        let w: WelcomeScreen = serde_yaml::from_str(yaml).unwrap();
        assert!(w.enabled);
        assert!(w.channels.is_empty());
    }

    #[test]
    fn full_welcome_screen() {
        let yaml = "\
enabled: true
description: Welcome!
channels:
  - channel: general
    description: Say hi
";
        let w: WelcomeScreen = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(w.description.as_deref(), Some("Welcome!"));
        assert_eq!(w.channels.len(), 1);
        assert_eq!(w.channels[0].channel, "general");
    }

    #[test]
    fn full_server_guide() {
        let yaml = "\
enabled: true
welcome_message: Welcome
recommended_channels:
  - channel: rules
    description: Read this first
";
        let g: ServerGuide = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(g.welcome_message.as_deref(), Some("Welcome"));
        assert_eq!(g.recommended_channels.len(), 1);
    }
}
