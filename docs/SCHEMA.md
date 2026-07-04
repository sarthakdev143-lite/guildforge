# YAML Schema Specification (v1)

> Authoritative specification for `guildforge` YAML config files.
> Schema version: **1.0**. Supersession requires a new ADR and a major
> version bump per the [stability policy](../ROADMAP.md).

This document specifies every top-level key, every nested type, every
validation rule, and every known limitation of the v1 schema. The Phase 1
implementation in `crates/config` and `crates/parser` must conform to this
document exactly; deviations are bugs.

## 1. File Conventions

- File extension: `.yaml` or `.yml` (case-sensitive on case-sensitive FS).
- Encoding: UTF-8. BOM is rejected.
- Top-level structure: a mapping with the keys defined in §3.
- Unknown top-level keys are **errors** (strict deserialization).
- Unknown nested keys are **errors**.
- Comments are allowed and preserved through `guildforge export` (YAML
  round-trip via `serde_yaml::Value` is forbidden; use the typed model).
- Multi-document YAML (`---` separators) is **not** supported in v1.
- Anchors and aliases (`&` / `*`) are **not** supported in v1 (they break
  git-friendliness and confuse reviewers).

## 2. Type Conventions

| YAML type | Rust type | Notes |
|---|---|---|
| string | `String` | non-empty unless otherwise noted |
| integer | `i64` | |
| boolean | `bool` | |
| sequence | `Vec<T>` | |
| mapping | `struct` (strongly typed; no `HashMap<String, Value>`) | |
| enum | `enum` with `#[serde(rename_all = "snake_case")]` | |
| optional | `Option<T>` | omit key for `None` |

## 3. Top-Level Keys

```yaml
server:               # required, exactly one
roles:                # optional, list
categories:           # optional, list
channels:             # optional, list (channels not in any category)
permissions:          # optional, mapping (shorthand permission block)
permission_overwrites: # optional, list (full permission overwrite spec)
webhooks:             # optional, list
invites:              # optional, list
forum_tags:           # optional, mapping (forum channel → tags)
welcome_screen:       # optional, mapping
server_guide:         # optional, mapping
ordering:             # optional, mapping (explicit position overrides)
```

### 3.1 `server` (required)

Guild-level settings. Exactly one per file.

```yaml
server:
  name: Augment Infotech          # required, 2-100 chars
  description: Internal guild.    # optional, max 120 chars (Discord limit)
  icon: ./assets/icon.png         # optional, path to PNG/JPEG, max 256 KiB
  banner: ./assets/banner.png     # optional, path to PNG, max 1 MiB
  verification_level: medium      # optional, one of: none|low|medium|high|very_high
  explicit_content_filter: all    # optional, one of: disabled|members_without_roles|all
  default_notifications: only_mentions # optional, one of: all_messages|only_mentions
  system_channel: general         # optional, name of an existing text channel
  system_channel_flags:           # optional, list
    - suppress_join_notifications
    - suppress_premium_subscriptions
  afk_channel: voice-afk          # optional, name of an existing voice channel
  afk_timeout: 300                # optional, seconds, one of: 60|300|900|1800|3600
  premium_progress_bar: true      # optional, bool
```

### 3.2 `roles` (optional)

Sequence of role declarations. The first role in the list with
`permissions: [administrator]` is treated as the admin role for ordering
purposes (highest non-`@everyone` position).

```yaml
roles:
  - name: Admin                    # required, 1-100 chars, unique within guild
    color: red                     # optional, named | hex | rgb | default
    hoist: true                    # optional, bool, default false
    mentionable: true              # optional, bool, default false
    permissions: [administrator]   # optional, list of permission names (see §10)
    position: 10                   # optional, explicit position (overrides auto)
    icon: ./assets/admin.png       # optional, path to PNG (Discord Nitro boost required)
    unicode_emoji: 🔒              # optional, alternative to icon
```

Color formats:

