//! Invites. See [`docs/SCHEMA.md` §3.9](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

/// An invite declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invite {
    /// Target channel name.
    pub channel: String,

    /// Max age in seconds (0 = never, max 604800 = 7 days).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u64>,

    /// Max uses (0 = unlimited, max 100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u32>,

    /// Whether the invite grants temporary membership.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,

    /// Whether to guarantee a unique invite code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_invite_parses() {
        let yaml = "channel: announcements\n";
        let i: Invite = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(i.channel, "announcements");
        assert!(i.max_age.is_none());
    }

    #[test]
    fn full_invite_parses() {
        let yaml = "\
channel: announcements
max_age: 86400
max_uses: 10
temporary: false
unique: true
";
        let i: Invite = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(i.max_age, Some(86400));
        assert_eq!(i.max_uses, Some(10));
        assert_eq!(i.temporary, Some(false));
        assert_eq!(i.unique, Some(true));
    }
}
