//! `guildforge plan <file>` — compute and print the execution plan.

use guildforge_engine::Engine;
use guildforge_planner::PlanFormat;
use std::path::Path;
use std::process::ExitCode;

/// Run the `plan` command.
pub async fn run(engine: &Engine, file: &Path, format: &str) -> ExitCode {
    let fmt = match format {
        "json" => PlanFormat::Json,
        _ => PlanFormat::Text,
    };
    match engine.plan(file).await {
        Ok(plan) => {
            let output = Engine::render_plan(&plan, fmt);
            print!("{output}");
            if plan.has_changes() {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("guildforge: plan error: {e}");
            ExitCode::from(4)
        }
    }
}
