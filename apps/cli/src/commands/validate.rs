//! `guildforge validate <file>` — parse and validate a config file.

use guildforge_parser::parse_file;
use guildforge_validation::{validate_collect, Severity};
use std::path::Path;
use std::process::ExitCode;

/// Run the `validate` command.
///
/// Exit codes per [`docs/CLI_REFERENCE.md`](../../../docs/CLI_REFERENCE.md):
/// - 0: valid
/// - 1: invalid (validation errors)
/// - 2: file not found / I/O error
/// - 3: parse error (invalid YAML)
pub fn run(file: &Path) -> ExitCode {
    // Step 1: parse (syntax + schema).
    let config = match parse_file(file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("guildforge: parse error: {e}");
            return match e {
                guildforge_parser::ParseError::Io(_) => ExitCode::from(2),
                _ => ExitCode::from(3),
            };
        }
    };

    // Step 2: semantic validation.
    let diags = validate_collect(&config);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    let warnings: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();

    if !errors.is_empty() {
        for d in &errors {
            let addr = d.addr.as_deref().unwrap_or("(config)");
            eprintln!("  {} [{}] {} — {}", d.severity, d.code, addr, d.message);
            if let Some(help) = &d.help {
                eprintln!("    help: {help}");
            }
        }
        if !warnings.is_empty() {
            for w in &warnings {
                let addr = w.addr.as_deref().unwrap_or("(config)");
                eprintln!("  {} [{}] {} – {}", w.severity, w.code, addr, w.message);
            }
        }
        eprintln!("\n{} error(s), {} warning(s)", errors.len(), warnings.len());
        return ExitCode::from(1);
    }

    // No errors. Print warnings (if any) then OK.
    if !warnings.is_empty() {
        for w in &warnings {
            let addr = w.addr.as_deref().unwrap_or("(config)");
            eprintln!("  {} [{}] {} – {}", w.severity, w.code, addr, w.message);
        }
        eprintln!("\n0 error(s), {} warning(s)", warnings.len());
    }

    println!("OK: {} is valid", file.display());
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(name)
    }

    #[test]
    fn run_validates_company_yaml_success() {
        let path = fixture("company.yaml");
        assert_eq!(run(&path), ExitCode::SUCCESS);
    }

    #[test]
    fn run_validates_community_yaml_success() {
        let path = fixture("community.yaml");
        assert_eq!(run(&path), ExitCode::SUCCESS);
    }

    #[test]
    fn run_nonexistent_file_returns_2() {
        let path = Path::new("/nonexistent/path/to/file.yaml");
        assert_eq!(run(path), ExitCode::from(2));
    }

    #[test]
    fn run_invalid_yaml_returns_3() {
        // Write a broken YAML to a temp file.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.yaml");
        std::fs::write(&path, "server:\n  name: Test\n  bogus_field: true\n").unwrap();
        assert_eq!(run(&path), ExitCode::from(3));
    }

    #[test]
    fn run_semantically_invalid_returns_1() {
        // Duplicate role name — passes parse, fails V001.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dup.yaml");
        std::fs::write(
            &path,
            "server:\n  name: Test\nroles:\n  - name: Admin\n  - name: admin\n",
        )
        .unwrap();
        assert_eq!(run(&path), ExitCode::from(1));
    }
}
