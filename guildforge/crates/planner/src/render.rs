//! Plan rendering — text and JSON output.
//!
//! See [`ADR-0003`](../../docs/adr/ADR-0003-planner-determinism.md)
//! for the determinism contract on JSON output.

use crate::ExecutionPlan;
use guildforge_provider::ResourceKind;

/// Output format for plan rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlanFormat {
    /// Human-readable text (default).
    #[default]
    Text,
    /// Stable JSON for machine consumption.
    Json,
}

/// Render a plan in the given format.
#[must_use]
pub fn render(plan: &ExecutionPlan, format: PlanFormat) -> String {
    match format {
        PlanFormat::Text => render_text(plan),
        PlanFormat::Json => render_json(plan),
    }
}

/// Render a plan as human-readable text.
#[must_use]
pub fn render_text(plan: &ExecutionPlan) -> String {
    if plan.is_empty() {
        return "No changes.\n".to_string();
    }
    let mut out = String::new();
    for op in &plan.operations {
        out.push_str(op.symbol());
        out.push(' ');
        out.push_str(kind_label(op));
        out.push_str("  ");
        out.push_str(op.addr().as_str());
        out.push('\n');
    }
    let s = plan.summary();
    out.push_str(&format!("\nPlan: {s}\n"));
    out
}

/// Render a plan as stable JSON.
#[must_use]
pub fn render_json(plan: &ExecutionPlan) -> String {
    serde_json::to_string_pretty(plan).unwrap_or_else(|_| "{}".to_string())
}

fn kind_label(op: &crate::Operation) -> &'static str {
    let kind = match op {
        crate::Operation::Create { desired } => desired.kind(),
        crate::Operation::Update { current, .. } => current.kind(),
        crate::Operation::Delete { current } => current.kind(),
        crate::Operation::Noop { current } => current.kind(),
    };
    match kind {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Operation;
    use guildforge_provider::RoleResource;

    #[test]
    fn empty_plan_text() {
        let plan = ExecutionPlan::default();
        assert_eq!(render_text(&plan), "No changes.\n");
    }

    #[test]
    fn create_op_text() {
        let plan = ExecutionPlan {
            operations: vec![Operation::Create {
                desired: guildforge_provider::Resource::Role(RoleResource::new(
                    "role/Admin",
                    "Admin",
                )),
            }],
        };
        let text = render_text(&plan);
        assert!(text.contains("+ role  role/Admin"));
        assert!(text.contains("Plan: +1 ~0 -0 =0"));
    }

    #[test]
    fn json_is_stable() {
        let plan = ExecutionPlan {
            operations: vec![Operation::Create {
                desired: guildforge_provider::Resource::Role(RoleResource::new(
                    "role/Admin",
                    "Admin",
                )),
            }],
        };
        let j1 = render_json(&plan);
        let j2 = render_json(&plan);
        assert_eq!(j1, j2);
    }

    #[test]
    fn json_contains_op_tag() {
        let plan = ExecutionPlan {
            operations: vec![Operation::Create {
                desired: guildforge_provider::Resource::Role(RoleResource::new(
                    "role/Admin",
                    "Admin",
                )),
            }],
        };
        let j = render_json(&plan);
        assert!(j.contains("\"op\":"));
        assert!(j.contains("create"));
    }
}
