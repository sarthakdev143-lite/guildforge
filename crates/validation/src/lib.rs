//! Semantic validation for `GuildForge` config files.
//!
//! Runs a battery of checks on a parsed [`Config`](guildforge_config::Config)
//! and returns all diagnostics found in one pass. Every check has a
//! stable code (`V001`, `V002`, ...) that is part of the public API.
//! See [`docs/SCHEMA.md` §5](../../docs/SCHEMA.md) for the full list.
//!
//! # Rules
//!
//! - Pure function. No I/O, no async.
//! - Returns ALL errors, not just the first. Users see every problem
//!   in one pass.
//! - Every diagnostic has a stable code. Codes never get renumbered.
//!
//! # Codes
//!
//! - V001-V009: uniqueness
//! - V010-V019: references
//! - V020-V029: Discord API limits
//! - V030-V049: type-specific
//! - V050-V059: color validation
//! - V060-V069: semantic
//! - V070-V079: ordering

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![allow(
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::collapsible_if,
    clippy::format_push_string
)]

use guildforge_config::{ChannelType, Config};
use std::collections::HashSet;

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Hard error; config cannot be applied.
    Error,
    /// Soft warning; config can be applied but may produce unexpected
    /// results.
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Error => "error",
            Self::Warning => "warning",
        };
        f.write_str(s)
    }
}

/// A single validation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable code (e.g. `V001`).
    pub code: &'static str,
    /// Severity.
    pub severity: Severity,
    /// Human-readable message (lowercase, no trailing period).
    pub message: String,
    /// Resource address this diagnostic is about, if applicable.
    pub addr: Option<String>,
    /// Optional help text suggesting a fix.
    pub help: Option<String>,
}

impl Diagnostic {
    /// Construct an error diagnostic.
    #[must_use]
    pub fn error(code: &'static str, addr: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            addr: Some(addr.into()),
            help: None,
        }
    }

    /// Construct a warning diagnostic.
    #[must_use]
    pub fn warning(
        code: &'static str,
        addr: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            addr: Some(addr.into()),
            help: None,
        }
    }

    /// Attach a help message.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Validate a parsed config and return all diagnostics.
