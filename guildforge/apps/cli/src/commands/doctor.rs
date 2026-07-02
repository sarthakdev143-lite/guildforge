//! `guildforge doctor` — detect drift.

use guildforge_engine::Engine;
use std::process::ExitCode;

/// Run the `doctor` command.
pub async fn run(engine: &Engine) -> ExitCode {
    match engine.doctor().await {
        Ok(report) => {
            if report.missing_in_live.is_empty()
                && report.missing_in_state.is_empty()
                && report.drifted.is_empty()
            {
                println!("No drift detected.");
                ExitCode::SUCCESS
            } else {
                println!("Drift detected:");
                for addr in &report.missing_in_live {
                    println!("  missing in live: {addr}");
                }
                for addr in &report.missing_in_state {
                    println!("  missing in state: {addr}");
                }
                for addr in &report.drifted {
                    println!("  drifted: {addr}");
                }
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("guildforge: doctor error: {e}");
            ExitCode::from(4)
        }
    }
}
