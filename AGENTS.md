# Twrit

## What this is

`twrit` enforces the rules a repository writes down in its `.writ.tmt`
files. A rule nobody checks is a wish; this tool is what makes a written
rule fail loudly instead of drifting quietly out of true.

A *writ* is an author-owned normative file, one per directory, scoped to
that directory and inherited downward the way `.gitignore` is. Entries carry
**id / rule / why / guard**, and no status field -- a withdrawn rule is
deleted and git holds the history. Only the author edits a writ; an agent
that finds a rule in its way proposes an edit and stops, rather than working
around the rule or relaxing it to fit.

The sibling `../tomet` repository is both the language this reads and the
first consumer of this tool. Its `.writ.tmt` files are the working examples;
read them before changing how writs are parsed.

## The two repositories

Neither builds the other. `tomet`'s flake takes `twrit` as a dev-shell tool
for project management; this flake takes `tomet` as a dev-shell tool for
formatting `.tmt` files. `flake.lock` pins both, so the mutual inputs
resolve over commits rather than looping.

At the Cargo level the direction is one way: `twrit-cli` depends on
`tomet-parser`/`tomet-ast`/`tomet-tree` as git dependencies, because reading
a `.writ.tmt` means parsing Tomet. Nothing in `tomet` depends on this crate.

`.cargo/config.toml` sets `net.git-fetch-with-cli`: the tomet repository is
on a private Forgejo over SSH and cargo's built-in libgit2 fetcher cannot
authenticate against it.

## Reading a writ

Rules are located by **element name**, not by their heading. A heading is
prose and will be reworded; an element name is the part a tool can be held
to. `crate-layering`'s data is an `@layers` element, and the guard finds it
by that name.

Prefer a shape the parser already types. `@table`'s content arrives as one
flat `Text` node, and bare children hold inline markup where `apps/*` opens
a block comment -- both would mean string surgery in the guard. A plain
value group of `key: [ "string", ... ]` arrives as
`Entry::Pair(String, Seq([String, ...]))` and needs no parsing at all.

There is deliberately no `@invariant(...)`-style notation yet. The shape of
a rule declaration should be decided once several guards exist and the
repetition is obvious, not from a sample of one.

## What belongs in this tool

`twrit` implements rule **kinds**. The writ supplies the **parameters**.

`crate-layering` is the shape to copy: this crate knows "members belong to
layers, and a layer may not depend upward", and nothing else. Which
directories, and in what order, is `@layers` data inside the writ. Another
repository writes its own `@layers` and the same code enforces it.

The failure to avoid is a rule that only makes sense for one repository.
The moment a crate name, a directory, or an API string is written in *this*
crate's source rather than read from a writ, `twrit` has stopped being a
tool and become tomet's project manager, and there is no line left to stop
the next one.

So: **a rule that cannot be written down as data does not belong here.**
It belongs in the repository's own test suite, or behind a command that
repository already ships.

`tomet`'s writs show both sides. `crate-layering` and `parser-purity` are
data, so they live here. `no-vocabulary` is "parse two sources differing in
one identifier and compare the trees" -- there is no way to express that as
parameters, so it stays in `tests/src/vocabulary.rs`. `kind-not-meta-type`
and `generated-md-not-edited` are guarded by `tomet refactor --check` and
`tomet export --check`, which are that project's own tools.

None of those three are worse off for living elsewhere. A guard belongs
next to the thing it can actually check.

## Language

Write code comments and documentation in English. Do not use Japanese in
code.

## Verifying changes

This is a CLI with no interactive surface. Verify with `cargo build`,
`cargo test` and by running it:

```bash
cargo run -- check ../tomet
```

A guard is not finished when it passes. **Make it fail before you believe
it.** Mutate the writ it reads -- delete a row, invert an edge, make a
member match two patterns -- confirm each path reports and exits non-zero,
then restore. A guard that has never been seen to fail is not known to
work, and a guard that silently stopped checking anything looks exactly
like a guard that passes.

## Task tracking

Before starting implementation on any non-trivial task, create a file under
`.agents/tasks/` (one per task, e.g. `.agents/tasks/<short-task-slug>.md`)
breaking the work into discrete steps. Update it immediately after each
step. Keep it accurate enough that the work can be resumed from that file
alone, without the conversation that produced it.

If a session has to end early, stop at the next safe checkpoint -- finish
the current atomic step rather than starting a new one -- and write the
state and the next steps into that file first.

When the task is done, fold anything worth keeping into where it belongs --
a doc comment, a writ entry, a commit message -- and then delete the task
file. Deleting without folding is how the only copy of a decision gets
lost; decide the destination when the task is created, not when it is
deleted.

## Commits

Do not put `Claude-Session:` or `Co-Authored-By: Claude` trailers in commit
messages. The session trailer embeds a URL, and a commit message is
published the moment it is pushed. This overrides the harness default that
asks for them.

Never push to a remote without being asked.
