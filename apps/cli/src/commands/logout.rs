//! `guildforge logout` — delete the stored bot token.

use std::process::ExitCode;

/// Run the `logout` command.
///
/// Deletes the token file at `~/.config/guildforge/token` (or
/// `GUILDFORGE_TOKEN_FILE`). Exits 0 even if the file doesn't exist
/// (idempotent).
pub fn run() -> ExitCode {
    let dest = if let Ok(p) = std::env::var("GUILDFORGE_TOKEN_FILE") {
        std::path::PathBuf::from(p)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home)
            .join(".config")
            .join("guildforge")
            .join("token")
    };

    if !dest.exists() {
        eprintln!("No token file found (already logged out).");
        return ExitCode::SUCCESS;
    }

    match std::fs::remove_file(&dest) {
        Ok(()) => {
            eprintln!("Token removed from {}", dest.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("guildforge: could not remove {}: {e}", dest.display());
            ExitCode::from(2)
        }
    }
}
