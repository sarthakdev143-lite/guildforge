//! Semantic validation for `GuildForge` config files.
//!
//! Runs a battery of checks on a parsed [`Config`](guildforge_config::Config)
//! and returns all diagnostics found in one pass. Every check has a
//! stable code (`V001`, `V002`, ...) that is part of the public API.
//! See [`docs/SCHEMA.md` §5](../../docs/SCHEMA.md) for the full list.
//!
//! # Rules
//!
//! - Pure function. No I/O, no async.
//! - Returns ALL errors, not just the first. Users see every problem
//!   in one pass.
//! - Every diagnostic has a stable code. Codes never get renumbered.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 1
//! (task `P1-005`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use guildforge_config::Config;
use miette::SourceSpan;

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Hard error; config cannot be applied.
    Error,
    /// Soft warning; config can be applied but may produce unexpected
    /// results.
    Warning,
}

/// A single validation diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Stable code (e.g. `V001`).
    pub code: &'static str,
    /// Severity.
    pub severity: Severity,
    /// Human-readable message (lowercase, no trailing period).
    pub message: String,
    /// Source span in the original YAML file.
    pub span: Option<SourceSpan>,
    /// Optional help text suggesting a fix.
    pub help: Option<String>,
}

/// Validate a parsed config and return all diagnostics.
///
/// Returns `Ok(())` if there are no errors. Warnings are returned
/// via the `warnings` field of the result; this function does not
/// fail on warnings.
///
/// # Errors
///
/// Returns `Err(Vec<Diagnostic>)` if any error-severity diagnostics
/// are found.
pub fn validate(_config: &Config) -> Result<(), Vec<Diagnostic>> {
    // Phase 0 stub: real implementation lands in task P1-005.
    // Will implement rules V001-V075 per docs/SCHEMA.md §5.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_returns_ok() {
        let cfg = guildforge_config::Config {
            schema_version: None,
            server: guildforge_config::Server {
                name: "Test".to_string(),
                description: None,
            },
            roles: vec![],
            categories: vec![],
            channels: vec![],
            permissions: std::collections::BTreeMap::new(),
            permission_overwrites: vec![],
            webhooks: vec![],
            invites: vec![],
            forum_tags: std::collections::BTreeMap::new(),
            welcome_screen: None,
            server_guide: None,
            ordering: None,
        };
        assert!(validate(&cfg).is_ok());
    }
}
