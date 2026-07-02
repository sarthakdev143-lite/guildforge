//! Webhook CRUD operations against Discord.
//!
//! API reference:
//! <https://discord.com/developers/docs/resources/webhook>

use crate::error::DiscordError;
use crate::DiscordProvider;
use guildforge_provider::{ResourceAddr, WebhookResource};
use guildforge_shared::{ResourceId, Snowflake};
use serde::{Deserialize, Serialize};

/// Discord API webhook object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordWebhook {
    /// Webhook ID.
    pub id: String,
    /// Webhook type (1 = incoming, 2 = channel follower).
    #[serde(default)]
    #[serde(rename = "type")]
    pub kind: u8,
    /// Webhook name.
    pub name: String,
    /// Avatar hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Channel ID.
    pub channel_id: String,
    /// Webhook URL (only returned on create).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Guild ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,
    /// Bot user ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_id: Option<String>,
}

/// Payload for `POST /channels/:id/webhooks`.
#[derive(Debug, Serialize)]
struct CreateWebhookPayload<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar: Option<&'a str>,
}

/// Payload for `PATCH /webhooks/:id`.
#[derive(Debug, Serialize)]
struct ModifyWebhookPayload<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar: Option<&'a str>,
}

fn parse_snowflake(s: &str) -> Option<Snowflake> {
    s.parse::<u64>().ok().map(Snowflake::new)
}

fn webhook_to_resource(
    w: DiscordWebhook,
    addr: ResourceAddr,
) -> Result<WebhookResource, DiscordError> {
    Ok(WebhookResource {
        addr,
        id: parse_snowflake(&w.id),
        name: w.name,
        channel_id: parse_snowflake(&w.channel_id).unwrap_or(Snowflake::new(0)),
        url: w.url,
        avatar: w.avatar,
    })
}

/// Parse a webhook address of form `webhook/<channel>/<name>`.
fn parse_webhook_addr(addr: &ResourceAddr) -> Result<String, DiscordError> {
    let s = addr.as_str();
    let rest = s
        .strip_prefix("webhook/")
        .ok_or_else(|| DiscordError::Unsupported(format!("not a webhook address: {s}")))?;
    let name = rest.rsplit('/').next().unwrap_or(rest);
    Ok(name.to_string())
}

/// Read a webhook by address. Lists all webhooks on the parent channel
/// (we don't know the channel ID without an address that includes it).
pub async fn read(
    provider: &DiscordProvider,
    addr: &ResourceAddr,
) -> Result<Option<WebhookResource>, DiscordError> {
    let name = parse_webhook_addr(addr)?;
    let channels: Vec<crate::resources::channel::DiscordChannel> = provider
        .http
        .get(&format!("/guilds/{}/channels", provider.guild_id))
        .await?;
    for ch in channels {
        if ch.kind == 4 {
            continue; // skip categories
        }
        let webhooks: Vec<DiscordWebhook> = provider
            .http
            .get(&format!("/channels/{}/webhooks", ch.id))
            .await?;
        for w in webhooks {
            if w.name.eq_ignore_ascii_case(&name) {
                return Ok(Some(webhook_to_resource(w, addr.clone())?));
            }
        }
    }
    Ok(None)
}

/// Create a webhook on the desired channel.
pub async fn create(
    provider: &DiscordProvider,
    desired: &WebhookResource,
) -> Result<WebhookResource, DiscordError> {
    let payload = CreateWebhookPayload {
        name: &desired.name,
        avatar: desired.avatar.as_deref(),
    };
    let w: DiscordWebhook = provider
        .http
        .post(
            &format!("/channels/{}/webhooks", desired.channel_id),
            &payload,
        )
        .await?;
    webhook_to_resource(w, desired.addr.clone())
}

/// Update a webhook.
pub async fn update(
    provider: &DiscordProvider,
    current: &WebhookResource,
    desired: &WebhookResource,
) -> Result<WebhookResource, DiscordError> {
    if current == desired {
        return Ok(current.clone());
    }
    let id = current
        .id
        .ok_or_else(|| DiscordError::Unsupported("update: webhook has no Discord ID".into()))?;
    let payload = ModifyWebhookPayload {
        name: &desired.name,
        avatar: desired.avatar.as_deref(),
    };
    let w: DiscordWebhook = provider
        .http
        .patch(&format!("/webhooks/{id}"), &payload)
        .await?;
    webhook_to_resource(w, desired.addr.clone())
}

/// Delete a webhook. Idempotent.
pub async fn delete(
    provider: &DiscordProvider,
    current: &WebhookResource,
) -> Result<(), DiscordError> {
    let Some(id) = current.id else {
        return Ok(());
    };
    match provider.http.delete(&format!("/webhooks/{id}")).await {
        Ok(()) => Ok(()),
        Err(DiscordError::Discord { status: 404, .. }) => Ok(()),
        Err(e) => Err(e),
    }
}

/// List all webhooks in the guild.
pub async fn list(provider: &DiscordProvider) -> Result<Vec<WebhookResource>, DiscordError> {
    let channels: Vec<crate::resources::channel::DiscordChannel> = provider
        .http
        .get(&format!("/guilds/{}/channels", provider.guild_id))
        .await?;
    let mut out = vec![];
    for ch in channels {
        if ch.kind == 4 {
            continue;
        }
        let webhooks: Vec<DiscordWebhook> = provider
            .http
            .get(&format!("/channels/{}/webhooks", ch.id))
            .await?;
        for w in webhooks {
            out.push(webhook_to_resource(w, ResourceId::new("webhook/_unknown"))?);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_webhook_addr_works() {
        let addr = ResourceId::new("webhook/announcements/CI Notifier");
        assert_eq!(parse_webhook_addr(&addr).unwrap(), "CI Notifier");
    }

    #[test]
    fn webhook_to_resource_decodes() {
        let w = DiscordWebhook {
            id: "1".into(),
            kind: 1,
            name: "CI".into(),
            avatar: Some("hash".into()),
            channel_id: "2".into(),
            url: Some("https://discord.com/api/webhooks/1/token".into()),
            guild_id: None,
            application_id: None,
        };
        let r = webhook_to_resource(w, ResourceId::new("webhook/c/CI")).unwrap();
        assert_eq!(r.id, Some(Snowflake::new(1)));
        assert_eq!(r.channel_id, Snowflake::new(2));
        assert!(r.url.is_some());
    }

    #[test]
    fn create_payload_omits_none_avatar() {
        let p = CreateWebhookPayload {
            name: "x",
            avatar: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("avatar"));
    }
}
