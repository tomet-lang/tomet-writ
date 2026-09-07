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

pub mod door;
pub mod layering;
pub mod purity;
pub mod writ;

use std::path::{Path, PathBuf};

use anyhow::Result;

/// What one rule did on this run.
///
/// `scope` is carried because "checked and clean" and "checked nothing
/// and therefore clean" are two answers -- a `layers` over a workspace
/// with no members would otherwise read as a rule being kept.
///
/// There is no "not declared" any more. Under `@rule(id)` a declared
/// rule always has an id and always runs; a malformed one is a parse
/// error or a `guard` error, not a silence. What used to need that
/// variant is now the census: a repository with no `layers` rule has
/// none to report, and the rule count says how many it does have.
pub struct Outcome {
    pub scope: String,
    pub violations: Vec<String>,
}

impl Outcome {
    pub fn checked(scope: String, violations: Vec<String>) -> Self {
        Outcome { scope, violations }
    }

    pub fn violations(&self) -> &[String] {
        &self.violations
    }
}

/// One rule's result, reported under the id its writ gave it.
///
/// The id and not the kind, because a kind can hold several rules: a
/// repository has one `layers`, but it may have a `door` for file I/O and
/// another for the way documents are read. "door: 2 violations" would
/// name neither.
pub struct RuleResult {
    pub id: String,
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
    /// Findings that belong to one rule rather than to a rule kind: a
    /// writ whose own account of itself is wrong.
    dead_guards: Vec<String>,
    rules: Vec<RuleResult>,
}

