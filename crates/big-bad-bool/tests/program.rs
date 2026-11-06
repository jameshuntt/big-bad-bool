//! `big_bad_bool::program` with the `program` feature: the program crate is
//! reachable through the facade, and a program with no framing builds,
//! verifies and steps using only facade paths. Everything but the last test
//! runs without Bowl, on the facade's rust-version; the last one needs
//! `program-bowl` (ob-serve-a-bowl, Rust 1.88).

#![cfg(feature = "program")]

use big_bad_bool::bitpack;
use big_bad_bool::program::{
    clause, contract, dimension, sequence, set, verify, Clause, ContainmentSpec, ContractSpec,
    FieldMatcher, Liaise, NoObserver, Observation, Observer, Program, RefusalRule, SequenceSpec,
    StepResult, Stepper, VerifiedProgram, VerifyLimits,
};

#[bitpack(storage = u8)]
pub struct Reading {
    #[bits(2)]
    #[labels(Cold = 0, Mild = 1, Hot = 2)]
    #[invalid(3)]
    pub level: u8,
}

#[bitpack(storage = u8)]
pub struct Heater {
    #[bits(1)]
    #[labels(Off = 0, On = 1)]
    pub state: u8,
}

#[bitpack(storage = u8)]
pub struct Command {
    #[bits(2)]
    #[labels(None = 0, StartHeat = 1, StopHeat = 2)]
    #[invalid(3)]
    pub kind: u8,
}

const COLD: u128 = Reading::LABEL_LEVEL_COLD.value;
const MILD: u128 = Reading::LABEL_LEVEL_MILD.value;
const HOT: u128 = Reading::LABEL_LEVEL_HOT.value;
const OFF: u128 = Heater::LABEL_STATE_OFF.value;
const ON: u128 = Heater::LABEL_STATE_ON.value;

const fn reading(level: u128) -> Clause {
    clause("reading", FieldMatcher::eq("level", level))
}
const fn heater(state: u128) -> Clause {
    clause("heater", FieldMatcher::eq("state", state))
}

const CONTRACTS: &[ContractSpec] = &[
    contract("heat-when-cold")
        .when(&[reading(COLD)])
        .require(&[heater(OFF)])
        .transition(&[set("heater", "state", ON)])
        .emit(&[set("command", "kind", Command::LABEL_KIND_STARTHEAT.value)]),
    contract("stop-when-hot")
        .when(&[reading(HOT)])
        .require(&[heater(ON)])
        .transition(&[set("heater", "state", OFF)])
        .emit(&[set("command", "kind", Command::LABEL_KIND_STOPHEAT.value)]),
    contract("hold").when(&[reading(MILD)]),
];

fn thermostat() -> VerifiedProgram {
    let spec = CONTRACTS
        .iter()
        .fold(
            Program::new()
                .add(dimension!(Reading, "reading", Evidence))
                .add(dimension!(Heater, "heater", State))
                .add(dimension!(Command, "command", Effect))
                .contain(ContainmentSpec {
                    max_interpretations: 1,
                    max_candidates: 4,
                    max_actualized: 4,
                    max_stack_depth: 1,
                    on_refusal: RefusalRule::SkipObservation,
                }),
            |program, c| program.contract(*c),
        )
        .build()
        .expect("every name resolves");
    verify(
        spec,
        VerifyLimits {
            max_states: 1_000,
            max_witness_tries: 1_024,
        },
    )
    .expect("the program verifies")
}

/// One line per position: the committed contracts and the effects, or the
/// refusal.
fn lines(program: &VerifiedProgram, results: &[StepResult]) -> Vec<String> {
    let spec = program.spec();
    results
        .iter()
        .map(|r| match r.refusals.last() {
            Some(refusal) => format!("{}: refused {}", r.position, refusal.code()),
            None => {
                let committed: Vec<&str> = r
                    .committed
                    .iter()
                    .map(|id| spec.contract(*id).unwrap().name)
                    .collect();
                let effects: Vec<String> = r
                    .effects
                    .iter()
                    .map(|e| spec.dimension(e.dimension).unwrap().describe(e.word))
                    .collect();
                format!(
                    "{}: {} {}",
                    r.position,
                    committed.join("+"),
                    effects.join(" ")
                )
                .trim_end()
                .to_owned()
            }
        })
        .collect()
}

#[test]
fn program_module_is_the_program_crate() {
    // The facade path and the crate path name the same items: this binding
    // only type-checks when the two `ContractSpec`s are one type.
    let through_facade: fn(&'static str) -> big_bad_bool_program::ContractSpec =
        big_bad_bool::program::contract;
    assert_eq!(through_facade("c").name, "c");
    assert_eq!(
        big_bad_bool::program::SPEC_FORMAT,
        big_bad_bool_program::SPEC_FORMAT
    );
}

#[test]
fn sequences_build_through_the_facade() {
    const EMPTY: SequenceSpec = sequence("empty", &[]);
    assert_eq!(EMPTY.name, "empty");
    assert!(EMPTY.items.is_empty());
}

/// Steps the four readings through `program` and returns every result and the observer.
fn run<O: Observer>(program: &VerifiedProgram, observer: O) -> (Vec<StepResult>, Stepper<'_, O>) {
    let reading = program.spec().dimension_id("reading").unwrap();
    let mut stepper = Stepper::new(program, observer);
    let mut results = Vec::new();
    for (position, level) in (0..).zip([COLD, MILD, HOT, HOT]) {
        results.extend(stepper.push(Observation::new(position).with(reading, level)));
    }
    results.extend(stepper.finish());
    (results, stepper)
}

#[test]
fn a_program_without_framing_builds_verifies_and_steps() {
    let program = thermostat();
    let spec = program.spec();
    let heater = spec.dimension_id("heater").unwrap();
    let (results, stepper) = run(&program, NoObserver);

    assert_eq!(
        lines(&program, &results),
        [
            "0: heat-when-cold {kind=StartHeat}",
            "1: hold",
            "2: stop-when-hot {kind=StopHeat}",
            // The heater is already off: stop-when-hot is refused and no
            // contract accepts the position.
            "3: refused BBBC0101",
        ]
    );
    assert_eq!(
        spec.dimension(heater)
            .unwrap()
            .describe(stepper.state().word(heater)),
        "{state=Off}"
    );
}

#[cfg(feature = "program-bowl")]
#[test]
fn program_bowl_turns_on_the_bowl_adapter() {
    use big_bad_bool::program::ob_serve_a_bowl::{
        ObservableWakeRuntime, ObserverConfig, WakeProfileCode,
    };
    use big_bad_bool::program::{BowlObserver, EventKind};

    let program = thermostat();
    let (plain, _) = run(&program, NoObserver);

    let mut runtime = ObservableWakeRuntime::new();
    runtime.register(ObserverConfig::new("trace", WakeProfileCode::InlineQueue));
    let (bowled, stepper) = run(&program, BowlObserver::new(runtime));
    assert_eq!(plain, bowled, "the adapter changes no result");

    let mut bowl = stepper.into_observer();
    let delivered = bowl.runtime_mut().drain_inline();
    assert_eq!(delivered.len(), bowl.published());
    // One Observed event per stepped position.
    let observed = delivered
        .iter()
        .filter(|e| e.key.phase() == EventKind::Observed as u64)
        .count();
    assert_eq!(observed, plain.len());
}
