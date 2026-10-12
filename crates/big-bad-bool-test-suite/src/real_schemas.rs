//! Concrete packed schemas that model realistic runtime/builder use cases.
//!
//! These are not macro mechanics tests. They are reusable examples showing how a
//! packed layout, semantic labels, reserved/invalid policy, coverage, and dense
//! dispatch table fit together into one protocol-shaped operator.

use big_bad_bool::bitpack;
use big_bad_bool_interact::{
    Action, CoverageOptions, CoveragePolicy, DenseDispatchTable, DispatchBuildError,
    DispatchTableBuilder, FieldMatcher, LabelRegistry, Rule, StatePolicy, ValueLabel,
};

/// A 16-bit prop negotiation tag.
///
/// Layout:
/// - bits 0..=5: selector family
/// - bits 6..=15: prop-hint code
#[bitpack(storage = u16)]
pub struct PropTag16 {
    #[bits(6)]
    #[labels(
        reserved_null = 0,
        component_start = 1,
        layout_start = 16,
        telemetry_start = 32,
        system_start = 48,
        reserved_max = 63,
    )]
    selector: u16,

    #[bits(10)]
    #[labels(
        hint_none = 0,
        hint_normal_max = 959,
        hint_invalid_start = 960,
    )]
    prop_hints: u16,
}

/// Runtime routing hint for small internal fast paths.
///
/// Layout:
/// - bits 0..=3: route family
/// - bits 4..=7: execution phase
/// - bits 8..=15: flags
#[bitpack(storage = u16)]
pub struct RuntimeHint16 {
    #[bits(4)]
    route: u16,

    #[bits(4)]
    phase: u16,

    #[bits(8)]
    flags: u16,
}

/// Compact telemetry header for local event buses or UDS packet metadata.
///
/// Layout:
/// - bits 0..=4: source family
/// - bits 5..=10: event kind
/// - bits 11..=13: severity
/// - bits 14..=15: flags
#[bitpack(storage = u16)]
pub struct TelemetryHeader16 {
    #[bits(5)]
    source: u16,

    #[bits(6)]
    event: u16,

    #[bits(3)]
    severity: u16,

    #[bits(2)]
    flags: u16,
}

/// Compact packet/framing header for tiny internal command envelopes.
///
/// Layout:
/// - bits 0..=3: channel
/// - bits 4..=7: kind
/// - bits 8..=12: length class
/// - bits 13..=15: flags
#[bitpack(storage = u16)]
pub struct PacketHeader16 {
    #[bits(4)]
    channel: u16,

    #[bits(4)]
    kind: u16,

    #[bits(5)]
    len_class: u16,

    #[bits(3)]
    flags: u16,
}

/// Packed node-kind descriptor for a builder/runtime graph.
///
/// Layout:
/// - bits 0..=3: node family
/// - bits 4..=9: variant
/// - bits 10..=12: surface/layer
/// - bits 13..=15: policy class
#[bitpack(storage = u16)]
pub struct BuilderNodeKind16 {
    #[bits(4)]
    family: u16,

    #[bits(6)]
    variant: u16,

    #[bits(3)]
    surface: u16,

    #[bits(3)]
    policy: u16,
}

static PROP_SELECTOR_0_RESERVED: &[FieldMatcher] = &[FieldMatcher::eq(PropTag16::FIELD_SELECTOR, 0)];
static PROP_SELECTOR_63_RESERVED: &[FieldMatcher] = &[FieldMatcher::eq(PropTag16::FIELD_SELECTOR, 63)];
static PROP_HINTS_INVALID: &[FieldMatcher] = &[
    FieldMatcher::range(PropTag16::FIELD_SELECTOR, 1, 62),
    FieldMatcher::range(PropTag16::FIELD_PROP_HINTS, 960, 1023),
];
static PROP_COMPONENT_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(PropTag16::FIELD_SELECTOR, 1, 15),
    FieldMatcher::range(PropTag16::FIELD_PROP_HINTS, 0, 959),
];
static PROP_LAYOUT_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(PropTag16::FIELD_SELECTOR, 16, 31),
    FieldMatcher::range(PropTag16::FIELD_PROP_HINTS, 0, 959),
];
static PROP_TELEMETRY_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(PropTag16::FIELD_SELECTOR, 32, 47),
    FieldMatcher::range(PropTag16::FIELD_PROP_HINTS, 0, 959),
];
static PROP_SYSTEM_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(PropTag16::FIELD_SELECTOR, 48, 62),
    FieldMatcher::range(PropTag16::FIELD_PROP_HINTS, 0, 959),
];

