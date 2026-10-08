//! The `twrit` command. Argument handling and an exit code; the work is
//! in the libraries (`twrit`, `twrit-rust`).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "twrit", about = "Check a repository against its own writs")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check every rule this tool knows how to enforce.
    Check {
        /// Repository root. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// List every rule the writs declare, and who holds it.
    List {
        /// Repository root. Defaults to the current directory.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("twrit: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn all_guards() -> Vec<twrit::Guard> {
    let mut guards = twrit_rust::rust_guards();
    guards.push(twrit::placement_guard());
    guards
}


fn run(command: Command) -> anyhow::Result<ExitCode> {
    let guards = all_guards();
    Ok(match command {
        Command::Check { path } => {
            let report = twrit::check(&path, &guards)?;
            print!("{}", report.render());
            if report.violation_count() == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        // A listing reports; it does not judge. A dead guard is a finding
        // and `check` is where a finding fails a run, so this exits zero
        // whatever it prints.
        Command::List { path } => {
            print!("{}", twrit::list(&path, &guards)?.render());
            ExitCode::SUCCESS
        }
    })
}
