//! Deterministic planner for `GuildForge`.
//!
//! Computes an [`ExecutionPlan`] from a desired [`Config`] and the
//! [`CurrentState`]. The planner is **pure**: no I/O, no async, no
//! env, no clock. The same `(config, state)` pair always produces a
//! byte-identical plan.
//!
//! See [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md)
//! for the full determinism contract.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 3
//! (tasks `P3-003`, `P3-004`, `P3-005`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use guildforge_config::Config;
use guildforge_shared::ResourceId;
use guildforge_state::CurrentState;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Planner error.
#[derive(Debug, Error)]
pub enum PlannerError {
    /// The config references a resource that doesn't exist in state
    /// and cannot be created (e.g. circular dependency).
    #[error("invalid reference: {0}")]
    InvalidReference(String),

    /// The dependency graph has a cycle.
    #[error("circular dependency: {0}")]
    CircularDependency(String),
}

/// A single operation in an execution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    /// Create a resource that exists in config but not in state.
    Create {
        /// The desired resource.
        desired: ResourcePayload,
    },
    /// Update a resource that exists in both but has changed fields.
    Update {
        /// The current state of the resource.
        current: ResourcePayload,
        /// The desired state of the resource.
        desired: ResourcePayload,
    },
    /// Delete a resource that exists in state but not in config.
    Delete {
        /// The current state of the resource.
        current: ResourcePayload,
    },
    /// Reorder a resource (position changed, content unchanged).
    Reorder {
        /// Resource address.
        addr: ResourceId,
        /// New position.
        new_position: u32,
    },
    /// No change.
    Noop {
        /// The current state of the resource.
        current: ResourcePayload,
    },
}

/// A simplified resource payload for the plan output.
///
/// In Phase 3 this becomes `guildforge_provider::Resource` directly;
/// for now it's a placeholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePayload {
    /// Resource address.
    pub addr: ResourceId,
    /// Resource kind.
    pub kind: String,
}

/// An execution plan: a list of operations in topological order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Operations in topological order, then sorted by (kind, addr).
    pub operations: Vec<Operation>,
}

impl ExecutionPlan {
    /// Returns `true` if the plan has no operations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Count operations by kind.
    #[must_use]
    pub fn summary(&self) -> PlanSummary {
        let mut s = PlanSummary::default();
        for op in &self.operations {
            match op {
                Operation::Create { .. } => s.create += 1,
                Operation::Update { .. } => s.update += 1,
                Operation::Delete { .. } => s.delete += 1,
                Operation::Reorder { .. } => s.reorder += 1,
                Operation::Noop { .. } => s.noop += 1,
            }
        }
        s
    }
}

/// Plan summary counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlanSummary {
    /// Number of `Create` operations.
    pub create: u32,
    /// Number of `Update` operations.
    pub update: u32,
    /// Number of `Delete` operations.
    pub delete: u32,
    /// Number of `Reorder` operations.
    pub reorder: u32,
    /// Number of `Noop` operations.
    pub noop: u32,
}

/// The planner. Stateless; construct once and reuse.
#[derive(Debug, Clone, Default)]
pub struct Planner;

impl Planner {
    /// Construct a new planner.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Compute an execution plan from `(config, state)`.
    ///
    /// # Errors
    ///
    /// Returns [`PlannerError`] if the config references invalid
    /// resources or the dependency graph has a cycle.
    pub fn plan(
        &self,
        _config: &Config,
        _state: &CurrentState,
    ) -> Result<ExecutionPlan, PlannerError> {
        // Phase 0 stub. Real implementation lands in tasks P3-003..P3-005.
        Ok(ExecutionPlan::default())
    }
}

/// Output format for plan rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanFormat {
    /// Human-readable text (default).
    #[default]
    Text,
    /// Stable JSON for machine consumption.
    Json,
    /// SARIF for GitHub Code Scanning.
    Sarif,
    /// Markdown table for PR comments.
    Markdown,
}

/// Render a plan in the given format.
#[must_use]
pub fn render(plan: &ExecutionPlan, format: PlanFormat) -> String {
    match format {
        PlanFormat::Text => render_text(plan),
        PlanFormat::Json => serde_json::to_string_pretty(plan).unwrap_or_default(),
        PlanFormat::Sarif => {
            // Phase 3: full SARIF output.
            let summary = plan.summary();
            format!(
                "{{\"version\":\"2.1.0\",\"runs\":[{{\"results\":[{{\"message\":{{\"text\":\"plan: +{create} ~{update} -{delete} >{reorder} ={noop}\"}}}}]}}]}}",
                create = summary.create,
                update = summary.update,
                delete = summary.delete,
                reorder = summary.reorder,
                noop = summary.noop
            )
        }
        PlanFormat::Markdown => {
            let s = plan.summary();
            format!(
                "| + | ~ | - | > | = |\n|---|---|---|---|---|\n| {} | {} | {} | {} | {} |",
                s.create, s.update, s.delete, s.reorder, s.noop
            )
        }
    }
}

fn render_text(plan: &ExecutionPlan) -> String {
    if plan.is_empty() {
        return "No changes.".to_string();
    }
    let mut out = String::new();
    for op in &plan.operations {
        let (sym, addr) = match op {
            Operation::Create { desired } => ("+", desired.addr.as_str()),
            Operation::Update { current, .. } => ("~", current.addr.as_str()),
            Operation::Delete { current } => ("-", current.addr.as_str()),
            Operation::Reorder { addr, .. } => (">", addr.as_str()),
            Operation::Noop { current } => ("=", current.addr.as_str()),
        };
        out.push_str(sym);
        out.push(' ');
        out.push_str(addr);
        out.push('\n');
    }
    let s = plan.summary();
    out.push_str("\nPlan: +");
    out.push_str(&s.create.to_string());
    out.push_str(" ~");
    out.push_str(&s.update.to_string());
    out.push_str(" -");
    out.push_str(&s.delete.to_string());
    out.push_str(" >");
    out.push_str(&s.reorder.to_string());
    out.push_str(" =");
    out.push_str(&s.noop.to_string());
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_plan_renders_no_changes() {
        let plan = ExecutionPlan::default();
        assert_eq!(render(&plan, PlanFormat::Text), "No changes.");
    }

    #[test]
    fn summary_counts() {
        let plan = ExecutionPlan {
            operations: vec![
                Operation::Create {
                    desired: ResourcePayload {
                        addr: ResourceId::new("role/Admin"),
                        kind: "role".to_string(),
                    },
                },
                Operation::Noop {
                    current: ResourcePayload {
                        addr: ResourceId::new("role/Staff"),
                        kind: "role".to_string(),
                    },
                },
            ],
        };
        let s = plan.summary();
        assert_eq!(s.create, 1);
        assert_eq!(s.noop, 1);
        assert_eq!(s.update, 0);
    }

    #[test]
    fn json_format_is_stable() {
        let plan = ExecutionPlan::default();
        let json1 = render(&plan, PlanFormat::Json);
        let json2 = render(&plan, PlanFormat::Json);
        assert_eq!(json1, json2);
    }
}
