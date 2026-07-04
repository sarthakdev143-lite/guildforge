//! Forum tags. See [`docs/SCHEMA.md` §3.10](../../docs/SCHEMA.md).

use serde::{Deserialize, Serialize};

/// A single forum tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForumTag {
    /// Tag name, 1-20 chars, unique within channel (case-insensitive).
    pub name: String,

    /// Whether posts with this tag require moderation approval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderated: Option<bool>,

    /// Optional unicode emoji (custom emoji not supported in v1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
}

/// Mapping of forum channel name → list of tags.
pub type ForumTagMap = std::collections::BTreeMap<String, Vec<ForumTag>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_tag_parses() {
        let yaml = "name: Question\n";
        let t: ForumTag = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(t.name, "Question");
        assert!(t.moderated.is_none());
    }

    #[test]
    fn full_tag_parses() {
        let yaml = "name: Answered\nmoderated: true\nemoji: \"x\"\n";
        let t: ForumTag = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(t.moderated, Some(true));
        assert_eq!(t.emoji.as_deref(), Some("x"));
    }

    #[test]
    fn tag_map_parses() {
        let yaml = "\
help:
  - name: Question
    emoji: \"q\"
  - name: Answered
    moderated: true
";
        let m: ForumTagMap = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m["help"].len(), 2);
        assert_eq!(m["help"][0].name, "Question");
    }
}
