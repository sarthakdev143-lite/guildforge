# Discord API Limitations

> Features that GuildForge **cannot** support in v1 because Discord does
> not expose them through the public bot API, or exposes them only with
> features GuildForge does not currently use. This document is the
> authoritative reference for what's missing and why.
>
> For each limitation, GuildForge emits a warning at validate time if
> the config attempts to use the unsupported feature, and refuses to
> apply.

## 1. AutoMod rules

**Status**: Not supported.

**Reason**: As of 2024-12, Discord does not expose AutoMod rule CRUD
through the public bot REST API. AutoMod rules are only configurable
through the Discord client UI.

**User impact**: Configs that try to declare AutoMod rules will be
rejected at validate time. The schema has no `automod` key.

**Tracking**: Revisit if Discord exposes the API. Tracked in
[`ROADMAP.md`](../ROADMAP.md) Phase 7+.

## 2. Server guide (onboarding) full prompt editing

**Status**: Partial — only `enabled` and `recommended_channels` are
writable.

**Reason**: Discord's `PATCH /guilds/:id/onboarding` endpoint requires
the **full** onboarding object in the request body, including all
prompts and options. Patching partial data clobbers the rest. GuildForge
v1 cannot safely merge config-supplied prompts with existing prompts
without a read-modify-write cycle that risks data loss.

**User impact**: `server_guide.enabled` and
`server_guide.recommended_channels` are respected. The
`server_guide.welcome_message` field is read-only (parsed but not
written back). Custom prompt definitions are silently ignored.

**Tracking**: Full prompt editing lands in Phase 7+ when we add a
read-modify-write reconcile pass.

## 3. Custom-emoji role icons

**Status**: Not supported.

**Reason**: Discord allows role icons to be either a PNG image (via
multipart upload) or a custom emoji reference. GuildForge v1 supports
the PNG path (via `roles[].icon`) and unicode emoji (via
`roles[].unicode_emoji`) but not custom-emoji references, because
custom emoji belong to specific guilds and managing them adds
significant complexity.

**User impact**: Configs that try to set a custom-emoji role icon will
be rejected at validate time. Use `unicode_emoji` or `icon` instead.

**Tracking**: Revisit in Phase 7+ alongside emoji management.

## 4. Custom-emoji forum tags

**Status**: Not supported.

**Reason**: Same as #3 — custom-emoji references add complexity.

**User impact**: `forum_tags[].emoji` only accepts unicode emoji (a
single character). Configs that try to use custom-emoji references are
rejected at validate time.

## 5. Voice channel region overrides

**Status**: Not supported.

**Reason**: Deprecated by Discord. The `rtc_region` field still exists
in the schema for backward compatibility but Discord ignores it on
write.

**User impact**: Setting `rtc_region` in a voice channel has no effect
at apply time. A warning is emitted.

## 6. Stage channel permission overwrites

**Status**: Partial.

**Reason**: Stage channels inherit permissions from their parent
category. Per-stage-overwrites beyond what the channel type allows are
silently rejected by Discord.

**User impact**: Permission overwrites on stage channels are best
applied at the category level, not the channel level.

## 7. Thread creation

**Status**: Not supported.

**Reason**: Threads are message-derived; they exist only in the context
of a parent message. GuildForge manages forum channel **structure**
(including available tags) but not threads inside forum channels.

**User impact**: There is no `threads:` key in the schema. Threads
created by users via the Discord client are left alone.

## 8. Emoji management

**Status**: Not supported in v1.

**Reason**: Emoji upload requires multipart form data, which our HTTP
client doesn't yet support. Emoji management also has tighter rate
limits than other resources.

**User impact**: No `emojis:` key in the schema. Existing guild emojis
are not touched by `apply` or `destroy`.

**Tracking**: Phase 7+.

## 9. Integration management

**Status**: Not supported in v1.

**Reason**: Integrations (Twitch, YouTube, bots) are managed through
different APIs and have complex permission requirements.

**User impact**: No `integrations:` key in the schema.

**Tracking**: Phase 7+.

## 10. Server guide / onboarding welcome message

**Status**: Not written.

**Reason**: The Discord onboarding API does not have a top-level
"welcome message" field; welcome text is part of the prompts structure
which we don't write (see #2).

**User impact**: `server_guide.welcome_message` is parsed from the
config and stored in state, but never written to Discord.

## 11. Webhook token rotation

**Status**: Not supported.

**Reason**: Discord generates webhook tokens at creation time. The
token is returned in the create response and stored in state. Discord
does not expose an endpoint to rotate a webhook token without deleting
and recreating the webhook.

**User impact**: To rotate a webhook token, run `guildforge destroy`
(which deletes the webhook) and then `guildforge apply` (which creates
a new one with a fresh token).

## 12. Channel follower (announcement → other guild) management

**Status**: Not supported.

**Reason**: Follower channels require cross-guild permissions that
GuildForge doesn't manage.

**User impact**: Announcement channels are created and configured, but
their followers (other guilds that subscribe to announcements) are not
managed.

## 13. Audit log access

**Status**: Not supported.

**Reason**: Audit logs are read-only and not part of the desired-state
model.

## 14. Guild template management

**Status**: Not supported.

**Reason**: Guild templates are a separate concept from desired-state
config; mixing them would be confusing.

## 15. Sticker management

**Status**: Not supported in v1.

**Reason**: Same as emoji — multipart upload, tight rate limits.

## Summary table

| Feature | Status | Tracking |
|---|---|---|
| AutoMod rules | Not supported | Phase 7+ (if API opens) |
| Server guide full prompts | Partial | Phase 7+ |
| Custom-emoji role icons | Not supported | Phase 7+ |
| Custom-emoji forum tags | Not supported | Phase 7+ |
| Voice region overrides | Not supported (deprecated by Discord) | — |
| Stage channel overwrites | Partial (use category) | — |
| Thread creation | Not supported | — |
| Emoji management | Not supported | Phase 7+ |
| Integration management | Not supported | Phase 7+ |
| Server guide welcome_message | Parsed only (not written) | Phase 7+ |
| Webhook token rotation | Not supported (delete + re-create) | — |
| Channel follower management | Not supported | — |
| Audit log access | Not supported (read-only) | — |
| Guild template management | Not supported | — |
| Sticker management | Not supported | Phase 7+ |

## Reporting a missing feature

If you believe a feature is missing from this list that should be
supported, open an issue with:

1. The feature name.
2. The Discord API endpoint you expected to use.
3. The use case (what you're trying to accomplish).
4. Whether the feature is in the public bot API.

We'll either add support or document the limitation here.
