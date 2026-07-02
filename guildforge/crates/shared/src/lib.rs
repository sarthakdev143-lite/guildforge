//! Cross-crate primitives for `GuildForge`.
//!
//! No I/O, no dependencies on other `GuildForge` crates. Every other crate
//! may depend on this one. See
//! [`docs/CRATE_LAYOUT.md`](../../docs/CRATE_LAYOUT.md) for the full
//! dependency rules.
//!
//! # Contents
//!
//! - [`ResourceId`] — stable string identifier for a resource.
//! - [`Snowflake`] — Discord snowflake (u64) with timestamp extraction.
//! - [`Hash`] — blake3 hash for content-addressing.
//! - [`Time`], [`Clock`] — injectable time for deterministic testing.
//! - [`IdempotencyKey`] — per-operation key for safe retries.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use serde::{Deserialize, Serialize};

// ===========================================================================
// ResourceId
// ===========================================================================

/// A stable, human-readable identifier for a resource.
///
/// Addresses follow `<kind>/<path>` per
/// [`docs/SCHEMA.md` §12](../../docs/SCHEMA.md). Examples:
/// - `role/Admin`
/// - `category/COMPANY`
/// - `channel/COMPANY/announcements`
///
/// `ResourceIds` are case-sensitive and ordered lexicographically.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ResourceId(pub String);

impl ResourceId {
    /// Construct a new `ResourceId`.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Construct a new `ResourceId`, returning an error if the input is
    /// empty.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResourceId`] if the input is empty.
    pub fn try_new(s: impl Into<String>) -> Result<Self, InvalidResourceId> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidResourceId::Empty);
        }
        Ok(Self(s))
    }

    /// Access the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into the underlying string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for ResourceId {
    type Err = InvalidResourceId;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

/// Error returned by [`ResourceId::try_new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidResourceId {
    /// The input was empty.
    #[error("resource id cannot be empty")]
    Empty,
}

// ===========================================================================
// Snowflake
// ===========================================================================

/// Discord epoch (2015-01-01T00:00:00.000Z) in milliseconds.
const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

/// A Discord snowflake (64-bit identifier).
///
/// Snowflakes encode creation time in the top 42 bits (ms since Discord
/// epoch), worker ID in 5 bits, process ID in 5 bits, and an increment
/// in 12 bits. See
/// <https://discord.com/developers/docs/reference#snowflakes>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Snowflake(pub u64);

impl Snowflake {
    /// Construct a new `Snowflake` from a raw `u64`.
    #[must_use]
    pub const fn new(v: u64) -> Self {
        Self(v)
    }

    /// Extract the timestamp portion (milliseconds since Unix epoch).
    #[must_use]
    pub const fn timestamp_ms(&self) -> u64 {
        (self.0 >> 22) + DISCORD_EPOCH_MS
    }

    /// Extract the internal worker ID (5 bits).
    #[must_use]
    pub const fn worker_id(&self) -> u64 {
        (self.0 >> 17) & 0x1F
    }

    /// Extract the internal process ID (5 bits).
    #[must_use]
    pub const fn process_id(&self) -> u64 {
        (self.0 >> 12) & 0x1F
    }

    /// Extract the increment (12 bits).
    #[must_use]
    pub const fn increment(&self) -> u64 {
        self.0 & 0xFFF
    }
}

impl std::fmt::Display for Snowflake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Snowflake {
    type Err = ParseSnowflakeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = u64::from_str(s).map_err(|_| ParseSnowflakeError::InvalidFormat)?;
        Ok(Self(v))
    }
}

impl From<u64> for Snowflake {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

impl From<Snowflake> for u64 {
    fn from(s: Snowflake) -> u64 {
        s.0
    }
}

/// Error returned by [`Snowflake::from_str`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseSnowflakeError {
    /// The input was not a valid decimal u64.
    #[error("invalid snowflake: expected decimal u64")]
    InvalidFormat,
}

// ===========================================================================
// Hash
// ===========================================================================

/// A blake3 hash used for content-addressing resource state.
///
/// See [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md) for
/// why the planner uses content hashes for diffing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(#[serde(with = "serde_helpers::hex")] pub blake3::Hash);

impl Hash {
    /// Compute the hash of an arbitrary byte slice.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes))
    }

    /// Construct a `Hash` from raw blake3 bytes.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(blake3::Hash::from_bytes(bytes))
    }

    /// Compute the hash of a serializable value by first serializing it
    /// to canonical JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn of_serializable<T: Serialize>(v: &T) -> Result<Self, serde_json::Error> {
        let bytes = serde_json::to_vec(v)?;
        Ok(Self(blake3::hash(&bytes)))
    }

    /// Access the inner blake3 hash.
    #[must_use]
    pub const fn inner(&self) -> &blake3::Hash {
        &self.0
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0.as_bytes() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

// ===========================================================================
// Time + Clock
// ===========================================================================

/// Re-export of `chrono::DateTime<chrono::Utc>` for consistent typing.
pub type Time = chrono::DateTime<chrono::Utc>;

/// A clock that returns the current time. Inject into test code for
/// deterministic timestamps.
pub trait Clock: Send + Sync {
    /// Return the current UTC time.
    fn now(&self) -> Time;
}

/// The system clock — reads `chrono::Utc::now()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Time {
        chrono::Utc::now()
    }
}