/// Behavior rules for [`PropTag16`].
pub static PROP_TAG16_RULES: &[Rule] = &[
    Rule::new(
        "prop-component-selector",
        PROP_COMPONENT_MATCHERS,
        Action::new(100, "prop/component", "Route component prop hints"),
    ),
    Rule::new(
        "prop-layout-selector",
        PROP_LAYOUT_MATCHERS,
        Action::new(110, "prop/layout", "Route layout prop hints"),
    ),
    Rule::new(
        "prop-telemetry-selector",
        PROP_TELEMETRY_MATCHERS,
        Action::new(120, "prop/telemetry", "Route telemetry prop hints"),
    ),
    Rule::new(
        "prop-system-selector",
        PROP_SYSTEM_MATCHERS,
        Action::new(130, "prop/system", "Route system or permission prop hints"),
    ),
];

/// Reserved and invalid state-space for [`PropTag16`].
pub static PROP_TAG16_POLICIES: &[StatePolicy] = &[
    StatePolicy::reserved(
        "selector-0-reserved",
        PROP_SELECTOR_0_RESERVED,
        "Selector 0 is left empty as a null/extension family",
    ),
    StatePolicy::reserved(
        "selector-63-reserved",
        PROP_SELECTOR_63_RESERVED,
        "Selector 63 is reserved for future protocol expansion",
    ),
    StatePolicy::invalid(
        "prop-hints-960-through-1023-invalid",
        PROP_HINTS_INVALID,
        "The top prop-hint band is forbidden for normal selectors",
    ),
];

/// Semantic labels for [`PropTag16`].
pub static PROP_TAG16_LABELS: &[ValueLabel] = &[
    ValueLabel::new(PropTag16::FIELD_SELECTOR, 0, "reserved-null", "Null/extension selector"),
    ValueLabel::new(PropTag16::FIELD_SELECTOR, 1, "component-start", "Component selector band start"),
    ValueLabel::new(PropTag16::FIELD_SELECTOR, 16, "layout-start", "Layout selector band start"),
    ValueLabel::new(PropTag16::FIELD_SELECTOR, 32, "telemetry-start", "Telemetry selector band start"),
    ValueLabel::new(PropTag16::FIELD_SELECTOR, 48, "system-start", "System selector band start"),
    ValueLabel::new(PropTag16::FIELD_SELECTOR, 63, "reserved-max", "Reserved max selector"),
    ValueLabel::new(PropTag16::FIELD_PROP_HINTS, 0, "hint-none", "No prop hint"),
    ValueLabel::new(PropTag16::FIELD_PROP_HINTS, 959, "hint-normal-max", "Highest normal prop hint"),
    ValueLabel::new(PropTag16::FIELD_PROP_HINTS, 960, "hint-invalid-start", "First invalid prop-hint value"),
];

/// Builds a complete dense dispatch table for [`PropTag16`].
pub fn prop_tag16_dispatch_table() -> Result<DenseDispatchTable<PropTag16>, DispatchBuildError> {
    DispatchTableBuilder::<PropTag16>::new("prop-tag16-dispatch")
        .rules(PROP_TAG16_RULES)
        .policy(CoveragePolicy::new(PROP_TAG16_POLICIES))
        .labels(LabelRegistry::new(PROP_TAG16_LABELS))
        .options(CoverageOptions::exhaustive_u16())
        .build_dense()
}

