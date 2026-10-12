//! Test schemas and exhaustive fixtures for the bitpack workspace.

use big_bad_bool::bitpack;

pub mod real_schemas;

#[bitpack(storage = u8)]
pub struct ByteCommand {
    #[bits(3)]
    index: u8,

    #[bits(5)]
    item: u8,
}

#[bitpack(storage = u8)]
pub struct PartialByte {
    #[bits(3)]
    low: u8,
}

#[bitpack(storage = u16)]
pub struct PropTag16 {
    #[bits(6)]
    #[labels(
        component = 1,
        layout = 2,
        animation = 3,
        permission = 4,
        reserved = 63,
    )]
    selector: u16,

    #[bits(10)]
    #[labels(
        none = 0,
        normal_max = 959,
        invalid_start = 960,
        max = 1023,
    )]
    prop_hints: u16,
}

#[bitpack(storage = u16)]
pub struct InteractionTag16 {
    #[bits(4)]
    family: u16,

    #[bits(4)]
    op: u16,

    #[bits(8)]
    flags: u16,
}

#[bitpack(storage = u32)]
pub struct WideCommand32 {
    #[bits(5)]
    source: u32,

    #[bits(11)]
    item: u32,

    #[bits(8)]
    rule: u32,

    #[bits(8)]
    flags: u32,
}


