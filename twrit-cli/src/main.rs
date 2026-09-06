//! The `twrit` command. Argument handling and an exit code; the work is
//! in the library beside this file.

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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Command::Check { path } = cli.command;

    match twrit_cli::check(&path) {
        Ok(report) => {
            print!("{}", report.render());
            if report.violation_count() == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("twrit: {e:#}");
            ExitCode::FAILURE
        }
    }
}