static RUNTIME_ROUTE_15_INVALID: &[FieldMatcher] = &[FieldMatcher::eq(RuntimeHint16::FIELD_ROUTE, 15)];
static RUNTIME_PHASE_15_RESERVED: &[FieldMatcher] = &[
    FieldMatcher::range(RuntimeHint16::FIELD_ROUTE, 0, 14),
    FieldMatcher::eq(RuntimeHint16::FIELD_PHASE, 15),
];
static RUNTIME_IDLE_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::eq(RuntimeHint16::FIELD_ROUTE, 0),
    FieldMatcher::range(RuntimeHint16::FIELD_PHASE, 0, 14),
];
static RUNTIME_RENDER_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(RuntimeHint16::FIELD_ROUTE, 1, 4),
    FieldMatcher::range(RuntimeHint16::FIELD_PHASE, 0, 14),
];
static RUNTIME_STORAGE_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(RuntimeHint16::FIELD_ROUTE, 5, 8),
    FieldMatcher::range(RuntimeHint16::FIELD_PHASE, 0, 14),
];
static RUNTIME_NETWORK_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(RuntimeHint16::FIELD_ROUTE, 9, 12),
    FieldMatcher::range(RuntimeHint16::FIELD_PHASE, 0, 14),
];
static RUNTIME_AUDIT_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(RuntimeHint16::FIELD_ROUTE, 13, 14),
    FieldMatcher::range(RuntimeHint16::FIELD_PHASE, 0, 14),
];

pub static RUNTIME_HINT16_RULES: &[Rule] = &[
    Rule::new("runtime-idle", RUNTIME_IDLE_MATCHERS, Action::new(200, "runtime/idle", "No-op or wait path")),
    Rule::new("runtime-render", RUNTIME_RENDER_MATCHERS, Action::new(210, "runtime/render", "Render/UI routing path")),
    Rule::new("runtime-storage", RUNTIME_STORAGE_MATCHERS, Action::new(220, "runtime/storage", "Storage or object IO path")),
    Rule::new("runtime-network", RUNTIME_NETWORK_MATCHERS, Action::new(230, "runtime/network", "UDS/network routing path")),
    Rule::new("runtime-audit", RUNTIME_AUDIT_MATCHERS, Action::new(240, "runtime/audit", "Security/audit routing path")),
];

pub static RUNTIME_HINT16_POLICIES: &[StatePolicy] = &[
    StatePolicy::invalid("runtime-route-15-invalid", RUNTIME_ROUTE_15_INVALID, "Route 15 is forbidden"),
    StatePolicy::reserved("runtime-phase-15-reserved", RUNTIME_PHASE_15_RESERVED, "Phase 15 is reserved"),
];

pub static RUNTIME_HINT16_LABELS: &[ValueLabel] = &[
    ValueLabel::new(RuntimeHint16::FIELD_ROUTE, 0, "idle", "Idle/no-op route"),
    ValueLabel::new(RuntimeHint16::FIELD_ROUTE, 1, "render-start", "Render route band start"),
    ValueLabel::new(RuntimeHint16::FIELD_ROUTE, 5, "storage-start", "Storage route band start"),
    ValueLabel::new(RuntimeHint16::FIELD_ROUTE, 9, "network-start", "Network route band start"),
    ValueLabel::new(RuntimeHint16::FIELD_ROUTE, 13, "audit-start", "Audit route band start"),
    ValueLabel::new(RuntimeHint16::FIELD_ROUTE, 15, "invalid-route", "Forbidden route"),
    ValueLabel::new(RuntimeHint16::FIELD_PHASE, 15, "reserved-phase", "Reserved phase"),
    ValueLabel::new(RuntimeHint16::FIELD_FLAGS, 0, "no-flags", "No runtime flags set"),
    ValueLabel::new(RuntimeHint16::FIELD_FLAGS, 255, "all-flags", "All runtime flags set"),
];

pub fn runtime_hint16_dispatch_table() -> Result<DenseDispatchTable<RuntimeHint16>, DispatchBuildError> {
    DispatchTableBuilder::<RuntimeHint16>::new("runtime-hint16-dispatch")
        .rules(RUNTIME_HINT16_RULES)
        .policy(CoveragePolicy::new(RUNTIME_HINT16_POLICIES))
        .labels(LabelRegistry::new(RUNTIME_HINT16_LABELS))
        .options(CoverageOptions::exhaustive_u16())
        .build_dense()
}

