//! Webhooks. See [`docs/SCHEMA.md` §3.8](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

/// A webhook declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Webhook {
    /// Webhook name, 1-80 chars.
    pub name: String,

    /// Target channel name (text or forum channel).
    pub channel: String,

    /// Optional avatar (path or URL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_webhook_parses() {
        let yaml = "name: CI Notifier\nchannel: deployments\n";
        let w: Webhook = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(w.name, "CI Notifier");
        assert_eq!(w.channel, "deployments");
        assert!(w.avatar.is_none());
    }

    #[test]
    fn webhook_with_avatar() {
        let yaml = "name: CI Notifier\nchannel: deployments\navatar: ./assets/ci.png\n";
        let w: Webhook = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(w.avatar.as_deref(), Some("./assets/ci.png"));
    }

    #[test]
    fn webhook_unknown_field_rejected() {
        let yaml = "name: CI\nchannel: c\nbogus: true\n";
        let r: Result<Webhook, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err());
    }
}
