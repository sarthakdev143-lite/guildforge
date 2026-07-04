# ADR-0001: Provider Trait

- **Status**: Accepted
- **Date**: 2026-07-01
- **Deciders**: founding eng
- **Tags**: provider, architecture, extensibility

## Context

GuildForge's mission is "Infrastructure as Code for Discord Workspaces", but
the project's stated architecture goal is to support Slack, MS Teams, and
Mattermost as future providers without engine changes. This means Discord
cannot be hardcoded into the engine, planner, executor, or state store.

The provider abstraction is the single most important architectural decision
in the project. Getting it wrong forces a rewrite when the second provider
is added; getting it right means future providers are pure additive work.

We need to answer:

1. What shape is the `Provider` trait — sync or async?
2. What granularity — one `apply(config)` method, or per-resource CRUD?
3. How are resources represented — typed enum, or trait objects?
4. How are errors represented — typed, erased, or string?
5. How does the engine discover and select providers?

## Decision

### Trait shape: async, per-resource CRUD

```rust
// crates/provider/src/lib.rs
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait Provider: Send + Sync {
    type Error: Error + Send + Sync + 'static;

    /// Read a single resource by address. Returns Ok(None) if not present.
    async fn read(&self, addr: &ResourceAddr) -> Result<Option<Resource>, Self::Error>;

    /// Create a new resource. The returned `Resource` includes server-assigned
    /// fields (ID, etc.) that were not in `desired`.
    async fn create(&self, desired: &Resource) -> Result<Resource, Self::Error>;

    /// Update an existing resource from `current` to `desired`. Providers
    /// should issue a PATCH-style update, not delete-and-recreate.
    async fn update(
        &self,
        current: &Resource,
        desired: &Resource,
    ) -> Result<Resource, Self::Error>;

    /// Delete a resource. Must be idempotent: deleting a non-existent
    /// resource returns Ok(()).
    async fn delete(&self, current: &Resource) -> Result<(), Self::Error>;

    /// Reorder a resource within its parent (channel within category, role
    /// within guild, etc.). Default impl is a no-op for resources that
    /// don't support ordering.
    async fn reorder(&self, addr: &ResourceAddr, new_position: u32)
        -> Result<(), Self::Error>;

    /// Return the list of all resources currently present in the provider,
    /// for drift detection (`guildforge doctor`) and for `guildforge import`.
    async fn list(&self, kind: ResourceKind) -> Result<Vec<Resource>, Self::Error>;

    /// Human-readable provider name (e.g. "discord", "slack").
    fn name(&self) -> &'static str;
}
```

