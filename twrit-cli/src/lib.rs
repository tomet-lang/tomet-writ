//! `twrit` -- enforces the rules a repository writes down in its
//! `.writ.tmt` files.
//!
//! A rule nobody checks is a wish. The point of this tool is that a writ
//! entry naming a guard can be held to it, so the rule fails loudly rather
//! than drifting quietly out of true.
//!
//! The binary is argument handling and an exit code; everything a test
//! would want to reach lives here. Rendering in particular: a test that
//! asserts on `Report::render` is asserting on the text the CLI prints,
//! without spawning it.

pub mod layering;
pub mod purity;
pub mod writ;

use std::path::{Path, PathBuf};

use anyhow::Result;

/// What became of one rule kind on this run.
///
/// The distinction is the whole of it. A guard that found no declaration
/// and a guard that checked everything and found nothing wrong both used
/// to return an empty list, and an empty list renders as success -- which
/// is how a rule that quietly stopped being enforced came to look exactly
/// like a rule being kept.
pub enum Outcome {
    /// No writ declares this rule's data, so it did not run.
    NotDeclared,
    /// It ran. `scope` says over how much, because "checked and clean"
    /// and "checked nothing and therefore clean" are also two answers.
    Checked { scope: String, violations: Vec<String> },
}

impl Outcome {
    pub fn checked(scope: String, violations: Vec<String>) -> Self {
        Outcome::Checked { scope, violations }
    }

    pub fn violations(&self) -> &[String] {
        match self {
            Outcome::NotDeclared => &[],
            Outcome::Checked { violations, .. } => violations,
        }
    }
}

/// One rule kind's result: the name it is reported under, and what became
/// of it.
pub struct RuleResult {
    pub name: &'static str,
    pub outcome: Outcome,
}

/// Everything one `check` run has to say.
pub struct Report {
    root: PathBuf,
    writ_count: usize,
    rules: Vec<RuleResult>,
}

impl Report {
    /// The total number of violations across every rule.
    pub fn violation_count(&self) -> usize {
        self.rules.iter().map(|r| r.outcome.violations().len()).sum()
    }

    pub fn rules(&self) -> &[RuleResult] {
        &self.rules
    }

    pub fn writ_count(&self) -> usize {
        self.writ_count
    }

    /// The report as the CLI prints it, trailing newline included.
    ///
    /// The summary names what each rule *did*, not what this tool is able
    /// to do. The two read the same on a clean repository and diverge
    /// exactly where it matters.
    pub fn render(&self) -> String {
        if self.writ_count == 0 {
            return format!("no .writ.tmt found under {}\n", self.root.display());
        }

        let mut out = String::new();
        for rule in &self.rules {
            for violation in rule.outcome.violations() {
                out.push_str(&format!("{}: {violation}\n", rule.name));
            }
        }

        let summary: Vec<String> = self
            .rules
            .iter()
            .map(|rule| match &rule.outcome {
                Outcome::NotDeclared => format!("{}: not declared", rule.name),
                Outcome::Checked { scope, violations } if violations.is_empty() => {
                    format!("{}: ok ({scope})", rule.name)
                }
                Outcome::Checked { scope, violations } => format!(
                    "{}: {} ({scope})",
                    rule.name,
                    plural(violations.len(), "violation")
                ),
            })
            .collect();

        out.push_str(&format!(
            "{}, {}\n",
            plural(self.writ_count, "writ"),
            summary.join(", ")
        ));
        out
    }
}

/// `1 member`, `2 members`. Written out rather than `member(s)` because
/// these end up in a line a person reads on every run.
pub fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// Reads every writ at or below `root` and runs each rule kind against it.
pub fn check(root: &Path) -> Result<Report> {
    let writs = writ::discover(root)?;
    if writs.is_empty() {
        return Ok(Report {
            root: root.to_path_buf(),
            writ_count: 0,
            rules: Vec::new(),
        });
    }

    let rules = vec![
        RuleResult {
            name: "crate-layering",
            outcome: layering::check(&writs, root)?,
        },
        RuleResult {
            name: "parser-purity",
            outcome: purity::check(&writs, root)?,
        },
    ];

    Ok(Report {
        root: root.to_path_buf(),
        writ_count: writs.len(),
        rules,
    })
}