static TELEMETRY_SOURCE_31_RESERVED: &[FieldMatcher] = &[FieldMatcher::eq(TelemetryHeader16::FIELD_SOURCE, 31)];
static TELEMETRY_EVENT_63_INVALID: &[FieldMatcher] = &[
    FieldMatcher::range(TelemetryHeader16::FIELD_SOURCE, 0, 30),
    FieldMatcher::eq(TelemetryHeader16::FIELD_EVENT, 63),
];
static TELEMETRY_SEVERITY_7_INVALID: &[FieldMatcher] = &[
    FieldMatcher::range(TelemetryHeader16::FIELD_SOURCE, 0, 30),
    FieldMatcher::range(TelemetryHeader16::FIELD_EVENT, 0, 62),
    FieldMatcher::eq(TelemetryHeader16::FIELD_SEVERITY, 7),
];
static TELEMETRY_TRACE_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(TelemetryHeader16::FIELD_SOURCE, 0, 30),
    FieldMatcher::range(TelemetryHeader16::FIELD_EVENT, 0, 62),
    FieldMatcher::range(TelemetryHeader16::FIELD_SEVERITY, 0, 1),
];
static TELEMETRY_NORMAL_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(TelemetryHeader16::FIELD_SOURCE, 0, 30),
    FieldMatcher::range(TelemetryHeader16::FIELD_EVENT, 0, 62),
    FieldMatcher::range(TelemetryHeader16::FIELD_SEVERITY, 2, 3),
];
static TELEMETRY_WARN_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(TelemetryHeader16::FIELD_SOURCE, 0, 30),
    FieldMatcher::range(TelemetryHeader16::FIELD_EVENT, 0, 62),
    FieldMatcher::range(TelemetryHeader16::FIELD_SEVERITY, 4, 5),
];
static TELEMETRY_CRITICAL_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(TelemetryHeader16::FIELD_SOURCE, 0, 30),
    FieldMatcher::range(TelemetryHeader16::FIELD_EVENT, 0, 62),
    FieldMatcher::eq(TelemetryHeader16::FIELD_SEVERITY, 6),
];

pub static TELEMETRY_HEADER16_RULES: &[Rule] = &[
    Rule::new("telemetry-trace-info", TELEMETRY_TRACE_MATCHERS, Action::new(300, "telemetry/trace-info", "Trace/info telemetry")),
    Rule::new("telemetry-normal", TELEMETRY_NORMAL_MATCHERS, Action::new(310, "telemetry/normal", "Normal telemetry")),
    Rule::new("telemetry-warning", TELEMETRY_WARN_MATCHERS, Action::new(320, "telemetry/warn", "Warning telemetry")),
    Rule::new("telemetry-critical", TELEMETRY_CRITICAL_MATCHERS, Action::new(330, "telemetry/critical", "Critical telemetry")),
];

pub static TELEMETRY_HEADER16_POLICIES: &[StatePolicy] = &[
    StatePolicy::reserved("telemetry-source-31-reserved", TELEMETRY_SOURCE_31_RESERVED, "Source 31 is reserved"),
    StatePolicy::invalid("telemetry-event-63-invalid", TELEMETRY_EVENT_63_INVALID, "Event 63 is forbidden"),
    StatePolicy::invalid("telemetry-severity-7-invalid", TELEMETRY_SEVERITY_7_INVALID, "Severity 7 is forbidden"),
];

pub static TELEMETRY_HEADER16_LABELS: &[ValueLabel] = &[
    ValueLabel::new(TelemetryHeader16::FIELD_SOURCE, 0, "runtime", "Runtime source"),
    ValueLabel::new(TelemetryHeader16::FIELD_SOURCE, 31, "reserved-source", "Reserved source"),
    ValueLabel::new(TelemetryHeader16::FIELD_EVENT, 0, "heartbeat", "Heartbeat event"),
    ValueLabel::new(TelemetryHeader16::FIELD_EVENT, 63, "invalid-event", "Forbidden event"),
    ValueLabel::new(TelemetryHeader16::FIELD_SEVERITY, 0, "trace", "Trace severity"),
    ValueLabel::new(TelemetryHeader16::FIELD_SEVERITY, 4, "warn", "Warning severity"),
    ValueLabel::new(TelemetryHeader16::FIELD_SEVERITY, 6, "critical", "Critical severity"),
    ValueLabel::new(TelemetryHeader16::FIELD_SEVERITY, 7, "invalid-severity", "Forbidden severity"),
];

