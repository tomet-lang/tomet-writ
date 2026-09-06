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
    /// Every rule the writs declare, held or not.
    ///
    /// Counted separately from `rules` below, which is the kinds this
    /// tool implements. The two answer different questions: how much this
    /// repository has written down, and how much of it ran here.
    rule_count: usize,
    /// Rules whose guard is `none:`. Written down as held by nobody,
    /// which is a decision and reads as one -- unlike a rule with no
    /// guard at all, which is refused.
    unguarded_count: usize,
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

    pub fn rule_count(&self) -> usize {
        self.rule_count
    }

    pub fn unguarded_count(&self) -> usize {
        self.unguarded_count
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

        // The census sits next to the writ count and before what ran,
        // because it is the number this tool exists to produce: how many
        // rules a repository has written down, and how many of them
        // nothing is holding.
        let census = if self.unguarded_count == 0 {
            plural(self.rule_count, "rule")
        } else {
            format!(
                "{} ({} unguarded)",
                plural(self.rule_count, "rule"),
                self.unguarded_count
            )
        };

        out.push_str(&format!(
            "{}, {census}, {}\n",
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

/// The rule kinds this tool implements, and the only values `guard:
/// { twrit: ... }` may take.
///
/// Named here rather than only in the `vec!` below so a writ can be
/// checked against it before anything runs.
pub const KINDS: &[&str] = &["layers", "pure"];

/// Reads every writ at or below `root` and runs each rule kind against it.
pub fn check(root: &Path) -> Result<Report> {
    let writs = writ::discover(root)?;
    if writs.is_empty() {
        return Ok(Report {
            root: root.to_path_buf(),
            writ_count: 0,
            rule_count: 0,
            unguarded_count: 0,
            rules: Vec::new(),
        });
    }

    let declared: Vec<writ::Rule> = writs
        .iter()
        .map(|w| w.rules())
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    // A rule naming a kind this tool does not implement is refused rather
    // than skipped. Skipping it would mean the writ says twrit holds the
    // rule and nothing does, which is the failure this whole tool is
    // about, spelled one level up.
    for rule in &declared {
        if let writ::Guard::Twrit(kind) = &rule.guard
            && !KINDS.contains(&kind.as_str())
        {
                anyhow::bail!(
                    "{}: `@rule({})` names `twrit: {kind}`, which this tool does not implement; \
                     it implements {}",
                    rule.path.display(),
                    rule.id,
                    KINDS.join(", ")
                );
        }
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
        rule_count: declared.len(),
        unguarded_count: declared
            .iter()
            .filter(|r| matches!(r.guard, writ::Guard::None(_)))
            .count(),
        rules,
    })
}
