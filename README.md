<!-- Generated from tmtroot/readme.tmt. Edit that, then `tomet export .`. -->

# Twrit

Rule enforcement toolchain for normative repository policies in the Tomet ecosystem.

## Overview

`twrit` enforces the architectural and stylistic rules a repository writes down in its `.writ.tmt` files. A rule nobody checks is a wish; this tool makes written rules fail loudly in CI instead of drifting quietly out of true.

A *writ* is an author-owned normative document, scoped to its directory and inherited downward similar to `.gitignore`. Entries carry **id / rule / why / guard**, ensuring every policy is auditable and verifiable.

## Workspace Layout

- `app` (`twrit-cli`) -- Main CLI binary (`twrit`).
- `crates/twrit` (`twrit`) -- Core parser, validator, and rule execution engine.
- `crates/twrit-rust` (`twrit-rust`) -- Built-in guards for Cargo workspaces and Rust crates.
- `tests` -- Comprehensive integration tests and workspace fixtures.

## Building and Testing

### Build workspace

```bash
cargo build
```

### Run tests

```bash
cargo test --workspace
```

### Check repository rules

```bash
cargo run -p twrit-cli -- check .
```