pub fn telemetry_header16_dispatch_table() -> Result<DenseDispatchTable<TelemetryHeader16>, DispatchBuildError> {
    DispatchTableBuilder::<TelemetryHeader16>::new("telemetry-header16-dispatch")
        .rules(TELEMETRY_HEADER16_RULES)
        .policy(CoveragePolicy::new(TELEMETRY_HEADER16_POLICIES))
        .labels(LabelRegistry::new(TELEMETRY_HEADER16_LABELS))
        .options(CoverageOptions::exhaustive_u16())
        .build_dense()
}

static PACKET_CHANNEL_15_RESERVED: &[FieldMatcher] = &[FieldMatcher::eq(PacketHeader16::FIELD_CHANNEL, 15)];
static PACKET_KIND_15_INVALID: &[FieldMatcher] = &[
    FieldMatcher::range(PacketHeader16::FIELD_CHANNEL, 0, 14),
    FieldMatcher::eq(PacketHeader16::FIELD_KIND, 15),
];
static PACKET_LEN_31_INVALID: &[FieldMatcher] = &[
    FieldMatcher::range(PacketHeader16::FIELD_CHANNEL, 0, 14),
    FieldMatcher::range(PacketHeader16::FIELD_KIND, 0, 14),
    FieldMatcher::eq(PacketHeader16::FIELD_LEN_CLASS, 31),
];
static PACKET_CONTROL_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(PacketHeader16::FIELD_CHANNEL, 0, 14),
    FieldMatcher::range(PacketHeader16::FIELD_KIND, 0, 3),
    FieldMatcher::range(PacketHeader16::FIELD_LEN_CLASS, 0, 30),
];
static PACKET_DATA_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(PacketHeader16::FIELD_CHANNEL, 0, 14),
    FieldMatcher::range(PacketHeader16::FIELD_KIND, 4, 7),
    FieldMatcher::range(PacketHeader16::FIELD_LEN_CLASS, 0, 30),
];
static PACKET_STREAM_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(PacketHeader16::FIELD_CHANNEL, 0, 14),
    FieldMatcher::range(PacketHeader16::FIELD_KIND, 8, 11),
    FieldMatcher::range(PacketHeader16::FIELD_LEN_CLASS, 0, 30),
];
static PACKET_ADMIN_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(PacketHeader16::FIELD_CHANNEL, 0, 14),
    FieldMatcher::range(PacketHeader16::FIELD_KIND, 12, 14),
    FieldMatcher::range(PacketHeader16::FIELD_LEN_CLASS, 0, 30),
];

pub static PACKET_HEADER16_RULES: &[Rule] = &[
    Rule::new("packet-control", PACKET_CONTROL_MATCHERS, Action::new(400, "packet/control", "Control packet")),
    Rule::new("packet-data", PACKET_DATA_MATCHERS, Action::new(410, "packet/data", "Data packet")),
    Rule::new("packet-stream", PACKET_STREAM_MATCHERS, Action::new(420, "packet/stream", "Stream packet")),
    Rule::new("packet-admin", PACKET_ADMIN_MATCHERS, Action::new(430, "packet/admin", "Admin packet")),
];

pub static PACKET_HEADER16_POLICIES: &[StatePolicy] = &[
    StatePolicy::reserved("packet-channel-15-reserved", PACKET_CHANNEL_15_RESERVED, "Channel 15 is reserved"),
    StatePolicy::invalid("packet-kind-15-invalid", PACKET_KIND_15_INVALID, "Kind 15 is forbidden"),
    StatePolicy::invalid("packet-len-class-31-invalid", PACKET_LEN_31_INVALID, "Length class 31 is forbidden"),
];

pub static PACKET_HEADER16_LABELS: &[ValueLabel] = &[
    ValueLabel::new(PacketHeader16::FIELD_CHANNEL, 0, "primary", "Primary channel"),
    ValueLabel::new(PacketHeader16::FIELD_CHANNEL, 15, "reserved-channel", "Reserved channel"),
    ValueLabel::new(PacketHeader16::FIELD_KIND, 0, "control-start", "Control kind band start"),
    ValueLabel::new(PacketHeader16::FIELD_KIND, 4, "data-start", "Data kind band start"),
    ValueLabel::new(PacketHeader16::FIELD_KIND, 8, "stream-start", "Stream kind band start"),
    ValueLabel::new(PacketHeader16::FIELD_KIND, 12, "admin-start", "Admin kind band start"),
    ValueLabel::new(PacketHeader16::FIELD_KIND, 15, "invalid-kind", "Forbidden kind"),
    ValueLabel::new(PacketHeader16::FIELD_LEN_CLASS, 31, "invalid-len-class", "Forbidden length class"),
];

