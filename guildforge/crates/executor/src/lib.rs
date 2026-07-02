//! Plan executor for `GuildForge`.
//!
//! Walks an [`ExecutionPlan`] in topological order, invoking the
//! [`Provider`](guildforge_provider::Provider) for each operation, and
//! persists state changes.
//!
//! See [`ADR-0007`](../../docs/adr/ADR-0007-idempotency-ordering.md)
//! for the full idempotency, retry, partial-failure, and cancellation
//! contract.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![allow(
    clippy::uninlined_format_args,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::too_many_lines,
    clippy::collapsible_match,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::uninlined_format_args
)]

use guildforge_planner::ExecutionPlan;
use guildforge_provider::{Provider, ProviderError, Resource};
use guildforge_shared::ResourceId;
use guildforge_state::{ResourceRecord, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

// ===========================================================================
// Errors
// ===========================================================================

/// Executor error.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// A permanent failure on a specific resource.
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

    /// The state transaction failed.
    #[error("state: {0}")]
    State(#[from] guildforge_state::StateError),
}

// ===========================================================================
// Operation result + report
// ===========================================================================

/// Result of a single operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum OperationResult {
    /// Operation succeeded.
    Success {
        /// Resource address.
        addr: String,
        /// Duration in milliseconds.
        duration_ms: u64,
    },
    /// Operation failed permanently.
    Failure {
        /// Resource address.
        addr: String,
        /// Error message.
        error: String,
        /// Number of retries attempted.
        retries: u32,
    },
    /// Operation skipped because an upstream dependency failed.
    Skipped {
        /// Resource address.
        addr: String,
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
}

impl ExecutionReport {
    /// Returns `true` if there were any failures or skipped ops.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.failed > 0 || self.skipped > 0
    }
}

impl std::fmt::Display for ExecutionReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "+{} ~{} -{} ={} (failed: {}, skipped: {}, tainted: {})",
            self.created,
            self.updated,
            self.deleted,
            self.noop,
            self.failed,
            self.skipped,
            self.tainted
        )
    }
}

// ===========================================================================
// Config
// ===========================================================================

/// Executor configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorConfig {
    /// Maximum retries per operation (default 2; initial + 2 = 3 attempts).
    pub max_retries: u32,
    /// Initial retry backoff (default 1s).
    pub initial_backoff: Duration,
    /// Maximum retry backoff (default 30s).
    pub max_backoff: Duration,
    /// Provider name (for state records).
    pub provider_name: String,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
            provider_name: "discord".to_string(),
        }
    }
}

// ===========================================================================
// Executor
// ===========================================================================

/// The executor. Holds a provider reference.
pub struct Executor {
    /// Provider (injected; never `provider-discord` directly in the
    /// engine layer).
    provider: Arc<dyn DynProvider>,
    /// Executor configuration.
    pub config: ExecutorConfig,
}

/// Type-erased provider trait object. Internal helper so the executor
/// can hold `Arc<dyn DynProvider>` instead of dealing with associated
/// types.
#[async_trait::async_trait]
pub trait DynProvider: Send + Sync {
    /// Read a resource.
    async fn read(&self, addr: &ResourceId) -> Result<Option<Resource>, ProviderError>;
    /// Create a resource.
    async fn create(&self, desired: &Resource) -> Result<Resource, ProviderError>;
    /// Update a resource.
    async fn update(
        &self,
        current: &Resource,
        desired: &Resource,
    ) -> Result<Resource, ProviderError>;
    /// Delete a resource.
    async fn delete(&self, current: &Resource) -> Result<(), ProviderError>;
    /// Reorder a resource.
    async fn reorder(&self, addr: &ResourceId, new_position: u32) -> Result<(), ProviderError>;
}

/// Wrap any `Provider` in a `DynProvider` by erasing the error type.
struct DynProviderWrapper<P: Provider> {
    inner: P,
}

#[async_trait::async_trait]
impl<P: Provider> DynProvider for DynProviderWrapper<P> {
    async fn read(&self, addr: &ResourceId) -> Result<Option<Resource>, ProviderError> {
        self.inner
            .read(addr)
            .await
            .map_err(|e| ProviderError::Permanent(e.to_string()))
    }
    async fn create(&self, desired: &Resource) -> Result<Resource, ProviderError> {
        self.inner
            .create(desired)
            .await
            .map_err(|e| ProviderError::Permanent(e.to_string()))
    }
    async fn update(
        &self,
        current: &Resource,
        desired: &Resource,
    ) -> Result<Resource, ProviderError> {
        self.inner
            .update(current, desired)
            .await
            .map_err(|e| ProviderError::Permanent(e.to_string()))
    }
    async fn delete(&self, current: &Resource) -> Result<(), ProviderError> {
        self.inner
            .delete(current)
            .await
            .map_err(|e| ProviderError::Permanent(e.to_string()))
    }
    async fn reorder(&self, addr: &ResourceId, new_position: u32) -> Result<(), ProviderError> {
        self.inner
            .reorder(addr, new_position)
            .await
            .map_err(|e| ProviderError::Permanent(e.to_string()))
    }
}

