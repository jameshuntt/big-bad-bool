use big_bad_bool_test_suite::ByteCommand;
use big_bad_bool_interact::{
    Action, CoveragePolicy, DispatchTableBuilder, FieldMatcher, LabelRegistry, Rule, StatePolicy,
    ValueLabel,
};

static NORMAL_INDEXES: &[FieldMatcher] = &[
    FieldMatcher::range(ByteCommand::FIELD_INDEX, 0, 5),
    FieldMatcher::any(ByteCommand::FIELD_ITEM),
];

static RULES: &[Rule] = &[Rule::new(
    "normal-byte-index",
    NORMAL_INDEXES,
    Action::new(7, "normal-byte-route", "Dispatch normal byte command states"),
)];

static INVALID_INDEX_6: &[FieldMatcher] = &[FieldMatcher::eq(ByteCommand::FIELD_INDEX, 6)];
static RESERVED_INDEX_7: &[FieldMatcher] = &[FieldMatcher::eq(ByteCommand::FIELD_INDEX, 7)];

static POLICIES: &[StatePolicy] = &[
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

static LABELS: &[ValueLabel] = &[
    ValueLabel::new(ByteCommand::FIELD_INDEX, 0, "primary", "Primary command family"),
    ValueLabel::new(ByteCommand::FIELD_INDEX, 6, "invalid", "Forbidden family"),
    ValueLabel::new(ByteCommand::FIELD_INDEX, 7, "reserved", "Reserved family"),
];

fn main() {
    let table = DispatchTableBuilder::<ByteCommand>::new("byte-dispatch")
        .rules(RULES)
        .policy(CoveragePolicy::new(POLICIES))
        .labels(LabelRegistry::new(LABELS))
        .build_dense()
        .expect("dispatch table should be fully covered");

    let command = ByteCommand::new(3, 17);
    let entry = table
        .dispatch(command)
        .expect("valid packed command should have a table entry");

    println!("{entry:?}");
    println!("{}", table.coverage().pretty_summary());
    println!("{}", table.coverage().to_markdown());
}
