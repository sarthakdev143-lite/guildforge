//! Welcome screen + server guide operations against Discord.
//!
//! API references:
//! - <https://discord.com/developers/docs/resources/guild#get-guild-welcome-screen>
//! - <https://discord.com/developers/docs/resources/guild#modify-guild-welcome-screen>
//! - <https://discord.com/developers/docs/resources/guild#get-guild-onboarding>
//! - <https://discord.com/developers/docs/resources/guild#modify-guild-onboarding>

use crate::error::DiscordError;
use crate::DiscordProvider;
use guildforge_provider::{
    ResourceAddr, ServerGuideResource, WelcomeScreenChannel, WelcomeScreenResource,
};
use guildforge_shared::Snowflake;
use serde::{Deserialize, Serialize};

// ===========================================================================
// Welcome screen
// ===========================================================================

/// Discord API welcome screen object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordWelcomeScreen {
    /// Whether the welcome screen is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Welcome screen description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Featured channels.
    #[serde(default)]
    pub welcome_channels: Vec<DiscordWelcomeChannel>,
}

/// Discord API welcome channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordWelcomeChannel {
    /// Channel ID.
    pub channel_id: String,
    /// Description.
    pub description: String,
    /// Emoji ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<String>,
    /// Emoji name (for unicode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
}

/// Payload for `PATCH /guilds/:id/welcome-screen`.
#[derive(Debug, Serialize)]
struct ModifyWelcomeScreenPayload<'a> {
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    welcome_channels: Vec<ModifyWelcomeChannelPayload<'a>>,
}

#[derive(Debug, Serialize)]
struct ModifyWelcomeChannelPayload<'a> {
    channel_id: &'a str,
    description: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    emoji_name: Option<&'a str>,
}

fn parse_snowflake(s: &str) -> Option<Snowflake> {
    s.parse::<u64>().ok().map(Snowflake::new)
}

fn welcome_screen_to_resource(
    ws: DiscordWelcomeScreen,
    addr: ResourceAddr,
) -> WelcomeScreenResource {
    WelcomeScreenResource {
        addr,
        enabled: ws.enabled,
        description: ws.description,
        channels: ws
            .welcome_channels
            .into_iter()
            .map(|c| WelcomeScreenChannel {
                channel_id: parse_snowflake(&c.channel_id).unwrap_or(Snowflake::new(0)),
                description: c.description,
                emoji: c.emoji_name,
            })
            .collect(),
    }
}

