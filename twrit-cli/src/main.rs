//! `twrit` -- enforces the rules a repository writes down in its
//! `.writ.tmt` files.
//!
//! A rule nobody checks is a wish. The point of this tool is that a writ
//! entry naming a guard can be held to it, so the rule fails loudly rather
//! than drifting quietly out of true.

mod layering;
mod writ;

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

    match run(&path) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("twrit: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(path: &std::path::Path) -> anyhow::Result<usize> {
    let writs = writ::discover(path)?;
    if writs.is_empty() {
        println!("no .writ.tmt found under {}", path.display());
        return Ok(0);
    }

    let violations = layering::check(&writs, path)?;

    for violation in &violations {
        println!("crate-layering: {violation}");
    }

    if violations.is_empty() {
        println!(
            "{} writ(s), crate-layering: ok",
            writs.len()
        );
    } else {
        println!("crate-layering: {} violation(s)", violations.len());
    }

    Ok(violations.len())
}
