//! `guildforge diff <a> <b>` — structural diff between two configs.

use guildforge_engine::diff_configs;
use guildforge_parser::parse_file;
use std::path::Path;
use std::process::ExitCode;

/// Run the `diff` command.
pub fn run(a: &Path, b: &Path) -> ExitCode {
    let config_a = match parse_file(a) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("guildforge: could not parse {}: {e}", a.display());
            return ExitCode::from(3);
        }
    };
    let config_b = match parse_file(b) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("guildforge: could not parse {}: {e}", b.display());
            return ExitCode::from(3);
        }
    };
    let report = diff_configs(&config_a, &config_b);
    print!("{report}");
    if report.entries.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
