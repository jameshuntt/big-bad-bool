# Big Bad Bool

Packed-schema / bit-packing library family.

Workspace crates:
- `big-bad-bool-core`
- `big-bad-bool-derive`
- `big-bad-bool`
- `big-bad-bool-interact`
- `big-bad-bool-test-suite`

## Program layer (optional)

`big-bad-bool` can also carry the program layer: dimensions over `#[bitpack]` types, contracts across them,
bounded verification, optional framing, and stepping with one write path. It is the independently published
[`big-bad-bool-program`](https://crates.io/crates/big-bad-bool-program) crate, re-exported whole as
`big_bad_bool::program` behind a feature. `#[bitpack]` and the core metadata do not need it.

```toml
big-bad-bool = { version = "0.1", features = ["program"] }
```

```rust
use big_bad_bool::program::{contract, dimension, sequence, Program, Stepper};
```

| feature        | turns on                                                        | Rust          |
|----------------|-----------------------------------------------------------------|---------------|
| (none)         | `#[bitpack]` and the core metadata                              | 1.85          |
| `program`      | `big_bad_bool::program`                                         | 1.85          |
| `program-bowl` | `program` plus the program crate's `bowl` feature (`BowlObserver`, ob-serve-a-bowl) | 1.88 |

Neither is a default feature. With neither, the facade is what it was in 0.1.1: `#[bitpack]` and the core
metadata are unchanged. The base set and `program` build on Rust 1.85, the workspace rust-version; `program-bowl` needs 1.88.
The program crate's API is documented at [docs.rs/big-bad-bool-program](https://docs.rs/big-bad-bool-program),
and its source, guide and examples are at [jameshuntt/big-bad-bool-program](https://github.com/jameshuntt/big-bad-bool-program).

## Releasing

Bump `version` in Cargo.toml, add a dated `## <version> (YYYY-MM-DD)` entry to CHANGELOG.md, and push to
`main`. CI (.github/workflows/ci.yml) runs the tests and clippy, then publishes to crates.io each crate whose
version is not there yet, in dependency order.
