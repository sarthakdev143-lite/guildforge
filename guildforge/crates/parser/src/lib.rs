//! YAML parser for `GuildForge` config files.
//!
//! Reads a YAML string and produces a strongly-typed
//! [`Config`](guildforge_config::Config). The parser does **no** semantic
//! validation — that lives in [`guildforge_validation`]. Spans are
//! preserved so that downstream stages can emit
//! [`miette`](https://docs.rs/miette) diagnostics pointing to the exact
//! line and column.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 1
//! (task `P1-004`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use guildforge_config::Config;
use std::path::Path;
use thiserror::Error;

/// A parse error.
///
/// Phase 0: minimal variant. Phase 1 will add `miette` spans for
/// beautiful diagnostics; see
/// [`ADR-0005`](../../docs/adr/ADR-0005-error-model.md).
#[derive(Debug, Error)]
pub enum ParseError {
    /// The YAML is syntactically invalid.
    #[error("invalid YAML: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),

    /// I/O error reading the file.
    #[error("could not read file: {0}")]
    Io(#[from] std::io::Error),

    /// The schema version is unsupported.
    #[error("unsupported schema version: {version}")]
    UnsupportedSchemaVersion {
        /// The version found in the file.
        version: u32,
    },
}

/// Parse a YAML string into a [`Config`].
///
/// # Errors
///
/// Returns [`ParseError::InvalidYaml`] if the YAML is syntactically
/// invalid or fails to deserialize into the [`Config`] schema (e.g.
/// unknown field, wrong type).
pub fn parse(text: &str) -> Result<Config, ParseError> {
    let cfg: Config = serde_yaml::from_str(text)?;
    if let Some(v) = cfg.schema_version {
        if v > 1 {
            return Err(ParseError::UnsupportedSchemaVersion { version: v });
        }
    }
    Ok(cfg)
}

/// Parse a YAML file from disk.
///
/// # Errors
///
/// Returns [`ParseError::Io`] if the file cannot be read, or
/// [`ParseError::InvalidYaml`] if parsing fails.
pub fn parse_file(path: &Path) -> Result<Config, ParseError> {
    let text = std::fs::read_to_string(path)?;
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let yaml = "server:\n  name: Test\n";
        let cfg = parse(yaml).expect("parse");
        assert_eq!(cfg.server.name, "Test");
    }

    #[test]
    fn rejects_unknown_field() {
        let yaml = "server:\n  name: Test\n  bogus: true\n";
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let yaml = "_schema_version: 2\nserver:\n  name: Test\n";
        assert!(matches!(
            parse(yaml),
            Err(ParseError::UnsupportedSchemaVersion { .. })
        ));
    }
}
