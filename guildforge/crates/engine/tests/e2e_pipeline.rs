//! End-to-end pipeline test.
//!
//! Exercises the full workflow: validate → plan → apply →
//! doctor → export → destroy, using a mock provider (no real Discord
//! connection). Verifies idempotency (apply twice = no-op).

use guildforge_engine::Engine;
use guildforge_provider::{Provider, ProviderError, Resource, ResourceKind};
use guildforge_shared::ResourceId;
use guildforge_state::Store;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// A mock provider that simulates Discord CRUD in-memory.
struct MockProvider {
    resources: std::sync::Mutex<std::collections::BTreeMap<ResourceId, Resource>>,
    create_count: AtomicU32,
    delete_count: AtomicU32,
}

impl MockProvider {
    fn new() -> Self {
        Self {
            resources: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            create_count: AtomicU32::new(0),
            delete_count: AtomicU32::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Provider for MockProvider {
    type Error = ProviderError;

    async fn read(&self, addr: &ResourceId) -> Result<Option<Resource>, Self::Error> {
        let resources = self.resources.lock().unwrap();
        Ok(resources.get(addr).cloned())
    }

    async fn create(&self, desired: &Resource) -> Result<Resource, Self::Error> {
        self.create_count.fetch_add(1, Ordering::Relaxed);
        let mut resources = self.resources.lock().unwrap();
        resources.insert(desired.addr().clone(), desired.clone());
        Ok(desired.clone())
    }

    async fn update(
        &self,
        _current: &Resource,
        desired: &Resource,
    ) -> Result<Resource, Self::Error> {
        let mut resources = self.resources.lock().unwrap();
        resources.insert(desired.addr().clone(), desired.clone());
        Ok(desired.clone())
    }

    async fn delete(&self, current: &Resource) -> Result<(), Self::Error> {
        self.delete_count.fetch_add(1, Ordering::Relaxed);
        let mut resources = self.resources.lock().unwrap();
        resources.remove(current.addr());
        Ok(())
    }

    async fn list(&self, _kind: ResourceKind) -> Result<Vec<Resource>, Self::Error> {
        let resources = self.resources.lock().unwrap();
        Ok(resources.values().cloned().collect())
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

fn write_config(yaml: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.yaml");
    std::fs::write(&path, yaml).unwrap();
    std::mem::forget(dir);
    path
}

const SIMPLE_CONFIG: &str = "\
server:
  name: Test Guild

roles:
  - name: Admin
    color: red
    permissions: [administrator]
  - name: Member
    color: blue

channels:
  - name: general
    type: text
    topic: General chat
  - name: announcements
    type: text
";

#[tokio::test]
async fn full_pipeline_validate_plan_apply_doctor_export_destroy() {
    let store = Arc::new(Store::open_in_memory().await.unwrap());
    let provider = MockProvider::new();
    let engine = Engine::new(provider, store);

    // 1. Validate — should pass.
    let config_path = write_config(SIMPLE_CONFIG);
    engine.validate(&config_path).unwrap();

    // 2. Plan — should show creates (empty state).
    let plan = engine.plan(&config_path).await.unwrap();
    assert!(
        plan.has_changes(),
        "plan should have changes against empty state"
    );
    let summary = plan.summary();
    assert!(summary.create > 0, "should have creates");
    assert_eq!(summary.delete, 0, "should have no deletes");

    // 3. Apply — should create resources.
    let report = engine.apply(&config_path, true).await.unwrap();
    assert!(report.created > 0, "should have created resources");
    assert_eq!(report.failed, 0, "should have no failures");

    // 4. Plan again — should be all no-ops (idempotent).
    let plan2 = engine.plan(&config_path).await.unwrap();
    let summary2 = plan2.summary();
    assert_eq!(summary2.create, 0, "second plan should have no creates");
    assert_eq!(summary2.update, 0, "second plan should have no updates");
    assert_eq!(summary2.delete, 0, "second plan should have no deletes");
    assert!(summary2.noop > 0, "second plan should have no-ops");

    // 5. Apply again — should be a no-op.
    let report2 = engine.apply(&config_path, true).await.unwrap();
    assert_eq!(report2.created, 0, "second apply should create nothing");
    assert_eq!(report2.updated, 0, "second apply should update nothing");
    assert_eq!(report2.deleted, 0, "second apply should delete nothing");

    // 6. Doctor — should report no drift.
    let drift = engine.doctor().await.unwrap();
    assert!(drift.missing_in_live.is_empty(), "no missing in live");
    assert!(drift.missing_in_state.is_empty(), "no missing in state");
    assert!(drift.drifted.is_empty(), "no drifted resources");

    // 7. Export — should produce YAML with the resources.
    let exported = engine.export("Test Guild").await.unwrap();
    assert!(exported.contains("server:"), "export should have server");
    assert!(exported.contains("Admin"), "export should have Admin role");

    // 8. Destroy — should delete everything.
    let destroy_report = engine.destroy(&config_path, true).await.unwrap();
    assert!(destroy_report.deleted > 0, "should have deleted resources");
    assert_eq!(
        destroy_report.failed, 0,
        "should have no failures on destroy"
    );

    // 9. Plan after destroy — should show creates again.
    let plan3 = engine.plan(&config_path).await.unwrap();
    let summary3 = plan3.summary();
    assert!(
        summary3.create > 0,
        "plan after destroy should have creates"
    );
    assert_eq!(
        summary3.delete, 0,
        "plan after destroy should have no deletes"
    );
}

#[tokio::test]
async fn diff_between_configs_detects_changes() {
    let a = write_config(SIMPLE_CONFIG);
    let b = write_config("server:\n  name: Different Guild\nroles:\n  - name: NewRole\n");
    let config_a = guildforge_parser::parse_file(&a).unwrap();
    let config_b = guildforge_parser::parse_file(&b).unwrap();
    let report = guildforge_engine::diff_configs(&config_a, &config_b);
    assert!(!report.entries.is_empty(), "diff should detect changes");
    assert!(
        report.entries.iter().any(|e| e.addr == "server/name"),
        "should detect server name change"
    );
}

#[tokio::test]
async fn backup_and_restore_state() {
    let dir = tempfile::tempdir().unwrap();
    let state_path = dir.path().join("test.db");
    let store = Arc::new(Store::open(&state_path).await.unwrap());
    let provider = MockProvider::new();
    let engine = Engine::new(provider, store);

    // Apply to populate state.
    let config_path = write_config(SIMPLE_CONFIG);
    engine.apply(&config_path, true).await.unwrap();

    // Backup.
    let backup_path = dir.path().join("backup.db");
    engine.backup(&backup_path).unwrap();
    assert!(backup_path.exists(), "backup file should exist");

    // Destroy to clear state.
    engine.destroy(&config_path, true).await.unwrap();

    // Restore from backup.
    engine.restore(&backup_path).unwrap();

    // Plan should now show deletes (state was restored but provider
    // was cleared by destroy). Actually, the mock provider still has
    // resources from the destroy (which deleted from both state and
    // provider). After restore, state has resources but provider
    // doesn't, so doctor would detect drift. This is expected behavior.
    // We just verify restore didn't corrupt anything.
    let drift = engine.doctor().await.unwrap();
    // After restore, state has resources but provider's mock store
    // was cleared by destroy. So doctor should detect missing_in_live.
    assert!(
        !drift.missing_in_live.is_empty(),
        "restored state resources should be missing in live (provider was cleared)"
    );
}
