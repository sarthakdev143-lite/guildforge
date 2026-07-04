//! Resource identifiers and idempotency keys.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A stable, human-readable identifier for a resource.
///
/// Addresses follow the format described in
/// [`docs/SCHEMA.md` §12](../../docs/SCHEMA.md): `<kind>/<path>` where
/// `<path>` is a `/`-separated list of names. Examples:
///
/// - `role/Admin`
/// - `category/COMPANY`
/// - `channel/COMPANY/announcements`
/// - `channel/_top/general`
///
/// ResourceIds are case-sensitive and compared lexicographically. Empty
/// strings are rejected at parse time.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct ResourceId(pub String);

impl ResourceId {
    /// Construct a new `ResourceId` from anything string-like.
    ///
    /// # Panics
    ///
    /// This never panics in the current implementation but reserves the
    /// right to validate inputs in the future. Use [`ResourceId::try_new`]
    /// for fallible construction.
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

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for ResourceId {
    type Err = InvalidResourceId;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

/// Error returned by [`ResourceId::try_new`] and
/// [`ResourceId::from_str`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidResourceId {
    /// The input was empty.
    #[error("resource id cannot be empty")]
    Empty,
}

/// An idempotency key, generated per operation and persisted with state.
///
/// Used by the executor to safely retry operations without side-effect
/// duplication. Format: `<timestamp_ms>-<random_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey(pub String);

impl IdempotencyKey {
    /// Generate a new random idempotency key.
    ///
    /// Uses a process-wide atomic counter plus the current time. Not
    /// cryptographically unique, but good enough for correlation within
    /// a single process and across process restarts (timestamp component
    /// dominates).
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

    /// Construct an idempotency key from a known string (mainly for
    /// testing).
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

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_rejects_empty() {
        assert!(ResourceId::try_new("").is_err());
        assert!(ResourceId::try_new("x").is_ok());
    }

    #[test]
    fn from_str_round_trip() {
        let id: ResourceId = "role/Admin".parse().unwrap();
        assert_eq!(id.as_str(), "role/Admin");
    }

    #[test]
    fn ordering() {
        let ids = vec![
            ResourceId::new("z"),
            ResourceId::new("a"),
            ResourceId::new("m"),
        ];
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(sorted[0].as_str(), "a");
        assert_eq!(sorted[1].as_str(), "m");
        assert_eq!(sorted[2].as_str(), "z");
    }

    #[test]
    fn idempotency_key_format() {
        let k = IdempotencyKey::new();
        let parts: Vec<&str> = k.as_str().split('-').collect();
        assert_eq!(parts.len(), 2, "expected 2 parts, got {k}");
    }
}
