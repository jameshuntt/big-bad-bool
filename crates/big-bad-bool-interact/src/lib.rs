//! Higher-level interaction API for generated bitpack values.
//!
//! The macro answers: "how do I pack and unpack this safely?"
//! This crate answers: "what should happen when I read this packed value?"

use big_bad_bool_core::{BitFieldSpec, PackedWord};
pub use big_bad_bool_core::ValueLabel;

pub mod reconstruction;
pub mod passthrough;
pub mod flow;
pub mod handlers;

pub use reconstruction::*;
pub use passthrough::*;
pub use flow::*;
pub use handlers::*;

/// A named operation returned by a rule match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub code: u16,
    pub name: &'static str,
    pub description: &'static str,
}

impl Action {
    #[must_use]
    pub const fn new(code: u16, name: &'static str, description: &'static str) -> Self {
        Self {
            code,
            name,
            description,
        }
    }
}

/// Match operation for a single field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchOp {
    Any,
    Eq(u128),
    NotEq(u128),
    RangeInclusive { min: u128, max: u128 },
    MaskAll { mask: u128 },
    MaskAny { mask: u128 },
}

impl MatchOp {
    #[must_use]
    pub const fn matches(self, value: u128) -> bool {
        match self {
            Self::Any => true,
            Self::Eq(expected) => value == expected,
            Self::NotEq(disallowed) => value != disallowed,
            Self::RangeInclusive { min, max } => value >= min && value <= max,
            Self::MaskAll { mask } => (value & mask) == mask,
            Self::MaskAny { mask } => (value & mask) != 0,
        }
    }
}

/// A condition against a named field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldMatcher {
    pub field: &'static str,
    pub op: MatchOp,
}

impl FieldMatcher {
    #[must_use]
    pub const fn any(field: &'static str) -> Self {
        Self {
            field,
            op: MatchOp::Any,
        }
    }

    #[must_use]
    pub const fn eq(field: &'static str, value: u128) -> Self {
        Self {
            field,
            op: MatchOp::Eq(value),
        }
    }

    #[must_use]
    pub const fn not_eq(field: &'static str, value: u128) -> Self {
        Self {
            field,
            op: MatchOp::NotEq(value),
        }
    }

    #[must_use]
    pub const fn range(field: &'static str, min: u128, max: u128) -> Self {
        Self {
            field,
            op: MatchOp::RangeInclusive { min, max },
        }
    }

    #[must_use]
    pub const fn mask_all(field: &'static str, mask: u128) -> Self {
        Self {
            field,
            op: MatchOp::MaskAll { mask },
        }
    }

    #[must_use]
    pub const fn mask_any(field: &'static str, mask: u128) -> Self {
        Self {
            field,
            op: MatchOp::MaskAny { mask },
        }
    }

    #[must_use]
    pub fn matches<T: PackedWord>(self, word: T) -> bool {
        word.read_field(self.field)
            .is_some_and(|value| self.op.matches(value))
    }
}

/// A named rule that maps packed-field observations to an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub name: &'static str,
    pub matchers: &'static [FieldMatcher],
    pub action: Action,
}

impl Rule {
    #[must_use]
    pub const fn new(
        name: &'static str,
        matchers: &'static [FieldMatcher],
        action: Action,
    ) -> Self {
        Self {
            name,
            matchers,
            action,
        }
    }

    #[must_use]
    pub fn matches<T: PackedWord>(&self, word: T) -> bool {
        self.matchers.iter().copied().all(|m| m.matches(word))
    }
}

/// Result of applying rules to a packed value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchReport {
    pub raw: u128,
    pub matched_actions: Vec<Action>,
    pub matched_rule_names: Vec<&'static str>,
}

impl DispatchReport {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.matched_actions.is_empty()
    }
}

/// Static rule table.
#[derive(Debug, Clone, Copy)]
pub struct RuleTable {
    pub name: &'static str,
    pub rules: &'static [Rule],
}

impl RuleTable {
    #[must_use]
    pub const fn new(name: &'static str, rules: &'static [Rule]) -> Self {
        Self { name, rules }
    }

    #[must_use]
    pub fn dispatch<T: PackedWord>(self, word: T) -> DispatchReport {
        let mut matched_actions = Vec::new();
        let mut matched_rule_names = Vec::new();

        for rule in self.rules {
            if rule.matches(word) {
                matched_actions.push(rule.action.clone());
                matched_rule_names.push(rule.name);
            }
        }

        DispatchReport {
            raw: word.raw_u128(),
            matched_actions,
            matched_rule_names,
        }
    }

    #[must_use]
    pub fn first_action<T: PackedWord>(self, word: T) -> Option<Action> {
        self.rules
            .iter()
            .find(|rule| rule.matches(word))
            .map(|rule| rule.action.clone())
    }

    /// Validates this table against a generated packed schema before using it.
    #[must_use]
    pub fn validate_for<T: PackedWord>(self) -> RuleValidationReport {
        validate_rule_table::<T>(self)
    }
}

/// Result of validating a rule table against a packed schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleValidationReport {
    pub accepted: bool,
    pub findings: Vec<RuleValidationFinding>,
}

