//! Cross-crate primitives for `GuildForge`.
//!
//! This crate has **no I/O** and **no dependencies on other `GuildForge`
//! crates**. Every other crate may depend on this one. See
//! [`docs/CRATE_LAYOUT.md`](../../docs/CRATE_LAYOUT.md) for the full
//! dependency rules.
//!
//! # Contents
//!
//! - [`ResourceId`] — newtype around a stable string identifier.
//! - [`Snowflake`] — Discord snowflake (u64) with parsing and display.
//! - [`Hash`] — wrapper around a blake3 hash for content-addressing.
//! - [`Time`] — `chrono` wrappers for consistent timestamps.
//! - [`IdempotencyKey`] — generated per-operation, persisted with state.
//!
//! Phase 0: this crate is a stub. Real implementations land in Phase 1
//! (task `P1-001` in [`TASKS.md`](../../TASKS.md)).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

/// A stable, human-readable identifier for a resource.
///
/// Example: `discord://guild/role/Admin`.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ResourceId(pub String);

impl ResourceId {
    /// Construct a new `ResourceId`.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Access the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A Discord snowflake (64-bit identifier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Snowflake(pub u64);

impl Snowflake {
    /// Construct a new `Snowflake` from a raw `u64`.
    #[must_use]
    pub const fn new(v: u64) -> Self {
        Self(v)
    }
}

impl std::fmt::Display for Snowflake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A blake3 hash used for content-addressing resource state.
///
/// See [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md) for
/// why the planner uses content hashes for diffing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Hash(#[serde(with = "crate::serde_helpers::hex")] pub blake3::Hash);

impl Hash {
    /// Compute the hash of an arbitrary byte slice.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes))
    }
}

/// Helper serialization modules.
pub mod serde_helpers {
    /// Hex serialization for `blake3::Hash`.
    pub mod hex {
        use serde::{Deserialize, Deserializer, Serializer};
        use std::fmt::Write as _;

        /// Serialize a `blake3::Hash` as a hex string.
        ///
        /// # Errors
        ///
        /// Returns the serializer's error if writing fails.
        pub fn serialize<S: Serializer>(h: &blake3::Hash, s: S) -> Result<S::Ok, S::Error> {
            let mut buf = String::with_capacity(64);
            for byte in h.as_bytes() {
                write!(&mut buf, "{byte:02x}").map_err(serde::ser::Error::custom)?;
            }
            s.serialize_str(&buf)
        }

        /// Deserialize a `blake3::Hash` from a hex string.
        ///
        /// # Errors
        ///
        /// Returns the deserializer's error if the input is not a valid
        /// hex string of length 64.
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<blake3::Hash, D::Error> {
            let s = String::deserialize(d)?;
            let bytes = hex_decode(&s).map_err(serde::de::Error::custom)?;
            // blake3::Hash::from_bytes returns Hash directly (not Result)
            Ok(blake3::Hash::from_bytes(bytes))
        }

        fn hex_decode(s: &str) -> Result<[u8; 32], String> {
            if s.len() != 64 {
                return Err(format!("expected 64 hex chars, got {}", s.len()));
            }
            let mut out = [0u8; 32];
            for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
                let hi = hex_val(chunk[0])?;
                let lo = hex_val(chunk[1])?;
                out[i] = (hi << 4) | lo;
            }
            Ok(out)
        }

        fn hex_val(c: u8) -> Result<u8, String> {
            match c {
                b'0'..=b'9' => Ok(c - b'0'),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'A'..=b'F' => Ok(c - b'A' + 10),
                _ => Err(format!("invalid hex char: {c:?}")),
            }
        }
    }
}

/// Timestamp wrapper for consistent serialization across crates.
pub type Time = chrono::DateTime<chrono::Utc>;

/// Generate the current UTC time.
#[must_use]
pub fn now() -> Time {
    chrono::Utc::now()
}

/// An idempotency key, generated per operation and persisted with state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct IdempotencyKey(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_id_roundtrip() {
        let id = ResourceId::new("discord://guild/role/Admin");
        assert_eq!(id.as_str(), "discord://guild/role/Admin");
        assert_eq!(format!("{id}"), "discord://guild/role/Admin");
    }

    #[test]
    fn snowflake_display() {
        let s = Snowflake::new(123_456_789_012_345_678);
        assert_eq!(format!("{s}"), "123456789012345678");
    }

    #[test]
    fn hash_of_known_input() {
        let h1 = Hash::of(b"hello");
        let h2 = Hash::of(b"hello");
        let h3 = Hash::of(b"world");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn hash_serde_roundtrip() {
        let h = Hash::of(b"hello");
        let json = serde_json::to_string(&h).expect("serialize");
        let h2: Hash = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(h, h2);
    }
}