/// Generate the current UTC time using the system clock.
#[must_use]
pub fn now() -> Time {
    SystemClock.now()
}

// ===========================================================================
// IdempotencyKey
// ===========================================================================

/// An idempotency key, generated per operation and persisted with state.
///
/// Used by the executor to safely retry operations. Format:
/// `<nanos_since_epoch_hex>-<counter_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    /// Generate a new idempotency key.
    #[must_use]
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};

        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(format!("{ts:020x}-{n:016x}"))
    }

    /// Construct from a known string (mainly for testing).
    #[must_use]
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Access the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for IdempotencyKey {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ===========================================================================
// Serde helpers (hex for blake3::Hash)
// ===========================================================================

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
        /// Returns the deserializer's error if the input is not 64 hex chars.
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<blake3::Hash, D::Error> {
            let s = String::deserialize(d)?;
            let bytes = hex_decode(&s).map_err(serde::de::Error::custom)?;
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

// ===========================================================================
// Tests
// ===========================================================================

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
    fn resource_id_try_new_rejects_empty() {
        assert!(ResourceId::try_new("").is_err());
        assert!(ResourceId::try_new("x").is_ok());
    }

    #[test]
    fn resource_id_from_str() {
        let id: ResourceId = "role/Admin".parse().unwrap();
        assert_eq!(id.as_str(), "role/Admin");
        assert!("".parse::<ResourceId>().is_err());
    }

    #[test]
    fn resource_id_ordering() {
        let mut ids = [
            ResourceId::new("z"),
            ResourceId::new("a"),
            ResourceId::new("m"),
        ];
        ids.sort();
        assert_eq!(ids[0].as_str(), "a");
        assert_eq!(ids[2].as_str(), "z");
    }

    #[test]
    fn snowflake_display() {
        let s = Snowflake::new(123_456_789_012_345_678);
        assert_eq!(format!("{s}"), "123456789012345678");
    }

    #[test]
    fn snowflake_from_str_round_trip() {
        let s: Snowflake = "175928847299117063".parse().unwrap();
        assert_eq!(s, Snowflake::new(175_928_847_299_117_063));
        assert!("hello".parse::<Snowflake>().is_err());
        assert!("-1".parse::<Snowflake>().is_err());
        assert!("".parse::<Snowflake>().is_err());
    }

    #[test]
    fn snowflake_timestamp_extraction() {
        let s = Snowflake::new(175_928_847_299_117_063);
        let ts = s.timestamp_ms();
        assert!(ts > DISCORD_EPOCH_MS);
        assert_eq!(ts, (s.0 >> 22) + DISCORD_EPOCH_MS);
    }

    #[test]
    fn snowflake_worker_process_increment() {
        let raw: u64 = (1u64 << 17) | (2u64 << 12) | 3u64;
        let s = Snowflake::new(raw);
        assert_eq!(s.worker_id(), 1);
        assert_eq!(s.process_id(), 2);
        assert_eq!(s.increment(), 3);
    }

    #[test]
    fn snowflake_from_u64_and_back() {
        let s: Snowflake = 42u64.into();
        let raw: u64 = s.into();
        assert_eq!(raw, 42);
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

    #[test]
    fn hash_display_is_hex() {
        let h = Hash::of(b"");
        let s = format!("{h}");
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_of_serializable_is_deterministic() {
        #[derive(Serialize)]
        struct Foo {
            a: u32,
            b: &'static str,
        }
        let v = Foo { a: 1, b: "hi" };
        let h1 = Hash::of_serializable(&v).unwrap();
        let h2 = Hash::of_serializable(&v).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_of_serializable_differs_on_value() {
        #[derive(Serialize)]
        struct Foo {
            a: u32,
        }
        let h1 = Hash::of_serializable(&Foo { a: 1 }).unwrap();
        let h2 = Hash::of_serializable(&Foo { a: 2 }).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn idempotency_key_new_is_unique() {
        let a = IdempotencyKey::new();
        let b = IdempotencyKey::new();
        assert_ne!(a, b);
    }

    #[test]
    fn idempotency_key_format() {
        let k = IdempotencyKey::new();
        let parts: Vec<&str> = k.as_str().split('-').collect();
        assert_eq!(parts.len(), 2, "expected 2 parts, got {k}");
    }

    #[test]
    fn system_clock_advances() {
        let c = SystemClock;
        let t1 = c.now();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let t2 = c.now();
        assert!(t2 > t1);
    }

    #[test]
    fn now_returns_utc() {
        let t = now();
        assert_eq!(t.timezone(), chrono::Utc);
    }
}
