//! Finding and reading `.writ.tmt` files.
//!
//! A writ is an author-owned normative file, one per directory, scoped to
//! that directory. This module only locates and parses them; deciding what
//! a given rule means is each guard's own job.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tomet_ast::{Block, Document, Element, ElementValue, Entry, Sigil, Value};

/// One parsed writ, kept with its path so a violation can name the file the
/// rule came from rather than just the rule.
pub struct Writ {
    pub path: PathBuf,
    pub doc: Document,
}

/// Every `.writ.tmt` at or below `root`, in walk order.
///
/// Hidden files are explicitly included. `.writ.tmt` is the nameless form of
/// the `<name>.<kind>.tmt` convention, so the leading dot is a kind
/// separator rather than a request to be invisible -- but every directory
/// walker treats it as the latter by default, which is precisely how the
/// tomet repository's own `check-links` came to skip these files.
pub fn discover(root: &Path) -> Result<Vec<Writ>> {
    let mut found = Vec::new();

    for entry in ignore::WalkBuilder::new(root).hidden(false).build() {
        let entry = entry.context("failed to walk the tree")?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        if entry.file_name() != ".writ.tmt" {
            continue;
        }

        let path = entry.path().to_path_buf();
        let src = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let doc = tomet_parser::parse_document(&src)
            .map_err(|e| anyhow::anyhow!("{}: parse error: {e}", path.display()))?;

        found.push(Writ { path, doc });
    }

    Ok(found)
}

impl Writ {
    /// The first top-level element named `name`, if this writ declares one.
    ///
    /// Rules are found by element name rather than by their heading, even
    /// though the heading is what a reader navigates by. A heading is prose
    /// and will be reworded; an element name is the part a tool can be held
    /// to.
    pub fn element(&self, name: &str) -> Option<&Element> {
        self.doc.blocks.iter().find_map(|block| match block {
            Block::Element(el) => match &el.sigil {
                Sigil::Named(n) if n.namespace.is_none() && n.name == name => Some(el),
                _ => None,
            },
            _ => None,
        })
    }
}

/// Reads an element's `{...}` group as `key: [string, ...]` pairs.
///
/// Returns an error rather than skipping anything it cannot read: a rule
/// half-understood is worse than a rule that refuses to run, because the
/// half it dropped is exactly where a violation would hide.
pub fn string_list_map(el: &Element, what: &str) -> Result<Vec<(String, Vec<String>)>> {
    let Some(ElementValue::Group(entries)) = &el.value else {
        anyhow::bail!("{what} has no `{{...}}` group");
    };

    entries
        .iter()
        .map(|entry| {
            let Entry::Pair(key, value) = entry else {
                anyhow::bail!("{what} holds an element where a `key: value` pair was expected");
            };
            let Value::Seq(items) = value else {
                anyhow::bail!("{what}'s `{key}` is not a list");
            };
            let strings = items
                .iter()
                .map(|item| match item {
                    Value::String(s) => Ok(s.clone()),
                    other => anyhow::bail!("{what}'s `{key}` holds a non-string entry: {other:?}"),
                })
                .collect::<Result<Vec<_>>>()?;
            Ok((key.clone(), strings))
        })
        .collect()
}
