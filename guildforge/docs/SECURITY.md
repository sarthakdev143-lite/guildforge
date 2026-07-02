# Security Model

> Threat model, token handling, disclosure policy, and operational security
> for GuildForge.

## Trust Boundaries

```
┌──────────────────────────────────────────────────────────────┐
│  User's machine                                              │
│                                                              │
│   ┌──────────┐    ┌──────────┐    ┌─────────────────────┐   │
│   │ YAML     │    │ SQLite   │    │ OS keychain / file  │   │
│   │ config   │    │ state    │    │ (bot token)         │   │
│   └──────────┘    └──────────┘    └─────────────────────┘   │
│         │              ▲                   ▲                 │
│         ▼              │                   │                 │
│   ┌─────────────────────────────────────────────────────┐   │
│   │               guildforge binary                     │   │
│   └────────────────────────┬────────────────────────────┘   │
│                            │                                 │
└────────────────────────────┼─────────────────────────────────┘
                             │
                             ▼  (TLS)
                   ┌──────────────────┐
                   │  Discord REST    │
                   │  api.discord.com │
                   └──────────────────┘
```

Trust boundary 1: the user's machine. We assume the filesystem is
trustworthy for the user's own files but treat the token as a secret to
protect from other local users and from accidental leakage.

Trust boundary 2: the network. TLS terminates at Discord. We trust
Discord's certificate chain; we do not pin certificates in v1.

Trust boundary 3: the YAML file. YAML is **untrusted input** — it may come
from a forked PR. The parser must be panic-free on arbitrary bytes; the
validator must reject any semantically invalid config without executing
side effects.

## The Bot Token

The Discord bot token is the **only** long-lived secret in GuildForge. It
grants full administrative control of every guild the bot is in. Treating
it carelessly is the single highest-impact security failure mode.

### Storage

- **Phase 1-5**: token stored in `~/.config/guildforge/token` with file
  mode `0600` (Unix) or ACL-restricted to the user (Windows). The file
  path is overridable via `GUILDFORGE_TOKEN_FILE`.
- **Phase 6+**: token stored in the OS keychain via the `keyring` crate
  (macOS Keychain, Windows Credential Manager, Linux Secret Service /
  `kwallet` / `gnome-keyring`). The plaintext file is removed on first
  `guildforge login` after the upgrade.
- **CI**: token provided via `GUILDFORGE_BOT_TOKEN` env var. Never written
  to disk in CI.

### Lifetime

- Token is read from storage at the start of every command that needs it.
- Token is held in memory for the duration of the command and dropped on
  exit.
- Token is **never** written to:
  - State files (SQLite).
  - Logs (`tracing` events).
  - Plan output (text, JSON, SARIF).
  - Error messages or diagnostics.
  - Crash dumps.
- Token is **never** sent to any host other than `discord.com` /
  `*.discord.com`.

### Logging

The `provider-discord` HTTP client uses a custom `tracing` span that
redacts the `Authorization` header before logging. Any log line that
includes HTTP request or response bodies is scrubbed of any header value
matching the token.

Code review checklist for any change to `provider-discord`:

- [ ] No new log line includes the `Authorization` header.
- [ ] No new log line includes the full URL of an authenticated request
      (query params may include token-like values).
- [ ] No new error message includes the token, even partially.
- [ ] No new debug log includes raw HTTP request/response bodies without
      going through the redaction filter.

### Revocation

If a token is compromised:

1. Revoke immediately in the Discord Developer Portal.
2. Run `guildforge logout` on every machine that had the token.
3. Rotate any state files that may have been accessed with the
   compromised token (state itself contains no token, but a compromised
   token + state could let an attacker destroy resources).
4. Audit `guildforge.db` access times on shared systems.

## State File Security

The SQLite state file at `./guildforge.db` (default) contains:

- Resource IDs (Discord snowflakes).
- Resource names and configuration values.
- Channel topics, role names, permission overwrites.
- Webhook URLs (which contain tokens — see below).
- Migration history.

The state file does **not** contain:

- The bot token.
- User passwords or session secrets.
- Anything from outside Discord.

### Webhook URLs

Webhook URLs are a special case. They contain a secret token that allows
anyone to post to the channel. GuildForge stores webhook URLs in state
because `destroy` needs them to delete the webhook.

**Mitigations**:

- State file mode is `0600` (set on creation).
- `guildforge export` redacts webhook tokens by default; `--include-secrets`
  is required to include them.
- Webhook URLs are never logged.
- `guildforge doctor` does not print webhook URLs.

### Backup Files

`guildforge backup` writes a copy of the state file to
`./guildforge-<timestamp>.db.bak`. The backup inherits `0600` mode.
Backups older than 30 days are not auto-deleted — that is the user's
responsibility.

## YAML as Untrusted Input