pub fn packet_header16_dispatch_table() -> Result<DenseDispatchTable<PacketHeader16>, DispatchBuildError> {
    DispatchTableBuilder::<PacketHeader16>::new("packet-header16-dispatch")
        .rules(PACKET_HEADER16_RULES)
        .policy(CoveragePolicy::new(PACKET_HEADER16_POLICIES))
        .labels(LabelRegistry::new(PACKET_HEADER16_LABELS))
        .options(CoverageOptions::exhaustive_u16())
        .build_dense()
}

static NODE_FAMILY_0_RESERVED: &[FieldMatcher] = &[FieldMatcher::eq(BuilderNodeKind16::FIELD_FAMILY, 0)];
static NODE_FAMILY_15_RESERVED: &[FieldMatcher] = &[FieldMatcher::eq(BuilderNodeKind16::FIELD_FAMILY, 15)];
static NODE_VARIANT_63_INVALID: &[FieldMatcher] = &[
    FieldMatcher::range(BuilderNodeKind16::FIELD_FAMILY, 1, 14),
    FieldMatcher::eq(BuilderNodeKind16::FIELD_VARIANT, 63),
];
static NODE_POLICY_7_INVALID: &[FieldMatcher] = &[
    FieldMatcher::range(BuilderNodeKind16::FIELD_FAMILY, 1, 14),
    FieldMatcher::range(BuilderNodeKind16::FIELD_VARIANT, 0, 62),
    FieldMatcher::eq(BuilderNodeKind16::FIELD_POLICY, 7),
];
static NODE_COMPONENT_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(BuilderNodeKind16::FIELD_FAMILY, 1, 3),
    FieldMatcher::range(BuilderNodeKind16::FIELD_VARIANT, 0, 62),
    FieldMatcher::range(BuilderNodeKind16::FIELD_POLICY, 0, 6),
];
static NODE_LAYOUT_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(BuilderNodeKind16::FIELD_FAMILY, 4, 6),
    FieldMatcher::range(BuilderNodeKind16::FIELD_VARIANT, 0, 62),
    FieldMatcher::range(BuilderNodeKind16::FIELD_POLICY, 0, 6),
];
static NODE_ANIMATION_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(BuilderNodeKind16::FIELD_FAMILY, 7, 9),
    FieldMatcher::range(BuilderNodeKind16::FIELD_VARIANT, 0, 62),
    FieldMatcher::range(BuilderNodeKind16::FIELD_POLICY, 0, 6),
];
static NODE_DATA_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(BuilderNodeKind16::FIELD_FAMILY, 10, 12),
    FieldMatcher::range(BuilderNodeKind16::FIELD_VARIANT, 0, 62),
    FieldMatcher::range(BuilderNodeKind16::FIELD_POLICY, 0, 6),
];
static NODE_SECURITY_MATCHERS: &[FieldMatcher] = &[
    FieldMatcher::range(BuilderNodeKind16::FIELD_FAMILY, 13, 14),
    FieldMatcher::range(BuilderNodeKind16::FIELD_VARIANT, 0, 62),
    FieldMatcher::range(BuilderNodeKind16::FIELD_POLICY, 0, 6),
];

pub static BUILDER_NODE_KIND16_RULES: &[Rule] = &[
    Rule::new("builder-component", NODE_COMPONENT_MATCHERS, Action::new(500, "builder/component", "Component node")),
    Rule::new("builder-layout", NODE_LAYOUT_MATCHERS, Action::new(510, "builder/layout", "Layout node")),
    Rule::new("builder-animation", NODE_ANIMATION_MATCHERS, Action::new(520, "builder/animation", "Animation node")),
    Rule::new("builder-data", NODE_DATA_MATCHERS, Action::new(530, "builder/data", "Data node")),
    Rule::new("builder-security", NODE_SECURITY_MATCHERS, Action::new(540, "builder/security", "Security/policy node")),
];

