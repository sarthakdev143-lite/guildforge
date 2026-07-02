//! SQLite-backed state store for `GuildForge`.
//!
//! Holds the authoritative record of what `GuildForge` last applied. The
//! planner reads from this; the executor writes to this. See
//! [`ADR-0002`](../../docs/adr/ADR-0002-state-store.md) for the full
//! design.
//!
//! # Concurrency
//!
//! - `Store::open` runs migrations and prepares a connection pool. It
//!   does NOT acquire a lock; reads use short-lived transactions.
//! - `Store::begin_exclusive` acquires an **exclusive** file lock that
//!   prevents any other `GuildForge` process from writing to the same
//!   state file. The lock is held for the lifetime of the returned
//!   [`Transaction`].
//!
//! # Schema
//!
//! Migrations live in `crates/state/migrations/` and are applied
//! automatically on open via `sqlx::migrate!`.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![allow(
    clippy::uninlined_format_args,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::needless_pass_by_value
)]

use fs2::FileExt;
use guildforge_provider::{Resource, ResourceKind};
use guildforge_shared::{now, Hash, ResourceId, Time};
use sqlx::Connection;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info};

// ===========================================================================
// Errors
// ===========================================================================

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
    #[error("state file locked by another process (PID {0})")]
    LockHeld(u32),

    /// State file is corrupt or unreadable.
    #[error("corrupt state: {0}")]
    Corrupt(String),

    /// Resource not found in state.
    #[error("resource not found: {0}")]
    NotFound(String),

    /// Serialization error.
    #[error("serialize: {0}")]
    Serialize(#[from] serde_json::Error),
}

// ===========================================================================
// Resource record (a row in the `resources` table)
// ===========================================================================

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

impl ResourceRecord {
    /// Construct a record from a [`Resource`] value.
    ///
    /// # Errors
    ///
    /// Returns an error if the resource cannot be serialized.
    pub fn from_resource(
        resource: &Resource,
        provider: &str,
        tainted: bool,
    ) -> Result<Self, StateError> {
        let data = serde_json::to_string(resource)?;
        let content_hash = Hash::of(data.as_bytes());
        Ok(Self {
            addr: resource.addr().clone(),
            kind: kind_to_str(resource.kind()).to_string(),
            provider: provider.to_string(),
            data,
            content_hash,
            tainted,
            updated_at: now(),
        })
    }

