//! Fuzz-like tests for the parser.
//!
//! These tests use `proptest` to generate arbitrary inputs and verify
//! that the parser never panics. This is equivalent to `cargo-fuzz`
//! but runs within the standard test framework (no nightly required).

use guildforge_parser::{parse, ParseError};
use proptest::prelude::*;

proptest! {
    /// Property: parsing arbitrary bytes never panics.
    #[test]
    fn fuzz_parser_never_panics_arbitrary_bytes(input in ".*") {
        let _ = parse(&input);
    }

    /// Property: parsing arbitrary UTF-8 never panics.
    #[test]
    fn fuzz_parser_never_panics_utf8(input in "[\\x00-\\x7F\\u{00A0}-\\u{FFFF}]{0,500}") {
        let _ = parse(&input);
    }

    /// Property: parsing binary data never panics.
    #[test]
    fn fuzz_parser_never_panics_binary(input in prop::collection::vec(any::<u8>(), 0..1000)) {
        let s = String::from_utf8_lossy(&input);
        let _ = parse(&s);
    }

    /// Property: empty/whitespace input always returns ParseError::Empty.
    #[test]
    fn fuzz_empty_always_returns_empty_error(
        ws in "[ \\t\\n\\r]{0,100}"
    ) {
        let result = parse(&ws);
        prop_assert!(
            matches!(result, Err(ParseError::Empty)),
            "expected Empty, got {:?}", result
        );
    }

    /// Property: schema_version > 1 always returns UnsupportedSchemaVersion.
    #[test]
    fn fuzz_bad_schema_version_rejected(v in 2u32..100_000) {
        let yaml = format!("_schema_version: {v}\nserver:\n  name: Test\n");
        let result = parse(&yaml);
        prop_assert!(
            matches!(result, Err(ParseError::UnsupportedSchemaVersion { version, .. }) if version == v),
            "expected UnsupportedSchemaVersion({v}), got {:?}", result
        );
    }

    /// Property: valid server name always parses successfully.
    #[test]
    fn fuzz_valid_server_name_parses(
        name in "[a-zA-Z][a-zA-Z0-9_-]{0,98}"
    ) {
        let yaml = format!("server:\n  name: {name}\n");
        let result = parse(&yaml);
        prop_assert!(result.is_ok(), "expected ok, got {:?}", result.err());
        let cfg = result.unwrap();
        prop_assert_eq!(cfg.server.name, name);
    }

    /// Property: YAML with only a server block and no other keys is valid.
    #[test]
    fn fuzz_minimal_valid_config(
        name in "[a-zA-Z][a-zA-Z0-9]{0,98}"
    ) {
        let yaml = format!(
            "server:\n  name: {name}\n  description: Test\n  verification_level: low\n"
        );
        let result = parse(&yaml);
        prop_assert!(result.is_ok(), "expected ok, got {:?}", result.err());
    }

    /// Property: deeply nested YAML doesn't cause stack overflow.
    #[test]
    fn fuzz_deeply_nested_yaml(depth in 1usize..100) {
        let mut yaml = String::from("server:\n  name: Test\n");
        for i in 0..depth {
            yaml.push_str(&"  ".repeat(i));
            yaml.push_str(&format!("key{i}:\n"));
            yaml.push_str(&"  ".repeat(i + 1));
            yaml.push_str("value\n");
        }
        // Should not panic, even if it fails to parse (unknown keys).
        let _ = parse(&yaml);
    }

    /// Property: very long strings don't cause issues.
    #[test]
    fn fuzz_long_strings(len in 100usize..50_000) {
        let name = "a".repeat(len);
        let yaml = format!("server:\n  name: {name}\n");
        let result = parse(&yaml);
        // Either parses (if len <= 100) or fails validation, but never panics.
        let _ = result;
    }

    /// Property: repeated YAML keys are handled gracefully.
    #[test]
    fn fuzz_repeated_keys(n in 1usize..50) {
        let mut yaml = String::from("server:\n  name: Test\n");
        for _ in 0..n {
            yaml.push_str("server:\n  name: Other\n");
        }
        // serde_yaml may accept or reject this; either way, no panic.
        let _ = parse(&yaml);
    }
}