pub static BUILDER_NODE_KIND16_POLICIES: &[StatePolicy] = &[
    StatePolicy::reserved("node-family-0-reserved", NODE_FAMILY_0_RESERVED, "Family 0 is reserved for null nodes"),
    StatePolicy::reserved("node-family-15-reserved", NODE_FAMILY_15_RESERVED, "Family 15 is reserved for future node families"),
    StatePolicy::invalid("node-variant-63-invalid", NODE_VARIANT_63_INVALID, "Variant 63 is forbidden"),
    StatePolicy::invalid("node-policy-7-invalid", NODE_POLICY_7_INVALID, "Policy class 7 is forbidden"),
];

pub static BUILDER_NODE_KIND16_LABELS: &[ValueLabel] = &[
    ValueLabel::new(BuilderNodeKind16::FIELD_FAMILY, 0, "reserved-null", "Null node family"),
    ValueLabel::new(BuilderNodeKind16::FIELD_FAMILY, 1, "component-start", "Component family band start"),
    ValueLabel::new(BuilderNodeKind16::FIELD_FAMILY, 4, "layout-start", "Layout family band start"),
    ValueLabel::new(BuilderNodeKind16::FIELD_FAMILY, 7, "animation-start", "Animation family band start"),
    ValueLabel::new(BuilderNodeKind16::FIELD_FAMILY, 10, "data-start", "Data family band start"),
    ValueLabel::new(BuilderNodeKind16::FIELD_FAMILY, 13, "security-start", "Security family band start"),
    ValueLabel::new(BuilderNodeKind16::FIELD_FAMILY, 15, "reserved-max", "Reserved max family"),
    ValueLabel::new(BuilderNodeKind16::FIELD_VARIANT, 63, "invalid-variant", "Forbidden variant"),
    ValueLabel::new(BuilderNodeKind16::FIELD_POLICY, 7, "invalid-policy", "Forbidden policy class"),
];

pub fn builder_node_kind16_dispatch_table() -> Result<DenseDispatchTable<BuilderNodeKind16>, DispatchBuildError> {
    DispatchTableBuilder::<BuilderNodeKind16>::new("builder-node-kind16-dispatch")
        .rules(BUILDER_NODE_KIND16_RULES)
        .policy(CoveragePolicy::new(BUILDER_NODE_KIND16_POLICIES))
        .labels(LabelRegistry::new(BUILDER_NODE_KIND16_LABELS))
        .options(CoverageOptions::exhaustive_u16())
        .build_dense()
}

#[cfg(test)]
mod tests {
    use super::*;
    use big_bad_bool_interact::DenseDispatchEntry;

    fn assert_matched_name(entry: &DenseDispatchEntry, expected: &str) {
        match entry {
            DenseDispatchEntry::Matched { action, .. } => assert_eq!(action.name, expected),
            other => panic!("expected matched dispatch entry, got {other:?}"),
        }
    }

    fn assert_table_is_complete<T: big_bad_bool_core::PackedWord>(table: &DenseDispatchTable<T>) {
        assert_eq!(table.len(), 65_536);
        assert!(table.coverage().accepted(), "{}", table.coverage().pretty_summary());
        assert_eq!(table.coverage().unmatched_states, 0);
        assert_eq!(table.coverage().overlap_states, 0);
        assert_eq!(table.coverage().policy_rule_conflict_states, 0);
    }

    #[test]
    fn prop_tag_schema_builds_complete_dense_table() {
        let table = prop_tag16_dispatch_table().expect("prop tag table should build");
        assert_table_is_complete(&table);

        assert_matched_name(
            table.dispatch(PropTag16::new(2, 12)).unwrap(),
            "prop/component",
        );
        assert_matched_name(
            table.dispatch(PropTag16::new(20, 111)).unwrap(),
            "prop/layout",
        );
        assert!(matches!(
            table.dispatch(PropTag16::new(0, 55)).unwrap(),
            DenseDispatchEntry::Reserved { .. }
        ));
        assert!(matches!(
            table.dispatch(PropTag16::new(12, 1000)).unwrap(),
            DenseDispatchEntry::Invalid { .. }
        ));
    }