#[bitpack(storage = u32, version = 7)]
pub struct ComplexContract32 {
    #[bits(5)]
    #[field_id(10)]
    #[describe("Top-level contract domain for a reducer/runtime operation.")]
    #[labels(
        reserved = 0,
        builder = 1,
        runtime = 2,
        old_domain = 3,
        invalid = 31,
    )]
    #[aliases(builder = ["ui_builder", "visual_builder"])]
    #[reserved(0, 28..=30)]
    #[invalid(31)]
    #[default(1)]
    #[deprecated(old_domain, since = "0.1.0", note = "Use builder or runtime instead.")]
    #[dispatch(key)]
    domain: u32,

    #[bits(6)]
    #[field_id(11)]
    #[describe("Nested state path family.")]
    #[labels(root = 0, node_props = 5, reserved_start = 56, invalid = 63)]
    #[reserved(56..=62)]
    #[invalid(63)]
    #[default(0)]
    #[dispatch(argument)]
    path_kind: u32,

    #[bits(6)]
    #[field_id(12)]
    #[describe("Reducer operation requested by the packed contract.")]
    #[labels(noop = 0, patch = 2, move_op = 7, invalid = 63)]
    #[requires(patch => policy.validate)]
    #[invalid(63)]
    #[dispatch(phase)]
    operation: u32,

    #[bits(5)]
    #[field_id(13)]
    #[describe("Policy gate applied before execution.")]
    #[labels(allow = 0, validate = 1, quarantine = 4, invalid = 31)]
    #[invalid(31)]
    #[dispatch(policy)]
    policy: u32,

    #[bits(5)]
    #[field_id(14)]
    #[describe("Composable execution flags.")]
    #[labels(none = 0, undoable = 1, audited = 2, optimistic = 4, replicated = 8)]
    #[flags(undoable = 0, audited = 1, optimistic = 2, replicated = 3)]
    #[conflicts(optimistic, replicated)]
    #[dispatch(flags)]
    flags: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use big_bad_bool_core::{BitpackErrorKind, PackedWord, ValueLabel};
    use big_bad_bool_interact::{
        negotiate_schema, Action, FieldMatcher, FieldRequirement, MatchOp, Rule, RuleTable,
        RuleValidationFinding, LabelRegistry,
    };

    #[test]
    fn byte_command_exhaustively_decodes_all_256_raw_variants() {
        for raw in u8::MIN..=u8::MAX {
            let packed = ByteCommand::from_raw(raw);
            let expected_index = raw & 0b0000_0111;
            let expected_item = (raw >> 3) & 0b0001_1111;

            assert_eq!(packed.raw(), raw);
            assert_eq!(packed.index(), expected_index);
            assert_eq!(packed.item(), expected_item);
            assert_eq!(packed.read_field("index"), Some(u128::from(expected_index)));
            assert_eq!(packed.read_field("item"), Some(u128::from(expected_item)));
            assert_eq!(ByteCommand::try_from_raw(raw), Some(packed));
            assert_eq!(ByteCommand::try_from_raw_u128(u128::from(raw)), Some(packed));
        }
    }

    #[test]
    fn byte_command_exhaustively_encodes_all_8_by_32_pairs() {
        for index in 0u8..8 {
            for item in 0u8..32 {
                let packed = ByteCommand::new(index, item);
                let expected = index | (item << 3);

                assert_eq!(packed.raw(), expected);
                assert_eq!(packed.index(), index);
                assert_eq!(packed.item(), item);
            }
        }
    }

    #[test]
    fn byte_command_setters_rewrite_only_their_own_bits() {
        for raw in u8::MIN..=u8::MAX {
            for index in 0u8..8 {
                let mut packed = ByteCommand::from_raw(raw);
                let old_item = packed.item();

                packed.set_index(index);

                assert_eq!(packed.index(), index);
                assert_eq!(packed.item(), old_item);
            }

            for item in 0u8..32 {
                let mut packed = ByteCommand::from_raw(raw);
                let old_index = packed.index();

                packed.set_item(item);

                assert_eq!(packed.index(), old_index);
                assert_eq!(packed.item(), item);
            }
        }
    }

    #[test]
    fn byte_command_try_setters_reject_overflow_values() {
        let mut packed = ByteCommand::ZERO;

        assert!(packed.try_set_index(7));
        assert!(!packed.try_set_index(8));
        assert_eq!(packed.index(), 7);

        assert!(packed.try_set_item(31));
        assert!(!packed.try_set_item(32));
        assert_eq!(packed.item(), 31);
    }

    #[test]
    fn prop_tag_exhaustively_decodes_all_65536_raw_variants() {
        for raw in u16::MIN..=u16::MAX {
            let packed = PropTag16::from_raw(raw);
            let expected_selector = raw & 0x003f;
            let expected_prop_hints = (raw >> 6) & 0x03ff;

            assert_eq!(packed.raw(), raw);
            assert_eq!(packed.selector(), expected_selector);
            assert_eq!(packed.prop_hints(), expected_prop_hints);
            assert_eq!(packed.read_field("selector"), Some(u128::from(expected_selector)));
            assert_eq!(packed.read_field("prop_hints"), Some(u128::from(expected_prop_hints)));
            assert_eq!(PropTag16::try_from_raw(raw), Some(packed));
            assert_eq!(PropTag16::try_from_raw_u128(u128::from(raw)), Some(packed));
        }
    }

    #[test]
    fn prop_tag_exhaustively_encodes_all_64_by_1024_pairs() {
        for selector in 0u16..64 {
            for prop_hints in 0u16..1024 {
                let packed = PropTag16::new(selector, prop_hints);
                let expected = selector | (prop_hints << 6);

                assert_eq!(packed.raw(), expected);
                assert_eq!(packed.selector(), selector);
                assert_eq!(packed.prop_hints(), prop_hints);
            }
        }
    }

    #[test]
    fn prop_tag_setters_rewrite_only_their_own_bits() {
        for raw in u16::MIN..=u16::MAX {
            let mut selector_rewrite = PropTag16::from_raw(raw);
            let old_prop_hints = selector_rewrite.prop_hints();
            selector_rewrite.set_selector(41);
            assert_eq!(selector_rewrite.selector(), 41);
            assert_eq!(selector_rewrite.prop_hints(), old_prop_hints);

            let mut hints_rewrite = PropTag16::from_raw(raw);
            let old_selector = hints_rewrite.selector();
            hints_rewrite.set_prop_hints(777);
            assert_eq!(hints_rewrite.selector(), old_selector);
            assert_eq!(hints_rewrite.prop_hints(), 777);
        }
    }

    #[test]
    fn prop_tag_try_setters_reject_overflow_values() {
        let mut packed = PropTag16::ZERO;

        assert!(packed.try_set_selector(63));
        assert!(!packed.try_set_selector(64));
        assert_eq!(packed.selector(), 63);

        assert!(packed.try_set_prop_hints(1023));
        assert!(!packed.try_set_prop_hints(1024));
        assert_eq!(packed.prop_hints(), 1023);
    }

    #[test]
    fn checked_constructors_reject_field_overflow_without_truncating() {
        assert_eq!(PropTag16::try_new(63, 1023), Some(PropTag16::new(63, 1023)));
        assert_eq!(PropTag16::try_new(64, 0), None);
        assert_eq!(PropTag16::try_new(0, 1024), None);

        let selector_error = PropTag16::new_checked(64, 0).unwrap_err();
        assert_eq!(
            selector_error.kind,
            BitpackErrorKind::FieldOverflow {
                field: "selector",
                value: 64,
                max: 63,
            }
        );

        let hint_error = PropTag16::new_checked(0, 1024).unwrap_err();
        assert_eq!(
            hint_error.kind,
            BitpackErrorKind::FieldOverflow {
                field: "prop_hints",
                value: 1024,
                max: 1023,
            }
        );
    }

    #[test]
    fn checked_raw_constructors_report_storage_and_layout_errors() {
        assert_eq!(
            ByteCommand::try_from_raw_u128_checked(255),
            Ok(ByteCommand::from_raw(255))
        );

        let storage_error = ByteCommand::try_from_raw_u128_checked(256).unwrap_err();
        assert_eq!(
            storage_error.kind,
            BitpackErrorKind::RawOverflow { raw: 256, max: 255 }
        );

        assert_eq!(PartialByte::USED_BITS, 3);
        assert_eq!(PartialByte::try_from_raw_checked(0b0000_0111), Ok(PartialByte::from_raw(7)));

        let layout_error = PartialByte::try_from_raw_checked(0b0000_1000).unwrap_err();
        assert_eq!(
            layout_error.kind,
            BitpackErrorKind::RawLayoutViolation {
                raw: 0b0000_1000,
                mask: 0b0000_0111,
            }
        );
    }

    #[test]
    fn interaction_tag_exhaustively_decodes_all_u16_variants() {
        for raw in u16::MIN..=u16::MAX {
            let packed = InteractionTag16::from_raw(raw);
            let expected_family = raw & 0x000f;
            let expected_op = (raw >> 4) & 0x000f;
            let expected_flags = (raw >> 8) & 0x00ff;

            assert_eq!(packed.raw(), raw);
            assert_eq!(packed.family(), expected_family);
            assert_eq!(packed.op(), expected_op);
            assert_eq!(packed.flags(), expected_flags);
        }
    }

    #[test]
    fn interaction_tag_exhaustively_encodes_all_field_combinations() {
        for family in 0u16..16 {
            for op in 0u16..16 {
                for flags in 0u16..=255 {
                    let packed = InteractionTag16::new(family, op, flags);
                    let expected = family | (op << 4) | (flags << 8);

                    assert_eq!(packed.raw(), expected);
                    assert_eq!(packed.family(), family);
                    assert_eq!(packed.op(), op);
                    assert_eq!(packed.flags(), flags);
                }
            }
        }
    }

    #[test]
    fn schema_metadata_is_generated_from_the_declaration() {
        assert_eq!(PropTag16::STORAGE_BITS, 16);
        assert_eq!(PropTag16::USED_BITS, 16);
        assert_eq!(PropTag16::RAW_MASK, u16::MAX);

        assert_eq!(PropTag16::FIELDS.len(), 2);
        assert_eq!(PropTag16::FIELD_SELECTOR, "selector");
        assert_eq!(PropTag16::FIELD_PROP_HINTS, "prop_hints");
        assert_eq!(PropTag16::FIELD_SELECTOR_SPEC, PropTag16::FIELDS[0]);
        assert_eq!(PropTag16::FIELD_PROP_HINTS_SPEC, PropTag16::FIELDS[1]);
        assert_eq!(PropTag16::FIELDS[0].name, "selector");
        assert_eq!(PropTag16::FIELDS[0].shift, 0);
        assert_eq!(PropTag16::FIELDS[0].width, 6);
        assert_eq!(PropTag16::FIELDS[0].value_mask, 63);
        assert_eq!(PropTag16::FIELDS[1].name, "prop_hints");
        assert_eq!(PropTag16::FIELDS[1].shift, 6);
        assert_eq!(PropTag16::FIELDS[1].width, 10);
        assert_eq!(PropTag16::FIELDS[1].value_mask, 1023);
    }

    #[test]
    fn schema_labels_are_generated_from_field_attributes() {
        assert!(ByteCommand::LABELS.is_empty());
        assert_eq!(PropTag16::LABELS.len(), 9);

        assert_eq!(
            PropTag16::LABEL_SELECTOR_COMPONENT,
            ValueLabel::named(PropTag16::FIELD_SELECTOR, 1, "component")
        );
        assert_eq!(
            PropTag16::LABEL_SELECTOR_LAYOUT,
            ValueLabel::named(PropTag16::FIELD_SELECTOR, 2, "layout")
        );
        assert_eq!(
            PropTag16::LABEL_SELECTOR_ANIMATION,
            ValueLabel::named(PropTag16::FIELD_SELECTOR, 3, "animation")
        );
        assert_eq!(
            PropTag16::LABEL_SELECTOR_PERMISSION,
            ValueLabel::named(PropTag16::FIELD_SELECTOR, 4, "permission")
        );
        assert_eq!(
            PropTag16::LABEL_SELECTOR_RESERVED,
            ValueLabel::named(PropTag16::FIELD_SELECTOR, 63, "reserved")
        );
        assert_eq!(
            PropTag16::LABEL_PROP_HINTS_NONE,
            ValueLabel::named(PropTag16::FIELD_PROP_HINTS, 0, "none")
        );
        assert_eq!(
            PropTag16::LABEL_PROP_HINTS_NORMAL_MAX,
            ValueLabel::named(PropTag16::FIELD_PROP_HINTS, 959, "normal_max")
        );
        assert_eq!(
            PropTag16::LABEL_PROP_HINTS_INVALID_START,
            ValueLabel::named(PropTag16::FIELD_PROP_HINTS, 960, "invalid_start")
        );
        assert_eq!(
            PropTag16::LABEL_PROP_HINTS_MAX,
            ValueLabel::named(PropTag16::FIELD_PROP_HINTS, 1023, "max")
        );

        let registry = LabelRegistry::new(PropTag16::LABELS);
        assert!(registry.validate_for::<PropTag16>().accepted);
        assert_eq!(
            registry.label_for(PropTag16::FIELD_SELECTOR, 1),
            Some(PropTag16::LABEL_SELECTOR_COMPONENT)
        );
        assert_eq!(
            registry.label_for(PropTag16::FIELD_PROP_HINTS, 960),
            Some(PropTag16::LABEL_PROP_HINTS_INVALID_START)
        );
        assert_eq!(registry.label_for(PropTag16::FIELD_SELECTOR, 62), None);
    }

    #[test]
    fn strict_raw_constructor_rejects_bits_outside_used_layout() {
        assert_eq!(ByteCommand::USED_BITS, 8);
        assert_eq!(ByteCommand::try_from_raw_u128(255), Some(ByteCommand::from_raw(255)));
        assert_eq!(ByteCommand::try_from_raw_u128(256), None);

        assert_eq!(PropTag16::USED_BITS, 16);
        assert_eq!(PropTag16::try_from_raw_u128(65_535), Some(PropTag16::from_raw(65_535)));
        assert_eq!(PropTag16::try_from_raw_u128(65_536), None);
    }

    #[test]
    fn high_level_rule_table_maps_read_values_to_actions() {
        static MATCH_SELECTOR_7: &[FieldMatcher] = &[
            FieldMatcher::eq("selector", 7),
            FieldMatcher::range("prop_hints", 100, 199),
        ];

        static MATCH_HINT_FLAG: &[FieldMatcher] = &[
            FieldMatcher::any("selector"),
            FieldMatcher {
                field: "prop_hints",
                op: MatchOp::MaskAny { mask: 0b1000 },
            },
        ];

        static RULES: &[Rule] = &[
            Rule::new(
                "selector-7-mid-hint",
                MATCH_SELECTOR_7,
                Action::new(10, "route-selector-7", "Route selector 7 with mid-range hints"),
            ),
            Rule::new(
                "any-selector-has-hint-flag-3",
                MATCH_HINT_FLAG,
                Action::new(20, "hint-flag-3", "Prop hint flag bit 3 is present"),
            ),
        ];

        let table = RuleTable::new("prop-tag-rules", RULES);
        let tag = PropTag16::new(7, 136);
        let report = table.dispatch(tag);

        assert_eq!(report.raw, u128::from(tag.raw()));
        assert_eq!(report.matched_rule_names, vec![
            "selector-7-mid-hint",
            "any-selector-has-hint-flag-3"
        ]);
        assert_eq!(report.matched_actions[0].name, "route-selector-7");
        assert_eq!(report.matched_actions[1].name, "hint-flag-3");
    }

    #[test]
    fn rule_table_validation_rejects_unknown_fields_and_impossible_values() {
        static VALID_MATCHERS: &[FieldMatcher] = &[
            FieldMatcher::eq(PropTag16::FIELD_SELECTOR, 7),
            FieldMatcher::range(PropTag16::FIELD_PROP_HINTS, 100, 199),
        ];
        static VALID_RULES: &[Rule] = &[Rule::new(
            "valid",
            VALID_MATCHERS,
            Action::new(1, "valid", "Valid selector/hint route"),
        )];

        let valid_report = RuleTable::new("valid-rules", VALID_RULES).validate_for::<PropTag16>();
        assert!(valid_report.accepted);
        assert!(valid_report.findings.is_empty());

        static UNKNOWN_FIELD: &[FieldMatcher] = &[FieldMatcher::eq("missing", 1)];
        static OVERFLOW_VALUE: &[FieldMatcher] = &[FieldMatcher::eq(PropTag16::FIELD_SELECTOR, 64)];
        static INVALID_RANGE: &[FieldMatcher] = &[FieldMatcher::range(PropTag16::FIELD_PROP_HINTS, 200, 100)];
        static MASK_OVERFLOW: &[FieldMatcher] = &[FieldMatcher::mask_any(PropTag16::FIELD_SELECTOR, 0b1_000000)];
        static BAD_RULES: &[Rule] = &[
            Rule::new("empty", &[], Action::new(10, "empty", "Empty rule")),
            Rule::new("unknown", UNKNOWN_FIELD, Action::new(11, "unknown", "Unknown field")),
            Rule::new("overflow", OVERFLOW_VALUE, Action::new(12, "overflow", "Overflow value")),
            Rule::new("invalid-range", INVALID_RANGE, Action::new(13, "invalid-range", "Invalid range")),
            Rule::new("mask-overflow", MASK_OVERFLOW, Action::new(14, "mask-overflow", "Mask overflow")),
        ];

        let report = RuleTable::new("bad-rules", BAD_RULES).validate_for::<PropTag16>();
        assert!(!report.accepted);
        assert_eq!(
            report.findings,
            vec![
                RuleValidationFinding::EmptyRule { rule: "empty" },
                RuleValidationFinding::UnknownField { rule: "unknown", field: "missing" },
                RuleValidationFinding::ValueOutOfRange {
                    rule: "overflow",
                    field: "selector",
                    value: 64,
                    max: 63,
                },
                RuleValidationFinding::InvalidRange {
                    rule: "invalid-range",
                    field: "prop_hints",
                    min: 200,
                    max: 100,
                },
                RuleValidationFinding::MaskOutOfRange {
                    rule: "mask-overflow",
                    field: "selector",
                    mask: 64,
                    field_max: 63,
                },
            ]
        );
    }

    #[test]
    fn schema_negotiation_accepts_required_fields_and_rejects_missing_or_narrow_fields() {
        let accepted = negotiate_schema::<PropTag16>(&[
            FieldRequirement::required("selector", 6),
            FieldRequirement::required("prop_hints", 10),
            FieldRequirement::optional("flags", 4),
        ]);

        assert!(accepted.accepted);
        assert_eq!(accepted.findings.len(), 3);

        let rejected_missing = negotiate_schema::<PropTag16>(&[
            FieldRequirement::required("selector", 6),
            FieldRequirement::required("flags", 1),
        ]);

        assert!(!rejected_missing.accepted);

        let rejected_narrow = negotiate_schema::<PropTag16>(&[
            FieldRequirement::required("selector", 7),
        ]);

        assert!(!rejected_narrow.accepted);
    }

    #[test]
    fn wide_command_uses_partial_u32_layout_and_masks_unused_bits() {
        assert_eq!(WideCommand32::USED_BITS, 32);

        let packed = WideCommand32::new(31, 2047, 255, 255);
        assert_eq!(packed.source(), 31);
        assert_eq!(packed.item(), 2047);
        assert_eq!(packed.rule(), 255);
        assert_eq!(packed.flags(), 255);
        assert_eq!(packed.raw(), u32::MAX);
    }
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    use big_bad_bool_interact::{
        Action, CoverageOptions, CoveragePolicy, CoverageScanError, FieldMatcher, Rule,
        RuleTable, StatePolicy,
    };

    static COVER_NORMAL_INDEXES: &[FieldMatcher] = &[
        FieldMatcher::range(ByteCommand::FIELD_INDEX, 0, 5),
        FieldMatcher::any(ByteCommand::FIELD_ITEM),
    ];

    static COVERAGE_RULES: &[Rule] = &[Rule::new(
        "normal-index-0-through-5",
        COVER_NORMAL_INDEXES,
        Action::new(100, "normal-route", "Route normal byte-command states"),
    )];

    static INVALID_INDEX_6: &[FieldMatcher] = &[FieldMatcher::eq(ByteCommand::FIELD_INDEX, 6)];
    static RESERVED_INDEX_7: &[FieldMatcher] = &[FieldMatcher::eq(ByteCommand::FIELD_INDEX, 7)];

    static BYTE_POLICIES: &[StatePolicy] = &[
        StatePolicy::invalid(
            "index-6-invalid",
            INVALID_INDEX_6,
            "Index 6 is representable but forbidden by policy",
        ),
        StatePolicy::reserved(
            "index-7-reserved",
            RESERVED_INDEX_7,
            "Index 7 is held for future expansion",
        ),
    ];

    #[test]
    fn coverage_engine_counts_full_state_space_with_reserved_and_invalid_policy() {
        let table = RuleTable::new("byte-coverage", COVERAGE_RULES);
        let report = table
            .coverage_for_with_policy::<ByteCommand>(CoveragePolicy::new(BYTE_POLICIES))
            .expect("u8 coverage should be small enough to scan exhaustively");

        assert_eq!(report.table, "byte-coverage");
        assert_eq!(report.used_bits, 8);
        assert_eq!(report.total_states, 256);
        assert_eq!(report.normal_states, 192);
        assert_eq!(report.covered_states, 192);
        assert_eq!(report.single_match_states, 192);
        assert_eq!(report.unmatched_states, 0);
        assert_eq!(report.overlap_states, 0);
        assert_eq!(report.invalid_states, 32);
        assert_eq!(report.reserved_states, 32);
        assert_eq!(report.policy_rule_conflict_states, 0);
        assert!(report.accepted());
        assert!(report.fully_covered());
        assert_eq!(report.normal_coverage_percent(), 100.0);
        assert!(!report.invalid_examples.is_empty());
        assert!(!report.reserved_examples.is_empty());
    }

    static RULE_INDEX_0_TO_1: &[FieldMatcher] = &[
        FieldMatcher::range(ByteCommand::FIELD_INDEX, 0, 1),
        FieldMatcher::any(ByteCommand::FIELD_ITEM),
    ];

    static RULE_INDEX_1_TO_2: &[FieldMatcher] = &[
        FieldMatcher::range(ByteCommand::FIELD_INDEX, 1, 2),
        FieldMatcher::any(ByteCommand::FIELD_ITEM),
    ];

    static OVERLAPPING_RULES: &[Rule] = &[
        Rule::new(
            "index-0-through-1",
            RULE_INDEX_0_TO_1,
            Action::new(1, "route-a", "First overlapping range"),
        ),
        Rule::new(
            "index-1-through-2",
            RULE_INDEX_1_TO_2,
            Action::new(2, "route-b", "Second overlapping range"),
        ),
    ];

    #[test]
    fn coverage_engine_reports_unmatched_and_overlapping_states() {
        let table = RuleTable::new("overlap-demo", OVERLAPPING_RULES);
        let report = table
            .coverage_for::<ByteCommand>()
            .expect("u8 coverage should be small enough to scan exhaustively");

        assert_eq!(report.total_states, 256);
        assert_eq!(report.normal_states, 256);
        assert_eq!(report.covered_states, 96);
        assert_eq!(report.single_match_states, 64);
        assert_eq!(report.overlap_states, 32);
        assert_eq!(report.unmatched_states, 160);
        assert!(!report.accepted());
        assert!(!report.fully_covered());
        assert!(!report.unmatched_examples.is_empty());
        assert!(!report.overlap_examples.is_empty());

        let overlap = &report.overlap_examples[0];
        assert_eq!(overlap.matched_rule_names.len(), 2);
        assert!(overlap.matched_rule_names.contains(&"index-0-through-1"));
        assert!(overlap.matched_rule_names.contains(&"index-1-through-2"));
    }

    static CONFLICTING_POLICY_MATCHERS: &[FieldMatcher] = &[FieldMatcher::eq(ByteCommand::FIELD_INDEX, 0)];
    static CONFLICTING_POLICIES: &[StatePolicy] = &[StatePolicy::reserved(
        "index-0-reserved-but-routed",
        CONFLICTING_POLICY_MATCHERS,
        "This intentionally conflicts with a rule to prove conflict reporting",
    )];

    #[test]
    fn coverage_engine_reports_rules_that_accidentally_hit_reserved_or_invalid_states() {
        let table = RuleTable::new("policy-conflict-demo", COVERAGE_RULES);
        let report = table
            .coverage_for_with_policy::<ByteCommand>(CoveragePolicy::new(CONFLICTING_POLICIES))
            .expect("u8 coverage should be small enough to scan exhaustively");

        assert_eq!(report.reserved_states, 32);
        assert_eq!(report.policy_rule_conflict_states, 32);
        assert!(!report.accepted());
        assert!(!report.policy_rule_conflict_examples.is_empty());
        assert_eq!(
            report.policy_rule_conflict_examples[0].policy_names,
            vec!["index-0-reserved-but-routed"]
        );
        assert_eq!(
            report.policy_rule_conflict_examples[0].matched_rule_names,
            vec!["normal-index-0-through-5"]
        );
    }

    #[test]
    fn coverage_engine_refuses_accidentally_huge_exhaustive_scans() {
        static WIDE_MATCHERS: &[FieldMatcher] = &[FieldMatcher::any(WideCommand32::FIELD_SOURCE)];
        static WIDE_RULES: &[Rule] = &[Rule::new(
            "wide-any-source",
            WIDE_MATCHERS,
            Action::new(1, "wide", "Wide command rule"),
        )];

        let wide_table = RuleTable::new("wide-refusal", WIDE_RULES);
        let error = wide_table
            .coverage_for_with_options::<WideCommand32>(
                CoveragePolicy::empty(),
                CoverageOptions::new(256, 4),
            )
            .unwrap_err();

        assert_eq!(
            error,
            CoverageScanError::TooManyStates {
                used_bits: 32,
                states: 1u128 << 32,
                limit: 256,
            }
        );
    }
}

