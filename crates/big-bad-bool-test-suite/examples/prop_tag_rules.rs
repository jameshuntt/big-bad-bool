use big_bad_bool_interact::{Action, FieldMatcher, Rule, RuleTable};
use big_bad_bool_test_suite::PropTag16;

static SELECTOR_7_WITH_MID_HINTS: &[FieldMatcher] = &[
    FieldMatcher::eq(PropTag16::FIELD_SELECTOR, 7),
    FieldMatcher::range(PropTag16::FIELD_PROP_HINTS, 100, 199),
];

static RULES: &[Rule] = &[Rule::new(
    "selector-7-mid-hints",
    SELECTOR_7_WITH_MID_HINTS,
    Action::new(10, "route-selector-7", "Route selector 7 to a special handler"),
)];

fn main() {
    let table = RuleTable::new("prop-tag-rules", RULES);
    let validation = table.validate_for::<PropTag16>();
    assert!(validation.accepted, "rule table should validate: {validation:?}");

    let tag = PropTag16::new_checked(7, 136).expect("valid prop tag");
    let report = table.dispatch(tag);

    println!("tag: {tag:?}");
    println!("report: {report:?}");
}