/// A schema-level problem found in a rule table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleValidationFinding {
    EmptyRule {
        rule: &'static str,
    },
    UnknownField {
        rule: &'static str,
        field: &'static str,
    },
    ValueOutOfRange {
        rule: &'static str,
        field: &'static str,
        value: u128,
        max: u128,
    },
    InvalidRange {
        rule: &'static str,
        field: &'static str,
        min: u128,
        max: u128,
    },
    RangeOutOfRange {
        rule: &'static str,
        field: &'static str,
        min: u128,
        max: u128,
        field_max: u128,
    },
    MaskOutOfRange {
        rule: &'static str,
        field: &'static str,
        mask: u128,
        field_max: u128,
    },
}

/// Checks whether all rule matchers reference real fields and possible values.
#[must_use]
pub fn validate_rule_table<T: PackedWord>(table: RuleTable) -> RuleValidationReport {
    let mut findings = Vec::new();

    for rule in table.rules {
        if rule.matchers.is_empty() {
            findings.push(RuleValidationFinding::EmptyRule { rule: rule.name });
        }

        for matcher in rule.matchers.iter().copied() {
            let Some(field) = T::field(matcher.field) else {
                findings.push(RuleValidationFinding::UnknownField {
                    rule: rule.name,
                    field: matcher.field,
                });
                continue;
            };

            let max = field.max_value();

            match matcher.op {
                MatchOp::Any => {}
                MatchOp::Eq(value) | MatchOp::NotEq(value) => {
                    if value > max {
                        findings.push(RuleValidationFinding::ValueOutOfRange {
                            rule: rule.name,
                            field: matcher.field,
                            value,
                            max,
                        });
                    }
                }
                MatchOp::RangeInclusive { min, max: range_max } => {
                    if min > range_max {
                        findings.push(RuleValidationFinding::InvalidRange {
                            rule: rule.name,
                            field: matcher.field,
                            min,
                            max: range_max,
                        });
                    } else if range_max > max {
                        findings.push(RuleValidationFinding::RangeOutOfRange {
                            rule: rule.name,
                            field: matcher.field,
                            min,
                            max: range_max,
                            field_max: max,
                        });
                    }
                }
                MatchOp::MaskAll { mask } | MatchOp::MaskAny { mask } => {
                    if mask > max {
                        findings.push(RuleValidationFinding::MaskOutOfRange {
                            rule: rule.name,
                            field: matcher.field,
                            mask,
                            field_max: max,
                        });
                    }
                }
            }
        }
    }

    RuleValidationReport {
        accepted: findings.is_empty(),
        findings,
    }
}

/// A requirement for negotiating whether a packed schema is suitable for a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldRequirement {
    pub field: &'static str,
    pub min_width: u32,
    pub required: bool,
}

impl FieldRequirement {
    #[must_use]
    pub const fn required(field: &'static str, min_width: u32) -> Self {
        Self {
            field,
            min_width,
            required: true,
        }
    }

    #[must_use]
    pub const fn optional(field: &'static str, min_width: u32) -> Self {
        Self {
            field,
            min_width,
            required: false,
        }
    }
}

/// A single negotiation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NegotiationFinding {
    Present {
        requirement: FieldRequirement,
        field: BitFieldSpec,
    },
    TooNarrow {
        requirement: FieldRequirement,
        field: BitFieldSpec,
    },
    MissingRequired {
        requirement: FieldRequirement,
    },
    MissingOptional {
        requirement: FieldRequirement,
    },
}

impl NegotiationFinding {
    #[must_use]
    pub const fn ok(&self) -> bool {
        matches!(
            self,
            Self::Present { .. } | Self::MissingOptional { .. }
        )
    }
}

/// Schema-level negotiation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiationReport {
    pub accepted: bool,
    pub findings: Vec<NegotiationFinding>,
}

/// Checks whether type `T` satisfies a higher-level API contract.
#[must_use]
pub fn negotiate_schema<T: PackedWord>(requirements: &[FieldRequirement]) -> NegotiationReport {
    let mut findings = Vec::with_capacity(requirements.len());
    let mut accepted = true;

    for requirement in requirements.iter().copied() {
        match T::field(requirement.field) {
            Some(field) if field.width >= requirement.min_width => {
                findings.push(NegotiationFinding::Present { requirement, field });
            }
            Some(field) => {
                accepted = false;
                findings.push(NegotiationFinding::TooNarrow { requirement, field });
            }
            None if requirement.required => {
                accepted = false;
                findings.push(NegotiationFinding::MissingRequired { requirement });
            }
            None => {
                findings.push(NegotiationFinding::MissingOptional { requirement });
            }
        }
    }

    NegotiationReport { accepted, findings }
}

/// Convenience reader for ergonomic field access in higher-level code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedView<T: PackedWord> {
    word: T,
}

impl<T: PackedWord> PackedView<T> {
    #[must_use]
    pub const fn new(word: T) -> Self {
        Self { word }
    }

    #[must_use]
    pub const fn word(self) -> T {
        self.word
    }

    #[must_use]
    pub fn get(self, field: &str) -> Option<u128> {
        self.word.read_field(field)
    }

    #[must_use]
    pub fn require(self, field: &str) -> Result<u128, MissingFieldError> {
        self.get(field).ok_or(MissingFieldError { field: field.into() })
    }
}

/// Returned when a caller asks for a missing field by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingFieldError {
    pub field: String,
}

/// Classification for a state-space policy entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatePolicyKind {
    /// The state is intentionally left unused for future expansion.
    Reserved,
    /// The state is representable by the bits but forbidden by the higher-level protocol.
    Invalid,
}

/// A named policy declaration over packed states.
///
/// This uses the same matcher vocabulary as rules, but it does not produce an
/// action. It marks portions of the representable state-space as deliberately
/// excluded from normal rule coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatePolicy {
    pub name: &'static str,
    pub kind: StatePolicyKind,
    pub matchers: &'static [FieldMatcher],
    pub description: &'static str,
}