#[cfg(test)]
mod dispatch_table_tests {
    use super::*;
    use big_bad_bool_core::PackedWord;
    use big_bad_bool_interact::{
        Action, CoveragePolicy, DenseDispatchEntry, DispatchBuildError, DispatchTableBuilder,
        FieldMatcher, LabelRegistry, LabelValidationFinding, Rule, StatePolicy, ValueLabel,
    };

    static NORMAL_INDEXES: &[FieldMatcher] = &[
        FieldMatcher::range(ByteCommand::FIELD_INDEX, 0, 5),
        FieldMatcher::any(ByteCommand::FIELD_ITEM),
    ];

    static DISPATCH_RULES: &[Rule] = &[Rule::new(
        "normal-byte-index",
        NORMAL_INDEXES,
        Action::new(7, "normal-byte-route", "Dispatch normal byte command states"),
    )];

    static INVALID_INDEX_6: &[FieldMatcher] = &[FieldMatcher::eq(ByteCommand::FIELD_INDEX, 6)];
    static RESERVED_INDEX_7: &[FieldMatcher] = &[FieldMatcher::eq(ByteCommand::FIELD_INDEX, 7)];

    static DISPATCH_POLICIES: &[StatePolicy] = &[
        StatePolicy::invalid(
            "index-6-invalid",
            INVALID_INDEX_6,
            "Index 6 is forbidden by this protocol",
        ),
        StatePolicy::reserved(
            "index-7-reserved",
            RESERVED_INDEX_7,
            "Index 7 is reserved for a later protocol version",
        ),
    ];

