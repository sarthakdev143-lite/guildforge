//! `guildforge login` — store the Discord bot token.
//!
//! Phase 6: writes to `~/.config/guildforge/token` with mode 0600.
//! Phase 7+: will use OS keychain via the `keyring` crate.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

/// Path to the token file.
fn token_file_path() -> PathBuf {
    if let Ok(p) = std::env::var("GUILDFORGE_TOKEN_FILE") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("guildforge")
        .join("token")
}

/// Run the `login` command.
///
/// Reads the token from stdin (hidden) or `--token-file`. Writes it to
/// `~/.config/guildforge/token` with mode 0600.
pub fn run(token_file: Option<&std::path::Path>) -> ExitCode {
    // Determine the token source.
    let token = if let Some(path) = token_file {
        match std::fs::read_to_string(path) {
            Ok(t) => t.trim().to_string(),
            Err(e) => {
                eprintln!(
                    "guildforge: could not read token file {}: {e}",
                    path.display()
                );
                return ExitCode::from(2);
            }
        }
    } else if !atty::is(atty::Stream::Stdin) {
        // Stdin is piped — read from it.
        let mut buf = String::new();
        if io::stdin().read_to_string(&mut buf).is_err() {
            eprintln!("guildforge: could not read token from stdin");
            return ExitCode::from(2);
        }
        buf.trim().to_string()
    } else {
        // Interactive prompt with hidden input.
        eprint!("Enter Discord bot token: ");
        let _ = io::stderr().flush();
        let token = rpassword::read_password().unwrap_or_default();
        token.trim().to_string()
    };

    if token.is_empty() {
        eprintln!("guildforge: token is empty");
        return ExitCode::from(2);
    }

    // Validate token format (basic check — Discord tokens are 50+ chars).
    if token.len() < 50 {
        eprintln!(
            "guildforge: token looks too short (expected 50+ chars, got {})",
            token.len()
        );
        eprintln!("  Discord bot tokens look like: MTIzNDU2Nzg5MDEyMzQ1Njc4.Gabc12.def456...");
        return ExitCode::from(2);
    }

    let dest = token_file_path();
    if let Some(parent) = dest.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("guildforge: could not create {}: {e}", parent.display());
            return ExitCode::from(2);
        }
    }

    match std::fs::write(&dest, &token) {
        Ok(()) => {
            // Set file permissions to 0600 on Unix.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&dest) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o600);
                    let _ = std::fs::set_permissions(&dest, perms);
                }
            }
            eprintln!("Token saved to {}", dest.display());
            eprintln!("Run `guildforge validate examples/company.yaml` to test.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("guildforge: could not write {}: {e}", dest.display());
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_file_path_respects_env() {
        std::env::set_var("GUILDFORGE_TOKEN_FILE", "/tmp/custom-token");
        assert_eq!(token_file_path(), PathBuf::from("/tmp/custom-token"));
        std::env::remove_var("GUILDFORGE_TOKEN_FILE");
    }

    #[test]
    fn token_file_path_defaults_to_home() {
        // Save and restore the env var to avoid interfering with other tests.
        let saved = std::env::var("GUILDFORGE_TOKEN_FILE").ok();
        std::env::remove_var("GUILDFORGE_TOKEN_FILE");
        let p = token_file_path();
        assert!(p.ends_with("token"));
        assert!(p.to_string_lossy().contains("guildforge"));
        if let Some(v) = saved {
            std::env::set_var("GUILDFORGE_TOKEN_FILE", v);
        }
    }
}