impl StatePolicy {
    #[must_use]
    pub const fn new(
        name: &'static str,
        kind: StatePolicyKind,
        matchers: &'static [FieldMatcher],
        description: &'static str,
    ) -> Self {
        Self {
            name,
            kind,
            matchers,
            description,
        }
    }

    #[must_use]
    pub const fn reserved(
        name: &'static str,
        matchers: &'static [FieldMatcher],
        description: &'static str,
    ) -> Self {
        Self::new(name, StatePolicyKind::Reserved, matchers, description)
    }

    #[must_use]
    pub const fn invalid(
        name: &'static str,
        matchers: &'static [FieldMatcher],
        description: &'static str,
    ) -> Self {
        Self::new(name, StatePolicyKind::Invalid, matchers, description)
    }

    #[must_use]
    pub fn matches<T: PackedWord>(self, word: T) -> bool {
        self.matchers.iter().copied().all(|m| m.matches(word))
    }
}

/// A static set of reserved/invalid state declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoveragePolicy {
    pub states: &'static [StatePolicy],
}

impl CoveragePolicy {
    #[must_use]
    pub const fn new(states: &'static [StatePolicy]) -> Self {
        Self { states }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self { states: &[] }
    }
}

/// Options controlling exhaustive state-space coverage scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverageOptions {
    /// Maximum number of raw states that may be scanned.
    ///
    /// This protects callers from accidentally trying to exhaustively scan a
    /// 32/64/128-bit layout. Raise this intentionally for larger schemas.
    pub max_states: u128,
    /// Maximum examples stored per example category.
    pub max_examples: usize,
}

impl CoverageOptions {
    #[must_use]
    pub const fn new(max_states: u128, max_examples: usize) -> Self {
        Self {
            max_states,
            max_examples,
        }
    }

    #[must_use]
    pub const fn exhaustive_u16() -> Self {
        Self {
            max_states: 1u128 << 16,
            max_examples: 16,
        }
    }
}

impl Default for CoverageOptions {
    fn default() -> Self {
        Self {
            max_states: 1u128 << 20,
            max_examples: 16,
        }
    }
}

/// A decoded field value captured in a coverage example.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoverageFieldValue {
    pub field: &'static str,
    pub value: u128,
}

/// One representative packed state from a coverage category.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoverageExample {
    pub raw: u128,
    pub fields: Vec<CoverageFieldValue>,
    pub matched_rule_names: Vec<&'static str>,
    pub policy_names: Vec<&'static str>,
}

/// Exhaustive coverage report for a rule table over a packed schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReport {
    pub table: &'static str,
    pub used_bits: u32,
    pub total_states: u128,
    pub normal_states: u128,
    pub covered_states: u128,
    pub single_match_states: u128,
    pub unmatched_states: u128,
    pub overlap_states: u128,
    pub reserved_states: u128,
    pub invalid_states: u128,
    pub policy_rule_conflict_states: u128,
    pub unmatched_examples: Vec<CoverageExample>,
    pub overlap_examples: Vec<CoverageExample>,
    pub reserved_examples: Vec<CoverageExample>,
    pub invalid_examples: Vec<CoverageExample>,
    pub policy_rule_conflict_examples: Vec<CoverageExample>,
}

impl CoverageReport {
    /// True when every normal state has exactly one rule and no policy state is
    /// accidentally matched by a rule.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.unmatched_states == 0
            && self.overlap_states == 0
            && self.policy_rule_conflict_states == 0
    }

    /// True when every normal state has at least one rule.
    #[must_use]
    pub const fn fully_covered(&self) -> bool {
        self.unmatched_states == 0
    }

    /// Percentage of normal states covered by at least one rule.
    #[must_use]
    pub fn normal_coverage_percent(&self) -> f64 {
        if self.normal_states == 0 {
            100.0
        } else {
            (self.covered_states as f64 / self.normal_states as f64) * 100.0
        }
    }
}


/// Result of validating reserved/invalid state declarations against a packed schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveragePolicyValidationReport {
    pub accepted: bool,
    pub findings: Vec<CoveragePolicyValidationFinding>,
}

/// A schema-level problem found in a coverage policy declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoveragePolicyValidationFinding {
    EmptyPolicy {
        policy: &'static str,
    },
    UnknownField {
        policy: &'static str,
        field: &'static str,
    },
    ValueOutOfRange {
        policy: &'static str,
        field: &'static str,
        value: u128,
        max: u128,
    },
    InvalidRange {
        policy: &'static str,
        field: &'static str,
        min: u128,
        max: u128,
    },
    RangeOutOfRange {
        policy: &'static str,
        field: &'static str,
        min: u128,
        max: u128,
        field_max: u128,
    },
    MaskOutOfRange {
        policy: &'static str,
        field: &'static str,
        mask: u128,
        field_max: u128,
    },
}

impl CoveragePolicy {
    /// Validates this reserved/invalid policy set against a generated packed schema.
    #[must_use]
    pub fn validate_for<T: PackedWord>(self) -> CoveragePolicyValidationReport {
        validate_coverage_policy::<T>(self)
    }
}