    static BYTE_LABELS: &[ValueLabel] = &[
        ValueLabel::new(ByteCommand::FIELD_INDEX, 0, "primary", "Primary command family"),
        ValueLabel::new(ByteCommand::FIELD_INDEX, 6, "invalid", "Forbidden family"),
        ValueLabel::new(ByteCommand::FIELD_INDEX, 7, "reserved", "Reserved family"),
    ];

    #[test]
    fn dense_dispatch_table_precomputes_all_byte_states() {
        let table = DispatchTableBuilder::<ByteCommand>::new("byte-dispatch")
            .rules(DISPATCH_RULES)
            .policy(CoveragePolicy::new(DISPATCH_POLICIES))
            .labels(LabelRegistry::new(BYTE_LABELS))
            .build_dense()
            .expect("fully covered byte dispatch should build");

        assert_eq!(table.name(), "byte-dispatch");
        assert_eq!(table.len(), 256);
        assert!(table.coverage().accepted());
        assert_eq!(table.coverage().normal_states, 192);
        assert_eq!(table.coverage().invalid_states, 32);
        assert_eq!(table.coverage().reserved_states, 32);

        let matched = table
            .dispatch(ByteCommand::new(3, 17))
            .expect("raw state should have an entry");
        assert_eq!(
            matched,
            &DenseDispatchEntry::Matched {
                rule_name: "normal-byte-index",
                action: Action::new(7, "normal-byte-route", "Dispatch normal byte command states"),
            }
        );

        let invalid = table
            .dispatch(ByteCommand::new(6, 1))
            .expect("invalid state should still have a dispatch entry");
        assert_eq!(
            invalid,
            &DenseDispatchEntry::Invalid {
                policy_names: vec!["index-6-invalid"],
            }
        );

        let reserved = table
            .dispatch(ByteCommand::new(7, 1))
            .expect("reserved state should still have a dispatch entry");
        assert_eq!(
            reserved,
            &DenseDispatchEntry::Reserved {
                policy_names: vec!["index-7-reserved"],
            }
        );

        assert_eq!(
            table.labels().label_for(ByteCommand::FIELD_INDEX, 7).unwrap().label,
            "reserved"
        );
    }

