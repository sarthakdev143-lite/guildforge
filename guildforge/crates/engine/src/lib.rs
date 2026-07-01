//! Workflow orchestrator for `GuildForge`.
//!
//! The engine wires together the parser, validator, planner, executor,
//! and state store. The CLI calls into the engine; the engine never
//! imports from `guildforge-provider-discord` — the concrete provider
//! is injected at construction.
//!
//! See [`ARCHITECTURE.md` §4](../../ARCHITECTURE.md) for the pipeline
//! diagram and [`ADR-0007`](../../docs/adr/ADR-0007-idempotency-ordering.md)
//! for the apply lifecycle.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 3
//! (task `P3-008`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use guildforge_config::Config;
use guildforge_executor::ExecutionReport;
use guildforge_planner::ExecutionPlan;
use guildforge_provider::Provider;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

/// Engine error.
///
/// The engine uses `anyhow` internally for ergonomic chaining; this
/// enum exists for the few cases where the CLI needs to match on a
/// specific engine error (e.g. `LockHeld`) to produce a specific exit
/// code. See [`ADR-0005`](../../docs/adr/ADR-0005-error-model.md).
#[derive(Debug, Error)]
pub enum EngineError {
    /// State file is locked by another process.
    #[error("state file locked by PID {0}")]
    LockHeld(u32),

    /// User aborted at the confirmation prompt.
    #[error("user aborted")]
    Aborted,

    /// Validation failed.
    #[error("validation failed: {0}")]
    Validation(String),

    /// Any other error, wrapped.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result alias for engine operations.
pub type Result<T> = std::result::Result<T, EngineError>;

/// Drift report produced by [`Engine::doctor`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DriftReport {
    /// Resources in state but not in live Discord (will be re-created
    /// on next apply).
    pub missing_in_live: Vec<String>,
    /// Resources in live Discord but not in state (will be deleted on
    /// next apply if not in config).
    pub missing_in_state: Vec<String>,
    /// Resources in both but with different content.
    pub drifted: Vec<String>,
}

/// The engine. Holds a provider reference and the path to the state
/// file.
///
/// Phase 0 stub. Real implementation (Phase 3) will own the state
/// store connection pool and the executor.
pub struct Engine {
    /// Provider (injected; never `provider-discord` directly).
    pub provider: Arc<dyn Provider<Error = anyhow::Error>>,
    /// Path to the `SQLite` state file.
    pub state_path: PathBuf,
}

impl Engine {
    /// Construct a new engine.
    ///
    /// The provider is injected here. This is the **only** place in
    /// the engine layer that knows about a concrete provider type —
    /// but it accepts `dyn Provider`, so even here the engine is
    /// provider-agnostic.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Other`] if the state file cannot be
    /// opened.
    pub fn new(
        provider: Arc<dyn Provider<Error = anyhow::Error>>,
        state_path: impl Into<PathBuf>,
    ) -> Result<Self> {
        Ok(Self {
            provider,
            state_path: state_path.into(),
        })
    }

    /// Validate a config file. Parses and runs all semantic checks.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Validation`] if validation fails, or
    /// [`EngineError::Other`] for I/O or parse errors.
    pub fn validate(&self, _path: &Path) -> Result<Config> {
        // Phase 0 stub. Real implementation lands in task P3-008.
        Ok(Config {
            schema_version: None,
            server: guildforge_config::Server {
                name: String::new(),
                description: None,
            },
            roles: vec![],
            categories: vec![],
            channels: vec![],
            permissions: std::collections::BTreeMap::new(),
            permission_overwrites: vec![],
            webhooks: vec![],
            invites: vec![],
            forum_tags: std::collections::BTreeMap::new(),
            welcome_screen: None,
            server_guide: None,
            ordering: None,
        })
    }

    /// Compute an execution plan.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on validation, state, or planner errors.
    #[allow(clippy::unused_async)]  // Phase 0 stub.
    pub async fn plan(&self, _path: &Path) -> Result<ExecutionPlan> {
        // Phase 0 stub.
        Ok(ExecutionPlan::default())
    }

    /// Apply a config: plan, prompt (unless auto-approve), execute,
    /// commit state.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on any failure. Partial failures return
    /// a non-zero code via the CLI's exit-code mapping.
    #[allow(clippy::unused_async)]  // Phase 0 stub.
    pub async fn apply(&self, _path: &Path, _auto_approve: bool) -> Result<ExecutionReport> {
        // Phase 0 stub.
        Ok(ExecutionReport::default())
    }

    /// Destroy every resource described in the config (inverse of
    /// `apply`).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on any failure.
    #[allow(clippy::unused_async)]  // Phase 0 stub.
    pub async fn destroy(&self, _path: &Path, _auto_approve: bool) -> Result<ExecutionReport> {
        // Phase 0 stub.
        Ok(ExecutionReport::default())
    }

    /// Detect drift: compare state to live Discord.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on state or provider errors.
    #[allow(clippy::unused_async)]  // Phase 0 stub.
    pub async fn doctor(&self) -> Result<DriftReport> {
        // Phase 0 stub.
        Ok(DriftReport::default())
    }
}
