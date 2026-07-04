//! Tracing initialization for `GuildForge`.
//!
//! Every binary initializes logging through this crate so that format
//! and filtering are consistent everywhere.
//!
//! # Environment variables
//!
//! - `GUILDFORGE_LOG_LEVEL` — `trace`|`debug`|`info`|`warn`|`error`. Default: `info`.
//! - `GUILDFORGE_LOG_FORMAT` — `pretty`|`json`|`compact`. Default: `pretty`.
//! - `GUILDFORGE_NO_COLOR` — if set, disables ANSI colors.
//! - `RUST_LOG` — standard tracing filter; takes precedence over
//!   `GUILDFORGE_LOG_LEVEL` for fine-grained module control.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use anyhow::Result;
use std::str::FromStr;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

/// Log level for the `GuildForge` subscriber.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Trace level — very verbose.
    Trace,
    /// Debug level.
    Debug,
    /// Info level — default.
    Info,
    /// Warn level.
    Warn,
    /// Error level.
    Error,
}

impl FromStr for LogLevel {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            other => anyhow::bail!("unknown log level: {other}"),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        };
        f.write_str(s)
    }
}

/// Output format for the `GuildForge` subscriber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    /// Human-readable pretty output (default).
    #[default]
    Pretty,
    /// Machine-readable JSON (for log aggregation).
    Json,
    /// Compact single-line format.
    Compact,
}

impl FromStr for LogFormat {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            "compact" => Ok(Self::Compact),
            other => anyhow::bail!("unknown log format: {other}"),
        }
    }
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pretty => "pretty",
            Self::Json => "json",
            Self::Compact => "compact",
        };
        f.write_str(s)
    }
}

/// Initialize the global tracing subscriber from explicit parameters.
///
/// Idempotent: a second call is a no-op (returns Ok).
///
/// # Errors
///
/// Returns an error only if subscriber installation fails for a reason
/// other than "already initialized".
pub fn init(level: LogLevel, format: LogFormat, no_color: bool) -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new(level.to_string()))?;

    let span_events = if matches!(level, LogLevel::Trace | LogLevel::Debug) {
        FmtSpan::NEW | FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    let result = match format {
        LogFormat::Pretty => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(!no_color)
            .with_span_events(span_events)
            .with_target(true)
            .try_init(),
        LogFormat::Json => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .json()
            .with_span_events(span_events)
            .with_target(true)
            .try_init(),
        LogFormat::Compact => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(!no_color)
            .compact()
            .with_span_events(span_events)
            .try_init(),
    };

    if let Err(e) = result {
        let msg = format!("{e}");
        // try_init fails when a subscriber is already set. Treat that as
        // success (idempotent init).
        if msg.contains("already been set") || msg.contains("already initialized") {
            return Ok(());
        }
        anyhow::bail!("failed to install tracing subscriber: {msg}");
    }
    Ok(())
}

/// Initialize the global tracing subscriber from environment variables.
///
/// Reads `GUILDFORGE_LOG_LEVEL`, `GUILDFORGE_LOG_FORMAT`, and
/// `GUILDFORGE_NO_COLOR`. Falls back to `info` / `pretty` / color
/// enabled if unset.
///
/// # Errors
///
/// Propagates errors from [`init`].
pub fn init_from_env() -> Result<()> {
    let level = std::env::var("GUILDFORGE_LOG_LEVEL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LogLevel::Info);
    let format = std::env::var("GUILDFORGE_LOG_FORMAT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();
    let no_color = std::env::var_os("GUILDFORGE_NO_COLOR").is_some();
    init(level, format, no_color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_parsing() {
        assert_eq!(LogLevel::from_str("trace").unwrap(), LogLevel::Trace);
        assert_eq!(LogLevel::from_str("INFO").unwrap(), LogLevel::Info);
        assert!(LogLevel::from_str("bogus").is_err());
    }

    #[test]
    fn log_level_display_round_trip() {
        for l in [
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ] {
            let s = l.to_string();
            assert_eq!(LogLevel::from_str(&s).unwrap(), l);
        }
    }

    #[test]
    fn log_format_default() {
        assert_eq!(LogFormat::default(), LogFormat::Pretty);
    }

    #[test]
    fn log_format_parsing() {
        assert_eq!(LogFormat::from_str("json").unwrap(), LogFormat::Json);
        assert_eq!(LogFormat::from_str("compact").unwrap(), LogFormat::Compact);
        assert_eq!(LogFormat::from_str("pretty").unwrap(), LogFormat::Pretty);
        assert!(LogFormat::from_str("bogus").is_err());
    }

    #[test]
    fn init_is_idempotent() {
        let _ = init(LogLevel::Info, LogFormat::Pretty, true);
        let r2 = init(LogLevel::Info, LogFormat::Pretty, true);
        assert!(r2.is_ok(), "second init should be a no-op: {r2:?}");
    }
}