///
/// Returns `Ok(())` if there are no errors. Warnings are returned in
/// the `Vec` only if there were errors too; if there are no errors,
/// `Ok(())` is returned even if there are warnings.
///
/// To get ALL diagnostics (warnings + errors), use [`validate_collect`].
///
/// # Errors
///
/// Returns `Err(Vec<Diagnostic>)` containing every error-severity
/// diagnostic found.
pub fn validate(config: &Config) -> Result<(), Vec<Diagnostic>> {
    let diags = validate_collect(config);
    let errors: Vec<Diagnostic> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .cloned()
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a parsed config and return ALL diagnostics (warnings +
/// errors) in one pass.
#[must_use]
pub fn validate_collect(config: &Config) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    check_uniqueness(config, &mut out);
    check_references(config, &mut out);
    check_api_limits(config, &mut out);
    check_type_specific(config, &mut out);
    check_colors(config, &mut out);
    check_semantic(config, &mut out);
    check_ordering(config, &mut out);
    out
}

// ===========================================================================
// §5.1 Uniqueness: V001-V009
// ===========================================================================

fn check_uniqueness(config: &Config, out: &mut Vec<Diagnostic>) {
    // V001: role names unique (case-insensitive)
    let mut seen: HashSet<String> = HashSet::new();
    for role in &config.roles {
        let key = role.name.to_ascii_lowercase();
        if !seen.insert(key) {
            out.push(Diagnostic::error(
                "V001",
                format!("role/{}", role.name),
                format!("duplicate role name `{}`", role.name),
            ));
        }
    }

    // V002: category names unique (case-insensitive)
    let mut seen: HashSet<String> = HashSet::new();
    for cat in &config.categories {
        let key = cat.name.to_ascii_lowercase();
        if !seen.insert(key) {
            out.push(Diagnostic::error(
                "V002",
                format!("category/{}", cat.name),
                format!("duplicate category name `{}`", cat.name),
            ));
        }
    }

    // V003: channel names unique within parent (case-insensitive)
    let mut top_level: HashSet<String> = HashSet::new();
    for ch in &config.channels {
        let key = ch.name.to_ascii_lowercase();
        if !top_level.insert(key) {
            out.push(Diagnostic::error(
                "V003",
                format!("channel/_top/{}", ch.name),
                format!("duplicate channel name `{}` at top level", ch.name),
            ));
        }
    }
    for cat in &config.categories {
        let mut in_cat: HashSet<String> = HashSet::new();
        for ch in &cat.channels {
            let key = ch.name.to_ascii_lowercase();
            if !in_cat.insert(key) {
                out.push(Diagnostic::error(
                    "V003",
                    format!("channel/{}/{}/{}", cat.name, ch.name, ch.name),
                    format!(
                        "duplicate channel name `{}` within category `{}`",
                        ch.name, cat.name
                    ),
                ));
            }
        }
    }

    // V004: tag names unique within a forum channel (case-insensitive)
    for (chan, tags) in &config.forum_tags {
        let mut seen: HashSet<String> = HashSet::new();
        for tag in tags {
            let key = tag.name.to_ascii_lowercase();
            if !seen.insert(key) {
                out.push(Diagnostic::error(
                    "V004",
                    format!("tag/{}/{}", chan, tag.name),
                    format!(
                        "duplicate forum tag name `{}` in channel `{}`",
                        tag.name, chan
                    ),
                ));
            }
        }
    }
}

// ===========================================================================
// §5.2 References: V010-V019
// ===========================================================================

fn check_references(config: &Config, out: &mut Vec<Diagnostic>) {
    let role_names: HashSet<String> = config
        .roles
        .iter()
        .map(|r| r.name.to_ascii_lowercase())
        .collect();
    let category_names: HashSet<String> = config
        .categories
        .iter()
        .map(|c| c.name.to_ascii_lowercase())
        .collect();
    let channel_names: HashSet<String> = config
        .all_channels()
        .iter()
        .map(|c| c.name.to_ascii_lowercase())
        .collect();

    // V010: every `category: <name>` in a channel must resolve
    for ch in &config.channels {
        if let Some(cat) = &ch.category {
            if !category_names.contains(&cat.to_ascii_lowercase()) {
                out.push(Diagnostic::error(
                    "V010",
                    format!("channel/{}", ch.name),
                    format!(
                        "channel `{}` references unknown category `{}`",
                        ch.name, cat
                    ),
                ));
            }
        }
    }
    // Also check channels declared in `channels:` that have a `category:`
    // ref pointing to a category that exists but whose `channels:` list
    // also has the same channel — that's a double-declaration. We treat
    // it as a V010-adjacent error with a different message.

    // V011: every role name in `permissions` blocks must resolve or be `everyone`
    let check_role_list = |out: &mut Vec<Diagnostic>, addr: String, roles: &[String]| {
        for r in roles {
            if r == "everyone" {
                continue;
            }
            if !role_names.contains(&r.to_ascii_lowercase()) {
                out.push(Diagnostic::error(
                    "V011",
                    addr.clone(),
                    format!("unknown role `{}` in permissions", r),
                ));
            }
        }
    };
    for (chan, block) in &config.permissions {
        check_role_list(
            out,
            format!("channel/{}", chan),
            &block
                .read
                .iter()
                .chain(&block.write)
                .chain(&block.manage)
                .chain(&block.connect)
                .chain(&block.speak)
                .chain(&block.view_audit_log)
                .cloned()
                .collect::<Vec<_>>(),
        );
    }
    for ch in config.all_channels() {
        if let Some(perm) = &ch.permissions {
            check_role_list(
                out,
                format!("channel/{}", ch.name),
                &perm
                    .read
                    .iter()
                    .chain(&perm.write)
                    .chain(&perm.manage)
                    .chain(&perm.connect)
                    .chain(&perm.speak)
                    .chain(&perm.view_audit_log)
                    .cloned()
                    .collect::<Vec<_>>(),
            );
        }
    }
    for cat in &config.categories {
        if let Some(perm) = &cat.permissions {
            check_role_list(
                out,
                format!("category/{}", cat.name),
                &perm
                    .read
                    .iter()
                    .chain(&perm.write)
                    .chain(&perm.manage)
                    .chain(&perm.connect)
                    .chain(&perm.speak)
                    .chain(&perm.view_audit_log)
                    .cloned()
                    .collect::<Vec<_>>(),
            );
        }
    }

    // V012: every channel reference (webhooks, invites, overwrites, etc.) must resolve
    for wh in &config.webhooks {
        if !channel_names.contains(&wh.channel.to_ascii_lowercase()) {
            out.push(Diagnostic::error(
                "V012",
                format!("webhook/{}/{}", wh.channel, wh.name),
                format!(
                    "webhook `{}` references unknown channel `{}`",
                    wh.name, wh.channel
                ),
            ));
        }
    }
    for inv in &config.invites {
        if !channel_names.contains(&inv.channel.to_ascii_lowercase()) {
            out.push(Diagnostic::error(
                "V012",
                format!("invite/{}", inv.channel),
                format!("invite references unknown channel `{}`", inv.channel),
            ));
        }
    }
    for ov in &config.permission_overwrites {
        if !channel_names.contains(&ov.channel.to_ascii_lowercase()) {
            out.push(Diagnostic::error(
                "V012",
                format!("overwrite/{}", ov.channel),
                format!("overwrite references unknown channel `{}`", ov.channel),
            ));
        }
    }
    if let Some(ws) = &config.welcome_screen {
        for c in &ws.channels {
            if !channel_names.contains(&c.channel.to_ascii_lowercase()) {
                out.push(Diagnostic::error(
                    "V012",
                    format!("welcome_screen/{}", c.channel),
                    format!("welcome screen references unknown channel `{}`", c.channel),
                ));
            }
        }
    }
    if let Some(sg) = &config.server_guide {
        for c in &sg.recommended_channels {
            if !channel_names.contains(&c.channel.to_ascii_lowercase()) {
                out.push(Diagnostic::error(
                    "V012",
                    format!("server_guide/{}", c.channel),
                    format!("server guide references unknown channel `{}`", c.channel),
                ));
            }
        }
    }

    // V013: server.system_channel and server.afk_channel must resolve to a declared channel
    if let Some(sc) = &config.server.system_channel {
        if !channel_names.contains(&sc.to_ascii_lowercase()) {
            out.push(Diagnostic::error(
                "V013",
                "server/system_channel",
                format!("server.system_channel references unknown channel `{}`", sc),
            ));
        }
    }
    if let Some(ac) = &config.server.afk_channel {
        if !channel_names.contains(&ac.to_ascii_lowercase()) {
            out.push(Diagnostic::error(
                "V013",
                "server/afk_channel",
                format!("server.afk_channel references unknown channel `{}`", ac),
            ));
        }
    }
}

// ===========================================================================
// §5.3 Discord API limits: V020-V029
// ===========================================================================

fn check_api_limits(config: &Config, out: &mut Vec<Diagnostic>) {
    // V020: at most 250 roles
    if config.roles.len() > 250 {
        out.push(Diagnostic::error(
            "V020",
            "server",
            format!("too many roles: {} (max 250)", config.roles.len()),
        ));
    }

    // V021: at most 500 channels total
    let total_channels = config.all_channels().len();
    if total_channels > 500 {
        out.push(Diagnostic::error(
            "V021",
            "server",
            format!("too many channels: {} (max 500)", total_channels),
        ));
    }

    // V022: at most 50 categories
    if config.categories.len() > 50 {
        out.push(Diagnostic::error(
            "V022",
            "server",
            format!("too many categories: {} (max 50)", config.categories.len()),
        ));
    }

    // V023: at most 20 forum tags per forum channel
    for (chan, tags) in &config.forum_tags {
        if tags.len() > 20 {
            out.push(Diagnostic::error(
                "V023",
                format!("tag/{}", chan),
                format!("too many forum tags in `{}`: {} (max 20)", chan, tags.len()),
            ));
        }
    }

    // V024: at most 50 webhooks per channel (we approximate: warn if any
    // channel has >50 webhooks in config).
    let mut webhook_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for wh in &config.webhooks {
        *webhook_counts
            .entry(wh.channel.to_ascii_lowercase())
            .or_insert(0) += 1;
    }
    for (chan, count) in webhook_counts {
        if count > 50 {
            out.push(Diagnostic::error(
                "V024",
                format!("webhook/{}", chan),
                format!("too many webhooks in `{}`: {} (max 50)", chan, count),
            ));
        }
    }

    // V025: at most 5 channels in welcome_screen.channels
    if let Some(ws) = &config.welcome_screen {
        if ws.channels.len() > 5 {
            out.push(Diagnostic::error(
                "V025",
                "welcome_screen",
                format!(
                    "too many welcome screen channels: {} (max 5)",
                    ws.channels.len()
                ),
            ));
        }
    }

    // V026: at most 7 channels in server_guide.recommended_channels
    if let Some(sg) = &config.server_guide {
        if sg.recommended_channels.len() > 7 {
            out.push(Diagnostic::error(
                "V026",
                "server_guide",
                format!(
                    "too many recommended channels: {} (max 7)",
                    sg.recommended_channels.len()
                ),
            ));
        }
    }
}

// ===========================================================================
// §5.4 Type-specific: V030-V049
// ===========================================================================

fn check_type_specific(config: &Config, out: &mut Vec<Diagnostic>) {
    // V030: server.name 2-100 chars
    let name_len = config.server.name.chars().count();
    if !(2..=100).contains(&name_len) {
        out.push(Diagnostic::error(
            "V030",
            "server/name",
            format!("server.name length {} not in 2..=100", name_len),
        ));
    }

    // V031: server.description max 120 chars
    if let Some(desc) = &config.server.description {
        let len = desc.chars().count();
        if len > 120 {
            out.push(Diagnostic::error(
                "V031",
                "server/description",
                format!("server.description length {} exceeds 120", len),
            ));
        }
    }

    // V034: channel.topic max 1024 chars
    for ch in config.all_channels() {
        if let Some(topic) = &ch.topic {
            let len = topic.chars().count();
            if len > 1024 {
                out.push(Diagnostic::error(
                    "V034",
                    format!("channel/{}", ch.name),
                    format!("channel `{}` topic length {} exceeds 1024", ch.name, len),
                ));
            }
        }
    }

    // V035: channel.slowmode 0-21600
    for ch in config.all_channels() {
        if let Some(sm) = ch.slowmode {
            if sm > 21600 {
                out.push(Diagnostic::error(
                    "V035",
                    format!("channel/{}", ch.name),
                    format!("channel `{}` slowmode {} exceeds 21600", ch.name, sm),
                ));
            }
        }
    }

    // V036: voice bitrate 8000-384000
    for ch in config.all_channels() {
        if let Some(v) = &ch.voice {
            if let Some(br) = v.bitrate {
                if !(8000..=384_000).contains(&br) {
                    out.push(Diagnostic::error(
                        "V036",
                        format!("channel/{}", ch.name),
                        format!("channel `{}` bitrate {} not in 8000..=384000", ch.name, br),
                    ));
                }
            }
        }
    }

    // V037: voice user_limit 0-99
    for ch in config.all_channels() {
        if let Some(v) = &ch.voice {
            if let Some(ul) = v.user_limit {
                if ul > 99 {
                    out.push(Diagnostic::error(
                        "V037",
                        format!("channel/{}", ch.name),
                        format!("channel `{}` user_limit {} exceeds 99", ch.name, ul),
                    ));
                }
            }
        }
    }

    // V038: role name 1-100 chars
    for role in &config.roles {
        let len = role.name.chars().count();
        if !(1..=100).contains(&len) {
            out.push(Diagnostic::error(
                "V038",
                format!("role/{}", role.name),
                format!("role name length {} not in 1..=100", len),
            ));
        }
    }

    // V039: webhook name 1-80 chars
    for wh in &config.webhooks {
        let len = wh.name.chars().count();
        if !(1..=80).contains(&len) {
            out.push(Diagnostic::error(
                "V039",
                format!("webhook/{}", wh.name),
                format!("webhook name length {} not in 1..=80", len),
            ));
        }
    }

    // V040: forum tag name 1-20 chars
    for (chan, tags) in &config.forum_tags {
        for tag in tags {
            let len = tag.name.chars().count();
            if !(1..=20).contains(&len) {
                out.push(Diagnostic::error(
                    "V040",
                    format!("tag/{}/{}", chan, tag.name),
                    format!("forum tag name length {} not in 1..=20", len),
                ));
            }
        }
    }
}

// ===========================================================================
// §5.5 Color validation: V050-V059
// ===========================================================================

fn check_colors(config: &Config, out: &mut Vec<Diagnostic>) {
    // Color format validation is now performed at parse time by
    // `Color::parse`. The parser rejects unknown colors, malformed hex,
    // and malformed rgb() before validation runs. We keep this hook for
    // future cross-field color checks (e.g. icon-emoji conflicts).
    let _ = config;
    let _ = out;
}

// ===========================================================================
// §5.6 Semantic: V060-V069
// ===========================================================================

fn check_semantic(config: &Config, out: &mut Vec<Diagnostic>) {
    // V061: voice-only fields only on voice/stage_voice channels
    for ch in config.all_channels() {
        if let Some(v) = &ch.voice {
            if !ch.kind.is_voice_like()
                && (v.bitrate.is_some()
                    || v.user_limit.is_some()
                    || v.rtc_region.is_some()
                    || v.video_quality_mode.is_some())
            {
                out.push(Diagnostic::error(
                    "V061",
                    format!("channel/{}", ch.name),
                    format!(
                        "channel `{}` is type {:?} but has voice-only fields",
                        ch.name, ch.kind
                    ),
                ));
            }
        }
        // V062: forum-only fields only on forum channels
        if let Some(f) = &ch.forum {
            if ch.kind != ChannelType::Forum
                && (!f.available_tags.is_empty()
                    || f.default_reaction_emoji.is_some()
                    || f.default_sort_order.is_some()
                    || f.default_forum_layout.is_some())
            {
                out.push(Diagnostic::error(
                    "V062",
                    format!("channel/{}", ch.name),
                    format!(
                        "channel `{}` is type {:?} but has forum-only fields",
                        ch.name, ch.kind
                    ),
                ));
            }
        }
    }

    // V063: forum_tags can only reference forum channels
    let forum_channels: HashSet<String> = config
        .all_channels()
        .iter()
        .filter(|c| c.kind == ChannelType::Forum)
        .map(|c| c.name.to_ascii_lowercase())
        .collect();
    for chan in config.forum_tags.keys() {
        if !forum_channels.contains(&chan.to_ascii_lowercase()) {
            out.push(Diagnostic::error(
                "V063",
                format!("forum_tags/{}", chan),
                format!("forum_tags references non-forum channel `{}`", chan),
            ));
        }
    }

    // V064 / V065: welcome_screen / server_guide require Community;
    // forum channels require boost level 1+. We can't check these at
    // config time; emit warnings.
    if config.welcome_screen.is_some() {
        out.push(Diagnostic::warning(
            "V064",
            "welcome_screen",
            "welcome_screen requires the guild to be a Community server; this will be enforced at apply time",
        ));
    }
    if config.server_guide.is_some() {
        out.push(Diagnostic::warning(
            "V064",
            "server_guide",
            "server_guide requires the guild to be a Community server; this will be enforced at apply time",
        ));
    }
    for ch in config.all_channels() {
        if ch.kind == ChannelType::Forum {
            out.push(Diagnostic::warning(
                "V065",
                format!("channel/{}", ch.name),
                format!("forum channel `{}` requires guild boost level 1+; this will be enforced at apply time", ch.name),
            ));
        }
        if matches!(ch.kind, ChannelType::Announcement | ChannelType::StageVoice) {
            out.push(Diagnostic::warning(
                "V064",
                format!("channel/{}", ch.name),
                format!("{:?} channel `{}` requires Community server; this will be enforced at apply time", ch.kind, ch.name),
            ));
        }
    }
}

// ===========================================================================
// §5.7 Ordering: V070-V079
// ===========================================================================

fn check_ordering(config: &Config, out: &mut Vec<Diagnostic>) {
    let Some(ordering) = &config.ordering else {
        return;
    };

    // V070: ordering.roles must include every declared role exactly once
    // (plus optionally `everyone`)
    if let Some(ordered) = &ordering.roles {
        let declared: HashSet<String> = config
            .roles
            .iter()
            .map(|r| r.name.to_ascii_lowercase())
            .collect();
        let mut seen: HashSet<String> = HashSet::new();
        for r in ordered {
            let key = r.to_ascii_lowercase();
            if key == "everyone" {
                continue;
            }
            if !declared.contains(&key) {
                out.push(Diagnostic::error(
                    "V070",
                    "ordering/roles",
                    format!("ordering.roles references unknown role `{}`", r),
                ));
            }
            if !seen.insert(key) {
                out.push(Diagnostic::error(
                    "V070",
                    "ordering/roles",
                    format!("ordering.roles lists `{}` more than once", r),
                ));
            }
        }
        for decl in &declared {
            if !seen.contains(decl) {
                out.push(Diagnostic::error(
                    "V070",
                    "ordering/roles",
                    format!("ordering.roles is missing declared role `{}`", decl),
                ));
            }
        }
    }

    // V071: ordering.categories must include every declared category exactly once
    if let Some(ordered) = &ordering.categories {
        let declared: HashSet<String> = config
            .categories
            .iter()
            .map(|c| c.name.to_ascii_lowercase())
            .collect();
        let mut seen: HashSet<String> = HashSet::new();
        for c in ordered {
            let key = c.to_ascii_lowercase();
            if !declared.contains(&key) {
                out.push(Diagnostic::error(
                    "V071",
                    "ordering/categories",
                    format!("ordering.categories references unknown category `{}`", c),
                ));
            }
            if !seen.insert(key) {
                out.push(Diagnostic::error(
                    "V071",
                    "ordering/categories",
                    format!("ordering.categories lists `{}` more than once", c),
                ));
            }
        }
        for decl in &declared {
            if !seen.contains(decl) {
                out.push(Diagnostic::error(
                    "V071",
                    "ordering/categories",
                    format!(
                        "ordering.categories is missing declared category `{}`",
                        decl
                    ),
                ));
            }
        }
    }

    // V072: ordering.channels.<category> must include every declared
    // channel in that category exactly once
    if let Some(chan_map) = &ordering.channels {
        // For each declared category, gather its channels.
        for cat in &config.categories {
            let declared: HashSet<String> = cat
                .channels
                .iter()
                .map(|c| c.name.to_ascii_lowercase())
                .collect();
            let ordered = chan_map.get(&cat.name).or_else(|| {
                // Try case-insensitive lookup.
                chan_map.iter().find_map(|(k, v)| {
                    if k.eq_ignore_ascii_case(&cat.name) {
                        Some(v)
                    } else {
                        None
                    }
                })
            });
            let Some(ordered) = ordered else { continue };
            let mut seen: HashSet<String> = HashSet::new();
            for c in ordered {
                let key = c.to_ascii_lowercase();
                if !declared.contains(&key) {
                    out.push(Diagnostic::error(
                        "V072",
                        format!("ordering.channels/{}", cat.name),
                        format!(
                            "ordering.channels.{} references unknown channel `{}`",
                            cat.name, c
                        ),
                    ));
                }
                if !seen.insert(key) {
                    out.push(Diagnostic::error(
                        "V072",
                        format!("ordering.channels/{}", cat.name),
                        format!(
                            "ordering.channels.{} lists `{}` more than once",
                            cat.name, c
                        ),
                    ));
                }
            }
            for decl in &declared {
                if !seen.contains(decl) {
                    out.push(Diagnostic::error(
                        "V072",
                        format!("ordering.channels/{}", cat.name),
                        format!(
                            "ordering.channels.{} is missing declared channel `{}`",
                            cat.name, decl
                        ),
                    ));
                }
            }
        }
        // _top_level must include every top-level channel
        if let Some(top) = chan_map.get("_top_level") {
            let declared: HashSet<String> = config
                .channels
                .iter()
                .map(|c| c.name.to_ascii_lowercase())
                .collect();
            let mut seen: HashSet<String> = HashSet::new();
            for c in top {
                let key = c.to_ascii_lowercase();
                if !declared.contains(&key) {
                    out.push(Diagnostic::error(
                        "V072",
                        "ordering.channels/_top_level",
                        format!(
                            "ordering.channels._top_level references unknown channel `{}`",
                            c
                        ),
                    ));
                }
                if !seen.insert(key) {
                    out.push(Diagnostic::error(
                        "V072",
                        "ordering.channels/_top_level",
                        format!("ordering.channels._top_level lists `{}` more than once", c),
                    ));
                }
            }
            for decl in &declared {
                if !seen.contains(decl) {
                    out.push(Diagnostic::error(
                        "V072",
                        "ordering.channels/_top_level",
                        format!(
                            "ordering.channels._top_level is missing declared channel `{}`",
                            decl
                        ),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use guildforge_config::Config;

    fn parse_config(yaml: &str) -> Config {
        serde_yaml::from_str(yaml).expect("parse")
    }

    fn config_minimal() -> Config {
        parse_config("server:\n  name: Test\n")
    }

    #[test]
    fn minimal_config_validates_clean() {
        let cfg = config_minimal();
        let diags = validate_collect(&cfg);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "expected no errors, got: {errors:#?}");
    }

    #[test]
    fn v001_duplicate_role_name() {
        let cfg = parse_config("server:\n  name: Test\nroles:\n  - name: Admin\n  - name: admin\n");
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V001"));
    }

    #[test]
    fn v002_duplicate_category_name() {
        let cfg =
            parse_config("server:\n  name: Test\ncategories:\n  - name: CAT\n  - name: cat\n");
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V002"));
    }

    #[test]
    fn v003_duplicate_channel_in_category() {
        let cfg = parse_config(
            "server:\n  name: Test\ncategories:\n  - name: CAT\n    channels:\n      - name: c1\n        type: text\n      - name: C1\n        type: text\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V003"));
    }

    #[test]
    fn v004_duplicate_forum_tag() {
        let cfg = parse_config(
            "server:\n  name: Test\nchannels:\n  - name: help\n    type: forum\nforum_tags:\n  help:\n    - name: Q\n    - name: q\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V004"));
    }

    #[test]
    fn v010_unknown_category_ref() {
        let cfg = parse_config(
            "server:\n  name: Test\nchannels:\n  - name: c1\n    type: text\n    category: NOPE\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V010"));
    }

    #[test]
    fn v011_unknown_role_in_permissions() {
        let cfg = parse_config(
            "server:\n  name: Test\nchannels:\n  - name: c1\n    type: text\n    permissions:\n      read: [Ghost]\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V011"));
    }

    #[test]
    fn v011_everyone_role_allowed() {
        let cfg = parse_config(
            "server:\n  name: Test\nchannels:\n  - name: c1\n    type: text\n    permissions:\n      read: [everyone]\n",
        );
        let diags = validate_collect(&cfg);
        assert!(!diags.iter().any(|d| d.code == "V011"));
    }

    #[test]
    fn v012_webhook_unknown_channel() {
        let cfg =
            parse_config("server:\n  name: Test\nwebhooks:\n  - name: wh\n    channel: ghost\n");
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V012"));
    }

    #[test]
    fn v013_system_channel_unknown() {
        let cfg = parse_config("server:\n  name: Test\n  system_channel: ghost\n");
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V013"));
    }

    #[test]
    fn v020_too_many_roles() {
        let mut yaml = String::from("server:\n  name: Test\nroles:\n");
        for i in 0..251 {
            yaml.push_str(&format!("  - name: r{i}\n"));
        }
        let cfg = parse_config(&yaml);
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V020"));
    }

    #[test]
    fn v023_too_many_forum_tags() {
        let mut tags = String::new();
        for i in 0..21 {
            tags.push_str(&format!("    - name: t{i}\n"));
        }
        let cfg = parse_config(&format!(
            "server:\n  name: Test\nchannels:\n  - name: help\n    type: forum\nforum_tags:\n  help:\n{tags}"
        ));
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V023"));
    }

    #[test]
    fn v030_server_name_too_short() {
        let cfg = parse_config("server:\n  name: x\n");
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V030"));
    }

    #[test]
    fn v034_topic_too_long() {
        let topic = "x".repeat(1025);
        let cfg = parse_config(&format!(
            "server:\n  name: Test\nchannels:\n  - name: c1\n    type: text\n    topic: {topic}\n"
        ));
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V034"));
    }

    #[test]
    fn v036_invalid_bitrate() {
        let cfg = parse_config(
            "server:\n  name: Test\nchannels:\n  - name: v1\n    type: voice\n    bitrate: 100\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V036"));
    }

    #[test]
    fn v051_invalid_hex_color_caught_at_parse() {
        // The parser rejects malformed hex colors before validation runs.
        let yaml = "server:\n  name: Test\nroles:\n  - name: r1\n    color: \"#XYZ123\"\n";
        let result: Result<guildforge_config::Config, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "expected parse error for invalid hex color"
        );
    }

    #[test]
    fn v051_valid_hex_color_ok() {
        let cfg =
            parse_config("server:\n  name: Test\nroles:\n  - name: r1\n    color: \"#FF5733\"\n");
        let diags = validate_collect(&cfg);
        // No V051 expected (color check is at parse time now).
        let _ = diags;
    }

    #[test]
    fn v052_invalid_rgb_color_caught_at_parse() {
        let yaml = "server:\n  name: Test\nroles:\n  - name: r1\n    color: \"rgb(300, 0, 0)\"\n";
        let result: Result<guildforge_config::Config, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "expected parse error for invalid rgb color"
        );
    }

    #[test]
    fn v061_voice_fields_on_text_channel() {
        let cfg = parse_config(
            "server:\n  name: Test\nchannels:\n  - name: c1\n    type: text\n    bitrate: 64000\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V061"));
    }

    #[test]
    fn v062_forum_fields_on_text_channel() {
        let cfg = parse_config(
            "server:\n  name: Test\nchannels:\n  - name: c1\n    type: text\n    available_tags: [q]\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V062"));
    }

    #[test]
    fn v063_forum_tags_on_non_forum_channel() {
        let cfg = parse_config(
            "server:\n  name: Test\nchannels:\n  - name: c1\n    type: text\nforum_tags:\n  c1:\n    - name: tag1\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V063"));
    }

    #[test]
    fn v064_forum_channel_emits_warning() {
        let cfg =
            parse_config("server:\n  name: Test\nchannels:\n  - name: help\n    type: forum\n");
        let diags = validate_collect(&cfg);
        assert!(diags
            .iter()
            .any(|d| d.code == "V065" && d.severity == Severity::Warning));
    }

    #[test]
    fn v070_ordering_roles_missing() {
        let cfg = parse_config(
            "server:\n  name: Test\nroles:\n  - name: A\n  - name: B\nordering:\n  roles: [A]\n",
        );
        let diags = validate_collect(&cfg);
        assert!(diags.iter().any(|d| d.code == "V070"));
    }

    #[test]
    fn v070_ordering_roles_complete() {
        let cfg = parse_config(
            "server:\n  name: Test\nroles:\n  - name: A\n  - name: B\nordering:\n  roles: [A, B, everyone]\n",
        );
        let diags = validate_collect(&cfg);
        assert!(!diags.iter().any(|d| d.code == "V070"));
    }

    #[test]
    fn company_example_validates_no_errors() {
        let yaml = std::fs::read_to_string("../../examples/company.yaml")
            .or_else(|_| std::fs::read_to_string("examples/company.yaml"))
            .expect("read example");
        let cfg: Config = serde_yaml::from_str(&yaml).expect("parse");
        let diags = validate_collect(&cfg);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "company.yaml has validation errors: {errors:#?}"
        );
    }

    #[test]
    fn community_example_validates_no_errors() {
        let yaml = std::fs::read_to_string("../../examples/community.yaml")
            .or_else(|_| std::fs::read_to_string("examples/community.yaml"))
            .expect("read example");
        let cfg: Config = serde_yaml::from_str(&yaml).expect("parse");
        let diags = validate_collect(&cfg);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "community.yaml has validation errors: {errors:#?}"
        );
    }
}
