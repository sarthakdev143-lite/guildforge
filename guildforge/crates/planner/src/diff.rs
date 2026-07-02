//! Deterministic diff algorithm.
//!
//! Compares desired resources against current state and produces an
//! [`ExecutionPlan`] sorted by topological level, then by `(kind, addr)`.
//!
//! See [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md).

use crate::{ExecutionPlan, Operation};
use guildforge_provider::Resource;
use guildforge_shared::ResourceId;
use guildforge_state::CurrentState;
use std::collections::BTreeMap;

/// Compute an [`ExecutionPlan`] from `(desired, state)`.
///
/// The plan is sorted by:
/// 1. Topological level (roles → categories → channels → overwrites →
///    webhooks → invites → tags).
/// 2. Within a level, by `(kind, addr)` lexicographically.
///
/// Tainted resources in state are treated as "needs recreate": the
/// planner emits `Delete` + `Create` instead of `Update`.
#[must_use]
pub fn plan(desired: &[Resource], state: &CurrentState) -> ExecutionPlan {
    // Build a lookup of desired resources by address.
    let desired_map: BTreeMap<&ResourceId, &Resource> =
        desired.iter().map(|r| (r.addr(), r)).collect();

    // Build a lookup of state resources by address (already a BTreeMap).
    // We need the decoded Resource from state records.
    let state_map: BTreeMap<&ResourceId, Resource> = state
        .resources
        .iter()
        .filter_map(|(addr, rec)| rec.to_resource().ok().map(|r| (addr, r)))
        .collect();

    let mut ops: Vec<Operation> = Vec::new();

    // Pass 1: desired resources — Create / Update / Noop.
    for (addr, desired_r) in &desired_map {
        if let Some(state_r) = state_map.get(addr) {
            // Resource exists in both. Check if it's tainted.
            let tainted = state
                .resources
                .get(*addr)
                .map(|r| r.tainted)
                .unwrap_or(false);
            if tainted {
                // Tainted: delete and recreate.
                ops.push(Operation::Delete {
                    current: state_r.clone(),
                });
                ops.push(Operation::Create {
                    desired: (*desired_r).clone(),
                });
            } else if content_differs(state_r, desired_r) {
                ops.push(Operation::Update {
                    current: state_r.clone(),
                    desired: (*desired_r).clone(),
                });
            } else {
                ops.push(Operation::Noop {
                    current: state_r.clone(),
                });
            }
        } else {
            // Resource exists in config but not in state → Create.
            ops.push(Operation::Create {
                desired: (*desired_r).clone(),
            });
        }
    }

    // Pass 2: state resources not in desired → Delete.
    for (addr, state_r) in &state_map {
        if !desired_map.contains_key(addr) {
            ops.push(Operation::Delete {
                current: state_r.clone(),
            });
        }
    }

    // Sort by topological level, then by (kind, addr).
    ops.sort_by(|a, b| {
        let ka = kind_level(a.addr());
        let kb = kind_level(b.addr());
        ka.cmp(&kb)
            .then_with(|| kind_str(a.addr()).cmp(kind_str(b.addr())))
            .then_with(|| a.addr().cmp(b.addr()))
    });

    ExecutionPlan { operations: ops }
}

/// Compare two resources for content differences.
///
/// We compare the content hash (blake3 of the JSON serialization,
/// excluding the `addr` field which is identity). This is fast and
/// deterministic.
fn content_differs(current: &Resource, desired: &Resource) -> bool {
    current.content_hash() != desired.content_hash()
}

/// Extract the kind prefix from an address (e.g. `role` from
/// `role/Admin`).
fn kind_str(addr: &ResourceId) -> &str {
    addr.as_str().split('/').next().unwrap_or("")
}

