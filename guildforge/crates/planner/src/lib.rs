//! Deterministic planner for `GuildForge`.
//!
//! Computes an [`ExecutionPlan`] from a desired [`Config`] and the
//! [`CurrentState`]. The planner is **pure**: no I/O, no async, no
//! env, no clock. The same `(config, state)` pair always produces a
//! byte-identical plan.
//!
//! # Pipeline
//!
//! 1. Convert the [`Config`] into a set of desired [`Resource`]s
//!    (see [`config_to_resources`]).
//! 2. For each desired resource, look up the corresponding resource in
//!    [`CurrentState`].
//! 3. Compare content hashes. Emit `Create` / `Update` / `Noop`.
//! 4. For each state resource not in the config, emit `Delete`.
//! 5. Sort operations by topological level, then by `(kind, addr)`.
//!
//! See [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md)
//! for the full determinism contract.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![allow(
    clippy::uninlined_format_args,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::match_same_arms,
    clippy::too_many_lines,
    clippy::manual_map,
    clippy::needless_pass_by_value,
    clippy::large_enum_variant,
    clippy::new_without_default,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::option_map_unit_fn,
    clippy::redundant_closure_for_method_calls,
    clippy::ref_option,
    clippy::unnecessary_wraps,
    clippy::manual_strip
)]

mod convert;
mod diff;
mod render;

pub use convert::config_to_resources;
pub use diff::plan;
pub use render::{render, render_json, render_text, PlanFormat};

use guildforge_shared::ResourceId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Planner error.
#[derive(Debug, Error)]
pub enum PlannerError {
    /// The config references a resource that doesn't exist in state
    /// and cannot be created (e.g. circular dependency).
    #[error("invalid reference: {0}")]
    InvalidReference(String),
}

/// A single operation in an execution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    /// Create a resource that exists in config but not in state.
    Create {
        /// The desired resource.
        desired: guildforge_provider::Resource,
    },
    /// Update a resource that exists in both but has changed fields.
    Update {
        /// The current state of the resource.
        current: guildforge_provider::Resource,
        /// The desired state of the resource.
        desired: guildforge_provider::Resource,
    },
    /// Delete a resource that exists in state but not in config.
    Delete {
        /// The current state of the resource.
        current: guildforge_provider::Resource,
    },
    /// No change.
    Noop {
        /// The current state of the resource.
        current: guildforge_provider::Resource,
    },
}

impl Operation {
    /// Get the address of the resource this operation acts on.
    #[must_use]
    pub fn addr(&self) -> &ResourceId {
        match self {
            Self::Create { desired } => desired.addr(),
            Self::Update { current, .. } => current.addr(),
            Self::Delete { current } => current.addr(),
            Self::Noop { current } => current.addr(),
        }
    }

    /// Get the symbol for this operation (`+`, `~`, `-`, `=`).
    #[must_use]
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Create { .. } => "+",
            Self::Update { .. } => "~",
            Self::Delete { .. } => "-",
            Self::Noop { .. } => "=",
        }
    }
}

/// An execution plan: a list of operations in topological order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Operations sorted by topological level, then by `(kind, addr)`.
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
                Operation::Noop { .. } => s.noop += 1,
            }
        }
        s
    }

    /// Returns `true` if the plan has no changes (all Noop or empty).
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.operations
            .iter()
            .any(|op| !matches!(op, Operation::Noop { .. }))
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
    /// Number of `Noop` operations.
    pub noop: u32,
}

impl std::fmt::Display for PlanSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "+{} ~{} -{} ={}",
            self.create, self.update, self.delete, self.noop
        )
    }
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
    /// Returns [`PlannerError`] if the config cannot be converted to
    /// resources.
    pub fn plan(
        &self,
        config: &guildforge_config::Config,
        state: &guildforge_state::CurrentState,
    ) -> Result<ExecutionPlan, PlannerError> {
        let desired = config_to_resources(config)?;
        Ok(plan(&desired, state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guildforge_config::Config;
    use guildforge_provider::RoleResource;
    use guildforge_state::CurrentState;

    fn config_from_yaml(yaml: &str) -> Config {
        serde_yaml::from_str(yaml).unwrap()
    }

    fn empty_state() -> CurrentState {
        CurrentState::default()
    }

    #[test]
    fn empty_config_empty_state_produces_empty_plan() {
        let cfg = config_from_yaml("server:\n  name: Test\n");
        let state = empty_state();
        let plan = Planner::new().plan(&cfg, &state).unwrap();
        assert!(plan.is_empty());
    }

    #[test]
    fn new_role_produces_create() {
        let cfg = config_from_yaml(
            "server:\n  name: Test\nroles:\n  - name: Admin\n    color: red\n    permissions: [administrator]\n",
        );
        let state = empty_state();
        let plan = Planner::new().plan(&cfg, &state).unwrap();
        let s = plan.summary();
        assert!(s.create >= 1);
        assert_eq!(s.delete, 0);
    }

    #[test]
    fn matching_role_produces_noop() {
        let cfg = config_from_yaml(
            "server:\n  name: Test\nroles:\n  - name: Admin\n    permissions: [administrator]\n",
        );
        let desired = config_to_resources(&cfg).unwrap();
        let mut state = CurrentState::default();
        for r in &desired {
            let record =
                guildforge_state::ResourceRecord::from_resource(r, "discord", false).unwrap();
            state.resources.insert(record.addr.clone(), record);
        }
        let plan = Planner::new().plan(&cfg, &state).unwrap();
        let s = plan.summary();
        assert_eq!(s.create, 0);
        assert_eq!(s.update, 0);
        assert_eq!(s.delete, 0);
        assert!(s.noop >= 1);
    }

    #[test]
    fn state_only_role_produces_delete() {
        let cfg = config_from_yaml("server:\n  name: Test\n");
        let mut state = CurrentState::default();
        let role = RoleResource::new("role/old", "old");
        let record = guildforge_state::ResourceRecord::from_resource(
            &guildforge_provider::Resource::Role(role),
            "discord",
            false,
        )
        .unwrap();
        state.resources.insert(record.addr.clone(), record);

        let plan = Planner::new().plan(&cfg, &state).unwrap();
        let s = plan.summary();
        assert!(s.delete >= 1);
        assert_eq!(s.create, 0);
    }

    #[test]
    fn plan_is_deterministic() {
        let cfg = config_from_yaml(
            "server:\n  name: Test\nroles:\n  - name: A\n  - name: B\nchannels:\n  - name: c1\n    type: text\n",
        );
        let state = empty_state();
        let p1 = Planner::new().plan(&cfg, &state).unwrap();
        let p2 = Planner::new().plan(&cfg, &state).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn summary_display() {
        let s = PlanSummary {
            create: 3,
            update: 1,
            delete: 0,
            noop: 5,
        };
        assert_eq!(format!("{s}"), "+3 ~1 -0 =5");
    }

    #[test]
    fn operation_symbol() {
        let role = guildforge_provider::Resource::Role(RoleResource::new("role/A", "A"));
        assert_eq!(
            Operation::Create {
                desired: role.clone()
            }
            .symbol(),
            "+"
        );
        assert_eq!(
            Operation::Update {
                current: role.clone(),
                desired: role.clone()
            }
            .symbol(),
            "~"
        );
        assert_eq!(
            Operation::Delete {
                current: role.clone()
            }
            .symbol(),
            "-"
        );
        assert_eq!(Operation::Noop { current: role }.symbol(), "=");
    }
}
