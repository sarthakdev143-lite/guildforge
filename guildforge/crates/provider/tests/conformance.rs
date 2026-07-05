//! Provider conformance test suite.
//!
//! Every provider implementation must pass these tests. They verify
//! the idempotency and CRUD contracts defined in ADR-0001 and ADR-0007.

use guildforge_provider::{
    Provider, ProviderError, Resource, ResourceAddr, ResourceKind, RoleResource,
};
use guildforge_shared::ResourceId;

/// A simple in-memory provider for testing conformance.
struct InMemoryProvider {
    resources: std::sync::Mutex<std::collections::BTreeMap<ResourceId, Resource>>,
}

impl InMemoryProvider {
    fn new() -> Self {
        Self {
            resources: std::sync::Mutex::new(std::collections::BTreeMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl Provider for InMemoryProvider {
    type Error = ProviderError;

    async fn read(&self, addr: &ResourceAddr) -> Result<Option<Resource>, Self::Error> {
        Ok(self.resources.lock().unwrap().get(addr).cloned())
    }

    async fn create(&self, desired: &Resource) -> Result<Resource, Self::Error> {
        let mut resources = self.resources.lock().unwrap();
        if let Some(existing) = resources.get(desired.addr()) {
            return Ok(existing.clone());
        }
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
        let mut resources = self.resources.lock().unwrap();
        resources.remove(current.addr());
        Ok(())
    }

    async fn list(&self, _kind: ResourceKind) -> Result<Vec<Resource>, Self::Error> {
        Ok(self.resources.lock().unwrap().values().cloned().collect())
    }

    fn name(&self) -> &'static str {
        "in-memory"
    }
}

fn make_provider() -> InMemoryProvider {
    InMemoryProvider::new()
}

fn make_role(name: &str) -> Resource {
    Resource::Role(RoleResource::new(format!("role/{name}"), name))
}

#[tokio::test]
async fn create_then_read_returns_same_resource() {
    let provider = make_provider();
    let role = make_role("Admin");
    let created = provider.create(&role).await.unwrap();
    let read = provider.read(created.addr()).await.unwrap();
    assert!(read.is_some());
    assert_eq!(read.unwrap().addr(), created.addr());
}

#[tokio::test]
async fn read_nonexistent_returns_none() {
    let provider = make_provider();
    let addr = ResourceId::new("role/Ghost");
    let result = provider.read(&addr).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn create_is_idempotent() {
    let provider = make_provider();
    let role = make_role("Admin");
    let created1 = provider.create(&role).await.unwrap();
    let created2 = provider.create(&role).await.unwrap();
    assert_eq!(created1.addr(), created2.addr());
}

#[tokio::test]
async fn update_changes_resource() {
    let provider = make_provider();
    let role = make_role("Admin");
    let created = provider.create(&role).await.unwrap();
    let desired = Resource::Role(RoleResource {
        name: "SuperAdmin".to_string(),
        ..match created.clone() {
            Resource::Role(r) => r,
            _ => unreachable!(),
        }
    });
    let updated = provider.update(&created, &desired).await.unwrap();
    assert_eq!(updated.addr(), created.addr());
}

#[tokio::test]
async fn delete_removes_resource() {
    let provider = make_provider();
    let role = make_role("Admin");
    let created = provider.create(&role).await.unwrap();
    provider.delete(&created).await.unwrap();
    let read = provider.read(created.addr()).await.unwrap();
    assert!(read.is_none());
}

#[tokio::test]
async fn delete_nonexistent_is_idempotent() {
    let provider = make_provider();
    let role = make_role("Ghost");
    let result = provider.delete(&role).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn list_returns_all_resources() {
    let provider = make_provider();
    provider.create(&make_role("A")).await.unwrap();
    provider.create(&make_role("B")).await.unwrap();
    provider.create(&make_role("C")).await.unwrap();
    let list = provider.list(ResourceKind::Role).await.unwrap();
    assert!(list.len() >= 3);
}

#[tokio::test]
async fn full_crud_lifecycle() {
    let provider = make_provider();
    let role = make_role("Test");
    let created = provider.create(&role).await.unwrap();
    assert!(provider.read(created.addr()).await.unwrap().is_some());
    let updated = Resource::Role(RoleResource {
        name: "Updated".to_string(),
        ..match created.clone() {
            Resource::Role(r) => r,
            _ => unreachable!(),
        }
    });
    provider.update(&created, &updated).await.unwrap();
    provider.delete(&updated).await.unwrap();
    assert!(provider.read(updated.addr()).await.unwrap().is_none());
}

#[tokio::test]
async fn provider_name_is_nonempty() {
    let provider = make_provider();
    assert!(!provider.name().is_empty());
}

#[tokio::test]
async fn multiple_resources_coexist() {
    let provider = make_provider();
    let r1 = make_role("Admin");
    let r2 = make_role("Mod");
    let r3 = make_role("User");
    provider.create(&r1).await.unwrap();
    provider.create(&r2).await.unwrap();
    provider.create(&r3).await.unwrap();
    assert!(provider.read(r1.addr()).await.unwrap().is_some());
    assert!(provider.read(r2.addr()).await.unwrap().is_some());
    assert!(provider.read(r3.addr()).await.unwrap().is_some());
    provider.delete(&r2).await.unwrap();
    assert!(provider.read(r1.addr()).await.unwrap().is_some());
    assert!(provider.read(r2.addr()).await.unwrap().is_none());
    assert!(provider.read(r3.addr()).await.unwrap().is_some());
}
