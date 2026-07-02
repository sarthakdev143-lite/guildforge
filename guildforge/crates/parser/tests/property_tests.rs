//! Property-based tests for the parser.
//!
//! These tests use `proptest` to verify that:
//! - Random YAML strings never cause panics (fuzz-like).
//! - The parser is total: every input produces either a `Config` or an
//!   `Err`, never a panic.

#![cfg(test)]

use guildforge_parser::{parse, ParseError};
use proptest::prelude::*;

proptest! {
    /// Property: parsing arbitrary bytes never panics.
    #[test]
    fn prop_never_panics_on_arbitrary_bytes(input in ".{0,200}") {
        let _ = parse(&input);
    }

    /// Property: parsing arbitrary Unicode never panics.
    #[test]
    fn prop_never_panics_on_unicode(input in "[\\x00-\\x7F\\u{00A0}-\\u{FFFF}]{0,100}") {
        let _ = parse(&input);
    }

    /// Property: empty/whitespace input always returns ParseError::Empty.
    #[test]
    fn prop_empty_input_returns_empty_error(
        whitespace in "[ \\t\\n\\r]{0,50}"
    ) {
        let result = parse(&whitespace);
        prop_assert!(matches!(result, Err(ParseError::Empty)), "expected Empty, got {:?}", result);
    }

    /// Property: minimal valid config (`server: { name: <str> }`)
    /// parses successfully for any non-empty name (no trailing whitespace
    /// because YAML strips it from unquoted scalars).
    #[test]
    fn prop_minimal_config_parses(
        name in "[a-zA-Z][a-zA-Z0-9]{0,98}"
    ) {
        let yaml = format!("server:\n  name: {name}\n");
        let result = parse(&yaml);
        prop_assert!(result.is_ok(), "expected ok, got {:?}", result.err());
        let cfg = result.unwrap();
        prop_assert_eq!(cfg.server.name, name);
    }

    /// Property: schema_version > 1 always returns UnsupportedSchemaVersion.
    #[test]
    fn prop_unsupported_schema_version_rejected(
        v in 2u32..1000
    ) {
        let yaml = format!("_schema_version: {v}\nserver:\n  name: Test\n");
        let result = parse(&yaml);
        prop_assert!(
            matches!(result, Err(ParseError::UnsupportedSchemaVersion { version, .. }) if version == v),
            "expected UnsupportedSchemaVersion({v}), got {:?}", result
        );
    }

    /// Property: schema_version == 1 always parses successfully.
    #[test]
    fn prop_schema_version_1_always_ok(
        name in "[a-zA-Z][a-zA-Z0-9]{0,98}"
    ) {
        let yaml = format!("_schema_version: 1\nserver:\n  name: {name}\n");
        let result = parse(&yaml);
        prop_assert!(result.is_ok(), "expected ok, got {:?}", result.err());
    }
}
