use big_bad_bool::bitpack;
use big_bad_bool_core::PackedWord;
use big_bad_bool_interact::{
    reconstruct_word, patch_word, CoverageOptions, DenseDispatchEntry, DispatchTableBuilder,
    FieldMatcher, FieldValue, FlowError, HandlerRegistry, PackedEnvelope, PackedFlow,
    PackedReconstructor, PassthroughMode, ReconstructionError, Rule, Action,
};
use big_bad_bool_test_suite::real_schemas::{prop_tag16_dispatch_table, PropTag16};

#[bitpack(storage = u8)]
struct MiniCommand {
    #[bits(2)]
    family: u8,

    #[bits(6)]
    item: u8,
}

static MINI_FAMILY_ONE: &[FieldMatcher] = &[FieldMatcher::eq(MiniCommand::FIELD_FAMILY, 1)];
static MINI_RULES: &[Rule] = &[Rule::new(
    "mini-family-one",
    MINI_FAMILY_ONE,
    Action::new(1, "mini/one", "Handle mini family one"),
)];

fn mini_dispatch_table() -> big_bad_bool_interact::DenseDispatchTable<MiniCommand> {
    DispatchTableBuilder::<MiniCommand>::new("mini-dispatch")
        .rules(MINI_RULES)
        .options(CoverageOptions::new(256, 8))
        .allow_unmatched(true)
        .build_dense()
        .expect("mini dispatch table builds with unmatched states allowed")
}

#[test]
fn reconstructs_prop_tag_from_symbolic_fields() {
    let tag = reconstruct_word::<PropTag16>(&[
        FieldValue::new(PropTag16::FIELD_SELECTOR, 2),
        FieldValue::new(PropTag16::FIELD_PROP_HINTS, 12),
    ])
    .expect("reconstruction should accept valid fields");

    assert_eq!(tag.selector(), 2);
    assert_eq!(tag.prop_hints(), 12);
    assert_eq!(tag.raw_u128(), PropTag16::new(2, 12).raw_u128());
}

#[test]
fn reconstruction_rejects_unknown_and_out_of_range_fields() {
    let unknown = PackedReconstructor::<PropTag16>::zeroed()
        .set_field("missing", 1)
        .expect_err("unknown field should fail");
    assert!(matches!(unknown, ReconstructionError::UnknownField { field: "missing" }));

    let overflow = PackedReconstructor::<PropTag16>::zeroed()
        .set_field(PropTag16::FIELD_SELECTOR, 64)
        .expect_err("selector is six bits, so 64 should fail");
    if let ReconstructionError::ValueOutOfRange { field, value, max } = overflow {
        assert_eq!(field, PropTag16::FIELD_SELECTOR);
        assert_eq!(value, 64);
        assert_eq!(max, 63);
    } else {
        panic!("expected ValueOutOfRange");
    }
}

#[test]
fn strict_overlay_rejects_duplicate_fields() {
    let duplicate = PackedReconstructor::<PropTag16>::zeroed()
        .overlay_fields_strict(&[
            FieldValue::new(PropTag16::FIELD_SELECTOR, 2),
            FieldValue::new(PropTag16::FIELD_SELECTOR, 3),
        ])
        .expect_err("strict overlay should reject duplicate assignments");

    if let ReconstructionError::DuplicateField { field } = duplicate {
        assert_eq!(field, PropTag16::FIELD_SELECTOR);
    } else {
        panic!("expected DuplicateField");
    }
}

#[test]
fn patching_keeps_unmentioned_fields_stable() {
    let original = PropTag16::new(2, 12);
    let patched = patch_word(
        original,
        &[FieldValue::new(PropTag16::FIELD_PROP_HINTS, 44)],
    )
    .expect("patch should succeed");

    assert_eq!(patched.selector(), 2);
    assert_eq!(patched.prop_hints(), 44);
}

#[test]
fn passthrough_dispatch_rejects_or_forwards_unmatched_states() {
    let table = mini_dispatch_table();
    let unmatched = MiniCommand::new(0, 7);

    let rejected = table.dispatch_with_passthrough(unmatched, PassthroughMode::RejectAllNonMatched);
    assert!(rejected.is_rejected());

    let forwarded = table.dispatch_with_passthrough(unmatched, PassthroughMode::ForwardUnmatched);
    assert!(forwarded.is_passthrough());
}

