//! `guildforge backup` — snapshot state to a file.

use guildforge_engine::Engine;
use std::process::ExitCode;

/// Run the `backup` command.
pub fn run(engine: &Engine, dest: &std::path::Path) -> ExitCode {
    match engine.backup(dest) {
        Ok(()) => {
            println!("Backup written to {}", dest.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("guildforge: backup error: {e}");
            ExitCode::from(2)
        }
    }
}
