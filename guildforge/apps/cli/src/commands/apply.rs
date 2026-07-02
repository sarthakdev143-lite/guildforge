//! `guildforge apply <file>` — apply a config.

use guildforge_engine::Engine;
use std::path::Path;
use std::process::ExitCode;

/// Run the `apply` command.
pub async fn run(engine: &Engine, file: &Path, auto_approve: bool) -> ExitCode {
    match engine.apply(file, auto_approve).await {
        Ok(report) => {
            if report.has_failures() {
                eprintln!("\nguildforge: apply completed with failures:");
                eprintln!("  {report}");
                for op in &report.operations {
                    if let guildforge_executor::OperationResult::Failure { addr, error, .. } = op {
                        eprintln!("    FAILED: {addr} — {error}");
                    }
                }
                ExitCode::from(1)
            } else {
                println!("Apply complete: {report}");
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("guildforge: apply error: {e}");
            match e {
                guildforge_engine::EngineError::Aborted => ExitCode::from(5),
                guildforge_engine::EngineError::LockHeld(_) => ExitCode::from(6),
                _ => ExitCode::from(4),
            }
        }
    }
}