impl Report {
    /// The total number of findings: rule violations, plus guards that
    /// point at nothing.
    ///
    /// A dead guard counts, and so fails the run. It is not a violation
    /// of the rule it sits on -- the code may well still obey -- but the
    /// writ says a test holds the rule and no test does, and a false
    /// statement in a writ is exactly what this tool is for.
    pub fn violation_count(&self) -> usize {
        self.rules.iter().map(|r| r.outcome.violations().len()).sum::<usize>()
            + self.dead_guards.len()
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
        for finding in &self.dead_guards {
            out.push_str(&format!("{finding}\n"));
        }
        for rule in &self.rules {
            for violation in rule.outcome.violations() {
                out.push_str(&format!("{}: {violation}\n", rule.id));
            }
        }

        let summary: Vec<String> = self
            .rules
            .iter()
            .map(|rule| {
                let Outcome { scope, violations } = &rule.outcome;
                if violations.is_empty() {
                    format!("{}: ok ({scope})", rule.id)
                } else {
                    format!(
                        "{}: {} ({scope})",
                        rule.id,
                        plural(violations.len(), "violation")
                    )
                }
            })
            .collect();

        // The census sits next to the writ count and before what ran,
        // because it is the number this tool exists to produce: how many
        // rules a repository has written down, and how many of them
        // nothing is holding.
        let mut census = plural(self.rule_count, "rule");
        if self.unguarded_count > 0 {
            census.push_str(&format!(" ({} unguarded)", self.unguarded_count));
        }
        if !self.dead_guards.is_empty() {
            census.push_str(&format!(", {}", plural(self.dead_guards.len(), "dead guard")));
        }

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
pub const KINDS: &[&str] = &["layers", "pure", "door"];

/// Kinds a repository may declare only once.
///
/// A layer order is one thing: two of them is not a stricter rule, it is
/// two answers to "which layer is this crate in" and no way to pick. The
/// others are plural by design -- one `door` per capability, one `pure`
/// per crate that claims it.
const SINGLETON_KINDS: &[&str] = &["layers"];

/// One rule as `twrit list` shows it: who holds it, and whether that
/// holder is still there.
pub struct Listed {
    pub id: String,
    /// The holder's spelling -- `twrit`, `runs`, `test`, `none`.
    pub holder: &'static str,
    /// What it points at, or the reason when nobody holds it.
    pub target: String,
    /// A pointer that no longer resolves. Only `test:` can be followed
    /// from here; `runs:` names a runner this tool does not execute, and
    /// verifying it would mean knowing every runner a repository ships.
    pub dead: bool,
}

/// Every rule a repository declares, and who holds it.
pub struct Listing {
    root: PathBuf,
    writ_count: usize,
    rules: Vec<Listed>,
}

impl Listing {
    pub fn rules(&self) -> &[Listed] {
        &self.rules
    }

    /// Rules whose guard points at something that is not there.
    pub fn dead_count(&self) -> usize {
        self.rules.iter().filter(|r| r.dead).count()
    }

    pub fn render(&self) -> String {
        if self.writ_count == 0 {
            return format!("no .writ.tmt found under {}\n", self.root.display());
        }

        // Two columns of padding, computed from the rows rather than
        // fixed: an id is as long as the author made it.
        let id_width = self.rules.iter().map(|r| r.id.len()).max().unwrap_or(0);
        let holder_width = self.rules.iter().map(|r| r.holder.len()).max().unwrap_or(0);

        let mut out = String::new();
        for rule in &self.rules {
            let dead = if rule.dead { " -- MISSING" } else { "" };
            out.push_str(&format!(
                "{:id_width$}  {:holder_width$}  {}{dead}\n",
                rule.id, rule.holder, rule.target
            ));
        }

        let unguarded = self
            .rules
            .iter()
            .filter(|r| r.holder == "none")
            .count();
        let mut summary = format!(
            "\n{}, {}",
            plural(self.writ_count, "writ"),
            plural(self.rules.len(), "rule")
        );
        if unguarded > 0 {
            summary.push_str(&format!(" ({unguarded} unguarded)"));
        }
        let dead = self.dead_count();
        if dead > 0 {
            summary.push_str(&format!(", {}", plural(dead, "dead guard")));
        }
        summary.push('\n');
        out.push_str(&summary);
        out
    }
}

/// Lists every rule the writs under `root` declare.
///
/// `check` asks whether the code obeys its rules. This asks whether
/// anything is holding them, which is the question that has no other
/// answer -- a rule guarded by a test nobody kept is still written down,
/// still named, and held by nothing.
pub fn list(root: &Path) -> Result<Listing> {
    let writs = writ::discover(root)?;
    if writs.is_empty() {
        return Ok(Listing {
            root: root.to_path_buf(),
            writ_count: 0,
            rules: Vec::new(),
        });
    }

    let mut rules = Vec::new();
    for w in &writs {
        for rule in w.rules()? {
            let (holder, target, dead) = match &rule.guard {
                writ::Guard::Twrit(kind) => ("twrit", kind.clone(), !KINDS.contains(&kind.as_str())),
                writ::Guard::Runs(cmd) => ("runs", cmd.clone(), false),
                writ::Guard::Test(path) => ("test", path.clone(), false),
                writ::Guard::None(why) => ("none", why.clone(), false),
            };
            rules.push(Listed {
                dead: dead || rule.guard.dead_pointer(root).is_some(),
                id: rule.id,
                holder,
                target,
            });
        }
    }

    Ok(Listing {
        root: root.to_path_buf(),
        writ_count: writs.len(),
        rules,
    })
}

/// Reads every writ at or below `root` and runs each rule kind against it.
pub fn check(root: &Path) -> Result<Report> {
    let writs = writ::discover(root)?;
    if writs.is_empty() {
        return Ok(Report {
            root: root.to_path_buf(),
            writ_count: 0,
            rule_count: 0,
            unguarded_count: 0,
            dead_guards: Vec::new(),
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

    // Built from what the writs declare, not from what this tool can do.
    // A repository is told about its own rules; the kinds it does not use
    // are this crate's business and not its.
    for kind in SINGLETON_KINDS {
        let ids: Vec<&str> = declared
            .iter()
            .filter(|r| r.guard == writ::Guard::Twrit((*kind).to_string()))
            .map(|r| r.id.as_str())
            .collect();
        if ids.len() > 1 {
            anyhow::bail!(
                "more than one rule guards `{kind}` ({}); a layer order has to be one thing",
                ids.join(", ")
            );
        }
    }

    let mut rules = Vec::new();
    for rule in &declared {
        let writ::Guard::Twrit(kind) = &rule.guard else {
            continue;
        };
        let outcome = match kind.as_str() {
            "layers" => layering::check(rule, root)?,
            "pure" => purity::check(rule, root)?,
            "door" => door::check(rule, root)?,
            _ => unreachable!("checked against KINDS above"),
        };
        rules.push(RuleResult {
            id: rule.id.clone(),
            outcome,
        });
    }

    Ok(Report {
        root: root.to_path_buf(),
        writ_count: writs.len(),
        dead_guards: declared
            .iter()
            .filter_map(|rule| {
                rule.guard.dead_pointer(root).map(|target| {
                    format!("{}: guard names {target}, which does not exist", rule.id)
                })
            })
            .collect(),
        rule_count: declared.len(),
        unguarded_count: declared
            .iter()
            .filter(|r| matches!(r.guard, writ::Guard::None(_)))
            .count(),
        rules,
    })
}
