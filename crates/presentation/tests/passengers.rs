//! A squad riding a carrier is off the field: its owner still reads it in
//! the publication, but no body of it is drawn, picked or heard. The battle
//! is the transport lab's: squads board an APC and a scout car before the
//! carriers drive off.
use contract::ids::Side;
use presentation::units::interpolate::TickInterpolator;
use sim::battle::Battle;
use sim::publication::reader::{Host, Reader};
use sim::publication::{self, Publisher};
use std::path::PathBuf;
use std::sync::Arc;

const MAP: &str = "transport";
const ENCOUNTER: &str = "board";
const SEED: u64 = 21;
/// Long enough for both squads to board, before the carriers drive.
const BOARDED_TICKS: u64 = 600;

#[test]
fn passengers_riding_a_carrier_have_no_body_to_draw() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules = presentation::static_world::files::rules(&root, MAP).unwrap();
    let scenario = sim::maps::scenario(MAP, ENCOUNTER, rules).unwrap();
    let mut battle = Battle::new(&scenario, SEED);
    for _ in 0..BOARDED_TICKS {
        battle.step();
    }
    let layout = publication::layout_json(&battle);
    let words: Vec<u32> = Publisher::new()
        .publish(&battle, Side::Blue)
        .unwrap()
        .iter()
        .map(|f| f.to_bits())
        .collect();
    let observation = Reader::new(&layout)
        .unwrap()
        .decode(&words)
        .unwrap()
        .unwrap();
    let riders: Vec<f64> = observation
        .own
        .iter()
        .filter(|u| {
            u.hold
                .as_ref()
                .is_some_and(|h| matches!(h.host, Host::Carrier { .. }) && &*h.phase == "inside")
        })
        .map(|u| u.id)
        .collect();
    assert!(
        !riders.is_empty(),
        "the encounter's squads are aboard by now"
    );

    let mut interpolator = TickInterpolator::new(battle.rules().tick_hz as f64);
    interpolator.push(Arc::new(observation.clone()), 0.0);
    let sample = interpolator.frame(observation.tick).unwrap();
    let drawn: Vec<f64> = sample.own.iter().map(|p| p.id).collect();
    for rider in &riders {
        assert!(
            !drawn.contains(rider),
            "rider {rider} still has a body: {drawn:?}"
        );
    }
    for u in observation.own.iter().filter(|u| !riders.contains(&u.id)) {
        assert!(drawn.contains(&u.id), "unit {} on the field is drawn", u.id);
    }
}
