//! `quire` CLI.
//!
//! Usage:
//!     quire check --policy <file> -- <cmd> [args...]
//!
//! Exit codes:
//!     0   allow       the harness may run the command unattended
//!     10  prompt      the harness should ask the human
//!     20  forbidden   the harness must reject the command
//!     30  no-match    no rule matched; fall back to harness default prompt
//!     2   usage / load error (policy file missing, syntax error, etc.)

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use quire::Policy;

#[derive(Debug, Parser)]
#[command(
    name = "quire",
    version,
    about = "Starlark-policy CLI for AI coding agents."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check a shell command against a policy file.
    Check {
        /// Path to the .policy file.
        #[arg(long)]
        policy: PathBuf,

        /// Output JSON instead of plain text.
        #[arg(long)]
        json: bool,

        /// The command and its arguments. Everything after `--` is argv.
        #[arg(last = true, required = true)]
        argv: Vec<String>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { policy, json, argv } => match run_check(&policy, &argv, json) {
            Ok(code) => ExitCode::from(code as u8),
            Err(err) => {
                eprintln!("quire: {err:#}");
                ExitCode::from(2)
            }
        },
    }
}

fn run_check(policy_path: &std::path::Path, argv: &[String], json: bool) -> anyhow::Result<i32> {
    let policy = Policy::from_file(policy_path)?;
    let decision = policy.evaluate(argv);
    if json {
        let out = serde_json::json!({
            "decision": decision.as_str(),
            "exit_code": decision.exit_code(),
            "argv": argv,
        });
        println!("{out}");
    } else {
        println!("{}", decision.as_str());
    }
    Ok(decision.exit_code())
}