    #[test]
    fn runtime_hint_schema_builds_complete_dense_table() {
        let table = runtime_hint16_dispatch_table().expect("runtime hint table should build");
        assert_table_is_complete(&table);

        assert_matched_name(
            table.dispatch(RuntimeHint16::new(9, 3, 0b1010)).unwrap(),
            "runtime/network",
        );
        assert!(matches!(
            table.dispatch(RuntimeHint16::new(15, 0, 0)).unwrap(),
            DenseDispatchEntry::Invalid { .. }
        ));
        assert!(matches!(
            table.dispatch(RuntimeHint16::new(3, 15, 0)).unwrap(),
            DenseDispatchEntry::Reserved { .. }
        ));
    }

    #[test]
    fn telemetry_header_schema_builds_complete_dense_table() {
        let table = telemetry_header16_dispatch_table().expect("telemetry table should build");
        assert_table_is_complete(&table);

        assert_matched_name(
            table.dispatch(TelemetryHeader16::new(2, 10, 6, 1)).unwrap(),
            "telemetry/critical",
        );
        assert!(matches!(
            table.dispatch(TelemetryHeader16::new(31, 10, 2, 0)).unwrap(),
            DenseDispatchEntry::Reserved { .. }
        ));
        assert!(matches!(
            table.dispatch(TelemetryHeader16::new(1, 63, 2, 0)).unwrap(),
            DenseDispatchEntry::Invalid { .. }
        ));
    }

    #[test]
    fn packet_header_schema_builds_complete_dense_table() {
        let table = packet_header16_dispatch_table().expect("packet table should build");
        assert_table_is_complete(&table);

        assert_matched_name(
            table.dispatch(PacketHeader16::new(1, 9, 12, 0)).unwrap(),
            "packet/stream",
        );
        assert!(matches!(
            table.dispatch(PacketHeader16::new(15, 1, 1, 0)).unwrap(),
            DenseDispatchEntry::Reserved { .. }
        ));
        assert!(matches!(
            table.dispatch(PacketHeader16::new(1, 15, 1, 0)).unwrap(),
            DenseDispatchEntry::Invalid { .. }
        ));
    }

    #[test]
    fn builder_node_schema_builds_complete_dense_table() {
        let table = builder_node_kind16_dispatch_table().expect("builder node table should build");
        assert_table_is_complete(&table);

        assert_matched_name(
            table.dispatch(BuilderNodeKind16::new(8, 12, 3, 2)).unwrap(),
            "builder/animation",
        );
        assert!(matches!(
            table.dispatch(BuilderNodeKind16::new(15, 1, 1, 1)).unwrap(),
            DenseDispatchEntry::Reserved { .. }
        ));
        assert!(matches!(
            table.dispatch(BuilderNodeKind16::new(4, 63, 1, 1)).unwrap(),
            DenseDispatchEntry::Invalid { .. }
        ));
    }

    #[test]
    fn semantic_labels_validate_for_each_real_schema() {
        assert!(LabelRegistry::new(PropTag16::LABELS).validate_for::<PropTag16>().accepted);
        assert_eq!(PropTag16::LABELS.len(), 9);
        assert_eq!(PropTag16::LABEL_SELECTOR_COMPONENT_START.label, "component_start");
        assert_eq!(PropTag16::LABEL_PROP_HINTS_HINT_INVALID_START.value, 960);

        assert!(LabelRegistry::new(PROP_TAG16_LABELS).validate_for::<PropTag16>().accepted);
        assert!(LabelRegistry::new(RUNTIME_HINT16_LABELS).validate_for::<RuntimeHint16>().accepted);
        assert!(LabelRegistry::new(TELEMETRY_HEADER16_LABELS)
            .validate_for::<TelemetryHeader16>()
            .accepted);
        assert!(LabelRegistry::new(PACKET_HEADER16_LABELS)
            .validate_for::<PacketHeader16>()
            .accepted);
        assert!(LabelRegistry::new(BUILDER_NODE_KIND16_LABELS)
            .validate_for::<BuilderNodeKind16>()
            .accepted);
    }
}