```yaml
color: red                  # named (see §11)
color: "#FF5733"            # hex with #
color: "0xFF5733"           # hex with 0x
color: rgb(255, 87, 51)     # rgb() function syntax
color: default              # Discord's default role color (varies by theme)
```

### 3.3 `categories` (optional)

Sequence of category declarations. Categories are channel groups; in Discord
they are themselves channels of type `guild_category`.

```yaml
categories:
  - name: COMPANY                  # required, 0-100 chars, unique within guild
    description: Company-wide channels. # optional, max 120 chars
    permissions:                   # optional, shorthand (see §3.6)
      read: [everyone]
      write: [Admin, Staff]
    channels:                      # optional, inline list of channels
      - name: announcements
        type: text
        topic: Read-only announcements.
      - name: general
        type: text
```

`permissions` here is the shorthand form applied to the category; channel
permissions are specified per-channel in `channels` or top-level `channels`.

### 3.4 `channels` (optional)

Sequence of channel declarations for channels not nested under a category.
Channels inside a category can be declared either inline (under
`categories[].channels`) or here with `category: <name>`.

```yaml
channels:
  - name: general                   # required, 1-100 chars
    type: text                      # required, see below
    category: COMPANY               # optional, name of a declared category
    topic: General chat             # optional, max 1024 chars
    nsfw: false                     # optional, bool, default false
    slowmode: 0                     # optional, seconds, 0-21600
    permissions:                    # optional, shorthand (see §3.6)
      read: [everyone]
      write: [Staff]
    bitrate: 64000                  # voice only, optional, 8000-384000
    user_limit: 0                   # voice only, optional, 0-99 (0 = unlimited)
    rtc_region: singapore           # voice only, optional (deprecated by Discord)
    available_tags: [help, dev]     # forum only, optional, list of tag names
    default_reaction_emoji: 👍      # forum only, optional
    default_sort_order: latest_activity # forum only, optional
```

Valid `type` values:

| Type | Discord type | Notes |
|---|---|---|
| `text` | `guild_text` | Standard text channel |
| `voice` | `guild_voice` | Voice channel |
| `forum` | `guild_forum` | Forum channel (requires server boost level 1+) |
| `announcement` | `guild_announcement` | Server must be a Community server |
| `stage_voice` | `guild_stage_voice` | Stage channel (requires Community) |
| `category` | `guild_category` | Not allowed here — use `categories` instead |

### 3.5 `permissions` (optional, mapping)

Shorthand permission block applied to channels. Keyed by channel name.

```yaml
permissions:
  announcements:
    read: [everyone]               # list of role names; "everyone" is the @everyone role
    write: [Admin]                 # list of role names
    manage: [Admin]                # optional, list of roles that can manage this channel
    connect: [everyone]            # voice only
    speak: [everyone]              # voice only
    view_audit_log: [Admin]        # rare, but supported
```

This is syntactic sugar for `permission_overwrites` (§3.7). The validator
expands shorthand into overwrites before planning.

### 3.6 Inline `permissions` shorthand (within channels and categories)

Same shape as §3.5 but applied to a single channel or category inline.

### 3.7 `permission_overwrites` (optional, list)

Full permission overwrite specification. Use this when shorthand is
insufficient (e.g. deny rules, role-vs-user distinction).

```yaml
permission_overwrites:
  - channel: announcements          # required, channel or category name
    type: role                      # required, "role" or "member"
    target: Admin                   # required, role name or member ID
    allow: [send_messages, manage_messages]  # optional, list of permission names
    deny: [create_public_threads]            # optional, list of permission names
```

`target: everyone` is shorthand for the `@everyone` role.

### 3.8 `webhooks` (optional, list)

```yaml
webhooks:
  - name: CI Notifier              # required, 1-80 chars
    channel: ci-deployments        # required, name of a text or forum channel
    avatar: ./assets/ci.png        # optional, path or URL
```

