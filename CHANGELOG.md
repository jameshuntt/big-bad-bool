# Changelog

## 0.1.1 (2026-10-16): big-bad-bool big-bad-bool-derive

- The published 0.1.0 packages had `repository = "https://github.com/jameshuntt"`, the GitHub profile, so the crates.io page linked nowhere useful. 0.1.1 points at this repo. The workspace manifest here already said so: [Cargo.toml @ b3a459d](https://github.com/jameshuntt/big-bad-bool/blob/b3a459d593fc45ec40740d964f480ed0174d704a/Cargo.toml).
- No code change. Each 0.1.1 package was built from its published 0.1.0 package with only `version` and `repository` changed, and `diff -r` of each 0.1.0 package's `src` against `crates/<name>/src` here prints nothing.