/// Checks whether all coverage-policy matchers reference real fields and possible values.
#[must_use]
pub fn validate_coverage_policy<T: PackedWord>(
    policy: CoveragePolicy,
) -> CoveragePolicyValidationReport {
    let mut findings = Vec::new();

    for state in policy.states {
        if state.matchers.is_empty() {
            findings.push(CoveragePolicyValidationFinding::EmptyPolicy {
                policy: state.name,
            });
        }

        for matcher in state.matchers.iter().copied() {
            let Some(field) = T::field(matcher.field) else {
                findings.push(CoveragePolicyValidationFinding::UnknownField {
                    policy: state.name,
                    field: matcher.field,
                });
                continue;
            };

            let max = field.max_value();

            match matcher.op {
                MatchOp::Any => {}
                MatchOp::Eq(value) | MatchOp::NotEq(value) => {
                    if value > max {
                        findings.push(CoveragePolicyValidationFinding::ValueOutOfRange {
                            policy: state.name,
                            field: matcher.field,
                            value,
                            max,
                        });
                    }
                }
                MatchOp::RangeInclusive { min, max: range_max } => {
                    if min > range_max {
                        findings.push(CoveragePolicyValidationFinding::InvalidRange {
                            policy: state.name,
                            field: matcher.field,
                            min,
                            max: range_max,
                        });
                    } else if range_max > max {
                        findings.push(CoveragePolicyValidationFinding::RangeOutOfRange {
                            policy: state.name,
                            field: matcher.field,
                            min,
                            max: range_max,
                            field_max: max,
                        });
                    }
                }
                MatchOp::MaskAll { mask } | MatchOp::MaskAny { mask } => {
                    if mask > max {
                        findings.push(CoveragePolicyValidationFinding::MaskOutOfRange {
                            policy: state.name,
                            field: matcher.field,
                            mask,
                            field_max: max,
                        });
                    }
                }
            }
        }
    }

    CoveragePolicyValidationReport {
        accepted: findings.is_empty(),
        findings,
    }
}

/// Failure returned before an exhaustive coverage scan can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageScanError {
    InvalidRuleTable {
        report: RuleValidationReport,
    },
    InvalidCoveragePolicy {
        report: CoveragePolicyValidationReport,
    },
    TooManyStates {
        used_bits: u32,
        states: u128,
        limit: u128,
    },
    StateCountUnrepresentable {
        used_bits: u32,
    },
}

impl core::fmt::Display for CoverageScanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidRuleTable { report } => {
                write!(f, "rule table failed validation: {:?}", report.findings)
            }
            Self::InvalidCoveragePolicy { report } => {
                write!(f, "coverage policy failed validation: {:?}", report.findings)
            }
            Self::TooManyStates {
                used_bits,
                states,
                limit,
            } => write!(
                f,
                "refusing exhaustive scan of {states} states from {used_bits} used bits; limit is {limit}"
            ),
            Self::StateCountUnrepresentable { used_bits } => write!(
                f,
                "cannot represent exhaustive state count for {used_bits} used bits in u128"
            ),
        }
    }
}

impl std::error::Error for CoverageScanError {}

impl RuleTable {
    /// Exhaustively scans all representable states for type `T` using default
    /// coverage options and no reserved/invalid policy declarations.
    pub fn coverage_for<T: PackedWord>(self) -> Result<CoverageReport, CoverageScanError> {
        self.coverage_for_with_options::<T>(CoveragePolicy::empty(), CoverageOptions::default())
    }

    /// Exhaustively scans all representable states for type `T` with reserved
    /// and invalid policy declarations.
    pub fn coverage_for_with_policy<T: PackedWord>(
        self,
        policy: CoveragePolicy,
    ) -> Result<CoverageReport, CoverageScanError> {
        self.coverage_for_with_options::<T>(policy, CoverageOptions::default())
    }

    /// Exhaustively scans all representable states for type `T` with full
    /// control over policy declarations and scan limits.
    pub fn coverage_for_with_options<T: PackedWord>(
        self,
        policy: CoveragePolicy,
        options: CoverageOptions,
    ) -> Result<CoverageReport, CoverageScanError> {
        exhaustive_coverage::<T>(self, policy, options)
    }
}