Webhooks are created with a random token (Discord-generated). The token is
**not** stored in state — only the webhook URL is. Regenerating a webhook
requires `guildforge destroy` + re-apply.

### 3.9 `invites` (optional, list)

```yaml
invites:
  - channel: announcements          # required, channel name
    max_age: 86400                  # optional, seconds, 0=never, max 604800
    max_uses: 0                     # optional, 0=unlimited, max 100
    temporary: false                # optional, bool
    unique: false                   # optional, bool
```

Invite codes are stored in state so they can be revoked by `guildforge
destroy`. Existing invites on a channel that are not in config are left
alone (GuildForge never revokes invites it did not create).

### 3.10 `forum_tags` (optional, mapping)

Keyed by forum channel name. Lists tags that must exist on the channel.

```yaml
forum_tags:
  help:
    - name: Question               # required, 1-20 chars
      moderated: false             # optional, bool
      emoji: ❓                    # optional, unicode emoji only (custom emoji not supported)
    - name: Answered
      moderated: true
      emoji: ✅
```

Discord allows max 20 tags per forum channel. Duplicate tag names within a
channel are errors.

### 3.11 `welcome_screen` (optional, mapping)

Server-wide welcome screen. Requires Community server feature.

```yaml
welcome_screen:
  enabled: true
  description: |
    Welcome to Augment Infotech! Head to #general to say hi.
  channels:                        # up to 5
    - channel: general
      description: Say hello
    - channel: announcements
      description: Read our latest news
```

### 3.12 `server_guide` (optional, mapping)

Server guide / onboarding. Limited by Discord API; see §13 for known
limitations.

```yaml
server_guide:
  enabled: true
  welcome_message: |
    Welcome! Here's how to get started.
  recommended_channels:            # up to 7
    - channel: announcements
      description: Start here
    - channel: general
      description: General chat
```

### 3.13 `ordering` (optional, mapping)

Explicit position overrides. By default GuildForge orders roles, categories,
and channels by their position in the YAML file. Use `ordering` to override.

```yaml
ordering:
  roles: [Admin, Staff, Member, everyone]  # list, top-to-bottom = highest to lowest
  categories: [COMPANY, ENGINEERING, SOCIAL]
  channels:
    COMPANY: [announcements, general, random]  # per-category
    ENGINEERING: [eng-general, eng-releases, eng-incidents]
    _top_level: [lobby]              # channels not in any category
```

## 4. Required vs Optional Summary

| Top-level key | Required | Default | Notes |
|---|---|---|---|
| `server` | ✅ | — | Exactly one |
| `roles` | ❌ | `[]` | |
| `categories` | ❌ | `[]` | |
| `channels` | ❌ | `[]` | |
| `permissions` | ❌ | `{}` | shorthand |
| `permission_overwrites` | ❌ | `[]` | full form |
| `webhooks` | ❌ | `[]` | |
| `invites` | ❌ | `[]` | |
| `forum_tags` | ❌ | `{}` | |
| `welcome_screen` | ❌ | `None` | |
| `server_guide` | ❌ | `None` | |
| `ordering` | ❌ | auto | |

## 5. Validation Rules

The validator (`crates/validation`) enforces every rule below. Each rule
has a stable error code (`V001`, `V002`, ...) used in diagnostics and tests.

### 5.1 Uniqueness

- **V001** — Role names must be unique (case-insensitive).
- **V002** — Category names must be unique (case-insensitive).
- **V003** — Channel names must be unique within their parent (category or
  top-level), case-insensitive.
- **V004** — Tag names must be unique within a forum channel, case-insensitive.

### 5.2 References

- **V010** — Every `category: <name>` reference in a channel must resolve to
  a declared `categories` entry.
- **V011** — Every role name in a `permissions` block must resolve to a
  declared role OR be the literal `everyone`.
