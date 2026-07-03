//! `guildforge export` — state to YAML.

use guildforge_engine::Engine;
use std::process::ExitCode;

/// Run the `export` command.
pub async fn run(engine: &Engine) -> ExitCode {
    match engine.export("exported").await {
        Ok(yaml) => {
            print!("{yaml}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("guildforge: export error: {e}");
            ExitCode::from(4)
        }
    }
}