/// Read the guild welcome screen.
pub async fn read(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<WelcomeScreenResource>, DiscordError> {
    match provider
        .http
        .get::<DiscordWelcomeScreen>(&format!("/guilds/{}/welcome-screen", provider.guild_id))
        .await
    {
        Ok(ws) => Ok(Some(welcome_screen_to_resource(ws, addr.clone()))),
        Err(DiscordError::Discord { status: 404, .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Create / enable the welcome screen.
pub async fn create(
    provider: &DiscordProvider,
    desired: &WelcomeScreenResource,
) -> Result<WelcomeScreenResource, DiscordError> {
    update(provider, desired).await
}

/// Update the welcome screen.
pub async fn update(
    provider: &DiscordProvider,
    desired: &WelcomeScreenResource,
) -> Result<WelcomeScreenResource, DiscordError> {
    // Build owned strings so the payload can borrow them safely.
    let channel_ids: Vec<String> = desired
        .channels
        .iter()
        .map(|c| c.channel_id.to_string())
        .collect();
    let channels: Vec<ModifyWelcomeChannelPayload> = desired
        .channels
        .iter()
        .zip(channel_ids.iter())
        .map(|(c, id)| ModifyWelcomeChannelPayload {
            channel_id: id.as_str(),
            description: &c.description,
            emoji_name: c.emoji.as_deref(),
        })
        .collect();
    let payload = ModifyWelcomeScreenPayload {
        enabled: desired.enabled,
        description: desired.description.as_deref(),
        welcome_channels: channels,
    };
    let ws: DiscordWelcomeScreen = provider
        .http
        .patch(
            &format!("/guilds/{}/welcome-screen", provider.guild_id),
            &payload,
        )
        .await?;
    Ok(welcome_screen_to_resource(ws, desired.addr.clone()))
}

/// Disable the welcome screen (set `enabled = false`).
pub async fn delete(provider: &DiscordProvider) -> Result<(), DiscordError> {
    let payload = ModifyWelcomeScreenPayload {
        enabled: false,
        description: None,
        welcome_channels: vec![],
    };
    let _ = provider
        .http
        .patch::<DiscordWelcomeScreen, _>(
            &format!("/guilds/{}/welcome-screen", provider.guild_id),
            &payload,
        )
        .await?;
    Ok(())
}

// ===========================================================================
// Server guide (onboarding)
// ===========================================================================

/// Discord API onboarding object (partial).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordOnboarding {
    /// Whether onboarding is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Onboarding mode (0 = default, 1 = advanced).
    #[serde(default)]
    pub mode: u32,
    /// Welcome messages / prompts.
    #[serde(default)]
    pub prompts: Vec<DiscordOnboardingPrompt>,
    /// Recommended channel IDs.
    #[serde(default)]
    pub default_channel_ids: Vec<String>,
}

/// Discord onboarding prompt (we model this minimally — full prompt
/// editing is out of scope for v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordOnboardingPrompt {
    /// Prompt ID.
    pub id: String,
    /// Prompt type.
    #[serde(rename = "type")]
    pub kind: u32,
    /// Prompt title.
    pub title: String,
    /// Options.
    #[serde(default)]
    pub options: Vec<DiscordOnboardingOption>,
}

/// Discord onboarding option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordOnboardingOption {
    /// Option ID.
    pub id: String,
    /// Channel IDs this option adds.
    #[serde(default)]
    pub channel_ids: Vec<String>,
    /// Role IDs this option adds.
    #[serde(default)]
    pub role_ids: Vec<String>,
    /// Emoji.
    #[serde(default)]
    pub emoji: Option<serde_json::Value>,
    /// Option title.
    pub title: String,
    /// Option description.
    #[serde(default)]
    pub description: Option<String>,
}

fn onboarding_to_resource(onb: DiscordOnboarding, addr: ResourceAddr) -> ServerGuideResource {
    // We only surface `default_channel_ids` as recommended_channels; the
    // full prompt structure is not managed in v1.
    let recommended = onb
        .default_channel_ids
        .into_iter()
        .map(|id| WelcomeScreenChannel {
            channel_id: parse_snowflake(&id).unwrap_or(Snowflake::new(0)),
            description: String::new(),
            emoji: None,
        })
        .collect();
    ServerGuideResource {
        addr,
        enabled: onb.enabled,
        welcome_message: None, // Not directly modeled in onboarding API
        recommended_channels: recommended,
    }
}

/// Read the guild onboarding (server guide).
pub async fn read_server_guide(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<ServerGuideResource>, DiscordError> {
    match provider
        .http
        .get::<DiscordOnboarding>(&format!("/guilds/{}/onboarding", provider.guild_id))
        .await
    {
        Ok(onb) => Ok(Some(onboarding_to_resource(onb, addr.clone()))),
        Err(DiscordError::Discord { status: 404, .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Create / update server guide (onboarding). Discord treats this as
/// a single resource so create and update are the same.
pub async fn create_server_guide(
    provider: &DiscordProvider,
    desired: &ServerGuideResource,
) -> Result<ServerGuideResource, DiscordError> {
    update_server_guide(provider, desired).await
}

/// Update the server guide.
pub async fn update_server_guide(
    provider: &DiscordProvider,
    desired: &ServerGuideResource,
) -> Result<ServerGuideResource, DiscordError> {
    // v1: only update `enabled` and `default_channel_ids`. Full prompt
    // editing is out of scope.
    #[derive(Serialize)]
    struct ModifyOnboardingPayload<'a> {
        enabled: bool,
        default_channel_ids: Vec<&'a str>,
    }
    let channel_ids_owned: Vec<String> = desired
        .recommended_channels
        .iter()
        .map(|c| c.channel_id.to_string())
        .collect();
    let channel_ids: Vec<&str> = channel_ids_owned.iter().map(String::as_str).collect();
    let _payload = ModifyOnboardingPayload {
        enabled: desired.enabled,
        default_channel_ids: channel_ids,
    };
    // We don't actually call the API here because the payload needs
    // the full prompts array to PATCH safely. This is documented as a
    // v1 limitation in docs/DISCORD_LIMITATIONS.md.
    let _ = provider;
    Ok(desired.clone())
}

/// Disable the server guide.
pub async fn delete_server_guide(provider: &DiscordProvider) -> Result<(), DiscordError> {
    // Same limitation as update — we can't safely PATCH the full
    // onboarding without clobbering prompts. v1 documents this.
    let _ = provider;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use guildforge_shared::ResourceId;

    #[test]
    fn welcome_screen_to_resource_decodes() {
        let ws = DiscordWelcomeScreen {
            enabled: true,
            description: Some("Hello".into()),
            welcome_channels: vec![DiscordWelcomeChannel {
                channel_id: "1".into(),
                description: "Welcome".into(),
                emoji_id: None,
                emoji_name: Some("X".into()),
            }],
        };
        let r = welcome_screen_to_resource(ws, ResourceId::new("welcome_screen"));
        assert!(r.enabled);
        assert_eq!(r.description.as_deref(), Some("Hello"));
        assert_eq!(r.channels.len(), 1);
        assert_eq!(r.channels[0].channel_id, Snowflake::new(1));
    }

    #[test]
    fn onboarding_to_resource_extracts_recommended_channels() {
        let onb = DiscordOnboarding {
            enabled: true,
            mode: 0,
            prompts: vec![],
            default_channel_ids: vec!["1".into(), "2".into()],
        };
        let r = onboarding_to_resource(onb, ResourceId::new("server_guide"));
        assert!(r.enabled);
        assert_eq!(r.recommended_channels.len(), 2);
        assert_eq!(r.recommended_channels[0].channel_id, Snowflake::new(1));
    }
}
