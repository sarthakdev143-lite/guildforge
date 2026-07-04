//! `guildforge import <guild-id>` — read live Discord, emit YAML.

use guildforge_engine::Engine;
use std::process::ExitCode;

/// Run the `import` command.
pub async fn run(engine: &Engine, guild_id: &str) -> ExitCode {
    match engine.import(guild_id).await {
        Ok(yaml) => {
            print!("{yaml}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("guildforge: import error: {e}");
            ExitCode::from(4)
        }
    }
}
