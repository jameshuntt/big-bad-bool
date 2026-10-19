# Big Bad Bool

Packed-schema / bit-packing library family.

Workspace crates:
- `big-bad-bool-core`
- `big-bad-bool-derive`
- `big-bad-bool`
- `big-bad-bool-interact`
- `big-bad-bool-test-suite`

## Releasing

Bump `version` in Cargo.toml, add a dated `## <version> (YYYY-MM-DD)` entry to CHANGELOG.md, and push to
`main`. CI (.github/workflows/ci.yml) runs the tests and clippy, then publishes to crates.io each crate whose
version is not there yet, in dependency order.