/// Exhaustively evaluates a rule table against every raw state in a generated
/// packed schema.
pub fn exhaustive_coverage<T: PackedWord>(
    table: RuleTable,
    policy: CoveragePolicy,
    options: CoverageOptions,
) -> Result<CoverageReport, CoverageScanError> {
    let validation = table.validate_for::<T>();
    if !validation.accepted {
        return Err(CoverageScanError::InvalidRuleTable { report: validation });
    }

    let policy_validation = policy.validate_for::<T>();
    if !policy_validation.accepted {
        return Err(CoverageScanError::InvalidCoveragePolicy {
            report: policy_validation,
        });
    }

    let total_states = total_states_for_used_bits(T::USED_BITS)?;
    if total_states > options.max_states {
        return Err(CoverageScanError::TooManyStates {
            used_bits: T::USED_BITS,
            states: total_states,
            limit: options.max_states,
        });
    }

    let mut report = CoverageReport {
        table: table.name,
        used_bits: T::USED_BITS,
        total_states,
        normal_states: 0,
        covered_states: 0,
        single_match_states: 0,
        unmatched_states: 0,
        overlap_states: 0,
        reserved_states: 0,
        invalid_states: 0,
        policy_rule_conflict_states: 0,
        unmatched_examples: Vec::new(),
        overlap_examples: Vec::new(),
        reserved_examples: Vec::new(),
        invalid_examples: Vec::new(),
        policy_rule_conflict_examples: Vec::new(),
    };

    let mut raw = 0u128;
    while raw < total_states {
        let word = T::from_raw_u128_truncating(raw);
        let matched_rule_names = matching_rule_names::<T>(table, word);
        let policy_hits = matching_policy_hits::<T>(policy, word);
        let policy_names = policy_hits.iter().map(|p| p.name).collect::<Vec<_>>();
        let has_invalid_policy = policy_hits
            .iter()
            .any(|p| p.kind == StatePolicyKind::Invalid);
        let has_reserved_policy = policy_hits
            .iter()
            .any(|p| p.kind == StatePolicyKind::Reserved);

        if has_invalid_policy {
            report.invalid_states += 1;
            push_example(
                &mut report.invalid_examples,
                options.max_examples,
                make_coverage_example::<T>(word, matched_rule_names.clone(), policy_names.clone()),
            );

            if !matched_rule_names.is_empty() {
                report.policy_rule_conflict_states += 1;
                push_example(
                    &mut report.policy_rule_conflict_examples,
                    options.max_examples,
                    make_coverage_example::<T>(word, matched_rule_names, policy_names),
                );
            }
        } else if has_reserved_policy {
            report.reserved_states += 1;
            push_example(
                &mut report.reserved_examples,
                options.max_examples,
                make_coverage_example::<T>(word, matched_rule_names.clone(), policy_names.clone()),
            );

            if !matched_rule_names.is_empty() {
                report.policy_rule_conflict_states += 1;
                push_example(
                    &mut report.policy_rule_conflict_examples,
                    options.max_examples,
                    make_coverage_example::<T>(word, matched_rule_names, policy_names),
                );
            }
        } else {
            report.normal_states += 1;

            match matched_rule_names.len() {
                0 => {
                    report.unmatched_states += 1;
                    push_example(
                        &mut report.unmatched_examples,
                        options.max_examples,
                        make_coverage_example::<T>(word, matched_rule_names, policy_names),
                    );
                }
                1 => {
                    report.covered_states += 1;
                    report.single_match_states += 1;
                }
                _ => {
                    report.covered_states += 1;
                    report.overlap_states += 1;
                    push_example(
                        &mut report.overlap_examples,
                        options.max_examples,
                        make_coverage_example::<T>(word, matched_rule_names, policy_names),
                    );
                }
            }
        }

        raw += 1;
    }

    Ok(report)
}

fn total_states_for_used_bits(used_bits: u32) -> Result<u128, CoverageScanError> {
    if used_bits >= 128 {
        Err(CoverageScanError::StateCountUnrepresentable { used_bits })
    } else {
        Ok(1u128 << used_bits)
    }
}

fn matching_rule_names<T: PackedWord>(table: RuleTable, word: T) -> Vec<&'static str> {
    table
        .rules
        .iter()
        .filter(|rule| rule.matches(word))
        .map(|rule| rule.name)
        .collect()
}

fn matching_policy_hits<T: PackedWord>(policy: CoveragePolicy, word: T) -> Vec<StatePolicy> {
    policy
        .states
        .iter()
        .copied()
        .filter(|state| state.matches(word))
        .collect()
}

fn make_coverage_example<T: PackedWord>(
    word: T,
    matched_rule_names: Vec<&'static str>,
    policy_names: Vec<&'static str>,
) -> CoverageExample {
    let fields = T::FIELDS
        .iter()
        .map(|field| CoverageFieldValue {
            field: field.name,
            value: word.read_field(field.name).unwrap_or_default(),
        })
        .collect();

    CoverageExample {
        raw: word.raw_u128(),
        fields,
        matched_rule_names,
        policy_names,
    }
}

fn push_example<T>(examples: &mut Vec<T>, max_examples: usize, example: T) {
    if examples.len() < max_examples {
        examples.push(example);
    }
}

/// Static label registry for semantic field-value labels.
///
/// Labels can be supplied externally or generated directly by `#[bitpack]` via
/// field-level `#[labels(name = value, ...)]` attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelRegistry {
    pub labels: &'static [ValueLabel],
}

impl LabelRegistry {
    #[must_use]
    pub const fn new(labels: &'static [ValueLabel]) -> Self {
        Self { labels }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self { labels: &[] }
    }

    #[must_use]
    pub fn label_for(self, field: &str, value: u128) -> Option<ValueLabel> {
        self.labels
            .iter()
            .copied()
            .find(|label| label.field == field && label.value == value)
    }

    #[must_use]
    pub fn validate_for<T: PackedWord>(self) -> LabelValidationReport {
        validate_label_registry::<T>(self)
    }
}

/// Validation result for semantic labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelValidationReport {
    pub accepted: bool,
    pub findings: Vec<LabelValidationFinding>,
}

/// Problems found in a value-label registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelValidationFinding {
    UnknownField {
        field: &'static str,
        label: &'static str,
    },
    ValueOutOfRange {
        field: &'static str,
        value: u128,
        max: u128,
        label: &'static str,
    },
    DuplicateLabel {
        field: &'static str,
        value: u128,
        first: &'static str,
        second: &'static str,
    },
}

