//! Plan executor for `GuildForge`.
//!
//! Walks an [`ExecutionPlan`] in topological order, invoking the
//! [`Provider`](guildforge_provider::Provider) for each operation,
//! and persists state changes.
//!
//! See [`ADR-0007`](../../docs/adr/ADR-0007-idempotency-ordering.md)
//! for the full idempotency, retry, partial-failure, and cancellation
//! contract.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 3
//! (tasks `P3-006`, `P3-007`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use guildforge_planner::ExecutionPlan;
use guildforge_shared::ResourceId;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

/// Executor error.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// The provider returned a permanent error and the operation could
    /// not be completed.
    #[error("permanent failure on {addr}: {message}")]
    Permanent {
        /// Failing resource address.
        addr: ResourceId,
        /// Error message.
        message: String,
    },

    /// The user canceled the operation.
    #[error("canceled")]
    Canceled,

    /// The state commit failed.
    #[error("state commit failed: {0}")]
    StateCommit(String),
}

/// Result of a single operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum OperationResult {
    /// Operation succeeded.
    Success {
        /// Resource address.
        addr: ResourceId,
        /// Duration in milliseconds.
        duration_ms: u64,
    },
    /// Operation failed permanently.
    Failure {
        /// Resource address.
        addr: ResourceId,
        /// Error message.
        error: String,
        /// Number of retries attempted.
        retries: u32,
    },
    /// Operation skipped because an upstream dependency failed.
    Skipped {
        /// Resource address.
        addr: ResourceId,
        /// Why the operation was skipped.
        reason: String,
    },
}

/// Execution report. Returned by [`Executor::execute`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// Number of successful creates.
    pub created: u32,
    /// Number of successful updates.
    pub updated: u32,
    /// Number of successful deletes.
    pub deleted: u32,
    /// Number of successful reorders.
    pub reordered: u32,
    /// Number of no-ops.
    pub noop: u32,
    /// Number of failures.
    pub failed: u32,
    /// Number of skipped operations.
    pub skipped: u32,
    /// Number of tainted resources.
    pub tainted: u32,
    /// Per-operation results.
    pub operations: Vec<OperationResult>,
    /// Start time (RFC 3339).
    pub started_at: String,
    /// End time (RFC 3339).
    pub ended_at: String,
}

/// Executor configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorConfig {
    /// Maximum concurrent operations (default 1).
    pub max_concurrency: usize,
    /// Maximum retries per operation (default 2; initial + 2 = 3 attempts).
    pub max_retries: u32,
    /// Initial retry backoff (default 1s).
    pub initial_backoff: Duration,
    /// Maximum retry backoff (default 30s).
    pub max_backoff: Duration,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 1,
            max_retries: 2,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
        }
    }
}

/// The executor. Holds a provider reference and state store.
///
/// Phase 0 stub. Real implementation (Phase 3) will hold
/// `Arc<dyn Provider>` and a state store handle.
pub struct Executor {
    /// Executor configuration.
    pub config: ExecutorConfig,
}

impl Executor {
    /// Construct a new executor with the given config.
    #[must_use]
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    /// Execute an [`ExecutionPlan`].
    ///
    /// Walks the plan in order, invoking the provider for each op.
    /// Honors the `cancel` token between operations.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutorError::Canceled`] if the cancellation token
    /// fires, or [`ExecutorError::Permanent`] on a permanent failure
    /// (after which the executor continues with independent operations
    /// and returns the report at the end).
    #[allow(clippy::unused_async)] // Phase 0 stub; real impl awaits provider calls.
    pub async fn execute(
        &self,
        _plan: &ExecutionPlan,
        _cancel: CancellationToken,
    ) -> Result<ExecutionReport, ExecutorError> {
        // Phase 0 stub. Real implementation lands in task P3-006.
        Ok(ExecutionReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_execute_returns_empty_report() {
        let exec = Executor::new(ExecutorConfig::default());
        let plan = ExecutionPlan::default();
        let cancel = CancellationToken::new();
        let report = exec.execute(&plan, cancel).await.unwrap();
        assert_eq!(report.created, 0);
        assert_eq!(report.failed, 0);
    }

    #[test]
    fn default_config_is_sequential() {
        let c = ExecutorConfig::default();
        assert_eq!(c.max_concurrency, 1);
        assert_eq!(c.max_retries, 2);
    }
}
