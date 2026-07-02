//! `guildforge` — Infrastructure as Code for Discord Workspaces.
//!
//! Phase 3: `validate`, `plan`, `apply`, `destroy`, `doctor`, `version`,
//! and `--help` are functional. Other commands are stubs.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::uninlined_format_args)]

mod commands;

use clap::{Parser, Subcommand};
use guildforge_engine::Engine;
use guildforge_provider_discord::DiscordProvider;
use std::path::PathBuf;
use std::process::ExitCode;

/// Global flags accepted by every subcommand.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "guildforge",
    version,
    about = "Infrastructure as Code for Discord Workspaces",
    long_about = None,
    propagate_version = true,
)]
pub struct Args {
    /// Path to the `SQLite` state file.
    #[arg(
        long,
        env = "GUILDFORGE_STATE_FILE",
        global = true,
        default_value = "./guildforge.db"
    )]
    pub state_file: PathBuf,

    /// Provider name (`discord` only in v1).
    #[arg(
        long,
        env = "GUILDFORGE_PROVIDER",
        global = true,
        default_value = "discord"
    )]
    pub provider: String,

    /// Path to a file containing the bot token.
    #[arg(long, env = "GUILDFORGE_TOKEN_FILE", global = true)]
    pub token_file: Option<PathBuf>,

    /// Log level.
    #[arg(
        long,
        env = "GUILDFORGE_LOG_LEVEL",
        global = true,
        default_value = "info"
    )]
    pub log_level: String,

    /// Log format.
    #[arg(
        long,
        env = "GUILDFORGE_LOG_FORMAT",
        global = true,
        default_value = "pretty"
    )]
    pub log_format: String,

    /// Disable colored output.
    #[arg(
        long,
        env = "GUILDFORGE_NO_COLOR",
        global = true,
        default_value_t = false
    )]
    pub no_color: bool,

    /// Subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// All `guildforge` subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Scaffold a new `guildforge.yaml` in the current directory.
    Init,
    /// Parse and validate a config file.
    Validate {
        /// Path to the YAML config file.
        file: PathBuf,
    },
    /// Compute and print the execution plan for a config.
    Plan {
        /// Path to the YAML config file.
        file: PathBuf,
        /// Output format: `text` or `json`.
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Apply a config: plan, prompt, execute, commit state.
    Apply {
        /// Path to the YAML config file.
        file: PathBuf,
        /// Skip interactive prompt.
        #[arg(long)]
        auto_approve: bool,
    },
    /// Destroy every resource described in a config.
    Destroy {
        /// Path to the YAML config file.
        file: PathBuf,
        /// Skip interactive prompt.
        #[arg(long)]
        auto_approve: bool,
    },
    /// Structural diff between two config files.
    Diff {
        /// Path to the first YAML config file.
        a: PathBuf,
        /// Path to the second YAML config file.
        b: PathBuf,
    },
    /// Read an existing Discord guild and emit a YAML config.
    Import {
        /// Discord guild ID to import from.
        guild_id: String,
    },
    /// Export current state to a YAML config.
    Export,
    /// Detect drift: compare state to live Discord.
    Doctor,
    /// Snapshot state to an external file.
    Backup,
    /// Restore state from a backup file.
    Restore {
        /// Path to the backup file.
        backup: PathBuf,
    },
    /// Store the Discord bot token.
    Login,
    /// Delete the stored token.
    Logout,
    /// Print version, build info, and linked provider versions.
    Version,
}

/// Entry point. Returns a process exit code.
#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();

    // Initialize logging (idempotent).
    let _ = guildforge_logging::init_from_env();

    match &args.command {
        Command::Version => commands::version::run(),
        Command::Validate { file } => commands::validate::run(file),
        Command::Plan { file, format } => {
            let engine = match make_engine(&args).await {
                Ok(e) => e,
                Err(code) => return code,
            };
            commands::plan::run(&engine, file, format).await
        }
        Command::Apply { file, auto_approve } => {
            let engine = match make_engine(&args).await {
                Ok(e) => e,
                Err(code) => return code,
            };
            commands::apply::run(&engine, file, *auto_approve).await
        }
        Command::Destroy { file, auto_approve } => {
            let engine = match make_engine(&args).await {
                Ok(e) => e,
                Err(code) => return code,
            };
            commands::destroy::run(&engine, file, *auto_approve).await
        }
        Command::Doctor => {
            let engine = match make_engine(&args).await {
                Ok(e) => e,
                Err(code) => return code,
            };
            commands::doctor::run(&engine).await
        }
        Command::Init
        | Command::Diff { .. }
        | Command::Import { .. }
        | Command::Export
        | Command::Backup
        | Command::Restore { .. }
        | Command::Login
        | Command::Logout => {
            eprintln!(
                "guildforge: `{}` is not implemented yet (phase 3).",
                command_name(&args.command)
            );
            eprintln!("See ROADMAP.md for the implementation schedule.");
            ExitCode::from(2)
        }
    }
}

/// Construct the engine from CLI args + environment.
async fn make_engine(args: &Args) -> Result<Engine, ExitCode> {
    let Ok(provider) = DiscordProvider::from_env() else {
        eprintln!("guildforge: no bot token found");
        eprintln!("  set GUILDFORGE_BOT_TOKEN or run `guildforge login`");
        return Err(ExitCode::from(2));
    };
    Engine::open(provider, &args.state_file).await.map_err(|e| {
        eprintln!("guildforge: could not open state file: {e}");
        ExitCode::from(3)
    })
}

/// Human-readable name of a subcommand.
fn command_name(c: &Command) -> &'static str {
    match c {
        Command::Init => "init",
        Command::Validate { .. } => "validate",
        Command::Plan { .. } => "plan",
        Command::Apply { .. } => "apply",
        Command::Destroy { .. } => "destroy",
        Command::Diff { .. } => "diff",
        Command::Import { .. } => "import",
        Command::Export => "export",
        Command::Doctor => "doctor",
        Command::Backup => "backup",
        Command::Restore { .. } => "restore",
        Command::Login => "login",
        Command::Logout => "logout",
        Command::Version => "version",
    }
}