    static ONLY_INDEX_ZERO: &[FieldMatcher] = &[
        FieldMatcher::eq(ByteCommand::FIELD_INDEX, 0),
        FieldMatcher::any(ByteCommand::FIELD_ITEM),
    ];

    static PARTIAL_RULES: &[Rule] = &[Rule::new(
        "only-index-zero",
        ONLY_INDEX_ZERO,
        Action::new(1, "only-zero", "Only dispatch index zero"),
    )];

    #[test]
    fn dense_dispatch_table_rejects_uncovered_normal_states_by_default() {
        let error = DispatchTableBuilder::<ByteCommand>::new("partial-dispatch")
            .rules(PARTIAL_RULES)
            .build_dense()
            .unwrap_err();

        match error {
            DispatchBuildError::CoverageRejected { rejection, coverage } => {
                assert_eq!(coverage.total_states, 256);
                assert_eq!(coverage.covered_states, 32);
                assert_eq!(coverage.unmatched_states, 224);
                assert_eq!(rejection.unmatched_states, 224);
                assert_eq!(rejection.overlap_states, 0);
                assert_eq!(rejection.policy_rule_conflict_states, 0);
            }
            other => panic!("unexpected dispatch build error: {other:?}"),
        }
    }

    #[test]
    fn dense_dispatch_table_can_intentionally_allow_unmatched_states() {
        let table = DispatchTableBuilder::<ByteCommand>::new("partial-allowed")
            .rules(PARTIAL_RULES)
            .allow_unmatched(true)
            .build_dense()
            .expect("explicitly allowing unmatched states should permit sparse dispatch");

        assert_eq!(table.len(), 256);
        assert_eq!(table.coverage().unmatched_states, 224);
        assert_eq!(
            table.dispatch(ByteCommand::new(1, 0)),
            Some(&DenseDispatchEntry::Unmatched)
        );
    }

