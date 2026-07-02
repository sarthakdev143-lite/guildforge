//! Explicit ordering overrides. See
//! [`docs/SCHEMA.md` §3.13](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

/// Explicit position overrides. By default `GuildForge` orders roles,
/// categories, and channels by their position in the YAML file. Use
/// `ordering` to override.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ordering {
    /// Role ordering (top-to-bottom = highest to lowest position).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,

    /// Category ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,

    /// Per-category channel ordering (and `_top_level` for top-level
    /// channels).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<std::collections::BTreeMap<String, Vec<String>>>,
}

/// Reserved key in `ordering.channels` for top-level channels.
pub const TOP_LEVEL_KEY: &str = "_top_level";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_ordering() {
        let yaml = "roles: [Admin, Member]\n";
        let o: Ordering = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            o.roles.as_deref(),
            Some(["Admin".to_string(), "Member".to_string()].as_slice())
        );
    }

    #[test]
    fn full_ordering() {
        let yaml = "\
roles: [Admin, Staff, Member]
categories: [COMPANY, SOCIAL]
channels:
  COMPANY: [announcements, general]
  _top_level: [welcome]
";
        let o: Ordering = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(o.categories.as_deref().unwrap().len(), 2);
        assert_eq!(o.channels.as_ref().unwrap().len(), 2);
        assert!(o.channels.as_ref().unwrap().contains_key(TOP_LEVEL_KEY));
    }
}
