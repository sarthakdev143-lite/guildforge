# CLI Reference

> Authoritative reference for the `guildforge` command-line interface.
> Every subcommand, flag, environment variable, and exit code is documented
> here. The Phase 1+ implementation in `apps/cli` must conform exactly.

## Synopsis

```
guildforge [GLOBAL FLAGS] <command> [COMMAND FLAGS] [ARGS]
```

## Global Flags

| Flag | Env | Default | Description |
|---|---|---|---|
| `--state-file <PATH>` | `GUILDFORGE_STATE_FILE` | `./guildforge.db` | Path to the SQLite state file |
| `--provider <NAME>` | `GUILDFORGE_PROVIDER` | `discord` | Provider to use (`discord` only in v1) |
| `--token-file <PATH>` | `GUILDFORGE_TOKEN_FILE` | `~/.config/guildforge/token` | Path to a file containing the bot token |
| `--log-level <LEVEL>` | `GUILDFORGE_LOG_LEVEL` | `info` | One of: `trace` `debug` `info` `warn` `error` |
| `--log-format <FORMAT>` | `GUILDFORGE_LOG_FORMAT` | `pretty` | One of: `pretty` `json` `compact` |
| `--no-color` | `GUILDFORGE_NO_COLOR` | `false` | Disable colored output |
| `--config <PATH>` | — | — | Path to an alternate config file (rare; most commands take YAML path as arg) |
| `-h, --help` | — | — | Print help |
| `-V, --version` | — | — | Print version |

Global flags may appear before or after the subcommand.

## Commands

### `guildforge init`

Scaffold a new `guildforge.yaml` in the current directory.

```
guildforge init [--template <NAME>] [--force]
```

| Flag | Description |
|---|---|
| `--template <NAME>` | Use a built-in template (`minimal`, `community`, `company`). Default: `minimal` |
| `--force` | Overwrite existing `guildforge.yaml` |

**Exit codes**: `0` success, `1` file exists without `--force`, `2` template not found.

### `guildforge validate <file>`

Parse and validate a config file. Does not access Discord, does not read state.

```
guildforge validate <file> [--schema-version <V>]
```

**Output**: `OK` on success, or one or more diagnostics (one per line) on failure.

**Exit codes**: `0` valid, `1` invalid, `2` file not found, `3` I/O error.

### `guildforge plan <file>`

Compute and print the execution plan for `<file>` against current state.

```
guildforge plan <file> [--format <FORMAT>] [--out <PATH>] [--refresh <BOOL>]
```

| Flag | Description |
|---|---|
| `--format <FORMAT>` | One of: `text` (default), `json`, `sarif`, `markdown` |
| `--out <PATH>` | Write plan to file instead of stdout |
| `--refresh <BOOL>` | Refresh state from live Discord before planning. Default: `true`. `false` = plan against state only (faster, may miss drift) |

**Exit codes**: `0` plan is empty (no changes), `1` plan has changes, `2` validation error, `3` state error, `4` provider error.

The exit code `1` on "has changes" is intentional — it lets CI detect drift.

### `guildforge apply <file>`

Apply a config: plan, prompt (unless `--auto-approve`), execute, commit state.

```
guildforge apply <file> [--auto-approve] [--plan <PATH>] [--no-color] [--refresh <BOOL>]
```

| Flag | Description |
|---|---|
| `--auto-approve` | Skip interactive prompt. Required in CI. |
| `--plan <PATH>` | Apply a previously-saved plan file (from `plan --out`) instead of re-planning. |
| `--refresh <BOOL>` | Same as `plan --refresh`. |

**Interactive prompt**: shows summary (`+ N, ~ M, - K, > P`) and asks
`Apply these changes? [y/N]`. `y` proceeds; anything else aborts with exit 0.

**Exit codes**: `0` success (no failures), `1` partial failure (some ops succeeded), `2` validation error, `3` state error, `4` provider error, `5` user aborted, `6` lock held by another process.

### `guildforge destroy <file>`

Destroy every resource described in `<file>` (inverse of `apply`).

```
guildforge destroy <file> [--auto-approve] [--refresh <BOOL>]
```

Same flags and exit codes as `apply`. The interactive prompt is **always**
shown unless `--auto-approve` is passed, even in CI (this is a deliberate
safety check; CI scripts must pass `--auto-approve` explicitly).

### `guildforge diff <a> <b>`

Structural diff between two config files. Does not access state or Discord.