- **V012** — Every `channel: <name>` reference (in `webhooks`, `invites`,
  `permission_overwrites`, `welcome_screen`, `server_guide`) must resolve to
  a declared channel.
- **V013** — `server.system_channel` and `server.afk_channel` must resolve
  to a declared channel of the correct type (text / voice).

### 5.3 Discord API Limits

- **V020** — At most 250 roles (Discord hard limit).
- **V021** — At most 500 channels total (Discord hard limit).
- **V022** — At most 50 categories (Discord hard limit).
- **V023** — At most 20 forum tags per forum channel.
- **V024** — At most 50 webhooks per channel.
- **V025** — At most 5 channels in `welcome_screen.channels`.
- **V026** — At most 7 channels in `server_guide.recommended_channels`.

### 5.4 Type-Specific

- **V030** — `server.name` must be 2-100 chars.
- **V031** — `server.description` must be ≤120 chars.
- **V032** — `server.verification_level` must be one of the 5 enum values.
- **V033** — `server.afk_timeout` must be one of: 60, 300, 900, 1800, 3600.
- **V034** — Channel `topic` must be ≤1024 chars.
- **V035** — Channel `slowmode` must be 0-21600.
- **V036** — Voice `bitrate` must be 8000-384000.
- **V037** — Voice `user_limit` must be 0-99.
- **V038** — Role `name` must be 1-100 chars.
- **V039** — Webhook `name` must be 1-80 chars.
- **V040** — Forum tag `name` must be 1-20 chars.

### 5.5 Color Validation

- **V050** — Named color must be one of the 20 standard Discord colors (§11).
- **V051** — Hex color must be `#RRGGBB` or `0xRRGGBB`, 6 hex digits.
- **V052** — `rgb()` syntax must have 3 integers 0-255.

### 5.6 Semantic

- **V060** — Categories cannot be nested (a category cannot have a `category`
  parent).
- **V061** — Voice-only fields (`bitrate`, `user_limit`, `rtc_region`) are
  only valid on `voice` or `stage_voice` channels.
- **V062** — Forum-only fields (`available_tags`, `default_reaction_emoji`,
  `default_sort_order`) are only valid on `forum` channels.
- **V063** — `forum_tags` can only reference forum channels.
- **V064** — `welcome_screen` and `server_guide` require the guild to be a
  Community server. (The validator cannot check this at config time; it emits
  a warning, and the provider enforces it at apply time.)
- **V065** — `category` of a `forum` channel requires guild boost level ≥1.
  (Same warning-only-at-validate, enforce-at-apply pattern.)

### 5.7 Ordering

- **V070** — `ordering.roles` must include every declared role exactly once
  plus optionally `everyone`.
- **V071** — `ordering.categories` must include every declared category
  exactly once.
- **V072** — `ordering.channels.<category>` must include every declared
  channel in that category exactly once.

## 6. Stable Export Format

`guildforge export` produces YAML that, when re-parsed, produces a `Config`
byte-identical to the original (after normalization). Normalization rules:

- Top-level keys in the order of §3.
- Within `roles` / `categories` / `channels`, items in declaration order
  (NOT alphabetical).
- Within each item, keys in the order specified in §3.2 / §3.3 / §3.4.
- Optional fields with default values are omitted.
- Strings are quoted only when necessary (avoid `serde_yaml`'s default of
  always quoting).
- Indentation: 2 spaces.
- Line endings: LF.
- Trailing newline: yes.

These rules make exported YAML diff cleanly in git.

## 7. Example (Minimal)

```yaml
server:
  name: Augment Infotech

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
        permissions:
          read: [everyone]
          write: [Admin]
      - name: general
        type: text
        permissions:
          read: [everyone]
          write: [Staff]
```

## 8. Example (Full)

See [`examples/company.yaml`](../examples/company.yaml) and
[`examples/community.yaml`](../examples/community.yaml).

## 9. Forbidden Patterns

