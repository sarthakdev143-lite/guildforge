//! YAML parser for `GuildForge` config files.
//!
//! Reads a YAML string and produces a strongly-typed
//! [`Config`](guildforge_config::Config). The parser does **no** semantic
//! validation — that lives in [`guildforge_validation`].
//!
//! # Errors
//!
//! Parse errors are categorized:
//!
//! - [`ParseError::InvalidYaml`] — YAML syntax error or schema mismatch
//!   (unknown field, wrong type).
//! - [`ParseError::Io`] — file I/O error.
//! - [`ParseError::UnsupportedSchemaVersion`] — `_schema_version` is
//!   present and greater than 1.
//! - [`ParseError::Empty`] — the input is empty or whitespace-only.
//! - [`ParseError::TooLarge`] — the input exceeds the max size limit.
//!
//! See [`ADR-0005`](../../docs/adr/ADR-0005-error-model.md) for the
//! error model.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use guildforge_config::Config;
use std::path::Path;
use thiserror::Error;

/// Default max config size: 10 MiB.
pub const DEFAULT_MAX_SIZE: usize = 10 * 1024 * 1024;

/// A parse error.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The YAML is syntactically invalid or fails to deserialize into
    /// the [`Config`] schema (unknown field, wrong type).
    #[error("invalid YAML: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),

    /// I/O error reading the file.
    #[error("could not read file: {0}")]
    Io(#[from] std::io::Error),

    /// The schema version is unsupported.
    #[error("unsupported schema version: {version} (this build supports up to {supported})")]
    UnsupportedSchemaVersion {
        /// The version found in the file.
        version: u32,
        /// The maximum version this build supports.
        supported: u32,
    },

    /// The input is empty or whitespace-only.
    #[error("config is empty")]
    Empty,

    /// The input exceeds the max size limit.
    #[error("config is too large: {actual} bytes (max {max} bytes)")]
    TooLarge {
        /// Actual size in bytes.
        actual: usize,
        /// Max allowed size in bytes.
        max: usize,
    },
}

/// Parse a YAML string into a [`Config`].
///
/// Performs syntax + schema validation only. Semantic validation lives
/// in [`guildforge_validation`].
///
/// # Errors
///
/// Returns [`ParseError::Empty`] if the input is empty,
/// [`ParseError::TooLarge`] if the input exceeds `DEFAULT_MAX_SIZE`,
/// [`ParseError::UnsupportedSchemaVersion`] if `_schema_version` is
/// present and > 1, or [`ParseError::InvalidYaml`] for any other syntax
/// or schema error.
pub fn parse(text: &str) -> Result<Config, ParseError> {
    parse_with_limit(text, DEFAULT_MAX_SIZE)
}

/// Parse a YAML string with a custom max size limit.
///
/// # Errors
///
/// See [`parse`].
pub fn parse_with_limit(text: &str, max_size: usize) -> Result<Config, ParseError> {
    if text.trim().is_empty() {
        return Err(ParseError::Empty);
    }
    if text.len() > max_size {
        return Err(ParseError::TooLarge {
            actual: text.len(),
            max: max_size,
        });
    }
    let cfg: Config = serde_yaml::from_str(text)?;
    if let Some(v) = cfg.schema_version {
        if v > guildforge_config::SCHEMA_VERSION {
            return Err(ParseError::UnsupportedSchemaVersion {
                version: v,
                supported: guildforge_config::SCHEMA_VERSION,
            });
        }
    }
    Ok(cfg)
}

/// Parse a YAML file from disk.
///
/// # Errors
///
/// Returns [`ParseError::Io`] if the file cannot be read, or other
/// variants per [`parse`].
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
        let err = parse(yaml).unwrap_err();
        assert!(matches!(
            err,
            ParseError::UnsupportedSchemaVersion {
                version: 2,
                supported: 1
            }
        ));
    }

    #[test]
    fn accepts_current_schema_version() {
        let yaml = "_schema_version: 1\nserver:\n  name: Test\n";
        let cfg = parse(yaml).expect("parse");
        assert_eq!(cfg.schema_version, Some(1));
    }

    #[test]
    fn rejects_empty_input() {
        assert!(matches!(parse(""), Err(ParseError::Empty)));
        assert!(matches!(parse("   \n\n"), Err(ParseError::Empty)));
    }

    #[test]
    fn rejects_too_large_input() {
        let huge = "x".repeat(DEFAULT_MAX_SIZE + 1);
        assert!(matches!(parse(&huge), Err(ParseError::TooLarge { .. })));
    }

    #[test]
    fn parses_full_company_example() {
        let yaml = std::fs::read_to_string("../../examples/company.yaml")
            .or_else(|_| std::fs::read_to_string("examples/company.yaml"))
            .expect("read example");
        let cfg = parse(&yaml).expect("parse company.yaml");
        assert_eq!(cfg.server.name, "Augment Infotech");
        assert!(!cfg.roles.is_empty());
        assert!(!cfg.categories.is_empty());
    }

    #[test]
    fn parses_community_example() {
        let yaml = std::fs::read_to_string("../../examples/community.yaml")
            .or_else(|_| std::fs::read_to_string("examples/community.yaml"))
            .expect("read example");
        let cfg = parse(&yaml).expect("parse community.yaml");
        assert_eq!(cfg.server.name, "OpenWidget Community");
    }

    #[test]
    fn parse_file_works() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/company.yaml");
        let cfg = parse_file(&path).expect("parse file");
        assert_eq!(cfg.server.name, "Augment Infotech");
    }
}
