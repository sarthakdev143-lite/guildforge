//! Tracing initialization for `GuildForge`.
//!
//! Every binary (the CLI, integration tests, the future dashboard
//! backend) initializes logging through this crate so that log format
//! and filtering are consistent everywhere.
//!
//! # Environment variables
//!
//! - `GUILDFORGE_LOG_LEVEL` — one of `trace`, `debug`, `info`, `warn`,
//!   `error`. Default: `info`.
//! - `GUILDFORGE_LOG_FORMAT` — one of `pretty`, `json`, `compact`.
//!   Default: `pretty`.
//! - `GUILDFORGE_NO_COLOR` — if set, disables ANSI colors.
//!
//! Phase 0: this crate is a stub. Real implementation lands in Phase 1
//! (task `P1-002`).

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]

use anyhow::Result;
use std::str::FromStr;

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

/// Initialize the global tracing subscriber from explicit parameters.
///
/// # Errors
///
/// Returns an error if the subscriber has already been initialized or
/// if the global default dispatcher cannot be set.
pub fn init(_level: LogLevel, _format: LogFormat, _no_color: bool) -> Result<()> {
    // Phase 0 stub: real implementation lands in task P1-002.
    // Will use tracing_subscriber::fmt() with the appropriate layer.
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
    fn log_format_default() {
        assert_eq!(LogFormat::default(), LogFormat::Pretty);
    }
}