    #[test]
    fn coverage_reports_export_pretty_markdown_and_json() {
        let report = big_bad_bool_interact::RuleTable::new("byte-report", DISPATCH_RULES)
            .coverage_for_with_policy::<ByteCommand>(CoveragePolicy::new(DISPATCH_POLICIES))
            .expect("byte coverage should scan");

        let pretty = report.pretty_summary();
        assert!(pretty.contains("coverage table `byte-report`"));
        assert!(pretty.contains("accepted: true"));

        let markdown = report.to_markdown();
        assert!(markdown.contains("# Bitpack coverage report: `byte-report`"));
        assert!(markdown.contains("| Total states | 256 |"));
        assert!(markdown.contains("## Reserved examples"));

        let json = report.to_json();
        assert!(json.contains("\"table\":\"byte-report\""));
        assert!(json.contains("\"total_states\":256"));
        assert!(json.contains("\"accepted\":true"));
    }

    #[test]
    fn value_labels_validate_against_generated_schema() {
        let valid = LabelRegistry::new(BYTE_LABELS).validate_for::<ByteCommand>();
        assert!(valid.accepted);
        assert!(valid.findings.is_empty());

        static BAD_LABELS: &[ValueLabel] = &[
            ValueLabel::new("missing", 0, "missing", "Unknown field"),
            ValueLabel::new(ByteCommand::FIELD_INDEX, 8, "too-large", "Too large"),
            ValueLabel::new(ByteCommand::FIELD_ITEM, 1, "first", "First duplicate"),
            ValueLabel::new(ByteCommand::FIELD_ITEM, 1, "second", "Second duplicate"),
        ];

        let invalid = LabelRegistry::new(BAD_LABELS).validate_for::<ByteCommand>();
        assert!(!invalid.accepted);
        assert_eq!(
            invalid.findings,
            vec![
                LabelValidationFinding::UnknownField {
                    field: "missing",
                    label: "missing",
                },
                LabelValidationFinding::ValueOutOfRange {
                    field: ByteCommand::FIELD_INDEX,
                    value: 8,
                    max: 7,
                    label: "too-large",
                },
                LabelValidationFinding::DuplicateLabel {
                    field: ByteCommand::FIELD_ITEM,
                    value: 1,
                    first: "first",
                    second: "second",
                },
            ]
        );
    }