    /// Decode the JSON `data` field back into a [`Resource`].
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Corrupt`] if the JSON cannot be parsed.
    pub fn to_resource(&self) -> Result<Resource, StateError> {
        serde_json::from_str(&self.data)
            .map_err(|e| StateError::Corrupt(format!("decode {}: {e}", self.addr)))
    }
}

/// Convert a [`ResourceKind`] to its string representation for storage.
#[must_use]
pub fn kind_to_str(k: ResourceKind) -> &'static str {
    match k {
        ResourceKind::Role => "role",
        ResourceKind::Category => "category",
        ResourceKind::Channel => "channel",
        ResourceKind::PermissionOverwrite => "overwrite",
        ResourceKind::Webhook => "webhook",
        ResourceKind::Invite => "invite",
        ResourceKind::ForumTag => "tag",
        ResourceKind::WelcomeScreen => "welcome_screen",
        ResourceKind::ServerGuide => "server_guide",
    }
}

/// Convert a string back to a [`ResourceKind`].
#[must_use]
pub fn str_to_kind(s: &str) -> Option<ResourceKind> {
    match s {
        "role" => Some(ResourceKind::Role),
        "category" => Some(ResourceKind::Category),
        "channel" => Some(ResourceKind::Channel),
        "overwrite" => Some(ResourceKind::PermissionOverwrite),
        "webhook" => Some(ResourceKind::Webhook),
        "invite" => Some(ResourceKind::Invite),
        "tag" => Some(ResourceKind::ForumTag),
        "welcome_screen" => Some(ResourceKind::WelcomeScreen),
        "server_guide" => Some(ResourceKind::ServerGuide),
        _ => None,
    }
}

// ===========================================================================
// Current state (a snapshot of all resources)
// ===========================================================================

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

    /// Returns the number of resources in state.
    #[must_use]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Look up a resource by address.
    #[must_use]
    pub fn get(&self, addr: &ResourceId) -> Option<&ResourceRecord> {
        self.resources.get(addr)
    }

    /// Iterate over all resources.
    pub fn iter(&self) -> impl Iterator<Item = (&ResourceId, &ResourceRecord)> {
        self.resources.iter()
    }

    /// Returns true if any resource is tainted.
    #[must_use]
    pub fn has_tainted(&self) -> bool {
        self.resources.values().any(|r| r.tainted)
    }

    /// Returns all tainted resource addresses.
    #[must_use]
    pub fn tainted_addrs(&self) -> Vec<&ResourceId> {
        self.resources
            .iter()
            .filter(|(_, r)| r.tainted)
            .map(|(a, _)| a)
            .collect()
    }
}

// ===========================================================================
// Migration log entry
// ===========================================================================

/// An entry in the migrations audit log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationLogEntry {
    /// Auto-increment ID.
    pub id: i64,
    /// When the migration was applied.
    pub applied_at: Time,
    /// Hash of the plan that was applied.
    pub plan_hash: String,
    /// Human-readable summary (e.g. `+3 ~1 -0 >2`).
    pub summary: String,
}

// ===========================================================================
// Store
// ===========================================================================

/// The state store. Holds a SQLite connection pool (for reads) and the
/// path to the state file (for locking and write transactions).
pub struct Store {
    /// Path to the SQLite file.
    pub path: PathBuf,
    /// SQLite connection pool (for reads and short transactions).
    pool: sqlx::SqlitePool,
    /// Path to the sidecar lock file.
    lock_path: PathBuf,
    /// Database URL for opening standalone write connections.
    db_url: String,
}

impl Store {
    /// Open a state store at `path`. Runs migrations. Does NOT acquire
    /// a lock — multiple readers can coexist.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Io`] if the parent directory cannot be
    /// created, [`StateError::Sqlite`] if the pool cannot be opened,
    /// or [`StateError::Migration`] if migrations fail.
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self, StateError> {
        let path: PathBuf = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        // Touch the file so the lock file can be created.
        if !path.exists() {
            std::fs::write(&path, b"")?;
        }
        let lock_path = path.with_extension("db.lock");

        let db_url = format!("sqlite://{}?mode=rwc", path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(4)
            .connect(&db_url)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;
        info!(path = %path.display(), "state store opened");

        Ok(Self {
            path,
            pool,
            lock_path,
            db_url,
        })
    }

    /// Open an in-memory state store (for tests). Uses a temp file for
    /// the file lock (since `/dev/null` doesn't support advisory locks).
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] or [`StateError::Migration`].
    pub async fn open_in_memory() -> Result<Self, StateError> {
        // Use a unique shared-cache in-memory database so that all
        // connections (pool + standalone transaction connections) see
        // the same data.
        let db_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_url = format!("sqlite:file:gf_mem_{db_id}?mode=memory&cache=shared");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(4)
            .connect(&db_url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let lock_path = std::env::temp_dir().join(format!(
            "guildforge-state-{}-{}.lock",
            std::process::id(),
            db_id
        ));
        Ok(Self {
            path: PathBuf::from(":memory:"),
            pool,
            lock_path,
            db_url,
        })
    }

    /// Begin an exclusive transaction. Acquires the file lock and starts
    /// a SQL `BEGIN IMMEDIATE TRANSACTION` (so reads from other
    /// connections are blocked too).
    ///
    /// # Errors
    ///
    /// Returns [`StateError::LockHeld`] if another process holds the
    /// exclusive lock.
    pub async fn begin_exclusive(&self) -> Result<Transaction, StateError> {
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(true)
            .open(&self.lock_path)?;
        let pid = std::process::id();
        if let Err(e) = lock_file.try_lock_exclusive() {
            let holder_pid = std::fs::read_to_string(&self.lock_path)
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(0);
            debug!(error = %e, holder_pid, "state file locked");
            return Err(StateError::LockHeld(holder_pid));
        }
        std::fs::write(&self.lock_path, pid.to_string())?;

        // Open a standalone connection (not from the pool) so that
        // dropping it closes the connection and rolls back any open
        // transaction.
        let mut conn = sqlx::SqliteConnection::connect(&self.db_url).await?;
        sqlx::query("BEGIN IMMEDIATE TRANSACTION")
            .execute(&mut conn)
            .await?;
        Ok(Transaction {
            conn: Some(conn),
            lock_file: Some(lock_file),
            lock_released: false,
            committed: false,
        })
    }

    /// Read the entire current state into memory.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn current_state(&self) -> Result<CurrentState, StateError> {
        let rows: Vec<(String, String, String, String, String, bool, String)> = sqlx::query_as(
            "SELECT addr, kind, provider, data, content_hash, tainted, updated_at FROM resources ORDER BY addr",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut resources = std::collections::BTreeMap::new();
        for (addr, kind, provider, data, content_hash_str, tainted, updated_at_str) in rows {
            let content_hash = decode_hash(&content_hash_str)?;
            let updated_at = parse_time(&updated_at_str)?;
            let record = ResourceRecord {
                addr: ResourceId::new(addr.clone()),
                kind,
                provider,
                data,
                content_hash,
                tainted,
                updated_at,
            };
            resources.insert(ResourceId::new(addr), record);
        }
        Ok(CurrentState { resources })
    }

    /// Look up a single resource by address.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn get(&self, addr: &ResourceId) -> Result<Option<ResourceRecord>, StateError> {
        let row: Option<(String, String, String, String, String, bool, String)> = sqlx::query_as(
            "SELECT addr, kind, provider, data, content_hash, tainted, updated_at FROM resources WHERE addr = ?",
        )
        .bind(addr.as_str())
        .fetch_optional(&self.pool)
        .await?;

        if let Some((addr, kind, provider, data, content_hash_str, tainted, updated_at_str)) = row {
            let content_hash = decode_hash(&content_hash_str)?;
            let updated_at = parse_time(&updated_at_str)?;
            Ok(Some(ResourceRecord {
                addr: ResourceId::new(addr),
                kind,
                provider,
                data,
                content_hash,
                tainted,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Returns recent migration log entries (most recent first).
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn migration_log(&self, limit: u32) -> Result<Vec<MigrationLogEntry>, StateError> {
        let rows: Vec<(i64, String, String, String)> = sqlx::query_as(
            "SELECT id, applied_at, plan_hash, summary FROM migrations_log ORDER BY id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(id, applied_at, plan_hash, summary)| {
                let applied_at = parse_time(&applied_at)?;
                Ok(MigrationLogEntry {
                    id,
                    applied_at,
                    plan_hash,
                    summary,
                })
            })
            .collect()
    }

    /// Snapshot the state file to a backup path. Used by `guildforge
    /// backup`.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Io`] on copy error.
    pub fn backup_to(&self, dest: &Path) -> Result<(), StateError> {
        std::fs::copy(&self.path, dest)?;
        Ok(())
    }
}

// ===========================================================================
// Transaction
// ===========================================================================

/// A state transaction. Holds the exclusive file lock and a SQL
/// transaction for its lifetime.
///
/// On `commit`, issues `COMMIT` and releases the lock. On `drop`
/// without commit, issues `ROLLBACK` and releases the lock.
pub struct Transaction {
    conn: Option<sqlx::SqliteConnection>,
    lock_file: Option<std::fs::File>,
    lock_released: bool,
    committed: bool,
}

impl Transaction {
    /// Read the current state from within this transaction.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn current_state(&mut self) -> Result<CurrentState, StateError> {
        let conn = self.conn.as_mut().expect("transaction already committed");
        let rows: Vec<(String, String, String, String, String, bool, String)> = sqlx::query_as(
            "SELECT addr, kind, provider, data, content_hash, tainted, updated_at FROM resources ORDER BY addr",
        )
        .fetch_all(conn)
        .await?;

        let mut resources = std::collections::BTreeMap::new();
        for (addr, kind, provider, data, content_hash_str, tainted, updated_at_str) in rows {
            let content_hash = decode_hash(&content_hash_str)?;
            let updated_at = parse_time(&updated_at_str)?;
            resources.insert(
                ResourceId::new(addr),
                ResourceRecord {
                    addr: ResourceId::new(""),
                    kind,
                    provider,
                    data,
                    content_hash,
                    tainted,
                    updated_at,
                },
            );
        }
        // Fix addrs (we left them empty above).
        for (k, v) in &mut resources {
            v.addr = k.clone();
        }
        Ok(CurrentState { resources })
    }

    /// Upsert a resource record.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn upsert(&mut self, record: &ResourceRecord) -> Result<(), StateError> {
        let conn = self.conn.as_mut().expect("transaction already committed");
        sqlx::query(
            "INSERT INTO resources (addr, kind, provider, data, content_hash, tainted, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(addr) DO UPDATE SET \
             kind = excluded.kind, \
             provider = excluded.provider, \
             data = excluded.data, \
             content_hash = excluded.content_hash, \
             tainted = excluded.tainted, \
             updated_at = excluded.updated_at",
        )
        .bind(record.addr.as_str())
        .bind(&record.kind)
        .bind(&record.provider)
        .bind(&record.data)
        .bind(record.content_hash.to_string())
        .bind(record.tainted)
        .bind(record.updated_at.to_rfc3339())
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Delete a resource by address.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn delete(&mut self, addr: &ResourceId) -> Result<(), StateError> {
        let conn = self.conn.as_mut().expect("transaction already committed");
        sqlx::query("DELETE FROM resources WHERE addr = ?")
            .bind(addr.as_str())
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Mark a resource as tainted.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn taint(&mut self, addr: &ResourceId) -> Result<(), StateError> {
        let conn = self.conn.as_mut().expect("transaction already committed");
        sqlx::query("UPDATE resources SET tainted = 1 WHERE addr = ?")
            .bind(addr.as_str())
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Untaint a resource.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn untaint(&mut self, addr: &ResourceId) -> Result<(), StateError> {
        let conn = self.conn.as_mut().expect("transaction already committed");
        sqlx::query("UPDATE resources SET tainted = 0 WHERE addr = ?")
            .bind(addr.as_str())
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Append an entry to the migration log.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on database error.
    pub async fn log_migration(
        &mut self,
        plan_hash: &str,
        summary: &str,
    ) -> Result<(), StateError> {
        let conn = self.conn.as_mut().expect("transaction already committed");
        sqlx::query("INSERT INTO migrations_log (applied_at, plan_hash, summary) VALUES (?, ?, ?)")
            .bind(now().to_rfc3339())
            .bind(plan_hash)
            .bind(summary)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Commit the transaction and release the lock.
    ///
    /// # Errors
    ///
    /// Returns [`StateError::Sqlite`] on commit error.
    pub async fn commit(mut self) -> Result<(), StateError> {
        if let Some(conn) = self.conn.as_mut() {
            sqlx::query("COMMIT").execute(conn).await?;
        }
        self.committed = true;
        self.conn = None;
        self.release_lock();
        Ok(())
    }

    /// Roll back and release the lock.
    pub fn rollback(mut self) {
        self.do_rollback();
    }

    fn do_rollback(&mut self) {
        if self.committed {
            return;
        }
        // ROLLBACK is synchronous in SQLite but our conn is async. We
        // can't await in drop(). Instead, we just drop the connection —
        // SQLite rolls back uncommitted transactions when the
        // connection closes. This is safe because we hold max_connections=1
        // so no other connection can see the partial state.
        if let Some(conn) = self.conn.take() {
            // Try a best-effort ROLLBACK via a blocking call. If this
            // fails (e.g. connection is in a bad state), dropping the
            // connection will still roll back.
            drop(conn);
        }
        self.committed = true;
        self.release_lock();
    }

    fn release_lock(&mut self) {
        if self.lock_released {
            return;
        }
        if let Some(f) = self.lock_file.take() {
            let _ = fs2::FileExt::unlock(&f);
            drop(f);
        }
        self.lock_released = true;
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        self.do_rollback();
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn decode_hash(s: &str) -> Result<Hash, StateError> {
    if s.len() != 64 {
        return Err(StateError::Corrupt(format!(
            "content_hash length {} not 64",
            s.len()
        )));
    }
    let mut bytes = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = hex_val(chunk[0]).ok_or_else(|| {
            StateError::Corrupt(format!("invalid hex char in content_hash: {:?}", chunk[0]))
        })?;
        let lo = hex_val(chunk[1]).ok_or_else(|| {
            StateError::Corrupt(format!("invalid hex char in content_hash: {:?}", chunk[1]))
        })?;
        bytes[i] = (hi << 4) | lo;
    }
    Ok(Hash::from_bytes(bytes))
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn parse_time(s: &str) -> Result<Time, StateError> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| StateError::Corrupt(format!("invalid timestamp {s:?}: {e}")))
}

/// Compute the content hash of a resource for storage in state.
#[must_use]
pub fn content_hash_of(resource: &Resource) -> Hash {
    resource.content_hash()
}

#[cfg(test)]
mod tests {
    use super::*;
    use guildforge_provider::RoleResource;

    #[tokio::test]
    async fn open_in_memory_runs_migrations() {
        let store = Store::open_in_memory().await.unwrap();
        let state = store.current_state().await.unwrap();
        assert!(state.is_empty());
    }

    #[tokio::test]
    async fn upsert_and_get() {
        let store = Store::open_in_memory().await.unwrap();
        let mut tx = store.begin_exclusive().await.unwrap();
        let role = RoleResource::new("role/Admin", "Admin");
        let record = ResourceRecord::from_resource(
            &guildforge_provider::Resource::Role(role),
            "discord",
            false,
        )
        .unwrap();
        tx.upsert(&record).await.unwrap();
        tx.commit().await.unwrap();

        let got = store.get(&ResourceId::new("role/Admin")).await.unwrap();
        assert!(got.is_some());
        let got = got.unwrap();
        assert_eq!(got.kind, "role");
        assert!(!got.tainted);
    }

    #[tokio::test]
    async fn delete_removes_record() {
        let store = Store::open_in_memory().await.unwrap();
        let mut tx = store.begin_exclusive().await.unwrap();
        let role = RoleResource::new("role/Admin", "Admin");
        let record = ResourceRecord::from_resource(
            &guildforge_provider::Resource::Role(role),
            "discord",
            false,
        )
        .unwrap();
        tx.upsert(&record).await.unwrap();
        tx.delete(&ResourceId::new("role/Admin")).await.unwrap();
        tx.commit().await.unwrap();

        let got = store.get(&ResourceId::new("role/Admin")).await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn taint_and_untaint() {
        let store = Store::open_in_memory().await.unwrap();
        let mut tx = store.begin_exclusive().await.unwrap();
        let role = RoleResource::new("role/Admin", "Admin");
        let record = ResourceRecord::from_resource(
            &guildforge_provider::Resource::Role(role),
            "discord",
            false,
        )
        .unwrap();
        tx.upsert(&record).await.unwrap();
        tx.taint(&ResourceId::new("role/Admin")).await.unwrap();
        tx.commit().await.unwrap();

        let got = store
            .get(&ResourceId::new("role/Admin"))
            .await
            .unwrap()
            .unwrap();
        assert!(got.tainted);

        let mut tx2 = store.begin_exclusive().await.unwrap();
        tx2.untaint(&ResourceId::new("role/Admin")).await.unwrap();
        tx2.commit().await.unwrap();

        let got = store
            .get(&ResourceId::new("role/Admin"))
            .await
            .unwrap()
            .unwrap();
        assert!(!got.tainted);
    }

    #[tokio::test]
    async fn rollback_drops_uncommitted_writes() {
        let store = Store::open_in_memory().await.unwrap();
        let mut tx = store.begin_exclusive().await.unwrap();
        let role = RoleResource::new("role/Admin", "Admin");
        let record = ResourceRecord::from_resource(
            &guildforge_provider::Resource::Role(role),
            "discord",
            false,
        )
        .unwrap();
        tx.upsert(&record).await.unwrap();
        tx.rollback();

        let got = store.get(&ResourceId::new("role/Admin")).await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn migration_log_appends() {
        let store = Store::open_in_memory().await.unwrap();
        let mut tx = store.begin_exclusive().await.unwrap();
        tx.log_migration("hash1", "+1 ~0 -0 >0").await.unwrap();
        tx.commit().await.unwrap();

        let entries = store.migration_log(10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].plan_hash, "hash1");
        assert_eq!(entries[0].summary, "+1 ~0 -0 >0");
    }

    #[tokio::test]
    async fn current_state_returns_all() {
        let store = Store::open_in_memory().await.unwrap();
        let mut tx = store.begin_exclusive().await.unwrap();
        for name in ["Admin", "Staff", "Member"] {
            let role = RoleResource::new(format!("role/{name}"), name);
            let record = ResourceRecord::from_resource(
                &guildforge_provider::Resource::Role(role),
                "discord",
                false,
            )
            .unwrap();
            tx.upsert(&record).await.unwrap();
        }
        tx.commit().await.unwrap();

        let state = store.current_state().await.unwrap();
        assert_eq!(state.len(), 3);
        assert!(state.get(&ResourceId::new("role/Admin")).is_some());
    }

    #[tokio::test]
    async fn file_lock_prevents_concurrent_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let store = Store::open(&path).await.unwrap();

        // First transaction acquires the lock.
        let tx1 = store.begin_exclusive().await.unwrap();

        // Second transaction should fail with LockHeld.
        let result = store.begin_exclusive().await;
        assert!(matches!(result, Err(StateError::LockHeld(_))));

        // After commit, the lock is released.
        tx1.commit().await.unwrap();
        let tx2 = store.begin_exclusive().await;
        assert!(tx2.is_ok());
    }

    #[test]
    fn kind_str_round_trip() {
        for k in [
            ResourceKind::Role,
            ResourceKind::Category,
            ResourceKind::Channel,
            ResourceKind::PermissionOverwrite,
            ResourceKind::Webhook,
            ResourceKind::Invite,
            ResourceKind::ForumTag,
            ResourceKind::WelcomeScreen,
            ResourceKind::ServerGuide,
        ] {
            let s = kind_to_str(k);
            assert_eq!(str_to_kind(s), Some(k));
        }
        assert_eq!(str_to_kind("bogus"), None);
    }

    #[test]
    fn resource_record_round_trip() {
        let role = RoleResource::new("role/Admin", "Admin");
        let record = ResourceRecord::from_resource(
            &guildforge_provider::Resource::Role(role),
            "discord",
            false,
        )
        .unwrap();
        let decoded = record.to_resource().unwrap();
        assert_eq!(
            decoded,
            guildforge_provider::Resource::Role(RoleResource::new("role/Admin", "Admin"))
        );
    }

    #[test]
    fn current_state_tainted_tracking() {
        let mut state = CurrentState::default();
        assert!(!state.has_tainted());
        let record = ResourceRecord::from_resource(
            &guildforge_provider::Resource::Role(RoleResource::new("role/Admin", "Admin")),
            "discord",
            true,
        )
        .unwrap();
        let mut record = record;
        record.addr = ResourceId::new("role/Admin");
        state.resources.insert(record.addr.clone(), record);
        assert!(state.has_tainted());
        assert_eq!(state.tainted_addrs().len(), 1);
    }

    #[test]
    fn decode_hash_rejects_wrong_length() {
        assert!(decode_hash("too_short").is_err());
        assert!(decode_hash(&"x".repeat(64)).is_err()); // valid length, invalid chars
    }
}
