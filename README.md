# GuildForge

> Infrastructure as Code for Discord Workspaces.

[![CI](https://img.shields.io/badge/CI-pending-lightgrey)](./.github/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78%2B-orange)](./rust-toolchain.toml)
[![Status](https://img.shields.io/badge/status-pre--alpha-red)](./PROJECT_STATE.md)

`GuildForge` lets you deploy an entire Discord workspace — server config, roles,
categories, channels, permissions, forum tags, webhooks, ordering — from a single
declarative YAML file, exactly the way Terraform deploys cloud infrastructure.

```bash
guildforge apply company.yaml
```

That one command creates (or reconciles) every role, category, channel, permission
overwrite, and webhook described in `company.yaml`. Run it again and GuildForge
computes a minimal diff and applies only what changed. Run `guildforge destroy`
and the workspace is torn back down — cleanly, idempotently, with a full audit
trail in local SQLite state.

---

## Why

Discord servers for communities, companies, and open-source projects routinely
grow to **hundreds of channels** across dozens of categories, with interlocking
permission overwrites that nobody fully understands. Manual configuration through
the Discord UI is:

- **Non-reproducible** — there is no "server config as code".
- **Drift-prone** — out-of-band UI edits silently diverge from intent.
- **Unreviewable** — permission changes cannot go through a PR.
- **Un-rollback-able** — there is no `git revert` for a deleted channel.

GuildForge fixes all four by treating a Discord guild as a *declarative resource
graph* that is planned, applied, versioned, and destroyed the same way Terraform
manages cloud infrastructure.

---

## Project Status

**Phase 0 — Architecture & Foundations** (this commit).

The Rust workspace compiles, the Cargo layout is finalized, the provider trait is
specified, the YAML schema is locked at v1, and the milestone roadmap is
committed. No runtime behavior is implemented yet. See
[`PROJECT_STATE.md`](./PROJECT_STATE.md) for the live status board and
[`ROADMAP.md`](./ROADMAP.md) for upcoming milestones.

---

## Quick Start (target UX)

> The commands below describe the *target* UX. Implementation lands milestone by
> milestone per [`ROADMAP.md`](./ROADMAP.md).

```bash
# Install
cargo install guildforge

# Authenticate (writes token to OS keychain)
guildforge login

# Scaffold a new config in the current directory
guildforge init

# Validate schema + semantics without touching Discord
guildforge validate guild.yaml

# Preview the diff against live state
guildforge plan guild.yaml

# Apply
guildforge apply guild.yaml

# Detect drift caused by manual UI edits
guildforge doctor

# Tear everything down
guildforge destroy
```

A minimal config looks like:

```yaml
server:
  name: Augment Infotech
  description: Internal guild for Augment Infotech staff.

roles:
  - name: Admin
    color: red
    permissions: [administrator]
  - name: Staff
    color: blue
    permissions: [send_messages, read_message_history]

categories:
  - name: COMPANY
    channels:
      - name: announcements
        type: text
        topic: Company-wide announcements. Read-only.
        permissions:
          read: [everyone]
          write: [Admin]
      - name: general
        type: text
        permissions:
          read: [everyone]
          write: [Staff]
```

---

## Architecture at a Glance

```
            ┌──────────┐     ┌──────────┐     ┌──────────┐
 YAML ───▶  │ parser   │ ──▶ │validation│ ──▶ │ planner  │
            └──────────┘     └──────────┘     └────┬─────┘
                                                   │
                                                   ▼
            ┌──────────┐     ┌──────────┐     ┌──────────┐
  state ◀── │ executor │ ◀── │  engine  │ ◀── │   plan   │
            └────┬─────┘     └──────────┘     └──────────┘
                 │
                 ▼
            ┌──────────────────┐
            │ provider trait   │
            └────────┬─────────┘
                     │
        ┌────────────┼────────────┐
        ▼            ▼            ▼
   ┌─────────┐ ┌──────────┐ ┌──────────┐
   │ Discord │ │  Slack   │ │ Teams    │   (future)
   └─────────┘ └──────────┘ └──────────┘
```

Discord is **never** accessed directly from the engine. Every external system is
reached through the `Provider` trait defined in `crates/provider`. Adding a new
platform (Slack, Mattermost, MS Teams) is a new crate, not an engine change.

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the full living architecture
document and [`docs/adr/`](./docs/adr/) for the design rationale.

---

## Repository Layout

```
guildforge/
├── apps/
│   ├── cli/                 # `guildforge` binary (clap)
│   └── dashboard/           # Next.js 16 + Tailwind + shadcn/ui web UI
├── crates/
│   ├── config/              # strongly-typed YAML models
│   ├── parser/              # YAML → typed config
│   ├── validation/          # semantic validation + diagnostics
│   ├── engine/              # workflow orchestrator
│   ├── planner/             # diff engine, deterministic plan output
│   ├── executor/            # applies plans, retry, rate limit, rollback
│   ├── state/               # SQLite-backed state store
│   ├── provider/            # Provider trait + shared types
│   ├── provider-discord/    # Discord implementation of Provider
│   ├── shared/              # cross-crate primitives (ids, hashing, time)
│   └── logging/             # tracing initialization
├── docs/                    # architecture, ADRs, schema, CLI reference
├── examples/                # runnable example YAML configs
├── templates/               # opinionated starter configs
├── tests/                   # cross-crate integration tests
└── .github/                 # CI workflows
```

Per-crate responsibilities and dependency rules are documented in
[`docs/CRATE_LAYOUT.md`](./docs/CRATE_LAYOUT.md).

---

## Documentation

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Living architecture overview |
| [docs/SCHEMA.md](./docs/SCHEMA.md) | YAML schema specification (v1) |
| [docs/CLI_REFERENCE.md](./docs/CLI_REFERENCE.md) | Every CLI command, flags, and exit codes |
| [docs/CRATE_LAYOUT.md](./docs/CRATE_LAYOUT.md) | Per-crate responsibilities & dependency rules |
| [docs/TESTING.md](./docs/TESTING.md) | Testing strategy, coverage targets, fixtures |
| [docs/SECURITY.md](./docs/SECURITY.md) | Token handling, threat model, disclosure |
| [docs/adr/](./docs/adr/) | Architecture Decision Records |
| [ROADMAP.md](./ROADMAP.md) | Milestones, 0 → 1.0 → 2.0 |
| [TASKS.md](./TASKS.md) | Live backlog |
| [PROJECT_STATE.md](./PROJECT_STATE.md) | Current snapshot |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | How to contribute |
| [DECISIONS.md](./DECISIONS.md) | Index of significant design decisions |

---

## Tech Stack

| Layer | Choice | Rationale |
|---|---|---|
| Language | Rust (edition 2021) | Type safety, zero-cost abstractions, async, single static binary |
| Async runtime | Tokio | De-facto Rust async standard |
| HTTP | Reqwest | Mature, streaming, middleware-friendly |
| Serialization | Serde + serde_yaml | Strongly typed, zero-copy |
| CLI | Clap v4 | Derive macros, great UX |
| Errors | Anyhow + ThisError + miette | Ergonomic + typed + diagnostic spans |
| Database | SQLite via SQLx | Single-file state, zero-ops, embeddable |
| Logging | `tracing` | Structured, span-aware, OTel-compatible |
| Dashboard | Next.js 16, Tailwind, shadcn/ui | Modern React, RSC, accessible primitives |
| CI | GitHub Actions | matrix builds, caching, release automation |

See [`docs/adr/`](./docs/adr/) for the full rationale on each choice.

---

## License

Dual-licensed under MIT OR Apache-2.0, matching the Rust ecosystem convention.
Contributions accepted under the same terms. See [`LICENSE-MIT`](./LICENSE-MIT)
and [`LICENSE-APACHE`](./LICENSE-APACHE).

---

## Acknowledgements

GuildForge's design is heavily inspired by
[HashiCorp Terraform](https://terraform.io),
[Pulumi](https://pulumi.com),
[AWS CDK](https://aws.amazon.com/cdk/), and
[cargo](https://doc.rust-lang.org/cargo/). Discord is a trademark of Discord
Inc.; this project is not affiliated with or endorsed by Discord.
