//! `guildforge version` — print version and build info.

use std::process::ExitCode;

/// Run the `version` command.
pub fn run() -> ExitCode {
    println!("guildforge {}", env!("CARGO_PKG_VERSION"));
    println!("  commit:    {}", env!("CARGO_PKG_VERSION"));
    println!("  providers: discord={}", env!("CARGO_PKG_VERSION"));
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_exits_success() {
        // We can't easily capture stdout from a process::ExitCode in
        // unit tests, but we can verify the function returns SUCCESS.
        let code = run();
        assert_eq!(code, ExitCode::SUCCESS);
    }
}
