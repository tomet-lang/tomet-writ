//! `twrit` -- enforces the rules a repository writes down in its
//! `.writ.tmt` files.
//!
//! A rule nobody checks is a wish. The point of this tool is that a writ
//! entry naming a guard can be held to it, so the rule fails loudly rather
//! than drifting quietly out of true.

mod layering;
mod purity;
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

    let rules: [(&str, Vec<String>); 2] = [
        ("crate-layering", layering::check(&writs, path)?),
        ("parser-purity", purity::check(&writs, path)?),
    ];

    let mut total = 0;
    for (name, violations) in &rules {
        for violation in violations {
            println!("{name}: {violation}");
        }
        total += violations.len();
    }

    if total == 0 {
        let names: Vec<&str> = rules.iter().map(|(name, _)| *name).collect();
        println!("{} writ(s), {}: ok", writs.len(), names.join(", "));
    } else {
        println!("{total} violation(s)");
    }

    Ok(total)
}
