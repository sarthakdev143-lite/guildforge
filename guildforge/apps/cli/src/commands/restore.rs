//! `guildforge restore <backup>` — restore state from a backup file.

use guildforge_engine::Engine;
use std::path::Path;
use std::process::ExitCode;

/// Run the `restore` command.
pub fn run(engine: &Engine, backup: &Path) -> ExitCode {
    match engine.restore(backup) {
        Ok(()) => {
            println!("State restored from {}", backup.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("guildforge: restore error: {e}");
            ExitCode::from(2)
        }
    }
}