    #[test]
    fn complex_schema_attributes_generate_protocol_metadata() {
        use big_bad_bool_core::{FieldRelationKind, ValueRangeKind};

        assert_eq!(ComplexContract32::VERSION, 7);
        assert_eq!(ComplexContract32::FIELDS.len(), 5);
        assert_eq!(ComplexContract32::FIELD_DOMAIN_SPEC.id, 10);
        assert_eq!(ComplexContract32::FIELD_PATH_KIND_SPEC.id, 11);
        assert_eq!(ComplexContract32::FIELD_OPERATION_SPEC.id, 12);
        assert_eq!(ComplexContract32::FIELD_POLICY_SPEC.id, 13);
        assert_eq!(ComplexContract32::FIELD_FLAGS_SPEC.id, 14);
        assert_eq!(ComplexContract32::FIELD_DOMAIN_SPEC.description, "Top-level contract domain for a reducer/runtime operation.");
        assert_eq!(ComplexContract32::FIELD_DOMAIN_SPEC.default, Some(1));

        assert_eq!(ComplexContract32::LABELS.len(), 22);
        assert_eq!(ComplexContract32::ALIASES.len(), 2);
        assert_eq!(ComplexContract32::ALIASES[0].field, ComplexContract32::FIELD_DOMAIN);
        assert_eq!(ComplexContract32::ALIASES[0].label, "builder");
        assert_eq!(ComplexContract32::ALIASES[0].alias, "ui_builder");

        assert!(ComplexContract32::RANGES.iter().any(|range| {
            range.field == ComplexContract32::FIELD_DOMAIN
                && range.kind == ValueRangeKind::Reserved
                && range.min == 28
                && range.max == 30
        }));
        assert!(ComplexContract32::RANGES.iter().any(|range| {
            range.field == ComplexContract32::FIELD_DOMAIN
                && range.kind == ValueRangeKind::Invalid
                && range.min == 31
                && range.max == 31
        }));

        assert_eq!(ComplexContract32::DEPRECATED.len(), 1);
        assert_eq!(ComplexContract32::DEPRECATED[0].field, ComplexContract32::FIELD_DOMAIN);
        assert_eq!(ComplexContract32::DEPRECATED[0].value, 3);
        assert_eq!(ComplexContract32::DEPRECATED[0].label, "old_domain");
        assert_eq!(ComplexContract32::DEPRECATED[0].since, "0.1.0");

        assert_eq!(ComplexContract32::FLAGS.len(), 4);
        assert_eq!(ComplexContract32::FLAG_FLAGS_UNDOABLE, 1);
        assert_eq!(ComplexContract32::FLAG_FLAGS_AUDITED, 2);
        assert_eq!(ComplexContract32::FLAG_FLAGS_OPTIMISTIC, 4);
        assert_eq!(ComplexContract32::FLAG_FLAGS_REPLICATED, 8);

        let op = ComplexContract32::new(1, 5, 2, 1, 0)
            .with_flags_undoable()
            .with_flags_audited();
        assert!(op.has_flags_undoable());
        assert!(op.has_flags_audited());
        assert!(!op.has_flags_optimistic());
        let op = op.without_flags_undoable();
        assert!(!op.has_flags_undoable());
        assert!(op.has_flags_audited());

        assert_eq!(ComplexContract32::RELATIONS.len(), 2);
        assert!(ComplexContract32::RELATIONS.iter().any(|relation| {
            relation.kind == FieldRelationKind::Requires
                && relation.field == ComplexContract32::FIELD_OPERATION
                && relation.when_label == "patch"
                && relation.other_field == "policy"
                && relation.other_label == "validate"
        }));
        assert!(ComplexContract32::RELATIONS.iter().any(|relation| {
            relation.kind == FieldRelationKind::Conflicts
                && relation.field == ComplexContract32::FIELD_FLAGS
                && relation.when_label == "optimistic"
                && relation.other_label == "replicated"
        }));

        assert_eq!(ComplexContract32::DISPATCH_HINTS.len(), 5);
        assert!(ComplexContract32::DISPATCH_HINTS.iter().any(|hint| {
            hint.field == ComplexContract32::FIELD_DOMAIN && hint.role == "key"
        }));
        assert!(ComplexContract32::DISPATCH_HINTS.iter().any(|hint| {
            hint.field == ComplexContract32::FIELD_FLAGS && hint.role == "flags"
        }));

        assert_eq!(ComplexContract32::field_by_id(10).unwrap().name, ComplexContract32::FIELD_DOMAIN);
        assert_eq!(op.read_field_id(14), Some(2));
    }

}
