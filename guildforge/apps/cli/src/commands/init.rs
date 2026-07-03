//! `guildforge init` — scaffold a new `guildforge.yaml` in the current
//! directory from a template.

use std::path::PathBuf;
use std::process::ExitCode;

/// Available built-in templates.
const MINIMAL_TEMPLATE: &str = include_str!("../../../../templates/minimal.yaml");

const COMPANY_TEMPLATE: &str = include_str!("../../../../examples/company.yaml");

const COMMUNITY_TEMPLATE: &str = include_str!("../../../../examples/community.yaml");

/// Run the `init` command.
///
/// Creates `guildforge.yaml` in the current directory (or `.`) from the
/// specified template. If the file already exists, exits with code 2
/// unless `--force` is passed.
pub fn run(template: &str, force: bool) -> ExitCode {
    let yaml = match template {
        "minimal" => MINIMAL_TEMPLATE,
        "company" => COMPANY_TEMPLATE,
        "community" => COMMUNITY_TEMPLATE,
        other => {
            eprintln!("guildforge: unknown template `{other}`");
            eprintln!("  available templates: minimal, company, community");
            return ExitCode::from(2);
        }
    };

    let dest: PathBuf = PathBuf::from("guildforge.yaml");

    if dest.exists() && !force {
        eprintln!(
            "guildforge: `{}` already exists (use --force to overwrite)",
            dest.display()
        );
        return ExitCode::from(2);
    }

    match std::fs::write(&dest, yaml) {
        Ok(()) => {
            println!("Created {} from template `{template}`", dest.display());
            println!("Next steps:");
            println!("  1. Edit guildforge.yaml to match your guild");
            println!("  2. Run `guildforge validate guildforge.yaml`");
            println!("  3. Run `guildforge plan guildforge.yaml`");
            println!("  4. Run `guildforge apply --auto-approve guildforge.yaml`");
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
    fn templates_are_nonempty() {
        assert!(MINIMAL_TEMPLATE.contains("server"));
        assert!(COMPANY_TEMPLATE.contains("server"));
        assert!(COMMUNITY_TEMPLATE.contains("server"));
    }

    #[test]
    fn minimal_template_has_server() {
        assert!(MINIMAL_TEMPLATE.contains("server:"));
        assert!(MINIMAL_TEMPLATE.contains("name:"));
    }

    #[test]
    fn company_template_has_roles() {
        assert!(COMPANY_TEMPLATE.contains("roles:"));
        assert!(COMPANY_TEMPLATE.contains("Admin"));
    }

    #[test]
    fn community_template_has_roles() {
        assert!(COMMUNITY_TEMPLATE.contains("roles:"));
        assert!(COMMUNITY_TEMPLATE.contains("Maintainer"));
    }
}