- **F001** — Dynamic keys (`{<role>: <permissions>}` at top level). Always
  use the declared structure.
- **F002** — `*` or `_` as a "match all" role name. Use `everyone` for the
  `@everyone` role.
- **F003** — Bare member IDs (numbers) in `permissions.target`. Member
  overwrites are only supported in `permission_overwrites` with `type:
  member` and an explicit numeric ID.
- **F004** — Channel IDs (Snowflakes) where names are expected. GuildForge
  is name-first; the import flow translates IDs to names.
- **F005** — YAML anchors / aliases. Forbidden in v1.

## 10. Permission Names

The complete set of Discord permissions recognized by GuildForge. Names are
`snake_case` and case-sensitive.

```
create_instant_invite          kick_members
ban_members                    administrator
manage_channels                manage_guild
add_reactions                  view_audit_log
priority_speaker               stream
view_channel                   send_messages
send_tts_messages              manage_messages
embed_links                    attach_files
read_message_history           mention_everyone
use_external_emojis            view_guild_insights
connect                        speak
mute_members                   deafen_members
move_members                   use_vad
change_nickname                manage_nicknames
manage_roles                   manage_webhooks
manage_emojis_and_stickers     use_application_commands
request_to_speak               manage_events
manage_threads                 create_public_threads
create_private_threads          use_external_stickers
send_messages_in_threads       use_embedded_activities
moderate_members               view_creator_monetization_analytics
use_soundboard                 create_expressions
use_external_sounds            send_voice_messages
```

Any permission name not in this list is a **V075** error.

## 11. Named Colors

The 20 standard Discord role colors:

```
default      white        black        dark_gray
lighter_gray darker_gray  light_gray   very_dark_gray
red          dark_red     orange       dark_orange
gold         dark_gold    yellow       dark_yellow
green        dark_green   teal         dark_teal
blue         dark_blue    purple        dark_purple
magenta      dark_magenta light_pink   dark_pink
```

Plus `default` (Discord's theme-dependent default).

## 12. Resource Addressing

Every resource in config has a stable address used in diagnostics, plan
output, and state:

| Resource | Address format |
|---|---|
| Role | `role/<name>` |
| Category | `category/<name>` |
| Channel (top-level) | `channel/<name>` |
| Channel (in category) | `channel/<category>/<name>` |
| Permission overwrite | `overwrite/<channel>/<role-or-member>` |
| Webhook | `webhook/<channel>/<name>` |
| Invite | `invite/<channel>/<code-or-index>` |
| Forum tag | `tag/<channel>/<name>` |

## 13. Known Limitations (Discord API)

The following are **not supported** in v1 because Discord does not expose
them through the public bot API, or exposes them only with features GuildForge
does not currently use:

- **AutoMod rules.** Not in the public bot API as of 2024-12. Tracked for
  future support if Discord opens it.
- **Server guide custom emojis.** API exists but is unstable; deferred.
- **Voice channel region overrides.** Deprecated by Discord; intentionally
  unsupported.
- **Role icons via custom emoji.** Only `unicode_emoji` and PNG `icon` are
  supported; custom-emoji role icons are not.
- **Stage channel permissions.** Stage channels inherit from category; we
  do not support per-stage-overwrites beyond what the channel type allows.
- **Thread creation.** Threads are message-derived; GuildForge manages
  forum channel structure but not threads inside forum channels.
- **Emoji management.** Deferred to Phase 7+.

For each limitation, GuildForge emits a warning at validate time if the
config attempts to use the unsupported feature, and refuses to apply.

## 14. Versioning

The schema version is encoded in the file as an optional top-level comment
or as a `_schema_version: 1` key. If present and > 1, the parser rejects with
a clear "schema version N not supported by this GuildForge build" message.
If absent, v1 is assumed.

Future v2 will require a `guildforge migrate` command that transforms v1
files to v2 syntax. v1 support is maintained for one major GuildForge version
after v2 ships.