/// Wrap a `Provider` into a `DynProvider` suitable for the executor.
#[must_use]
pub fn erase_provider<P: Provider + 'static>(provider: P) -> Arc<dyn DynProvider> {
    Arc::new(DynProviderWrapper { inner: provider })
}

impl Executor {
    /// Construct a new executor.
    #[must_use]
    pub fn new(provider: Arc<dyn DynProvider>, config: ExecutorConfig) -> Self {
        Self { provider, config }
    }

    /// Execute an [`ExecutionPlan`].
    ///
    /// Walks the plan in order, invoking the provider for each op.
    /// Honors the `cancel` token between operations. On failure, marks
    /// the resource tainted and continues with independent operations.
    /// Commits state at the end (even on partial failure).
    ///
    /// # Errors
    ///
    /// Returns [`ExecutorError::Canceled`] if the cancellation token
    /// fires before all operations are processed.
    pub async fn execute(
        &self,
        plan: &ExecutionPlan,
        cancel: CancellationToken,
        tx: &mut Transaction,
    ) -> Result<ExecutionReport, ExecutorError> {
        let mut report = ExecutionReport::default();
        let mut failed_addrs: Vec<ResourceId> = Vec::new();

        for op in &plan.operations {
            // Check for cancellation between ops.
            if cancel.is_cancelled() {
                return Err(ExecutorError::Canceled);
            }

            let addr = op.addr().clone();
            let start = std::time::Instant::now();

            // Skip if an upstream dependency failed.
            if failed_addrs
                .iter()
                .any(|f| addr.as_str().starts_with(f.as_str()))
            {
                report.skipped += 1;
                report.operations.push(OperationResult::Skipped {
                    addr: addr.to_string(),
                    reason: "upstream dependency failed".to_string(),
                });
                continue;
            }

            let result = self.execute_op(op, &mut report, tx).await;
            match result {
                Ok(()) => {}
                Err(ExecutorError::Permanent {
                    addr: fail_addr, ..
                }) => {
                    failed_addrs.push(fail_addr.clone());
                    // Mark tainted in state.
                    if let Err(e) = tx.taint(&addr).await {
                        warn!(%addr, error = %e, "could not mark tainted");
                    }
                    report.tainted += 1;
                }
                Err(e) => return Err(e),
            }

            debug!(%addr, elapsed = ?start.elapsed(), "op complete");
        }

        // Log the migration.
        let summary = format!("{report}");
        let plan_hash =
            guildforge_shared::Hash::of(serde_json::to_string(plan).unwrap_or_default().as_bytes());
        if let Err(e) = tx.log_migration(&plan_hash.to_string(), &summary).await {
            warn!(error = %e, "could not log migration");
        }

        info!(%report, "execution complete");
        Ok(report)
    }

