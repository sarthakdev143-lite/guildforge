//! Structural diff between two config files.
//!
//! Compares two [`Config`]s and reports what changed at the YAML
//! level (not at the resource level — that's the planner's job).

use guildforge_config::Config;
use std::collections::BTreeSet;

/// A single diff entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    /// Resource address (e.g. `role/Admin`).
    pub addr: String,
    /// Change type: `+` (added), `-` (removed), `~` (changed).
    pub change: char,
    /// Human-readable description.
    pub description: String,
}

/// A diff report between two configs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffReport {
    /// All diff entries, sorted by address.
    pub entries: Vec<DiffEntry>,
}

impl std::fmt::Display for DiffReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.entries.is_empty() {
            return f.write_str("No differences.\n");
        }
        for e in &self.entries {
            writeln!(f, "{} {} — {}", e.change, e.addr, e.description)?;
        }
        Ok(())
    }
}

/// Compute a structural diff between two configs.
#[must_use]
pub fn diff_configs(a: &Config, b: &Config) -> DiffReport {
    let mut entries = Vec::new();

    // Compare roles.
    let a_roles: BTreeSet<String> = a.roles.iter().map(|r| r.name.clone()).collect();
    let b_roles: BTreeSet<String> = b.roles.iter().map(|r| r.name.clone()).collect();
    for name in a_roles.intersection(&b_roles) {
        let ra = a.roles.iter().find(|r| &r.name == name);
        let rb = b.roles.iter().find(|r| &r.name == name);
        if ra != rb {
            entries.push(DiffEntry {
                addr: format!("role/{name}"),
                change: '~',
                description: "role changed".to_string(),
            });
        }
    }
    for name in a_roles.difference(&b_roles) {
        entries.push(DiffEntry {
            addr: format!("role/{name}"),
            change: '-',
            description: "role removed".to_string(),
        });
    }
    for name in b_roles.difference(&a_roles) {
        entries.push(DiffEntry {
            addr: format!("role/{name}"),
            change: '+',
            description: "role added".to_string(),
        });
    }

    // Compare categories.
    let a_cats: BTreeSet<String> = a.categories.iter().map(|c| c.name.clone()).collect();
    let b_cats: BTreeSet<String> = b.categories.iter().map(|c| c.name.clone()).collect();
    for name in b_roles.difference(&a_roles) {
        let _ = name; // suppress unused
    }
    for name in a_cats.difference(&b_cats) {
        entries.push(DiffEntry {
            addr: format!("category/{name}"),
            change: '-',
            description: "category removed".to_string(),
        });
    }
    for name in b_cats.difference(&a_cats) {
        entries.push(DiffEntry {
            addr: format!("category/{name}"),
            change: '+',
            description: "category added".to_string(),
        });
    }

    // Compare channels (top-level + nested).
    let a_chans: BTreeSet<String> = a.all_channels().iter().map(|c| c.name.clone()).collect();
    let b_chans: BTreeSet<String> = b.all_channels().iter().map(|c| c.name.clone()).collect();
    for name in a_chans.difference(&b_chans) {
        entries.push(DiffEntry {
            addr: format!("channel/{name}"),
            change: '-',
            description: "channel removed".to_string(),
        });
    }
    for name in b_chans.difference(&a_chans) {
        entries.push(DiffEntry {
            addr: format!("channel/{name}"),
            change: '+',
            description: "channel added".to_string(),
        });
    }

    // Compare server settings.
    if a.server.name != b.server.name {
        entries.push(DiffEntry {
            addr: "server/name".to_string(),
            change: '~',
            description: format!("name: {} → {}", a.server.name, b.server.name),
        });
    }

    entries.sort_by(|a, b| a.addr.cmp(&b.addr));
    DiffReport { entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Config {
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn identical_configs_have_no_diff() {
        let a = parse("server:\n  name: Test\n");
        let b = parse("server:\n  name: Test\n");
        let d = diff_configs(&a, &b);
        assert!(d.entries.is_empty());
    }

    #[test]
    fn added_role_shows_plus() {
        let a = parse("server:\n  name: Test\n");
        let b = parse("server:\n  name: Test\nroles:\n  - name: Admin\n");
        let d = diff_configs(&a, &b);
        assert!(d
            .entries
            .iter()
            .any(|e| e.change == '+' && e.addr == "role/Admin"));
    }

    #[test]
    fn removed_role_shows_minus() {
        let a = parse("server:\n  name: Test\nroles:\n  - name: Admin\n");
        let b = parse("server:\n  name: Test\n");
        let d = diff_configs(&a, &b);
        assert!(d
            .entries
            .iter()
            .any(|e| e.change == '-' && e.addr == "role/Admin"));
    }

    #[test]
    fn changed_server_name_shows_tilde() {
        let a = parse("server:\n  name: OldName\n");
        let b = parse("server:\n  name: NewName\n");
        let d = diff_configs(&a, &b);
        assert!(d
            .entries
            .iter()
            .any(|e| e.change == '~' && e.addr == "server/name"));
    }

    #[test]
    fn display_empty() {
        let d = DiffReport::default();
        assert_eq!(format!("{d}"), "No differences.\n");
    }

    #[test]
    fn display_with_entries() {
        let d = DiffReport {
            entries: vec![DiffEntry {
                addr: "role/Admin".into(),
                change: '+',
                description: "role added".into(),
            }],
        };
        let s = format!("{d}");
        assert!(s.contains("+ role/Admin"));
    }
}
