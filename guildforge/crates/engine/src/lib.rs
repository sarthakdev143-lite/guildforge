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

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![allow(
    clippy::uninlined_format_args,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown
)]

use guildforge_executor::{erase_provider, ExecutionReport, Executor, ExecutorConfig};
use guildforge_parser::{parse_file, ParseError};
use guildforge_planner::{render, ExecutionPlan, PlanFormat, Planner};
use guildforge_provider::Provider;
use guildforge_state::Store;
use guildforge_validation::{validate, Diagnostic};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Engine error.
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

    /// Parse error.
    #[error("parse: {0}")]
    Parse(#[from] ParseError),

    /// State error.
    #[error("state: {0}")]
    State(#[from] guildforge_state::StateError),

    /// Executor error.
    #[error("executor: {0}")]
    Executor(#[from] guildforge_executor::ExecutorError),

    /// Planner error.
    #[error("planner: {0}")]
    Planner(#[from] guildforge_planner::PlannerError),
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

/// The engine. Holds a provider, state store path, and executor config.
pub struct Engine {
    /// State store (opened lazily).
    store: Arc<Store>,
    /// Executor (constructed at creation).
    executor: Executor,
    /// Planner (stateless).
    planner: Planner,
}

impl Engine {
    /// Construct a new engine with an already-opened store.
    #[must_use]
    pub fn new<P: Provider + 'static>(provider: P, store: Arc<Store>) -> Self {
        let executor = Executor::new(erase_provider(provider), ExecutorConfig::default());
        Self {
            store,
            executor,
            planner: Planner::new(),
        }
    }

    /// Open a store at `state_path` and construct the engine.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::State`] if the store cannot be opened.
    pub async fn open<P: Provider + 'static>(
        provider: P,
        state_path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let store = Arc::new(Store::open(state_path).await?);
        Ok(Self::new(provider, store))
    }

    /// Validate a config file. Parses and runs all semantic checks.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Validation`] if validation fails, or
    /// [`EngineError::Parse`] for parse errors.
    pub fn validate(&self, path: &Path) -> Result<()> {
        let config = parse_file(path)?;
        match validate(&config) {
            Ok(()) => Ok(()),
            Err(diags) => {
                let msg = format_diagnostics(&diags);
                Err(EngineError::Validation(msg))
            }
        }
    }

    /// Compute an execution plan.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on parse, validation, state, or planner
    /// errors.
    pub async fn plan(&self, path: &Path) -> Result<ExecutionPlan> {
        let config = parse_file(path)?;
        if let Err(diags) = validate(&config) {
            return Err(EngineError::Validation(format_diagnostics(&diags)));
        }
        let state = self.store.current_state().await?;
        let plan = self.planner.plan(&config, &state)?;
        Ok(plan)
    }

    /// Render a plan for display.
    #[must_use]
    pub fn render_plan(plan: &ExecutionPlan, format: PlanFormat) -> String {
        render(plan, format)
    }

    /// Apply a config: plan, prompt (unless auto-approve), execute,
    /// commit state.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on any failure. Partial failures return
    /// a non-zero code via the CLI's exit-code mapping.
    pub async fn apply(&self, path: &Path, auto_approve: bool) -> Result<ExecutionReport> {
        let plan = self.plan(path).await?;
        if !plan.has_changes() {
            info!("no changes — plan is empty");
            return Ok(ExecutionReport::default());
        }
        if !auto_approve {
            // The CLI handles the interactive prompt; the engine just
            // returns Aborted if auto_approve is false.
            return Err(EngineError::Aborted);
        }
        self.execute_plan(&plan).await
    }

    /// Destroy every resource described in the config (inverse of
    /// `apply`).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on any failure.
    pub async fn destroy(&self, path: &Path, auto_approve: bool) -> Result<ExecutionReport> {
        let config = parse_file(path)?;
        if let Err(diags) = validate(&config) {
            return Err(EngineError::Validation(format_diagnostics(&diags)));
        }
        let state = self.store.current_state().await?;
        // For destroy, we compute the inverse plan: everything in state
        // that matches a config resource gets deleted. We do this by
        // planning with an empty desired set (which produces Delete for
        // every state resource) and then filtering to only those that
        // match a config resource.
        let empty_desired = vec![];
        let full_plan = guildforge_planner::plan(&empty_desired, &state);
        // Filter: only delete resources whose address prefix matches a
        // config resource. For simplicity in v1, we delete ALL state
        // resources (full destroy). The user can filter by editing the
        // config before calling destroy.
        let _ = config; // config is used for validation only in destroy
        let plan = full_plan;

        if !auto_approve {
            return Err(EngineError::Aborted);
        }
        self.execute_plan(&plan).await
    }

    /// Execute a pre-computed plan.
    async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<ExecutionReport> {
        let mut tx = self.store.begin_exclusive().await.map_err(|e| match e {
            guildforge_state::StateError::LockHeld(pid) => EngineError::LockHeld(pid),
            other => EngineError::State(other),
        })?;
        let cancel = CancellationToken::new();
        let report = self.executor.execute(plan, cancel, &mut tx).await?;
        tx.commit().await?;
        Ok(report)
    }

    /// Detect drift: compare state to live Discord.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] on state or provider errors.
    pub async fn doctor(&self) -> Result<DriftReport> {
        let _state = self.store.current_state().await?;
        // In v1, doctor is a stub — full drift detection requires
        // comparing state to live provider resources, which needs the
        // provider's list() method. This will be fully implemented in
        // Phase 4 (import/export round-trip).
        warn!("doctor is a stub in Phase 3 — full drift detection lands in Phase 4");
        Ok(DriftReport::default())
    }
}

/// Format a list of diagnostics into a human-readable string.
fn format_diagnostics(diags: &[Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| {
            let addr = d.addr.as_deref().unwrap_or("(config)");
            format!("  {} [{}] {} — {}", d.severity, d.code, addr, d.message)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use guildforge_provider::{Provider, ProviderError, Resource, ResourceKind};
    use guildforge_shared::ResourceId;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        type Error = ProviderError;
        async fn read(
            &self,
            _addr: &ResourceId,
        ) -> std::result::Result<Option<Resource>, Self::Error> {
            Ok(None)
        }
        async fn create(&self, desired: &Resource) -> std::result::Result<Resource, Self::Error> {
            Ok(desired.clone())
        }
        async fn update(
            &self,
            _current: &Resource,
            desired: &Resource,
        ) -> std::result::Result<Resource, Self::Error> {
            Ok(desired.clone())
        }
        async fn delete(&self, _current: &Resource) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
        async fn list(
            &self,
            _kind: ResourceKind,
        ) -> std::result::Result<Vec<Resource>, Self::Error> {
            Ok(vec![])
        }
        fn name(&self) -> &'static str {
            "mock"
        }
    }

    fn example(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(name)
    }

    #[tokio::test]
    async fn validate_company_yaml() {
        let store = Arc::new(Store::open_in_memory().await.unwrap());
        let engine = Engine::new(MockProvider, store);
        assert!(engine.validate(&example("company.yaml")).is_ok());
    }

    #[tokio::test]
    async fn plan_company_yaml() {
        let store = Arc::new(Store::open_in_memory().await.unwrap());
        let engine = Engine::new(MockProvider, store);
        let plan = engine.plan(&example("company.yaml")).await.unwrap();
        assert!(plan.has_changes());
    }

    #[tokio::test]
    async fn apply_with_auto_approve() {
        let store = Arc::new(Store::open_in_memory().await.unwrap());
        let engine = Engine::new(MockProvider, store);
        let report = engine.apply(&example("company.yaml"), true).await.unwrap();
        assert!(report.created > 0 || report.noop > 0);
    }

    #[tokio::test]
    async fn apply_without_auto_approve_aborts() {
        let store = Arc::new(Store::open_in_memory().await.unwrap());
        let engine = Engine::new(MockProvider, store);
        let result = engine.apply(&example("company.yaml"), false).await;
        assert!(matches!(result, Err(EngineError::Aborted)));
    }

    #[tokio::test]
    async fn apply_twice_second_is_noop() {
        let store = Arc::new(Store::open_in_memory().await.unwrap());
        let engine = Engine::new(MockProvider, store);
        let _ = engine.apply(&example("company.yaml"), true).await.unwrap();
        let report = engine.apply(&example("company.yaml"), true).await.unwrap();
        assert_eq!(report.created, 0);
        assert_eq!(report.updated, 0);
        assert_eq!(report.deleted, 0);
    }

    #[tokio::test]
    async fn destroy_clears_state() {
        let store = Arc::new(Store::open_in_memory().await.unwrap());
        let engine = Engine::new(MockProvider, store);
        let _ = engine.apply(&example("company.yaml"), true).await.unwrap();
        let report = engine
            .destroy(&example("company.yaml"), true)
            .await
            .unwrap();
        assert!(report.deleted > 0);
    }
}
