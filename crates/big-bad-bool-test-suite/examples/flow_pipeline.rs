use big_bad_bool_interact::{
    FieldValue, HandlerRegistry, PackedFlow, PackedReconstructor, PassthroughMode,
};
use big_bad_bool_test_suite::real_schemas::{prop_tag16_dispatch_table, PropTag16};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let word = PackedReconstructor::<PropTag16>::zeroed()
        .overlay_fields_strict(&[
            FieldValue::new(PropTag16::FIELD_SELECTOR, 2),
            FieldValue::new(PropTag16::FIELD_PROP_HINTS, 12),
        ])?
        .build()?;

    let table = prop_tag16_dispatch_table()?;
    let registry = HandlerRegistry::<PropTag16, Vec<String>>::new().on_action(
        100,
        |word, action, ctx| {
            ctx.push(format!(
                "handled {} selector={} hints={}",
                action.name,
                word.selector(),
                word.prop_hints()
            ));
            Ok(())
        },
    );

    let flow = PackedFlow::<PropTag16, Vec<String>>::new("prop-tag-flow")
        .dispatch_dense(table)
        .passthrough(PassthroughMode::RejectAllNonMatched)
        .handle_with_registry(registry);

    let mut ctx = Vec::new();
    let envelope = flow.run_word(word, &mut ctx)?;

    println!("raw: {}", envelope.raw);
    println!("fields: {:?}", envelope.fields);
    println!("action: {:?}", envelope.action);
    println!("ctx: {:?}", ctx);

    Ok(())
}