    async fn execute_op(
        &self,
        op: &guildforge_planner::Operation,
        report: &mut ExecutionReport,
        tx: &mut Transaction,
    ) -> Result<(), ExecutorError> {
        use guildforge_planner::Operation;
        let addr = op.addr().clone();

        // Handle Noop separately — no provider call needed.
        if let Operation::Noop { .. } = op {
            report.noop += 1;
            report.operations.push(OperationResult::Success {
                addr: addr.to_string(),
                duration_ms: 0,
            });
            return Ok(());
        }

        let mut retries = 0u32;
        loop {
            let result = match op {
                Operation::Create { desired } => self.provider.create(desired).await,
                Operation::Update { current, desired } => {
                    self.provider.update(current, desired).await
                }
                Operation::Delete { current } => {
                    let current_clone = current.clone();
                    self.provider.delete(current).await.map(|()| current_clone)
                }
                Operation::Noop { .. } => unreachable!(),
            };

            match result {
                Ok(resource) => {
                    // Persist the result to state.
                    match op {
                        Operation::Delete { .. } => {
                            tx.delete(&addr).await.ok();
                            report.deleted += 1;
                        }
                        Operation::Create { .. } | Operation::Update { .. } => {
                            let rec = ResourceRecord::from_resource(
                                &resource,
                                &self.config.provider_name,
                                false,
                            )
                            .unwrap_or_else(|_| ResourceRecord {
                                addr: addr.clone(),
                                kind: "unknown".into(),
                                provider: self.config.provider_name.clone(),
                                data: "{}".into(),
                                content_hash: guildforge_shared::Hash::of(b"{}"),
                                tainted: false,
                                updated_at: guildforge_shared::now(),
                            });
                            tx.upsert(&rec).await.ok();
                            if matches!(op, Operation::Create { .. }) {
                                report.created += 1;
                            } else {
                                report.updated += 1;
                            }
                        }
                        _ => {}
                    }
                    report.operations.push(OperationResult::Success {
                        addr: addr.to_string(),
                        duration_ms: 0,
                    });
                    return Ok(());
                }
                Err(e) => match e {
                    ProviderError::Transient(msg) => {
                        if retries < self.config.max_retries {
                            retries += 1;
                            let backoff = self.backoff_for(retries);
                            warn!(%addr, retry = retries, %msg, ?backoff, "transient error, retrying");
                            tokio::time::sleep(backoff).await;
                            continue;
                        }
                        report.failed += 1;
                        report.operations.push(OperationResult::Failure {
                            addr: addr.to_string(),
                            error: msg,
                            retries,
                        });
                        return Err(ExecutorError::Permanent {
                            addr,
                            message: format!("transient error after {retries} retries"),
                        });
                    }
                    ProviderError::Conflict(msg) => {
                        if retries == 0 {
                            retries = 1;
                            warn!(%addr, %msg, "conflict, retrying once");
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            continue;
                        }
                        report.failed += 1;
                        report.operations.push(OperationResult::Failure {
                            addr: addr.to_string(),
                            error: msg,
                            retries,
                        });
                        return Err(ExecutorError::Permanent {
                            addr,
                            message: "conflict".to_string(),
                        });
                    }
                    ProviderError::Auth(msg) => {
                        report.failed += 1;
                        report.operations.push(OperationResult::Failure {
                            addr: addr.to_string(),
                            error: msg.clone(),
                            retries,
                        });
                        return Err(ExecutorError::Permanent { addr, message: msg });
                    }
                    ProviderError::Permanent(msg)
                    | ProviderError::Decode(msg)
                    | ProviderError::Unsupported(msg) => {
                        report.failed += 1;
                        report.operations.push(OperationResult::Failure {
                            addr: addr.to_string(),
                            error: msg.clone(),
                            retries,
                        });
                        return Err(ExecutorError::Permanent { addr, message: msg });
                    }
                },
            }
        }
    }