/// Checks whether value labels reference real fields and possible values.
#[must_use]
pub fn validate_label_registry<T: PackedWord>(registry: LabelRegistry) -> LabelValidationReport {
    let mut findings = Vec::new();

    for (idx, label) in registry.labels.iter().copied().enumerate() {
        let Some(field) = T::field(label.field) else {
            findings.push(LabelValidationFinding::UnknownField {
                field: label.field,
                label: label.label,
            });
            continue;
        };

        let max = field.max_value();
        if label.value > max {
            findings.push(LabelValidationFinding::ValueOutOfRange {
                field: label.field,
                value: label.value,
                max,
                label: label.label,
            });
        }

        for later in registry.labels.iter().copied().skip(idx + 1) {
            if later.field == label.field && later.value == label.value {
                findings.push(LabelValidationFinding::DuplicateLabel {
                    field: label.field,
                    value: label.value,
                    first: label.label,
                    second: later.label,
                });
            }
        }
    }

    LabelValidationReport {
        accepted: findings.is_empty(),
        findings,
    }
}

impl CoverageReport {
    /// Compact human-readable summary suitable for CLI output or test logs.
    #[must_use]
    pub fn pretty_summary(&self) -> String {
        format!(
            "coverage table `{}`\n  used_bits: {}\n  total_states: {}\n  normal_states: {}\n  covered_states: {} ({:.2}%)\n  single_match_states: {}\n  unmatched_states: {}\n  overlap_states: {}\n  reserved_states: {}\n  invalid_states: {}\n  policy_rule_conflict_states: {}\n  accepted: {}",
            self.table,
            self.used_bits,
            self.total_states,
            self.normal_states,
            self.covered_states,
            self.normal_coverage_percent(),
            self.single_match_states,
            self.unmatched_states,
            self.overlap_states,
            self.reserved_states,
            self.invalid_states,
            self.policy_rule_conflict_states,
            self.accepted(),
        )
    }

    /// Markdown report for documentation, audits, or builder diagnostics.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Bitpack coverage report: `{}`\n\n", self.table));
        out.push_str("| Metric | Value |\n|---|---:|\n");
        out.push_str(&format!("| Used bits | {} |\n", self.used_bits));
        out.push_str(&format!("| Total states | {} |\n", self.total_states));
        out.push_str(&format!("| Normal states | {} |\n", self.normal_states));
        out.push_str(&format!("| Covered states | {} |\n", self.covered_states));
        out.push_str(&format!("| Coverage | {:.2}% |\n", self.normal_coverage_percent()));
        out.push_str(&format!("| Single-match states | {} |\n", self.single_match_states));
        out.push_str(&format!("| Unmatched states | {} |\n", self.unmatched_states));
        out.push_str(&format!("| Overlap states | {} |\n", self.overlap_states));
        out.push_str(&format!("| Reserved states | {} |\n", self.reserved_states));
        out.push_str(&format!("| Invalid states | {} |\n", self.invalid_states));
        out.push_str(&format!(
            "| Policy/rule conflict states | {} |\n",
            self.policy_rule_conflict_states
        ));
        out.push_str(&format!("| Accepted | {} |\n", self.accepted()));

        append_markdown_examples(&mut out, "Unmatched examples", &self.unmatched_examples);
        append_markdown_examples(&mut out, "Overlap examples", &self.overlap_examples);
        append_markdown_examples(&mut out, "Reserved examples", &self.reserved_examples);
        append_markdown_examples(&mut out, "Invalid examples", &self.invalid_examples);
        append_markdown_examples(
            &mut out,
            "Policy/rule conflict examples",
            &self.policy_rule_conflict_examples,
        );

        out
    }

    /// Dependency-free JSON export for builder UIs and external tooling.
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                "{{",
                "\"table\":\"{}\",",
                "\"used_bits\":{},",
                "\"total_states\":{},",
                "\"normal_states\":{},",
                "\"covered_states\":{},",
                "\"single_match_states\":{},",
                "\"unmatched_states\":{},",
                "\"overlap_states\":{},",
                "\"reserved_states\":{},",
                "\"invalid_states\":{},",
                "\"policy_rule_conflict_states\":{},",
                "\"normal_coverage_percent\":{:.6},",
                "\"accepted\":{},",
                "\"unmatched_examples\":{},",
                "\"overlap_examples\":{},",
                "\"reserved_examples\":{},",
                "\"invalid_examples\":{},",
                "\"policy_rule_conflict_examples\":{}",
                "}}"
            ),
            escape_json(self.table),
            self.used_bits,
            self.total_states,
            self.normal_states,
            self.covered_states,
            self.single_match_states,
            self.unmatched_states,
            self.overlap_states,
            self.reserved_states,
            self.invalid_states,
            self.policy_rule_conflict_states,
            self.normal_coverage_percent(),
            self.accepted(),
            examples_json(&self.unmatched_examples),
            examples_json(&self.overlap_examples),
            examples_json(&self.reserved_examples),
            examples_json(&self.invalid_examples),
            examples_json(&self.policy_rule_conflict_examples),
        )
    }
}

/// One precomputed dense dispatch-table cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenseDispatchEntry {
    /// No rule matched this normal state.
    Unmatched,
    /// Exactly one normal rule matched this state.
    Matched {
        rule_name: &'static str,
        action: Action,
    },
    /// Multiple rules matched this normal state.
    Overlap {
        rule_names: Vec<&'static str>,
        actions: Vec<Action>,
    },
    /// The state is intentionally reserved and no rule matched it.
    Reserved {
        policy_names: Vec<&'static str>,
    },
    /// The state is forbidden by policy and no rule matched it.
    Invalid {
        policy_names: Vec<&'static str>,
    },
    /// A rule matched a state that policy marked reserved or invalid.
    PolicyRuleConflict {
        policy_names: Vec<&'static str>,
        rule_names: Vec<&'static str>,
        actions: Vec<Action>,
    },
}