/// Topological level for a resource kind.
///
/// Lower levels are applied first:
/// - 0: roles
/// - 1: categories
/// - 2: channels
/// - 3: overwrites, tags
/// - 4: webhooks, invites
fn kind_level(addr: &ResourceId) -> u8 {
    match kind_str(addr) {
        "role" => 0,
        "category" => 1,
        "channel" => 2,
        "overwrite" | "tag" => 3,
        "webhook" | "invite" => 4,
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guildforge_provider::RoleResource;
    use guildforge_state::{CurrentState, ResourceRecord};

    fn make_state(resources: &[Resource]) -> CurrentState {
        let mut state = CurrentState::default();
        for r in resources {
            let rec = ResourceRecord::from_resource(r, "discord", false).unwrap();
            state.resources.insert(rec.addr.clone(), rec);
        }
        state
    }

    #[test]
    fn empty_desired_empty_state_no_ops() {
        let plan = plan(&[], &CurrentState::default());
        assert!(plan.is_empty());
    }

    #[test]
    fn desired_not_in_state_creates() {
        let role = Resource::Role(RoleResource::new("role/Admin", "Admin"));
        let plan = plan(&[role], &CurrentState::default());
        assert_eq!(plan.operations.len(), 1);
        assert!(matches!(plan.operations[0], Operation::Create { .. }));
    }

    #[test]
    fn state_not_in_desired_deletes() {
        let role = Resource::Role(RoleResource::new("role/old", "old"));
        let state = make_state(&[role]);
        let plan = plan(&[], &state);
        assert_eq!(plan.operations.len(), 1);
        assert!(matches!(plan.operations[0], Operation::Delete { .. }));
    }

    #[test]
    fn matching_resource_is_noop() {
        let role = Resource::Role(RoleResource::new("role/Admin", "Admin"));
        let state = make_state(&[role.clone()]);
        let plan = plan(&[role], &state);
        assert_eq!(plan.operations.len(), 1);
        assert!(matches!(plan.operations[0], Operation::Noop { .. }));
    }

    #[test]
    fn changed_resource_updates() {
        let role1 = Resource::Role(RoleResource::new("role/Admin", "Admin"));
        let role2 = Resource::Role(RoleResource {
            name: "Admin".into(),
            permissions: 8,
            ..RoleResource::new("role/Admin", "Admin")
        });
        let state = make_state(&[role1]);
        let plan = plan(&[role2], &state);
        assert_eq!(plan.operations.len(), 1);
        assert!(matches!(plan.operations[0], Operation::Update { .. }));
    }

    #[test]
    fn tainted_resource_is_deleted_and_recreated() {
        let role = Resource::Role(RoleResource::new("role/Admin", "Admin"));
        let mut state = make_state(&[role.clone()]);
        // Mark as tainted.
        state
            .resources
            .get_mut(&ResourceId::new("role/Admin"))
            .unwrap()
            .tainted = true;
        let plan = plan(&[role], &state);
        assert_eq!(plan.operations.len(), 2);
        assert!(matches!(plan.operations[0], Operation::Delete { .. }));
        assert!(matches!(plan.operations[1], Operation::Create { .. }));
    }

    #[test]
    fn plan_is_sorted_by_topological_level() {
        let desired = vec![
            Resource::Role(RoleResource::new("role/A", "A")),
            Resource::Role(RoleResource::new("role/B", "B")),
        ];
        let plan = plan(&desired, &CurrentState::default());
        // Both are roles (level 0), sorted by addr.
        assert_eq!(plan.operations.len(), 2);
        assert_eq!(plan.operations[0].addr().as_str(), "role/A");
        assert_eq!(plan.operations[1].addr().as_str(), "role/B");
    }

    #[test]
    fn plan_is_deterministic() {
        let desired = vec![
            Resource::Role(RoleResource::new("role/B", "B")),
            Resource::Role(RoleResource::new("role/A", "A")),
        ];
        let p1 = plan(&desired, &CurrentState::default());
        let p2 = plan(&desired, &CurrentState::default());
        assert_eq!(p1, p2);
    }
}