    fn backoff_for(&self, attempt: u32) -> Duration {
        let multiplier = 2u32.saturating_pow(attempt.saturating_sub(1));
        let base = self
            .config
            .initial_backoff
            .checked_mul(multiplier)
            .unwrap_or(self.config.max_backoff);
        std::cmp::min(base, self.config.max_backoff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use guildforge_planner::Operation;
    use guildforge_provider::{Provider, ProviderError, ResourceKind};
    use guildforge_state::Store;

    /// A mock provider that records calls and can be configured to fail.
    struct MockProvider {
        create_count: std::sync::atomic::AtomicU32,
        fail_on_create: bool,
    }

    impl MockProvider {
        fn new() -> Self {
            Self {
                create_count: std::sync::atomic::AtomicU32::new(0),
                fail_on_create: false,
            }
        }

        fn failing() -> Self {
            Self {
                create_count: std::sync::atomic::AtomicU32::new(0),
                fail_on_create: true,
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        type Error = ProviderError;

        async fn read(&self, _addr: &ResourceId) -> Result<Option<Resource>, Self::Error> {
            Ok(None)
        }
        async fn create(&self, desired: &Resource) -> Result<Resource, Self::Error> {
            self.create_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if self.fail_on_create {
                return Err(ProviderError::Permanent("mock failure".to_string()));
            }
            Ok(desired.clone())
        }
        async fn update(
            &self,
            _current: &Resource,
            desired: &Resource,
        ) -> Result<Resource, Self::Error> {
            Ok(desired.clone())
        }
        async fn delete(&self, _current: &Resource) -> Result<(), Self::Error> {
            Ok(())
        }
        async fn reorder(&self, _addr: &ResourceId, _new_position: u32) -> Result<(), Self::Error> {
            Ok(())
        }
        async fn list(&self, _kind: ResourceKind) -> Result<Vec<Resource>, Self::Error> {
            Ok(vec![])
        }
        fn name(&self) -> &'static str {
            "mock"
        }
    }

    async fn make_executor(provider: MockProvider) -> (Executor, Arc<Store>) {
        let store = Arc::new(Store::open_in_memory().await.unwrap());
        let dyn_provider = erase_provider(provider);
        let exec = Executor::new(
            dyn_provider,
            ExecutorConfig {
                max_retries: 0,
                initial_backoff: Duration::from_millis(1),
                max_backoff: Duration::from_millis(10),
                provider_name: "mock".to_string(),
            },
        );
        (exec, store)
    }

    #[tokio::test]
    async fn execute_create_succeeds() {
        let (exec, store) = make_executor(MockProvider::new()).await;
        let mut tx = store.begin_exclusive().await.unwrap();
        let plan = ExecutionPlan {
            operations: vec![Operation::Create {
                desired: Resource::Role(guildforge_provider::RoleResource::new("role/A", "A")),
            }],
        };
        let report = exec
            .execute(&plan, CancellationToken::new(), &mut tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(report.created, 1);
        assert_eq!(report.failed, 0);
    }

    #[tokio::test]
    async fn execute_failure_marks_tainted() {
        let (exec, store) = make_executor(MockProvider::failing()).await;
        let mut tx = store.begin_exclusive().await.unwrap();
        let plan = ExecutionPlan {
            operations: vec![Operation::Create {
                desired: Resource::Role(guildforge_provider::RoleResource::new("role/A", "A")),
            }],
        };
        let result = exec.execute(&plan, CancellationToken::new(), &mut tx).await;
        tx.commit().await.unwrap();
        // The executor returns Ok with a report (partial failure), not Err.
        assert!(result.is_ok());
        let report = result.unwrap();
        assert_eq!(report.failed, 1);
        assert_eq!(report.tainted, 1);
    }

    #[tokio::test]
    async fn execute_noop_passes_through() {
        let (exec, store) = make_executor(MockProvider::new()).await;
        let mut tx = store.begin_exclusive().await.unwrap();
        let plan = ExecutionPlan {
            operations: vec![Operation::Noop {
                current: Resource::Role(guildforge_provider::RoleResource::new("role/A", "A")),
            }],
        };
        let report = exec
            .execute(&plan, CancellationToken::new(), &mut tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(report.noop, 1);
        assert_eq!(report.created, 0);
    }

    #[tokio::test]
    async fn execute_cancellation_returns_canceled() {
        let (exec, store) = make_executor(MockProvider::new()).await;
        let mut tx = store.begin_exclusive().await.unwrap();
        let plan = ExecutionPlan {
            operations: vec![
                Operation::Noop {
                    current: Resource::Role(guildforge_provider::RoleResource::new("role/A", "A")),
                },
                Operation::Noop {
                    current: Resource::Role(guildforge_provider::RoleResource::new("role/B", "B")),
                },
            ],
        };
        let token = CancellationToken::new();
        token.cancel();
        let result = exec.execute(&plan, token.clone(), &mut tx).await;
        tx.rollback();
        assert!(matches!(result, Err(ExecutorError::Canceled)));
    }

    #[tokio::test]
    async fn execute_delete_removes_from_state() {
        let (exec, store) = make_executor(MockProvider::new()).await;
        // Seed state with a resource.
        {
            let mut tx = store.begin_exclusive().await.unwrap();
            let role = Resource::Role(guildforge_provider::RoleResource::new("role/old", "old"));
            let rec = ResourceRecord::from_resource(&role, "mock", false).unwrap();
            tx.upsert(&rec).await.unwrap();
            tx.commit().await.unwrap();
        }
        // Delete it.
        let mut tx = store.begin_exclusive().await.unwrap();
        let plan = ExecutionPlan {
            operations: vec![Operation::Delete {
                current: Resource::Role(guildforge_provider::RoleResource::new("role/old", "old")),
            }],
        };
        let report = exec
            .execute(&plan, CancellationToken::new(), &mut tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(report.deleted, 1);
        assert!(store
            .get(&ResourceId::new("role/old"))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn backoff_grows() {
        let _store = Arc::new(Store::open_in_memory().await.unwrap());
        let exec = Executor::new(
            erase_provider(MockProvider::new()),
            ExecutorConfig {
                max_retries: 5,
                initial_backoff: Duration::from_secs(1),
                max_backoff: Duration::from_secs(30),
                provider_name: "mock".to_string(),
            },
        );
        assert_eq!(exec.backoff_for(1), Duration::from_secs(1));
        assert_eq!(exec.backoff_for(2), Duration::from_secs(2));
        assert_eq!(exec.backoff_for(3), Duration::from_secs(4));
        assert_eq!(exec.backoff_for(10), Duration::from_secs(30));
    }
}