```
guildforge diff <a> <b> [--format <FORMAT>]
```

**Output**: a unified-diff-style listing keyed by resource address.

**Exit codes**: `0` identical, `1` different, `2` one or both files invalid.

### `guildforge import <guild-id>`

Read an existing Discord guild and emit a YAML config describing it.

```
guildforge import <guild-id> [--out <PATH>] [--include <LIST>] [--exclude <LIST>]
```

| Flag | Description |
|---|---|
| `--out <PATH>` | Write YAML to file instead of stdout |
| `--include <LIST>` | Comma-separated resource types to include (default: all) |
| `--exclude <LIST>` | Comma-separated resource types to exclude |

**Exit codes**: `0` success, `1` partial (some resources unreadable), `2` no token, `3` guild not found, `4` provider error.

### `guildforge export`

Export current state to a YAML config. Round-trips with `import`.

```
guildforge export [--out <PATH>]
```

**Exit codes**: `0` success, `1` state empty, `2` state error.

### `guildforge doctor`

Detect drift: compare state to live Discord.

```
guildforge doctor [--format <FORMAT>] [--fix <BOOL>]
```

| Flag | Description |
|---|---|
| `--fix` | Update state to match live (does NOT change live). Useful for absorbing intentional out-of-band edits. |

**Exit codes**: `0` no drift, `1` drift detected, `2` state error, `3` provider error.

### `guildforge backup`

Snapshot state to an external file.

```
guildforge backup [--out <PATH>]
```

Default `--out`: `./guildforge-<timestamp>.db.bak`.

**Exit codes**: `0` success, `1` state empty, `2` I/O error.

### `guildforge restore <backup>`

Restore state from a backup file. Overwrites current state.

```
guildforge restore <backup> [--force]
```

`--force` is required if current state is non-empty.

**Exit codes**: `0` success, `1` backup invalid, `2` current state non-empty without `--force`, `3` I/O error.

### `guildforge login`

Store the Discord bot token. Phase 1 implementation: writes to
`~/.config/guildforge/token` with mode 0600. Phase 6: uses OS keychain via
the `keyring` crate.

```
guildforge login [--token-file <PATH>]
```

If `--token-file` is omitted, prompts interactively with hidden input.

**Exit codes**: `0` success, `1` token invalid (Discord rejected it), `2` I/O error.

### `guildforge logout`

Delete the stored token.

```
guildforge logout
```

**Exit codes**: `0` success, `1` no token stored.

### `guildforge version`

Print version, build info, and linked provider versions.

```
guildforge version [--format <FORMAT>]
```

**Output** (text):

```
guildforge 0.1.0
  commit:    abc1234
  built:     2026-07-01
  rustc:     1.78.0
  providers: discord=0.1.0
```

**Exit codes**: `0` always.

## Environment Variables

All global flags have environment variable equivalents (see table in
"Global Flags" above). CLI flags take precedence over env vars. Env vars
take precedence over defaults.

Additional env vars:

| Var | Description |
|---|---|
| `GUILDFORGE_BOT_TOKEN` | Provide token directly (useful for CI; not recommended for daily use) |
| `GUILDFORGE_HTTP_PROXY` | HTTP(S) proxy URL |
| `GUILDFORGE_NO_NETWORK` | If set, fail any command that would touch the network. Useful for testing. |
| `NO_COLOR` | Standard `NO_COLOR` env var; respected |

## Exit Code Summary

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Soft failure (validation, plan-has-changes, partial-apply, drift) |
| 2 | User error (file not found, invalid args, I/O) |
| 3 | State error (lock held, corrupt, missing) |
| 4 | Provider error (HTTP failure, rate-limited, etc.) |
| 5 | User aborted (declined prompt) |
| 6 | Lock held (for `apply` / `destroy`) |

## Output Formats

`text` (default) is human-readable. `json` is a stable, versioned schema
documented in `docs/JSON_OUTPUT.md` (TBD Phase 3). `sarif` is for CI
integration (GitHub Code Scanning). `markdown` is for PR comments.

All non-`text` formats are stable across patch versions.

## Completion

`guildforge` ships shell completions for bash, zsh, fish, and PowerShell
(Phase 6). Generated via `clap_complete` from the clap derive spec; the
`guildforge completions <shell>` subcommand prints them to stdout.

## Man Pages

`guildforge.1` is generated via `clap_mangen` (Phase 6) and shipped with
distro packages. It mirrors this document.
