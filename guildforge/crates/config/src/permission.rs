//! Permission overwrites. See
//! [`docs/SCHEMA.md` §3.5, §3.6, §3.7](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

/// Shorthand permission block applied to a single channel or category.
///
/// The validator expands shorthand into `PermissionOverwrite` records
/// before planning.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionShorthand {
    /// Roles that can read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read: Vec<String>,

    /// Roles that can write.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub write: Vec<String>,

    /// Roles that can manage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub manage: Vec<String>,

    /// Voice-only: roles that can connect.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connect: Vec<String>,

    /// Voice-only: roles that can speak.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speak: Vec<String>,

    /// Rare: roles that can view the audit log.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub view_audit_log: Vec<String>,
}

impl PermissionShorthand {
    /// Returns `true` if no permissions are set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.read.is_empty()
            && self.write.is_empty()
            && self.manage.is_empty()
            && self.connect.is_empty()
            && self.speak.is_empty()
            && self.view_audit_log.is_empty()
    }
}

/// Alias for [`PermissionShorthand`] kept for clarity at use sites.
pub type PermissionBlock = PermissionShorthand;

/// Mapping of channel name → shorthand permission block.
pub type PermissionMap = std::collections::BTreeMap<String, PermissionBlock>;

/// Full-form permission overwrite.
///
/// Use this when shorthand is insufficient (e.g. deny rules,
/// role-vs-member distinction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionOverwrite {
    /// Target channel or category name.
    pub channel: String,

    /// Whether the target is a role or a member.
    #[serde(rename = "type")]
    pub kind: OverwriteKind,

    /// Role name or member ID. `everyone` is shorthand for the
    /// `@everyone` role.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_shorthand_is_empty() {
        let s = PermissionShorthand::default();
        assert!(s.is_empty());
    }

    #[test]
    fn populated_shorthand_not_empty() {
        let s = PermissionShorthand {
            read: vec!["everyone".to_string()],
            ..Default::default()
        };
        assert!(!s.is_empty());
    }

    #[test]
    fn full_permission_overwrite_parses() {
        let yaml = "\
channel: announcements
type: role
target: Admin
allow: [send_messages, manage_messages]
deny: [create_public_threads]
";
        let o: PermissionOverwrite = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(o.channel, "announcements");
        assert_eq!(o.kind, OverwriteKind::Role);
        assert_eq!(o.target, "Admin");
        assert_eq!(
            o.allow,
            vec!["send_messages".to_string(), "manage_messages".to_string()]
        );
        assert_eq!(o.deny, vec!["create_public_threads".to_string()]);
    }

    #[test]
    fn member_overwrite_parses() {
        let yaml = "channel: announcements\ntype: member\ntarget: \"12345\"\nallow: []\n";
        let o: PermissionOverwrite = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(o.kind, OverwriteKind::Member);
    }

    #[test]
    fn shorthand_block_with_voice_fields() {
        let yaml = "\
read: [everyone]
write: [Staff]
connect: [everyone]
speak: [Staff]
";
        let s: PermissionShorthand = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(s.read, vec!["everyone".to_string()]);
        assert_eq!(s.connect, vec!["everyone".to_string()]);
        assert_eq!(s.speak, vec!["Staff".to_string()]);
    }
}
