use big_bad_bool_interact::DenseDispatchEntry;
use big_bad_bool_test_suite::real_schemas::{
    builder_node_kind16_dispatch_table, packet_header16_dispatch_table, prop_tag16_dispatch_table,
    runtime_hint16_dispatch_table, telemetry_header16_dispatch_table, BuilderNodeKind16,
    PacketHeader16, PropTag16, RuntimeHint16, TelemetryHeader16,
};

fn print_entry(name: &str, entry: &DenseDispatchEntry) {
    println!("{name}: {entry:?}");
}

fn main() {
    let prop_table = prop_tag16_dispatch_table().expect("prop tag dispatch should build");
    let runtime_table = runtime_hint16_dispatch_table().expect("runtime hint dispatch should build");
    let telemetry_table = telemetry_header16_dispatch_table().expect("telemetry dispatch should build");
    let packet_table = packet_header16_dispatch_table().expect("packet dispatch should build");
    let node_table = builder_node_kind16_dispatch_table().expect("builder node dispatch should build");

    print_entry(
        "prop/component",
        prop_table.dispatch(PropTag16::new(2, 12)).unwrap(),
    );
    print_entry(
        "runtime/network",
        runtime_table.dispatch(RuntimeHint16::new(9, 3, 0b1010)).unwrap(),
    );
    print_entry(
        "telemetry/critical",
        telemetry_table.dispatch(TelemetryHeader16::new(2, 10, 6, 1)).unwrap(),
    );
    print_entry(
        "packet/stream",
        packet_table.dispatch(PacketHeader16::new(1, 9, 12, 0)).unwrap(),
    );
    print_entry(
        "builder/animation",
        node_table.dispatch(BuilderNodeKind16::new(8, 12, 3, 2)).unwrap(),
    );

    println!("\n{}", prop_table.coverage().pretty_summary());
}
