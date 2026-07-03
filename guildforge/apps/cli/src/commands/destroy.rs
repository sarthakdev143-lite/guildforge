//! `guildforge destroy <file>` — destroy all resources.

use guildforge_engine::Engine;
use std::path::Path;
use std::process::ExitCode;

/// Run the `destroy` command.
pub async fn run(engine: &Engine, file: &Path, auto_approve: bool) -> ExitCode {
    match engine.destroy(file, auto_approve).await {
        Ok(report) => {
            if report.has_failures() {
                eprintln!("\nguildforge: destroy completed with failures:");
                eprintln!("  {report}");
                ExitCode::from(1)
            } else {
                println!("Destroy complete: {report}");
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("guildforge: destroy error: {e}");
            match e {
                guildforge_engine::EngineError::Aborted => ExitCode::from(5),
                guildforge_engine::EngineError::LockHeld(_) => ExitCode::from(6),
                _ => ExitCode::from(4),
            }
        }
    }
}
