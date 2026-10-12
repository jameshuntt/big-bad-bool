# big-bad-bool-interact

Higher-level API for negotiation and rules over generated `PackedWord` values.

Use this crate when you want to map packed fields to behavior without hand-writing bit checks at every call site.

It includes:

- `RuleTable::dispatch(...)`
- `RuleTable::first_action(...)`
- `RuleTable::validate_for::<T>()`
- `negotiate_schema::<T>(...)`
- `PackedView<T>`

## Coverage engine

This crate also includes an exhaustive coverage engine for small packed schemas:

- `RuleTable::coverage_for::<T>()`
- `RuleTable::coverage_for_with_policy::<T>(...)`
- `RuleTable::coverage_for_with_options::<T>(...)`
- `CoveragePolicy`
- `StatePolicy::reserved(...)`
- `StatePolicy::invalid(...)`

The engine walks all representable raw states for a packed type, then reports which states are covered, uncovered, overlapping, reserved, invalid, or accidentally hit by rules despite being policy-excluded.

## Flow/reconstruction layer

The interaction crate also includes the runtime-side helpers:

- `PackedReconstructor<T>` reconstructs a packed word from symbolic field assignments.
- `patch_word` overlays field updates onto an existing packed word.
- `PassthroughMode` controls whether unmatched/reserved/invalid/overlap states reject or continue down-chain.
- `PackedFlow<T, Ctx>` carries a packed envelope through FP-style steps such as patching, dispatch, passthrough, tap, and handler execution.
- `HandlerRegistry<T, Ctx>` maps dispatch actions into executable closures.

The macro remains responsible only for bit layout. The flow layer is behavior orchestration on top of generated packed words.

## Label sources

`LabelRegistry` accepts both external label slices and macro-generated schema labels such as `PropTag16::LABELS`. Stable labels can now live in the `#[bitpack]` declaration while dynamic/spec labels can still be supplied by higher-level crates.