impl DenseDispatchEntry {
    #[must_use]
    pub fn accepted(&self) -> bool {
        matches!(self, Self::Matched { .. } | Self::Reserved { .. } | Self::Invalid { .. })
    }

    #[must_use]
    pub fn is_matched(&self) -> bool {
        matches!(self, Self::Matched { .. })
    }
}

/// Precomputed dispatch table for small packed state spaces.
///
/// For `u8` and full `u16` layouts this gives a direct raw-value lookup. The
/// table is intentionally built through the same validation and coverage path as
/// the report engine, so runtime dispatch cannot disagree with coverage output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenseDispatchTable<T: PackedWord> {
    name: &'static str,
    entries: Vec<DenseDispatchEntry>,
    coverage: CoverageReport,
    labels: LabelRegistry,
    _marker: core::marker::PhantomData<T>,
}

impl<T: PackedWord> DenseDispatchTable<T> {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn coverage(&self) -> &CoverageReport {
        &self.coverage
    }

    #[must_use]
    pub fn labels(&self) -> LabelRegistry {
        self.labels
    }

    /// Returns the precomputed dispatch cell for a packed word.
    #[must_use]
    pub fn dispatch(&self, word: T) -> Option<&DenseDispatchEntry> {
        let raw = word.raw_u128();
        let index = usize::try_from(raw).ok()?;
        self.entries.get(index)
    }

    /// Returns the precomputed dispatch cell for a raw value if it is valid for `T`.
    #[must_use]
    pub fn dispatch_raw(&self, raw: u128) -> Option<&DenseDispatchEntry> {
        let word = T::try_from_raw_u128(raw)?;
        self.dispatch(word)
    }
}

/// Builder for dense dispatch tables.
#[derive(Debug, Clone, Copy)]
pub struct DispatchTableBuilder<T: PackedWord> {
    name: &'static str,
    rules: &'static [Rule],
    policy: CoveragePolicy,
    labels: LabelRegistry,
    options: CoverageOptions,
    allow_unmatched: bool,
    allow_overlaps: bool,
    allow_policy_rule_conflicts: bool,
    _marker: core::marker::PhantomData<T>,
}

impl<T: PackedWord> DispatchTableBuilder<T> {
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            rules: &[],
            policy: CoveragePolicy::empty(),
            labels: LabelRegistry::empty(),
            options: CoverageOptions {
                max_states: 1u128 << 20,
                max_examples: 16,
            },
            allow_unmatched: false,
            allow_overlaps: false,
            allow_policy_rule_conflicts: false,
            _marker: core::marker::PhantomData,
        }
    }

    #[must_use]
    pub const fn rules(mut self, rules: &'static [Rule]) -> Self {
        self.rules = rules;
        self
    }

    #[must_use]
    pub const fn policy(mut self, policy: CoveragePolicy) -> Self {
        self.policy = policy;
        self
    }

    #[must_use]
    pub const fn policy_states(mut self, states: &'static [StatePolicy]) -> Self {
        self.policy = CoveragePolicy::new(states);
        self
    }

    #[must_use]
    pub const fn labels(mut self, labels: LabelRegistry) -> Self {
        self.labels = labels;
        self
    }

    #[must_use]
    pub const fn label_values(mut self, labels: &'static [ValueLabel]) -> Self {
        self.labels = LabelRegistry::new(labels);
        self
    }

    #[must_use]
    pub const fn options(mut self, options: CoverageOptions) -> Self {
        self.options = options;
        self
    }

    #[must_use]
    pub const fn allow_unmatched(mut self, allow: bool) -> Self {
        self.allow_unmatched = allow;
        self
    }

    #[must_use]
    pub const fn allow_overlaps(mut self, allow: bool) -> Self {
        self.allow_overlaps = allow;
        self
    }

    #[must_use]
    pub const fn allow_policy_rule_conflicts(mut self, allow: bool) -> Self {
        self.allow_policy_rule_conflicts = allow;
        self
    }

    pub fn build_dense(self) -> Result<DenseDispatchTable<T>, DispatchBuildError> {
        let label_report = self.labels.validate_for::<T>();
        if !label_report.accepted {
            return Err(DispatchBuildError::InvalidLabels { report: label_report });
        }

        let table = RuleTable::new(self.name, self.rules);
        let coverage = table.coverage_for_with_options::<T>(self.policy, self.options)?;
        let rejection = DispatchRejection::from_report(
            &coverage,
            self.allow_unmatched,
            self.allow_overlaps,
            self.allow_policy_rule_conflicts,
        );

        if !rejection.accepted() {
            return Err(DispatchBuildError::CoverageRejected { coverage, rejection });
        }

        let total_states = total_states_for_used_bits(T::USED_BITS)?;
        if total_states > usize::MAX as u128 {
            return Err(DispatchBuildError::TooManyStatesForPlatform {
                states: total_states,
                usize_max: usize::MAX as u128,
            });
        }

        let mut entries = Vec::with_capacity(total_states as usize);
        let mut raw = 0u128;
        while raw < total_states {
            let word = T::from_raw_u128_truncating(raw);
            entries.push(make_dense_entry::<T>(table, self.policy, word));
            raw += 1;
        }

        Ok(DenseDispatchTable {
            name: self.name,
            entries,
            coverage,
            labels: self.labels,
            _marker: core::marker::PhantomData,
        })
    }
}

/// Reasons a dense dispatch table refused to build after coverage finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRejection {
    pub unmatched_states: u128,
    pub overlap_states: u128,
    pub policy_rule_conflict_states: u128,
}

