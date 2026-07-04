# Introduction

GuildForge is **Infrastructure as Code for Discord Workspaces**.

You write a single YAML file declaring the desired state of a Discord
guild — roles, categories, channels, permissions, webhooks, forum tags,
ordering — and GuildForge computes the minimal diff against the live
guild and applies it safely, idempotently, and with a full audit trail
in local SQLite state.

```bash
guildforge apply company.yaml
```

That one command creates (or reconciles) every role, category, channel,
permission overwrite, and webhook described in `company.yaml`. Run it
again and GuildForge computes a minimal diff and applies only what
changed. Run `guildforge destroy` and the workspace is torn back down —
cleanly, idempotently, with a full audit trail in local SQLite state.

## Why

Discord servers for communities, companies, and open-source projects
routinely grow to hundreds of channels across dozens of categories,
with interlocking permission overwrites that nobody fully understands.
Manual configuration through the Discord UI is non-reproducible,
drift-prone, unreviewable, and un-rollback-able.

GuildForge fixes all four by treating a Discord guild as a declarative
resource graph that is planned, applied, versioned, and destroyed the
same way Terraform manages cloud infrastructure.

## Project Status

GuildForge is at v1.0.0. All 15 CLI commands are implemented, 256+
tests pass, and the dashboard builds successfully.

## License

Dual-licensed under MIT OR Apache-2.0, matching the Rust ecosystem
convention.