#[test]
fn dense_dispatch_still_handles_matched_states() {
    let table = prop_tag16_dispatch_table().expect("real prop table builds");
    let tag = PropTag16::new(2, 12);
    let decision = table.dispatch_with_passthrough(tag, PassthroughMode::RejectAllNonMatched);

    assert!(decision.is_handled());
}

#[test]
fn flow_dispatches_and_executes_registered_handler() {
    let table = prop_tag16_dispatch_table().expect("real prop table builds");
    let registry = HandlerRegistry::<PropTag16, Vec<&'static str>>::new().on_action(
        100,
        |word, action, ctx| {
            assert_eq!(word.selector(), 2);
            ctx.push(action.name);
            Ok(())
        },
    );

    let flow = PackedFlow::<PropTag16, Vec<&'static str>>::new("prop-flow")
        .dispatch_dense(table)
        .passthrough(PassthroughMode::RejectAllNonMatched)
        .handle_with_registry(registry);

    let mut ctx = Vec::new();
    let envelope = flow
        .run_word(PropTag16::new(2, 12), &mut ctx)
        .expect("matched flow should execute");

    assert_eq!(ctx, vec!["prop/component"]);
    assert!(matches!(
        envelope.dispatch,
        Some(DenseDispatchEntry::Matched {
            rule_name: "prop-component-selector",
            ..
        })
    ));
    assert_eq!(envelope.action.expect("action is attached").code, 100);
}

#[test]
fn flow_can_patch_then_redispatch() {
    let table = prop_tag16_dispatch_table().expect("real prop table builds");
    let flow = PackedFlow::<PropTag16, ()>::new("patch-flow")
        .patch_field(PropTag16::FIELD_SELECTOR, 16)
        .dispatch_dense(table)
        .passthrough(PassthroughMode::RejectAllNonMatched);

    let mut ctx = ();
    let envelope = flow
        .run_word(PropTag16::new(2, 12), &mut ctx)
        .expect("patched value should redispatch");

    assert_eq!(envelope.word.selector(), 16);
    assert_eq!(envelope.word.prop_hints(), 12);
    assert_eq!(envelope.action.expect("layout action attached").code, 110);
}

#[test]
fn flow_can_forward_passthrough_to_registry() {
    let table = mini_dispatch_table();
    let registry = HandlerRegistry::<MiniCommand, Vec<&'static str>>::new().on_passthrough(
        |word, entry, ctx| {
            assert_eq!(word.family(), 0);
            assert!(matches!(entry, DenseDispatchEntry::Unmatched));
            ctx.push("forwarded");
            Ok(())
        },
    );

    let flow = PackedFlow::<MiniCommand, Vec<&'static str>>::new("passthrough-flow")
        .dispatch_dense(table)
        .passthrough(PassthroughMode::ForwardUnmatched)
        .handle_with_registry(registry);

    let mut ctx = Vec::new();
    let envelope = flow
        .run_word(MiniCommand::new(0, 7), &mut ctx)
        .expect("unmatched state should pass through");

    assert_eq!(ctx, vec!["forwarded"]);
    assert!(envelope.action.is_none());
    assert_eq!(envelope.notes, vec!["passthrough:Unmatched"]);
}

#[test]
fn flow_decodes_raw_strictly() {
    let table = prop_tag16_dispatch_table().expect("real prop table builds");
    let flow = PackedFlow::<PropTag16, ()>::new("raw-flow")
        .dispatch_dense(table)
        .passthrough(PassthroughMode::RejectAllNonMatched);

    let mut ctx = ();
    let ok = flow.run_raw_checked(PropTag16::new(2, 12).raw_u128(), &mut ctx);
    assert!(ok.is_ok());

    let bad = PackedEnvelope::<PropTag16>::from_raw_checked(1u128 << 16);
    assert!(matches!(bad, Err(FlowError::Decode(_))));
}