impl DispatchRejection {
    #[must_use]
    pub const fn accepted(&self) -> bool {
        self.unmatched_states == 0
            && self.overlap_states == 0
            && self.policy_rule_conflict_states == 0
    }

    #[must_use]
    pub fn from_report(
        report: &CoverageReport,
        allow_unmatched: bool,
        allow_overlaps: bool,
        allow_policy_rule_conflicts: bool,
    ) -> Self {
        Self {
            unmatched_states: if allow_unmatched {
                0
            } else {
                report.unmatched_states
            },
            overlap_states: if allow_overlaps {
                0
            } else {
                report.overlap_states
            },
            policy_rule_conflict_states: if allow_policy_rule_conflicts {
                0
            } else {
                report.policy_rule_conflict_states
            },
        }
    }
}

/// Failure to build a dense dispatch table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchBuildError {
    CoverageScan(CoverageScanError),
    InvalidLabels {
        report: LabelValidationReport,
    },
    CoverageRejected {
        coverage: CoverageReport,
        rejection: DispatchRejection,
    },
    TooManyStatesForPlatform {
        states: u128,
        usize_max: u128,
    },
}

impl From<CoverageScanError> for DispatchBuildError {
    fn from(value: CoverageScanError) -> Self {
        Self::CoverageScan(value)
    }
}

impl core::fmt::Display for DispatchBuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CoverageScan(error) => write!(f, "coverage scan failed: {error}"),
            Self::InvalidLabels { report } => write!(
                f,
                "label registry failed validation: {:?}",
                report.findings
            ),
            Self::CoverageRejected { rejection, .. } => write!(
                f,
                "dispatch coverage rejected: unmatched={}, overlaps={}, policy_rule_conflicts={}",
                rejection.unmatched_states,
                rejection.overlap_states,
                rejection.policy_rule_conflict_states
            ),
            Self::TooManyStatesForPlatform { states, usize_max } => write!(
                f,
                "dense dispatch table has {states} states, exceeding platform usize max {usize_max}"
            ),
        }
    }
}

impl std::error::Error for DispatchBuildError {}

fn make_dense_entry<T: PackedWord>(
    table: RuleTable,
    policy: CoveragePolicy,
    word: T,
) -> DenseDispatchEntry {
    let matched_rules = matching_rules::<T>(table, word);
    let policy_hits = matching_policy_hits::<T>(policy, word);
    let policy_names = policy_hits.iter().map(|p| p.name).collect::<Vec<_>>();
    let has_invalid_policy = policy_hits
        .iter()
        .any(|p| p.kind == StatePolicyKind::Invalid);
    let has_reserved_policy = policy_hits
        .iter()
        .any(|p| p.kind == StatePolicyKind::Reserved);

    let rule_names = matched_rules.iter().map(|rule| rule.name).collect::<Vec<_>>();
    let actions = matched_rules
        .iter()
        .map(|rule| rule.action.clone())
        .collect::<Vec<_>>();

    if has_invalid_policy || has_reserved_policy {
        if matched_rules.is_empty() {
            if has_invalid_policy {
                DenseDispatchEntry::Invalid { policy_names }
            } else {
                DenseDispatchEntry::Reserved { policy_names }
            }
        } else {
            DenseDispatchEntry::PolicyRuleConflict {
                policy_names,
                rule_names,
                actions,
            }
        }
    } else {
        match matched_rules.as_slice() {
            [] => DenseDispatchEntry::Unmatched,
            [rule] => DenseDispatchEntry::Matched {
                rule_name: rule.name,
                action: rule.action.clone(),
            },
            _ => DenseDispatchEntry::Overlap { rule_names, actions },
        }
    }
}

fn matching_rules<T: PackedWord>(table: RuleTable, word: T) -> Vec<&'static Rule> {
    table.rules.iter().filter(|rule| rule.matches(word)).collect()
}

fn append_markdown_examples(out: &mut String, title: &str, examples: &[CoverageExample]) {
    if examples.is_empty() {
        return;
    }

    out.push_str(&format!("\n## {title}\n\n"));
    out.push_str("| Raw | Fields | Rules | Policies |\n|---:|---|---|---|\n");
    for example in examples {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            example.raw,
            markdown_field_values(&example.fields),
            markdown_names(&example.matched_rule_names),
            markdown_names(&example.policy_names),
        ));
    }
}

fn markdown_field_values(fields: &[CoverageFieldValue]) -> String {
    fields
        .iter()
        .map(|field| format!("`{}={}`", field.field, field.value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn markdown_names(names: &[&'static str]) -> String {
    if names.is_empty() {
        "—".to_owned()
    } else {
        names
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn examples_json(examples: &[CoverageExample]) -> String {
    let body = examples
        .iter()
        .map(example_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

fn example_json(example: &CoverageExample) -> String {
    format!(
        "{{\"raw\":{},\"fields\":{},\"matched_rule_names\":{},\"policy_names\":{}}}",
        example.raw,
        field_values_json(&example.fields),
        names_json(&example.matched_rule_names),
        names_json(&example.policy_names),
    )
}

fn field_values_json(fields: &[CoverageFieldValue]) -> String {
    let body = fields
        .iter()
        .map(|field| {
            format!(
                "{{\"field\":\"{}\",\"value\":{}}}",
                escape_json(field.field),
                field.value
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

fn names_json(names: &[&'static str]) -> String {
    let body = names
        .iter()
        .map(|name| format!("\"{}\"", escape_json(name)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

fn escape_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}
