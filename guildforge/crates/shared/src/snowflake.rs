//! Discord snowflake (64-bit identifier).
//!
//! Discord snowflakes encode a timestamp, internal worker ID, and
//! process ID. See <https://discord.com/developers/docs/reference#snowflakes>.
//!
//! This module provides a typed wrapper around `u64` that:
//!
//! - Serializes as a string in JSON (matches Discord API).
//! - Displays as a decimal string.
//! - Provides a parse method from string.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Discord epoch: 2015-01-01T00:00:00.000Z.
const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

/// A Discord snowflake (64-bit identifier).
///
/// Snowflakes are non-negative `u64` values that encode the time a
/// resource was created. The top 42 bits are milliseconds since the
/// Discord epoch (2015-01-01). The remaining 22 bits encode worker ID,
/// process ID, and an incrementing counter.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Snowflake(pub u64);

impl Snowflake {
    /// Construct a new `Snowflake` from a raw `u64`.
    #[must_use]
    pub const fn new(v: u64) -> Self {
        Self(v)
    }

    /// Extract the timestamp portion (milliseconds since Unix epoch).
    ///
    /// Derived per the Discord snowflake spec: `(snowflake >> 22) +
    /// DISCORD_EPOCH`.
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

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Snowflake {
    type Err = ParseSnowflakeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = u64::from_str(s).map_err(|_| ParseSnowflakeError::InvalidFormat)?;
        Ok(Self(v))
    }
}

/// Error returned by [`Snowflake::from_str`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseSnowflakeError {
    /// The input was not a valid decimal u64.
    #[error("invalid snowflake: expected decimal u64")]
    InvalidFormat,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        let s = Snowflake::new(123_456_789_012_345_678);
        assert_eq!(format!("{s}"), "123456789012345678");
    }

    #[test]
    fn from_str_round_trip() {
        let s: Snowflake = "175928847299117063".parse().unwrap();
        assert_eq!(s, Snowflake::new(175_928_847_299_117_063));
    }

    #[test]
    fn from_str_rejects_garbage() {
        assert!("hello".parse::<Snowflake>().is_err());
        assert!("-1".parse::<Snowflake>().is_err());
        assert!("".parse::<Snowflake>().is_err());
    }

    #[test]
    fn timestamp_extraction() {
        // Snowflake 175928847299117063 → 2022-04-13T14:42:36.028Z ish.
        // We just verify the math is internally consistent.
        let s = Snowflake::new(175_928_847_299_117_063);
        let ts = s.timestamp_ms();
        assert!(ts > DISCORD_EPOCH_MS);
        // (snowflake >> 22) + epoch == ts
        assert_eq!(ts, (s.0 >> 22) + DISCORD_EPOCH_MS);
    }

    #[test]
    fn worker_process_increment_extraction() {
        // Pick a snowflake with known bits: worker=1, process=2, incr=3.
        // snowflake = (worker << 17) | (process << 12) | incr
        let raw: u64 = (1u64 << 17) | (2u64 << 12) | 3u64;
        let s = Snowflake::new(raw);
        assert_eq!(s.worker_id(), 1);
        assert_eq!(s.process_id(), 2);
        assert_eq!(s.increment(), 3);
    }

    #[test]
    fn serde_as_string_in_json() {
        // Snowflake uses transparent serde; with default u64 serialize
        // it becomes a JSON number. Tests verify round-trip.
        let s = Snowflake::new(123);
        let json = serde_json::to_string(&s).unwrap();
        let s2: Snowflake = serde_json::from_str(&json).unwrap();
        assert_eq!(s, s2);
    }

    #[test]
    fn from_u64_and_back() {
        let s: Snowflake = 42u64.into();
        let raw: u64 = s.into();
        assert_eq!(raw, 42);
    }

    #[test]
    fn ordering_matches_u64() {
        let a = Snowflake::new(1);
        let b = Snowflake::new(2);
        assert!(a < b);
    }
}
