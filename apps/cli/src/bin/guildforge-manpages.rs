//! Man page generation for guildforge.
//!
//! Run with: `cargo run --bin guildforge-manpages`
//!
//! Output: `assets/man/guildforge.1`

use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(clap::Parser)]
#[command(name = "guildforge-manpages")]
struct ManpageArgs {
    /// Output directory for the man page.
    #[arg(long, default_value = "assets/man")]
    output: PathBuf,
}

fn main() {
    let args = ManpageArgs::parse();

    let cmd = clap::Command::new("guildforge")
        .about("Infrastructure as Code for Discord Workspaces")
        .version("1.0.0")
        .arg(
            clap::Arg::new("state-file")
                .long("state-file")
                .env("GUILDFORGE_STATE_FILE")
                .default_value("./guildforge.db")
                .help("Path to the SQLite state file")
                .global(true),
        )
        .arg(
            clap::Arg::new("log-level")
                .long("log-level")
                .env("GUILDFORGE_LOG_LEVEL")
                .default_value("info")
                .help("Log level")
                .global(true),
        )
        .subcommand(
            clap::Command::new("init")
                .about("Scaffold a new guildforge.yaml")
                .arg(
                    clap::Arg::new("template")
                        .long("template")
                        .default_value("minimal"),
                )
                .arg(
                    clap::Arg::new("force")
                        .long("force")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            clap::Command::new("validate")
                .about("Parse and validate a config")
                .arg(clap::Arg::new("file").required(true)),
        )
        .subcommand(
            clap::Command::new("plan")
                .about("Compute execution plan")
                .arg(clap::Arg::new("file").required(true))
                .arg(
                    clap::Arg::new("format")
                        .long("format")
                        .default_value("text"),
                ),
        )
        .subcommand(
            clap::Command::new("apply")
                .about("Apply a config")
                .arg(clap::Arg::new("file").required(true))
                .arg(
                    clap::Arg::new("auto-approve")
                        .long("auto-approve")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            clap::Command::new("destroy")
                .about("Destroy resources")
                .arg(clap::Arg::new("file").required(true))
                .arg(
                    clap::Arg::new("auto-approve")
                        .long("auto-approve")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            clap::Command::new("diff")
                .about("Diff two configs")
                .arg(clap::Arg::new("a").required(true))
                .arg(clap::Arg::new("b").required(true)),
        )
        .subcommand(
            clap::Command::new("import")
                .about("Import live guild to YAML")
                .arg(clap::Arg::new("guild-id").required(true)),
        )
        .subcommand(clap::Command::new("export").about("Export state to YAML"))
        .subcommand(clap::Command::new("doctor").about("Detect drift"))
        .subcommand(clap::Command::new("backup").about("Backup state"))
        .subcommand(
            clap::Command::new("restore")
                .about("Restore state")
                .arg(clap::Arg::new("backup").required(true)),
        )
        .subcommand(clap::Command::new("login").about("Store bot token"))
        .subcommand(clap::Command::new("logout").about("Delete token"))
        .subcommand(clap::Command::new("version").about("Print version"))
        .subcommand(
            clap::Command::new("completions")
                .about("Generate shell completions")
                .arg(clap::Arg::new("shell").required(true).value_parser([
                    "bash",
                    "zsh",
                    "fish",
                    "powershell",
                    "elvish",
                ])),
        );

    let man = clap_mangen::Man::new(cmd);
    let mut buffer = Vec::new();
    man.render(&mut buffer).expect("render man page");

    fs::create_dir_all(&args.output).expect("create output dir");
    let man_path = args.output.join("guildforge.1");
    fs::write(&man_path, buffer).expect("write man page");

    eprintln!("Man page written to {}", man_path.display());
}
