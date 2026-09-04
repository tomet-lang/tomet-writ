//! The `@pure` guard: crates that may not reach the outside world.
//!
//! Two halves, because there are two ways to acquire the ability:
//!
//! 1. **`allow-external`** -- every external crate in the dependency
//!    closure is named there. Something new cannot arrive without a
//!    decision, which is the property; "no external crates at all" would be
//!    a nicer sentence and a false one.
//! 2. **`forbid`** -- none of the named paths appears in the crate's
//!    source, so it cannot be done bare-handed either.
//!
//! Neither name nor path is written here. Which crates, and which paths,
//! are `@pure` data in the writ -- see `AGENTS.md`'s "What belongs in this
//! tool".

use std::collections::{BTreeSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand};
use tomet_ast::{Element, ElementValue, Entry, Value};

use crate::writ::Writ;

/// One `@pure` declaration.
struct Pure {
    /// Directory paths, relative to the workspace root.
    crates: Vec<String>,
    allow_external: Vec<String>,
    forbid: Vec<String>,
}

impl Pure {
    fn read(el: &Element) -> Result<Self> {
        let Some(ElementValue::Group(entries)) = &el.value else {
            anyhow::bail!("`@pure` has no `{{...}}` group");
        };

        let mut crates = Vec::new();
        let mut allow_external = Vec::new();
        let mut forbid = Vec::new();

        for entry in entries {
            let Entry::Pair(key, value) = entry else {
                anyhow::bail!("`@pure` holds an element where a `key: value` pair was expected");
            };
            match key.as_str() {
                "crates" => crates = strings(key, value)?,
                "forbid" => forbid = strings(key, value)?,
                "allow-external" => allow_external = strings(key, value)?,
                other => anyhow::bail!("`@pure` does not know the key `{other}`"),
            }
        }

        anyhow::ensure!(!crates.is_empty(), "`@pure` names no crates");
        Ok(Pure {
            crates,
            allow_external,
            forbid,
        })
    }
}

fn strings(key: &str, value: &Value) -> Result<Vec<String>> {
    let Value::Seq(items) = value else {
        anyhow::bail!("`@pure`'s `{key}` is not a list");
    };
    items
        .iter()
        .map(|item| match item {
            Value::String(s) => Ok(s.clone()),
            other => anyhow::bail!("`@pure`'s `{key}` holds a non-string entry: {other:?}"),
        })
        .collect()
}

/// Runs every `@pure` declared across `writs`, returning one line per
/// violation.
pub fn check(writs: &[Writ], workspace_root: &Path) -> Result<Vec<String>> {
    let declarations: Vec<&Element> = writs.iter().filter_map(|w| w.element("pure")).collect();
    if declarations.is_empty() {
        return Ok(Vec::new());
    }

    let metadata = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .context("failed to read the workspace with `cargo metadata`")?;

    let mut violations = Vec::new();
    for el in declarations {
        let pure = Pure::read(el)?;
        for dir in &pure.crates {
            check_one(&metadata, workspace_root, dir, &pure, &mut violations)?;
        }
    }

    violations.sort();
    violations.dedup();
    Ok(violations)
}

fn check_one(
    metadata: &Metadata,
    workspace_root: &Path,
    dir: &str,
    pure: &Pure,
    violations: &mut Vec<String>,
) -> Result<()> {
    let root = metadata.workspace_root.as_std_path();
    let package = metadata.workspace_packages().into_iter().find(|p| {
        p.manifest_path
            .as_std_path()
            .parent()
            .and_then(|d| d.strip_prefix(root).ok())
            .is_some_and(|d| d.to_string_lossy().replace('\\', "/") == dir)
    });
    let Some(package) = package else {
        violations.push(format!("`@pure` names {dir}, which is not a workspace member"));
        return Ok(());
    };

    {
        let members: BTreeSet<&str> = metadata
            .workspace_packages()
            .iter()
            .map(|p| p.name.as_str())
            .collect();

        // The closure, not just the direct list: a dependency that stays
        // internal is worth nothing if what *it* pulls in does not.
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([package.name.as_str()]);
        while let Some(name) = queue.pop_front() {
            if !seen.insert(name) {
                continue;
            }
            let Some(pkg) = metadata.packages.iter().find(|p| p.name.as_str() == name) else {
                continue;
            };
            for dep in &pkg.dependencies {
                if dep.kind != DependencyKind::Normal {
                    continue;
                }
                if members.contains(dep.name.as_str()) {
                    queue.push_back(dep.name.as_str());
                } else if !pure.allow_external.iter().any(|a| a == dep.name.as_str()) {
                    violations.push(format!(
                        "{dir} reaches an external crate not in `allow-external`: {} -> {}",
                        name, dep.name
                    ));
                }
            }
        }
    }

    for path in &pure.forbid {
        for hit in grep_sources(&workspace_root.join(dir), path)? {
            violations.push(format!("{dir} names a forbidden path: {path} in {hit}"));
        }
    }

    Ok(())
}

/// Files under `dir`'s `src/` mentioning `needle`.
///
/// Text search, deliberately. Anything cleverer needs to parse Rust, and a
/// crate that has no business touching the filesystem has no business
/// writing `std::fs` in a comment either.
fn grep_sources(dir: &Path, needle: &str) -> Result<Vec<String>> {
    let mut hits = Vec::new();
    let src = dir.join("src");
    if !src.is_dir() {
        return Ok(hits);
    }
    for entry in ignore::WalkBuilder::new(&src).hidden(false).build() {
        let entry = entry.context("failed to walk the crate's sources")?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if entry.path().extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        if text.contains(needle) {
            hits.push(entry.path().display().to_string());
        }
    }
    hits.sort();
    Ok(hits)
}
