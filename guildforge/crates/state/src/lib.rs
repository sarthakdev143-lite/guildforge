//! SQLite-backed state store for `GuildForge`.
//!
//! Holds the authoritative record of what `GuildForge` last applied. The
//! planner reads from this; the executor writes to this. See
//! [`ADR-0002`](../../docs/adr/ADR-0002-state-store.md) for the full
//! design.
//!
//! # Concurrency
//!
//! - `Store::open` acquires a **shared** (read) file lock.
//! - `Store::begin_exclusive` upgrades to an **exclusive** (write) lock.
//! - Locks are released on drop.
//!
//! # Schema
//!
//! See [`ADR-0002`](../../docs/adr/ADR-0002-state-store.md) for the SQL
//! DDL. Migrations live in `crates/state/migrations/` and are applied
//! automatically on open.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 3
//! (task `P3-001`, `P3-002`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use guildforge_provider::Resource;
use guildforge_shared::{Hash, ResourceId, Time};
use std::path::PathBuf;
use thiserror::Error;

/// State store error.
#[derive(Debug, Error)]
pub enum StateError {
    /// I/O error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// `SQLite` error.
    #[error("sqlite: {0}")]
    Sqlite(#[from] sqlx::Error),

    /// Migration error.
    #[error("migration: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// State file is locked by another process.
    #[error("state file locked by PID {0}")]
    LockHeld(u32),

    /// State file is corrupt or unreadable.
    #[error("corrupt state: {0}")]
    Corrupt(String),
}

/// A single resource record in state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRecord {
    /// Stable resource address (primary key).
    pub addr: ResourceId,
    /// Resource kind (e.g. `role`).
    pub kind: String,
    /// Provider name (e.g. `discord`).
    pub provider: String,
    /// JSON-serialized resource.
    pub data: String,
    /// Blake3 hash of `data`, for fast diffing.
    pub content_hash: Hash,
    /// Whether the resource is tainted (last apply failed).
    pub tainted: bool,
    /// Last-updated timestamp.
    pub updated_at: Time,
}

/// The current state — a snapshot of all resources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurrentState {
    /// All resources in state, keyed by address.
    pub resources: std::collections::BTreeMap<ResourceId, ResourceRecord>,
}

impl CurrentState {
    /// Returns `true` if state is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Look up a resource by address.
    #[must_use]
    pub fn get(&self, addr: &ResourceId) -> Option<&ResourceRecord> {
        self.resources.get(addr)
    }
}

/// The state store. Phase 0 stub.
///
/// Real implementation (Phase 3) will hold a `sqlx::SqlitePool` and a
/// file lock guard.
pub struct Store {
    /// Path to the `SQLite` file.
    pub path: PathBuf,
}

impl Store {
    /// Open a state store at `path`. Acquires a shared file lock and
    /// runs migrations.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Io`] if the file cannot be opened,
    /// [`StateError::LockHeld`] if another process holds the exclusive
    /// lock, or [`StateError::Migration`] if migrations fail.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, StateError> {
        // Phase 0 stub. Real implementation lands in task P3-001.
        Ok(Self { path: path.into() })
    }

    /// Begin an exclusive transaction. Acquires the exclusive file lock.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::LockHeld`] if another process holds any
    /// lock.
    #[allow(clippy::unused_async)] // Phase 0 stub; real impl awaits SQLite.
    pub async fn begin_exclusive(&self) -> Result<Transaction, StateError> {
        // Phase 0 stub.
        Ok(Transaction {
            current: CurrentState::default(),
        })
    }
}

/// A state transaction. Holds the exclusive lock for its lifetime.
///
/// On `commit`, writes the new state atomically. On `rollback` or
/// `drop`, discards changes and releases the lock.
pub struct Transaction {
    /// The current state (read at begin).
    pub current: CurrentState,
}

impl Transaction {
    /// Read the current state.
    #[must_use]
    pub fn current(&self) -> &CurrentState {
        &self.current
    }

    /// Commit the new state and release the lock.
    ///
    /// # Errors
    ///
    /// Returns [`StateError`] if the commit fails.
    #[allow(clippy::unused_async)] // Phase 0 stub; real impl awaits SQLite.
    pub async fn commit(self, _new: CurrentState) -> Result<(), StateError> {
        // Phase 0 stub. Real implementation will write to SQLite in a
        // single transaction.
        Ok(())
    }

    /// Roll back and release the lock.
    pub fn rollback(self) {
        // Phase 0 stub. Drop releases the lock.
    }
}

/// Compute the content hash of a resource, for storage in state.
#[must_use]
pub fn content_hash_of(resource: &Resource) -> Hash {
    let json = serde_json::to_string(resource).unwrap_or_default();
    Hash::of(json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_open_and_begin() {
        let store = Store::open(":memory:").unwrap();
        let tx = store.begin_exclusive().await.unwrap();
        assert!(tx.current().is_empty());
        tx.rollback();
    }

    #[test]
    fn current_state_get() {
        let mut state = CurrentState::default();
        let addr = ResourceId::new("role/Admin");
        state.resources.insert(
            addr.clone(),
            ResourceRecord {
                addr: addr.clone(),
                kind: "role".to_string(),
                provider: "discord".to_string(),
                data: "{}".to_string(),
                content_hash: Hash::of(b"{}"),
                tainted: false,
                updated_at: guildforge_shared::now(),
            },
        );
        assert!(state.get(&addr).is_some());
        assert!(state.get(&ResourceId::new("role/None")).is_none());
    }
}