### Resource representation: typed enum, not trait objects

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Resource {
    Role(RoleResource),
    Category(CategoryResource),
    Channel(ChannelResource),
    PermissionOverwrite(PermissionOverwriteResource),
    Webhook(WebhookResource),
    Invite(InviteResource),
    ForumTag(ForumTagResource),
    WelcomeScreen(WelcomeScreenResource),
    ServerGuide(ServerGuideResource),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Role,
    Category,
    Channel,
    PermissionOverwrite,
    Webhook,
    Invite,
    ForumTag,
    WelcomeScreen,
    ServerGuide,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ResourceAddr(pub String);  // e.g. "discord://guild/role/Admin"
```

### Error representation: associated type

Each provider has its own `Error` type via the trait's associated type. This
lets `provider-discord` expose typed HTTP/rate-limit errors while letting
a future `provider-slack` expose Slack-specific errors. The engine erases
the type at the engine boundary with `anyhow::Error::from`.

### Discovery and selection

Providers are registered in `apps/cli/src/main.rs` at startup. The engine
receives an `Arc<dyn Provider>` (erasing the associated `Error` via a
blanket impl) and never knows which provider it is talking to.

```rust
// apps/cli/src/main.rs (sketch)
let provider: Arc<dyn Provider> = match args.provider.as_str() {
    "discord" => Arc::new(DiscordProvider::from_env()?),
    other => bail!("unknown provider: {other}"),
};
let engine = Engine::new(provider, state_path)?;
```

To erase the associated type, a blanket impl wraps any `Provider` in a
`DynProvider` struct that exposes `async fn read(...) -> Result<_,
Box<dyn Error + Send + Sync>>`. This is internal to `crates/provider`.

## Alternatives Considered

### A1: Sync trait

Rejected. Every provider does network I/O. A sync trait would force the
engine to spawn threads or block on async, neither of which is acceptable
in Rust. The `async-trait` macro has minor overhead but the ergonomics
are decisive.

### A2: Single `apply(config)` method

Rejected. This pushes diffing into the provider, which means every
provider reimplements the planner. It also prevents the planner from
producing a reviewable plan without contacting the provider. The whole
point of GuildForge is that planning is decoupled from execution.

### A3: Trait objects for resources (`Box<dyn Resource>`)

Rejected. Resources are data, not behavior. They have no methods beyond
`addr()`, `kind()`, and serialization. An enum is more memory-efficient,
serializes deterministically, and pattern-matches cleanly. The only
downside is that adding a resource type touches the enum — but that is
intentional; it forces the engine to handle the new type explicitly.

### A4: String errors

Rejected. Strings lose structure. The engine needs to distinguish
"rate-limited, retry" from "403 forbidden, do not retry" from "500 server
error, retry". Typed errors make this trivial; strings require parsing.

### A5: Provider discovery via inventory / linkme crates

Rejected. Auto-discovery is magic. Explicit registration in `main.rs` is
one line per provider, makes the dependency obvious, and lets the linker
strip unused providers from release binaries.

## Consequences

### Becomes easier

- Adding a new provider = new crate, new `Provider` impl, one-line
  registration in `cli`. No engine changes.
- Testing the engine: inject a `MockProvider` that implements `Provider`.
  No HTTP, no Discord.
- Plan output is provider-agnostic: the same `ExecutionPlan` rendering
  code works for Discord, Slack, etc.

### Becomes harder

- The `Resource` enum grows with every resource type. Adding forum tags
  touched 4 places: the enum, the serde tag, the planner match, the
  provider impl. This is intentional friction.
- The associated `Error` type requires a wrapper (`DynProvider`) for type
  erasure. This is internal complexity but pays for itself in engine
  simplicity.
- Cross-provider operations (e.g. "mirror this Discord channel to Slack")
  are not naturally expressible. A future ADR would address this if demand
  appears.

### New constraints

- Every resource type must fit the `Resource` enum shape. If a future
  provider has a resource that doesn't fit (e.g. a streaming pipeline),
  we revisit this ADR.
- Providers must implement ALL CRUD methods, even if some are no-ops. The
  `reorder` default-impl mitigates this for non-orderable resources.
- `list` is mandatory because `doctor` and `import` need it. This may
  exclude some providers that lack a list API; we revisit if needed.

## Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Trait evolution breaks providers | The trait is semver-stable from v1.0. Changes require a new ADR. |
| Associated type erasure adds runtime overhead | Benchmark in Phase 3; if material, switch to `Box<dyn Error>` in trait directly. |
| `Resource` enum explosion | Acceptable; max ~15 variants projected for v1.x. Revisit if it exceeds 30. |
| Provider impls diverge in semantic interpretation | Provider conformance test suite (`crates/provider/tests/conformance.rs`) every provider must pass. |

## References

- [Terraform Provider spec](https://developer.hashicorp.com/terraform/plugin/sdkv2)
- [Pulumi Provider spec](https://www.pulumi.com/docs/concepts/components/providers/)
- [async-trait crate](https://docs.rs/async-trait)
- Related: [ADR-0007](./ADR-0007-idempotency-ordering.md) (executor uses this trait),
  [ADR-0005](./ADR-0005-error-model.md) (error handling)