YAML configs from forks must be safe to `validate` and `plan` without
side effects. `apply` requires explicit user confirmation (or
`--auto-approve` in CI, which is the user's deliberate opt-in).

**Threats considered**:

- **Path traversal** in `server.icon`, `roles[].icon`, `webhooks[].avatar`.
  GuildForge reads these paths and uploads them to Discord. Paths are
  canonicalized and rejected if they escape the config file's parent
  directory.
- **Resource exhaustion**. A YAML with 100,000 channels is valid syntax
  but would exhaust memory. The validator enforces Discord's API limits
  (max 500 channels, etc.) before any I/O.
- **Slowloris via YAML**. `serde_yaml` is bounded; we set a 10 MiB max
  file size by default (`--max-config-size` to override).
- **Arbitrary code execution via YAML tags**. `serde_yaml` does not
  support `!!python/exec` or similar; tags are rejected. Anchors and
  aliases are forbidden in v1 (per `docs/SCHEMA.md`).
- **SSRF via webhook avatar URLs**. Webhook `avatar` accepts a URL.
  GuildForge fetches the URL server-side. SSRF is mitigated by:
  - Refusing URLs that resolve to private IP ranges (RFC 1918, link-local,
    loopback).
  - Refusing URLs with non-HTTP(S) schemes.
  - Setting a 5-second fetch timeout.
  - Limiting response size to 1 MiB.

## Network Security

- All HTTP traffic to Discord is over TLS 1.2+ (reqwest enforces).
- TLS root store is the platform's native store (`rustls-native-certs`).
- Certificate pinning is **not** done in v1; it breaks legitimate cert
  rotations.
- HTTP proxy support via `GUILDFORGE_HTTP_PROXY`. The proxy URL must be
  HTTPS or a known-internal HTTP host. The token is sent to the proxy
  via the `Authorization` header — the user accepts this risk by setting
  the env var.
- Outgoing requests have a 30-second timeout by default. Long-running
  operations (file uploads) have a 5-minute timeout.

## Dashboard Security (Phase 5)

The dashboard is a Next.js 16 app that runs on the user's own machine (or
a trusted internal server). It does **not** expose the bot token to the
browser.

- Token is stored server-side, encrypted at rest with a key derived from
  a user-chosen passphrase (or, optionally, the OS keychain).
- Browser never sees the token. All Discord API calls are proxied through
  Next.js API routes.
- Dashboard binds to `127.0.0.1` by default; binding to `0.0.0.0` requires
  explicit `--host 0.0.0.0` and prints a warning.
- Auth: single-user passphrase in v1. Multi-user auth is a Phase 7+
  concern.
- All API routes require auth except `/api/login`.
- CSRF: same-site cookies + custom header requirement on mutations.
- Rate limiting: per-IP rate limit on `/api/login` (5 attempts / minute).

## Supply Chain

- `cargo-deny` config in `deny.toml` rejects:
  - Licenses not in the allowlist (MIT, Apache-2.0, BSD-3, ISC, MPL-2.0,
    Unicode-DFS-2016).
  - Known vulnerable crates (advisory database).
  - Crates with suspicious provenance (e.g. yanked, replaced).
- `cargo-audit` runs in CI on every push.
- Dependabot opens PRs for minor version bumps; major version bumps are
  manual.
- New direct dependencies require an ADR justifying the addition. The
  bar is high; we prefer fewer, well-maintained deps.
- Binary releases are reproducible from source. Release artifacts include
  an SBOM (CycloneDX format).

## Threats Considered and Rejected

| Threat | Mitigation | Status |
|---|---|---|
| Attacker steals token from `~/.config/guildforge/token` | File mode 0600; keychain in Phase 6 | Partial until Phase 6 |
| Attacker reads state file | 0600 mode; webhook URLs redacted on export | Accepted residual risk |
| Attacker submits malicious YAML PR | Validator enforces API limits; paths canonicalized; no YAML tags | Mitigated |
| Attacker submits YAML with SSRF avatar URL | Private IP rejection; size limit; timeout | Mitigated |
| Attacker compromises a dependency | cargo-deny + cargo-audit in CI | Mitigated; residual accepted |
| Attacker exploits bug in parser | Property tests + fuzzing | Mitigated; residual accepted |
| Attacker gains code execution via CI | GitHub Actions OIDC; no long-lived tokens in CI; least-privilege PATs | Mitigated |
| Attacker steals token from process memory | Out of scope for v1; documented | Accepted |
| Attacker impersonates Discord via DNS hijack | TLS cert validation; no pinning | Partial; residual accepted |
| Attacker tricks user into `apply` of malicious YAML | `--auto-approve` required in CI; interactive prompt by default; plan review | Mitigated |

## Disclosure Policy

**Reporting a vulnerability**: email `security@guildforge.dev` (TBD; until
then, open a private security advisory on GitHub). Do not open a public
issue.

**Response SLA**:

- Acknowledgement within 72 hours.
- Initial assessment within 7 days.
- Fix or mitigation within 90 days (sooner for high-severity).
- Coordinated disclosure: we credit the reporter unless they prefer
  anonymity.

**Severity ratings** use [CVSS 3.1](https://www.first.org/cvss/).

## Security Hardening Checklist (for release)

Before tagging v1.0:

- [ ] `cargo audit` clean.
- [ ] `cargo deny check` clean.
- [ ] No `unwrap()` / `expect()` / `panic!()` outside tests.
- [ ] No token in any log line (verified by grepping test output).
- [ ] State file mode 0600 on all platforms.
- [ ] Keychain integration shipped (`keyring` crate).
- [ ] SSRF protections on all URL-accepting fields.
- [ ] Path traversal protections on all file-accepting fields.
- [ ] Fuzz targets running nightly without crashes for 7 days.
- [ ] Threat model reviewed by an external party.
- [ ] SBOM generation in release pipeline.
- [ ] Reproducible builds verified.
