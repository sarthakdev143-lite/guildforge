# Templates

Opinionated starter configs for common guild shapes. Used by
`guildforge init --template <name>` (Phase 1+).

## Available templates

| Name | Description | Use case |
|---|---|---|
| `minimal` | Just a server + 1 role + 1 channel | Trying out GuildForge for the first time |
| `community` | Open-source community shape (public read-mostly) | OSS project Discord |
| `company` | Internal company guild shape | Company-internal Discord |
| `opensource-project` | Like `community` but with maintainer/contributor roles | OSS with contributor ladder |

## File layout

```
templates/
├── minimal.yaml
├── community.yaml
├── company.yaml
└── opensource-project.yaml
```

Each template is a complete, valid YAML file that passes
`guildforge validate`. The `guildforge init --template <name>` command
copies the file to `./guildforge.yaml` in the current directory.

## Adding a template

1. Add `templates/<name>.yaml`.
2. Add a row to the table above.
3. Add an entry to `TEMPLANTS` in `apps/cli/src/commands/init.rs` (Phase 1+).
4. Add a snapshot test that asserts `guildforge init --template <name>`
   produces the expected file.
5. Add a snapshot test that asserts `guildforge validate templates/<name>.yaml`
   exits 0.
